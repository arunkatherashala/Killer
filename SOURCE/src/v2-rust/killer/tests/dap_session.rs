//! A scripted Debug Adapter Protocol session against the real `killer_dap` binary over pipes.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Dap {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    seq: i64,
    log: Vec<String>,
}

impl Dap {
    fn start() -> Dap {
        let mut child = Command::new(env!("CARGO_BIN_EXE_killer_dap"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Dap { child, stdin, out, seq: 0, log: Vec::new() }
    }

    fn send(&mut self, command: &str, args: &str) -> i64 {
        self.seq += 1;
        let body = format!(r#"{{"seq":{},"type":"request","command":"{}","arguments":{}}}"#, self.seq, command, args);
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
        self.seq
    }

    fn read(&mut self) -> String {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            assert!(self.out.read_line(&mut line).unwrap() > 0, "adapter closed; log: {:#?}", self.log);
            let t = line.trim();
            if t.is_empty() {
                break;
            }
            if let Some(v) = t.strip_prefix("Content-Length:") {
                len = v.trim().parse().unwrap();
            }
        }
        let mut buf = vec![0u8; len];
        self.out.read_exact(&mut buf).unwrap();
        let m = String::from_utf8(buf).unwrap();
        self.log.push(m.clone());
        m
    }

    /// Read messages until one contains all of `needles`; returns it.
    fn until(&mut self, needles: &[&str]) -> String {
        for _ in 0..200 {
            let m = self.read();
            if needles.iter().all(|n| m.contains(n)) {
                return m;
            }
        }
        panic!("never saw {needles:?}; log: {:#?}", self.log);
    }

    fn request(&mut self, command: &str, args: &str) -> String {
        let id = self.send(command, args);
        self.until(&[&format!("\"request_seq\":{id}"), "\"type\":\"response\""])
    }
}

#[test]
fn breakpoint_step_variables_continue() {
    let dir = std::env::temp_dir().join(format!("killer_dap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let prog = dir.join("prog.killer");
    std::fs::write(&prog, "x = 1\nfn f(a) {\n  b = a + 1\n  return b\n}\ny = f(x)\nprint(\"finished\")\n").unwrap();
    let path = prog.display().to_string().replace('\\', "/");

    let mut d = Dap::start();
    let r = d.request("initialize", r#"{"adapterID":"killer"}"#);
    assert!(r.contains("supportsConfigurationDoneRequest"), "{r}");
    d.until(&["\"event\":\"initialized\""]);
    assert!(d.request("launch", &format!(r#"{{"program":"{path}"}}"#)).contains("\"success\":true"));
    let r = d.request("setBreakpoints", &format!(r#"{{"source":{{"path":"{path}"}},"breakpoints":[{{"line":3}}]}}"#));
    assert!(r.contains("\"verified\":true") && r.contains("\"line\":3"), "{r}");
    d.request("configurationDone", "{}");

    let stopped = d.until(&["\"event\":\"stopped\""]);
    assert!(stopped.contains("\"reason\":\"breakpoint\""), "{stopped}");

    let st = d.request("stackTrace", r#"{"threadId":1}"#);
    assert!(st.contains("\"line\":3") && st.contains("\"name\":\"f\"") && st.contains("prog.killer"), "{st}");
    let sc = d.request("scopes", r#"{"frameId":1}"#);
    assert!(sc.contains("Locals") && sc.contains("Globals"), "{sc}");
    let locals = d.request("variables", r#"{"variablesReference":1}"#);
    assert!(locals.contains("\"name\":\"a\"") && locals.contains("\"value\":\"1\""), "{locals}");
    let globals = d.request("variables", r#"{"variablesReference":2}"#);
    assert!(globals.contains("\"name\":\"x\""), "{globals}");

    // step to the next line: `b` now exists
    d.request("next", r#"{"threadId":1}"#);
    let stopped = d.until(&["\"event\":\"stopped\""]);
    assert!(stopped.contains("\"reason\":\"step\""), "{stopped}");
    let st = d.request("stackTrace", r#"{"threadId":1}"#);
    assert!(st.contains("\"line\":4"), "{st}");
    let locals = d.request("variables", r#"{"variablesReference":1}"#);
    assert!(locals.contains("\"name\":\"b\"") && locals.contains("\"value\":\"2\""), "{locals}");

    d.request("continue", r#"{"threadId":1}"#);
    let out = d.until(&["\"event\":\"output\"", "finished"]);
    assert!(out.contains("stdout"));
    d.until(&["\"event\":\"terminated\""]);
    d.request("disconnect", "{}");
    let _ = d.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stop_on_entry_and_step_over_a_call() {
    let dir = std::env::temp_dir().join(format!("killer_dap2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let prog = dir.join("p2.killer");
    std::fs::write(&prog, "fn g() {\n  return 5\n}\nv = g()\nw = v + 1\nprint(w)\n").unwrap();
    let path = prog.display().to_string().replace('\\', "/");
    let mut d = Dap::start();
    d.request("initialize", "{}");
    d.request("launch", &format!(r#"{{"program":"{path}","stopOnEntry":true}}"#));
    d.request("setBreakpoints", &format!(r#"{{"source":{{"path":"{path}"}},"breakpoints":[]}}"#));
    d.request("configurationDone", "{}");
    d.until(&["\"reason\":\"entry\""]);
    assert!(d.request("stackTrace", "{}").contains("\"line\":4"));
    // `next` over `v = g()` must land on line 5, not inside g
    d.request("next", "{}");
    d.until(&["\"event\":\"stopped\""]);
    assert!(d.request("stackTrace", "{}").contains("\"line\":5"));
    d.request("continue", "{}");
    d.until(&["\"event\":\"terminated\""]);
    d.request("disconnect", "{}");
    let _ = d.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Hang guard: a regression must fail the run, not block it forever on a pipe read.
fn watchdog(secs: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        eprintln!("dap_session watchdog: test still blocked after {secs} s");
        std::process::exit(101);
    });
}

fn temp_prog(tag: &str, text: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("killer_dap_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let prog = dir.join("prog.killer");
    std::fs::write(&prog, text).unwrap();
    let path = prog.display().to_string().replace('\\', "/");
    (prog, path)
}

#[test]
fn a_debuggee_that_dies_before_connecting_still_ends_the_session() {
    watchdog(120);
    // syntax error: the child exits with code 1 without ever connecting to the adapter
    let (_p, path) = temp_prog("dies", "fn f( {\n");
    let mut d = Dap::start();
    d.request("initialize", "{}");
    d.request("launch", &format!(r#"{{"program":"{path}"}}"#));
    d.request("configurationDone", "{}");
    let exited = d.until(&["\"event\":\"exited\""]);
    assert!(exited.contains("\"exitCode\":1"), "{exited}");
    d.until(&["\"event\":\"terminated\""]);
    // its error output is delivered before the exit
    let before_exit = d.log.iter().position(|m| m.contains("\"event\":\"output\"")).expect("an output event");
    let exit_pos = d.log.iter().position(|m| m.contains("\"event\":\"exited\"")).unwrap();
    assert!(before_exit < exit_pos, "{:#?}", d.log);
}

#[test]
fn set_breakpoints_for_another_file_does_not_replace_the_programs_breakpoints() {
    watchdog(120);
    let (_p, path) = temp_prog("bps", "x = 1\nfn f(a) {\n  b = a + 1\n  return b\n}\ny = f(x)\nprint(\"finished\")\n");
    let mut d = Dap::start();
    d.request("initialize", "{}");
    d.request("launch", &format!(r#"{{"program":"{path}"}}"#));
    let r = d.request("setBreakpoints", &format!(r#"{{"source":{{"path":"{path}"}},"breakpoints":[{{"line":3}}]}}"#));
    assert!(r.contains("\"verified\":true"), "{r}");
    // another file's breakpoints: reported unverified, and the program's own are kept
    let other = if cfg!(windows) { "C:/nowhere/other.killer" } else { "/nowhere/other.killer" };
    let r = d.request("setBreakpoints", &format!(r#"{{"source":{{"path":"{other}"}},"breakpoints":[{{"line":7}}]}}"#));
    assert!(r.contains("\"verified\":false"), "{r}");
    d.request("configurationDone", "{}");
    let stopped = d.until(&["\"event\":\"stopped\""]);
    assert!(stopped.contains("\"reason\":\"breakpoint\""), "{stopped}");
    let st = d.request("stackTrace", r#"{"threadId":1}"#);
    assert!(st.contains("\"line\":3"), "{st}");
    d.request("continue", r#"{"threadId":1}"#);
    d.until(&["\"event\":\"terminated\""]);
}

#[test]
fn oversized_and_non_utf8_frames_do_not_close_the_adapter() {
    watchdog(120);
    let mut d = Dap::start();
    // a frame over the cap: refused, body discarded, next request answered
    let big = 33 * 1024 * 1024usize;
    write!(d.stdin, "Content-Length: {}\r\n\r\n", big).unwrap();
    let chunk = vec![b' '; 1024 * 1024];
    for _ in 0..33 {
        d.stdin.write_all(&chunk).unwrap();
    }
    d.stdin.flush().unwrap();
    // a body with bytes that are not UTF-8
    let mut body = br#"{"seq":50,"type":"request","command":"threads","arguments":{"x":""#.to_vec();
    body.extend_from_slice(&[0xff, 0xfe]);
    body.extend_from_slice(br#""}}"#);
    write!(d.stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    d.stdin.write_all(&body).unwrap();
    d.stdin.flush().unwrap();
    let r = d.until(&["\"request_seq\":50"]);
    assert!(r.contains("\"success\":true"), "{r}");
    let r = d.request("threads", "{}");
    assert!(r.contains("\"main\""), "{r}");
}
