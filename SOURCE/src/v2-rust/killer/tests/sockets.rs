//! Real sockets: TCP, UDP and WebSocket builtins (`src/net_sock.rs`), all on 127.0.0.1 with OS-chosen ports.
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn script(src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("killer_sock_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
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

#[test]
fn tcp_listen_connect_accept_send_and_receive() {
    let (ok, out, err) = run(r#"l = tcp_listen("127.0.0.1:0")
c = tcp_connect(sock_addr(l))
s = tcp_accept(l)
sock_send(c, "hello\nworld")
println(sock_recv_line(s))
println(sock_recv(s, 100, 1000))
println(sock_recv(s, 100, 100))
sock_send(s, "pong")
println(sock_recv(c))
println(tcp_accept(l, 50))
sock_close(c)
println("[" + sock_recv(s) + "]")
"#);
    assert!(ok, "{err}");
    assert_eq!(out, "hello\nworld\nnull\npong\nnull\n[]");
}

#[test]
fn udp_send_and_receive_with_timeout() {
    let (ok, out, err) = run(r#"u = udp_bind("127.0.0.1:0")
v = udp_bind("127.0.0.1:0")
udp_send(u, sock_addr(v), "ping")
r = udp_recv(v, 100, 1000)
println(r["data"], r["from"] == sock_addr(u))
println(udp_recv(v, 100, 50))
"#);
    assert!(ok, "{err}");
    assert_eq!(out, "ping true\nnull");
}

#[test]
fn bad_handles_and_addresses_are_clear_errors() {
    for src in [
        "sock_send(99, \"x\")",
        "tcp_connect(\"not an address\")",
        "ws_connect(\"http://x\")",
        "ws_connect(\"wss://localhost/\")",
        "u = udp_bind(\"127.0.0.1:0\")\nsock_send(u, \"x\")",
    ] {
        let (ok, _, err) = run(src);
        assert!(!ok, "should fail: {src}");
        assert!(!err.is_empty());
    }
}

#[test]
fn websocket_echo_between_two_killer_processes() {
    let server_src = r#"l = tcp_listen("127.0.0.1:0")
println(sock_addr(l))
w = ws_accept(l, 20000)
m = ws_recv(w, 20000)
ws_send(w, "echo:" + m)
ws_send(w, repeat("y", 70000))
println(ws_recv(w, 20000))
"#;
    let file = script(server_src);
    let mut server = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap());
    let mut addr = String::new();
    lines.read_line(&mut addr).unwrap();
    let addr = addr.trim().to_string();
    assert!(addr.starts_with("127.0.0.1:"), "server printed {addr:?}");

    let (ok, out, err) = run(&format!(
        r#"c = ws_connect("ws://{addr}/chat")
ws_send(c, "hi")
println(ws_recv(c, 20000))
println(len(ws_recv(c, 20000)))
ws_close(c)
"#
    ));
    assert!(ok, "{err}");
    assert_eq!(out, "echo:hi\n70000");
    // the server sees the close as the end of the conversation (null)
    let mut last = String::new();
    lines.read_line(&mut last).unwrap();
    assert_eq!(last.trim(), "null");
    assert!(server.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

// ---------------------------------------------------------------------------------------------
// TLS (through the system `openssl`): skipped when the tool is missing

fn have_openssl() -> bool {
    Command::new("openssl").arg("version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

struct TlsServer {
    child: std::process::Child,
    port: u16,
    cert: String,
    dir: PathBuf,
}

impl Drop for TlsServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn tls_server() -> TlsServer {
    let dir = std::env::temp_dir().join(format!("killer_tls_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
    let made = Command::new("openssl")
        .env("MSYS_NO_PATHCONV", "1")
        .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2", "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost"])
        .arg("-keyout")
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(made.success(), "openssl could not make a test certificate");
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let child = Command::new("openssl")
        .args(["s_server", "-quiet", "-www", "-accept", &port.to_string()])
        .arg("-cert")
        .arg(&cert)
        .arg("-key")
        .arg(&key)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    TlsServer { child, port, cert: cert.to_string_lossy().replace('\\', "/"), dir }
}

#[test]
fn tls_connection_checks_the_certificate() {
    if !have_openssl() {
        eprintln!("skipped: openssl is not installed");
        return;
    }
    let s = tls_server();
    // trusted through ca_file: the request works
    let (ok, out, err) = run(&format!(
        "c = tcp_connect_tls(\"localhost:{}\", {{\"ca_file\": \"{}\"}})\nsock_send(c, \"GET / HTTP/1.0\\r\\n\\r\\n\")\nprintln(sock_recv_line(c, 10000))\nprintln(sock_peer(c))\nsock_close(c)\n",
        s.port, s.cert
    ));
    assert!(ok, "{err}");
    assert!(out.starts_with("HTTP/1.0 200"), "{out}");
    // not trusted: the connection is dropped and tls_error says why
    let (ok, out, err) = run(&format!(
        "c = tcp_connect_tls(\"localhost:{}\")\nsock_send(c, \"GET / HTTP/1.0\\r\\n\\r\\n\")\nprintln(\"[\" + str(sock_recv(c, 100, 10000)) + \"]\")\nprintln(contains(lower(tls_error(c)), \"verif\"))\n",
        s.port
    ));
    assert!(ok, "{err}");
    assert_eq!(out, "[]\ntrue");
}

#[test]
fn wss_to_an_untrusted_server_is_a_clear_error() {
    if !have_openssl() {
        eprintln!("skipped: openssl is not installed");
        return;
    }
    let s = tls_server();
    let (ok, _, err) = run(&format!("ws_connect(\"wss://localhost:{}/\")\n", s.port));
    assert!(!ok);
    assert!(err.contains("closed during the handshake") && err.contains("TLS"), "{err}");
}
