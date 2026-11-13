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
