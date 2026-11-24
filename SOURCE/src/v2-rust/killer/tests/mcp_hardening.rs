//! killer-mcp robustness: oversized / malformed / deeply nested requests, tool panics, run budget, optional token.
//! Every test spawns the real binary and talks framed JSON-RPC over stdio.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Mcp {
    child: Child,
    rx: mpsc::Receiver<String>,
}

fn spawn(envs: &[(&str, &str)]) -> Mcp {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer-mcp"));
    cmd.env_remove("KILLER_MCP_TOKEN").env_remove("KILLER_MCP_RUN_TIMEOUT_S").env_remove("KILLER_NATIVE");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut r = BufReader::new(out);
        loop {
            let mut len = None;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                let t = line.trim();
                if t.is_empty() {
                    break;
                }
                if let Some(v) = t.strip_prefix("Content-Length:") {
                    len = v.trim().parse::<usize>().ok();
                }
            }
            let mut buf = vec![0u8; len.unwrap_or(0)];
            if r.read_exact(&mut buf).is_err() {
                return;
            }
            if tx.send(String::from_utf8_lossy(&buf).into_owned()).is_err() {
                return;
            }
        }
    });
    Mcp { child, rx }
}

impl Mcp {
    fn send_raw(&mut self, bytes: &[u8]) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    }
    fn send(&mut self, body: &str) {
        let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        self.send_raw(framed.as_bytes());
    }
    fn recv(&self, secs: u64) -> String {
        self.rx.recv_timeout(Duration::from_secs(secs)).expect("no reply from killer-mcp in time")
    }
    fn ping_ok(&mut self, id: u32) {
        self.send(&format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#));
        let r = self.recv(20);
        assert!(r.contains("\"result\"") && r.contains(&format!("\"id\":{id}")), "{r}");
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn huge_content_length_is_refused_without_allocating_and_server_survives() {
    let mut m = spawn(&[]);
    // header only claims 100 GB; the old code did vec![0; n] (allocation failure / abort)
    m.send_raw(b"Content-Length: 100000000000\r\n\r\n");
    let r = m.recv(20);
    assert!(r.contains("-32600") && r.contains("too large"), "{r}");
    // close stdin: the server must exit by itself, cleanly
    drop(m.child.stdin.take());
    let status = m.child.wait().unwrap();
    assert!(status.success(), "{status:?}");
}

#[test]
fn body_just_over_the_cap_is_drained_and_the_next_request_still_works() {
    let mut m = spawn(&[]);
    let big = 9 * 1024 * 1024;
    m.send_raw(format!("Content-Length: {}\r\n\r\n", big).as_bytes());
    let chunk = vec![b' '; 1024 * 1024];
    for _ in 0..9 {
        m.send_raw(&chunk);
    }
    let r = m.recv(30);
    assert!(r.contains("too large"), "{r}");
    m.ping_ok(7);
}

#[test]
fn deeply_nested_json_is_rejected_not_a_stack_overflow() {
    let mut m = spawn(&[]);
    let deep = format!("{}1{}", "[".repeat(200_000), "]".repeat(200_000));
    m.send(&deep);
    let r = m.recv(30);
    assert!(r.contains("-32700") && r.contains("nested deeper"), "{r}");
    // 64 levels is fine, 65 is not
    let ok = format!(r#"{{"jsonrpc":"2.0","id":3,"method":"ping","params":{}1{}}}"#, "[".repeat(60), "]".repeat(60));
    m.send(&ok);
    let r = m.recv(20);
    assert!(r.contains("\"result\""), "{r}");
    let too = format!(r#"{{"jsonrpc":"2.0","id":4,"method":"ping","params":{}1{}}}"#, "[".repeat(70), "]".repeat(70));
    m.send(&too);
    assert!(m.recv(20).contains("-32700"));
    m.ping_ok(2);
}

#[test]
fn malformed_json_and_bad_headers_get_errors_and_the_server_keeps_running() {
    let mut m = spawn(&[]);
    m.send("{not json");
    assert!(m.recv(20).contains("-32700"));
    m.send_raw(b"Content-Length: abc\r\n\r\n");
    assert!(m.recv(20).contains("-32700"));
    // invalid UTF-8 inside a body is replaced, not fatal
    let mut body = br#"{"jsonrpc":"2.0","id":5,"method":"ping","x":""#.to_vec();
    body.extend_from_slice(&[0xff, 0xfe]);
    body.extend_from_slice(br#""}"#);
    m.send_raw(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    m.send_raw(&body);
    assert!(m.recv(20).contains("\"id\":5"));
    m.ping_ok(6);
}

#[test]
fn a_pathological_compile_does_not_take_the_server_down() {
    let mut m = spawn(&[]);
    let src = "(".repeat(100_000);
    let params = format!(r#"{{"name":"killer_compile","arguments":{{"source":"{}"}}}}"#, src);
    m.send(&format!(r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}}"#, params));
    let r = m.recv(120);
    assert!(r.contains("\"id\":1"), "{}", &r[..r.len().min(300)]);
    m.ping_ok(2);
}

#[test]
fn token_is_enforced_only_when_configured() {
    // no token configured: open
    let mut open = spawn(&[]);
    open.ping_ok(1);
    drop(open);

    let mut m = spawn(&[("KILLER_MCP_TOKEN", "s3cret-token")]);
    m.send(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
    let r = m.recv(20);
    assert!(r.contains("-32001") && r.contains("unauthorized"), "{r}");
    m.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"token":"wrong"}}}"#);
    assert!(m.recv(20).contains("-32001"));
    m.send(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{"_meta":{"token":"s3cret-token"}}}"#);
    let r = m.recv(20);
    assert!(r.contains("killer_run") && !r.contains("-32001"), "{r}");
    m.send(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"killer_version","arguments":{}}}"#);
    assert!(m.recv(20).contains("-32001"));
}

#[test]
fn killer_run_is_killed_when_it_exceeds_its_budget() {
    let native = env!("CARGO_BIN_EXE_killer");
    let mut m = spawn(&[("KILLER_NATIVE", native), ("KILLER_MCP_RUN_TIMEOUT_S", "2")]);
    // a normal program still works
    m.send(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"killer_run","arguments":{"source":"println(6 * 7)"}}}"#);
    let r = m.recv(60);
    assert!(r.contains("42"), "{r}");
    // an endless loop is killed
    let start = std::time::Instant::now();
    m.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"killer_run","arguments":{"source":"while true {\n}\n"}}}"#);
    let r = m.recv(60);
    assert!(r.contains("budget") && r.contains("\"isError\":true"), "{r}");
    assert!(start.elapsed() < Duration::from_secs(40));
    m.ping_ok(3);
}
