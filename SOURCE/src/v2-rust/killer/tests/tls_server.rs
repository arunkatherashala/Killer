//! TLS listeners (`tls_listen` / `tls_accept`, `wss://` server, `serve` with `tls`), through the system `openssl`.
//! Every test is skipped when the `openssl` tool is missing.
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn have_openssl() -> bool {
    Command::new("openssl").arg("version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A temp dir holding a self-signed certificate for localhost, or None when openssl is missing.
fn creds() -> Option<(Dir, String, String)> {
    if !have_openssl() {
        eprintln!("openssl not found: skipping");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("killer_tlss_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
    let made = Command::new("openssl")
        .env("MSYS_NO_PATHCONV", "1")
        .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2", "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost"])
        .arg("-keyout")
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .output()
        .unwrap();
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    // forward slashes work in Killer strings and on every platform
    let p = |f: PathBuf| f.to_string_lossy().replace('\\', "/");
    Some((Dir(dir), p(cert), p(key)))
}

fn script(src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("killer_tlss_s_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    file
}

fn run(src: &str) -> (bool, String, String) {
    let file = script(src);
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    let s = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.success(), s(&out.stdout).trim_end().to_string(), s(&out.stderr))
}

fn have_curl() -> bool {
    Command::new("curl").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

/// Start a Killer script and return it with the first line it prints (its listening address).
fn start(src: &str) -> (std::process::Child, BufReader<std::process::ChildStdout>, String, PathBuf) {
    let file = script(src);
    let mut server = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap());
    let mut addr = String::new();
    lines.read_line(&mut addr).unwrap();
    (server, lines, addr.trim().to_string(), file)
}

/// `curl -k` (trust the self-signed certificate) against `https://addr/path`: (success, body)
fn curl(addr: &str, path: &str, insecure: bool) -> (bool, Vec<u8>) {
    let mut c = Command::new("curl");
    c.args(["-sS", "--max-time", "90"]);
    if insecure {
        c.arg("-k");
    }
    let o = c.arg(format!("https://{addr}{path}")).output().unwrap();
    (o.status.success(), o.stdout)
}

const REPLY: &str = r#"fn serve_one(s, body) {
    line = sock_recv_line(s, 60000)
    while true {
        h = sock_recv_line(s, 60000)
        if h == null || h == "" || h == "\r" { break }
    }
    sock_send(s, "HTTP/1.1 200 OK\r\nContent-Length: " + str(len(body)) + "\r\nConnection: close\r\n\r\n" + body)
    return line
}
"#;

#[test]
fn tls_listener_accepts_several_sequential_clients() {
    let Some((_d, cert, key)) = creds() else { return };
    if !have_curl() {
        eprintln!("curl not found: skipping");
        return;
    }
    let (mut server, mut lines, addr, file) = start(&format!(
        r#"{REPLY}l = tls_listen("127.0.0.1:0", "{cert}", "{key}")
println(sock_addr(l))
for i in range(3) {{
    s = tls_accept(l, 120000)
    println(serve_one(s, "reply " + str(i)))
    sock_close(s)
}}
println(tls_accept(l, 50))
sock_close(l)
"#
    ));
    assert!(addr.starts_with("127.0.0.1:"), "server printed {addr:?}");
    for i in 0..3 {
        let (ok, body) = curl(&addr, &format!("/p{i}"), true);
        assert!(ok);
        assert_eq!(String::from_utf8_lossy(&body), format!("reply {i}"));
        let mut l = String::new();
        lines.read_line(&mut l).unwrap();
        assert_eq!(l.trim(), format!("GET /p{i} HTTP/1.1"));
    }
    let mut last = String::new();
    lines.read_line(&mut last).unwrap();
    assert_eq!(last.trim(), "null");
    let st = server.wait().unwrap();
    assert!(st.success());
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn tls_server_sends_a_large_reply_and_survives_a_close_right_after_send() {
    let Some((_d, cert, key)) = creds() else { return };
    if !have_curl() {
        return;
    }
    let (mut server, _lines, addr, file) = start(&format!(
        r#"{REPLY}l = tls_listen("127.0.0.1:0", "{cert}", "{key}")
println(sock_addr(l))
s = tls_accept(l, 120000)
serve_one(s, repeat("z", 300000))
sock_close(s)
"#
    ));
    let (ok, body) = curl(&addr, "/big", true);
    assert!(ok);
    assert_eq!(body.len(), 300000);
    assert!(body.iter().all(|b| *b == b'z'));
    assert!(server.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn an_untrusting_client_is_refused_and_the_server_keeps_working() {
    let Some((_d, cert, key)) = creds() else { return };
    if !have_curl() {
        return;
    }
    let (mut server, mut lines, addr, file) = start(&format!(
        r#"{REPLY}l = tls_listen("127.0.0.1:0", "{cert}", "{key}")
println(sock_addr(l))
bad = tls_accept(l, 120000)
r = sock_recv_line(bad, 30000)
println("bad:", r == null || r == "")
sock_close(bad)
good = tls_accept(l, 120000)
println(serve_one(good, "ok"))
sock_close(good)
"#
    ));
    // curl without -k refuses the self-signed certificate
    let (ok, _) = curl(&addr, "/", false);
    assert!(!ok, "curl should reject the self-signed certificate");
    let (ok, body) = curl(&addr, "/x", true);
    assert!(ok);
    assert_eq!(body, b"ok");
    let mut out = String::new();
    std::io::Read::read_to_string(&mut lines, &mut out).unwrap();
    let out = out.replace('\r', "");
    assert!(server.wait().unwrap().success());
    assert!(out.contains("bad: true") && out.contains("GET /x HTTP/1.1"), "{out}");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn secure_websocket_server_and_client() {
    let Some((_d, cert, key)) = creds() else { return };
    let server_src = format!(
        r#"l = tls_listen("127.0.0.1:0", "{cert}", "{key}")
println(sock_addr(l))
w = ws_accept(l, 30000)
m = ws_recv(w, 20000)
ws_send(w, "echo:" + m)
println(ws_recv(w, 20000))
"#
    );
    let file = script(&server_src);
    let mut server = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap());
    let mut addr = String::new();
    lines.read_line(&mut addr).unwrap();
    let port = addr.trim().rsplit(':').next().unwrap().to_string();
    let (ok, out, err) = run(&format!(
        r#"c = ws_connect("wss://localhost:{port}/chat", {{"ca_file": "{cert}"}})
ws_send(c, "hi")
println(ws_recv(c, 20000))
ws_close(c)
"#
    ));
    assert!(ok, "{err}");
    assert_eq!(out, "echo:hi");
    let mut last = String::new();
    lines.read_line(&mut last).unwrap();
    assert_eq!(last.trim(), "null");
    assert!(server.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn serve_over_https_handles_several_requests() {
    let Some((_d, cert, key)) = creds() else { return };
    if !have_curl() {
        return;
    }
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let server_src = format!(
        r#"fn h(req) {{
    return {{"status": 200, "headers": {{"X-Seen": req["path"]}}, "body": "hello " + req["method"] + " " + req["path"]}}
}}
n = serve({port}, h, {{"max_requests": 3, "tls": {{"cert": "{cert}", "key": "{key}"}}}})
println("served", n)
"#
    );
    let file = script(&server_src);
    let server = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    for p in ["/a", "/b", "/c"] {
        let mut got = None;
        for _ in 0..100 {
            let o = Command::new("curl").args(["-sS", "-k", "-i", "--max-time", "90"]).arg(format!("https://127.0.0.1:{port}{p}")).output().unwrap();
            if o.status.success() {
                got = Some(String::from_utf8_lossy(&o.stdout).to_string());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200)); // server not up yet
        }
        let r = got.expect("no answer over https");
        assert!(r.contains("200 OK"), "{r}");
        assert!(r.contains(&format!("hello GET {p}")), "{r}");
        assert!(r.to_lowercase().contains(&format!("x-seen: {p}")), "{r}");
    }
    let o = server.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "served 3");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn clear_errors_for_missing_or_mismatched_credentials() {
    let Some((d, cert, key)) = creds() else { return };
    let other = creds().unwrap();
    for (src, needle) in [
        (format!("tls_listen(\"127.0.0.1:0\", \"{cert}.missing\", \"{key}\")"), "certificate file"),
        (format!("tls_listen(\"127.0.0.1:0\", \"{cert}\", \"{key}.missing\")"), "private key file"),
        (format!("tls_listen(\"127.0.0.1:0\", \"{key}\", \"{key}\")"), "not a valid PEM certificate"),
        (format!("tls_listen(\"127.0.0.1:0\", \"{cert}\", \"{}\")", other.2), "does not match"),
        (format!("l = tcp_listen(\"127.0.0.1:0\")\ntls_accept(l, 10)"), "not a TLS listener"),
        (format!("serve(0, fn(r) {{ return \"x\" }}, {{\"tls\": {{\"cert\": \"{cert}\"}}}})"), "tls must be a dict"),
        (format!("serve(0, fn(r) {{ return \"x\" }}, {{\"tls\": {{\"cert\": \"{cert}.nope\", \"key\": \"{key}\"}}}})"), "certificate file"),
    ] {
        let (ok, _, err) = run(&src);
        assert!(!ok, "should fail: {src}");
        assert!(err.contains(needle), "wanted {needle:?} in {err:?}");
    }
    drop(d);
}

#[test]
fn a_missing_openssl_is_reported() {
    let Some((_d, cert, key)) = creds() else { return };
    let file = script(&format!("tls_listen(\"127.0.0.1:0\", \"{cert}\", \"{key}\")"));
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).env("PATH", "").output().unwrap();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("openssl"), "{err}");
}
