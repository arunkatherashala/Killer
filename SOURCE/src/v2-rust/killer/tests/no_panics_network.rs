//! Hostile HTTP requests against `serve` (loopback only, ports 20000-32000).  The server
//! process must survive every one of them, keep answering a normal request afterwards,
//! and never print a panic.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn pick_port() -> u16 {
    // spread by pid so parallel test binaries do not collide; verify it is free
    let mut p = 20000 + (std::process::id() % 11000) as u16;
    for _ in 0..200 {
        if TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return p;
        }
        p = if p >= 31990 { 20000 } else { p + 7 };
    }
    panic!("no free port in 20000-32000");
}

fn send(port: u16, bytes: &[u8], read_back: bool) -> Vec<u8> {
    let mut s = match TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(5)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(6)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(10)));
    // the server may close early on oversized input: ignore write errors
    let _ = s.write_all(bytes);
    let _ = s.flush();
    let mut out = Vec::new();
    if read_back {
        let _ = s.read_to_end(&mut out);
    }
    out
}

fn hostile_requests() -> Vec<(&'static str, Vec<u8>)> {
    let mut v: Vec<(&'static str, Vec<u8>)> = Vec::new();
    v.push(("empty_connection", Vec::new()));
    v.push(("garbage_bytes", (0u8..=255).collect()));
    v.push(("nul_bytes", vec![0u8; 4096]));
    v.push(("only_crlf", b"\r\n\r\n\r\n".to_vec()));
    v.push(("giant_request_line", {
        let mut b = b"GET /".to_vec();
        b.extend(std::iter::repeat(b'a').take(2_000_000));
        b.extend_from_slice(b" HTTP/1.1\r\n\r\n");
        b
    }));
    v.push(("no_terminator", b"GET / HTTP/1.1\r\nHost: x".to_vec()));
    v.push(("many_headers", {
        let mut b = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..100_000 {
            b.extend_from_slice(format!("X-{i}: v\r\n").as_bytes());
        }
        b.extend_from_slice(b"\r\n");
        b
    }));
    v.push(("giant_header_value", {
        let mut b = b"GET / HTTP/1.1\r\nX: ".to_vec();
        b.extend(std::iter::repeat(b'b').take(10_000_000));
        b.extend_from_slice(b"\r\n\r\n");
        b
    }));
    v.push(("content_length_overflow", b"POST / HTTP/1.1\r\nContent-Length: 99999999999999999999999\r\n\r\nabc".to_vec()));
    v.push(("content_length_negative", b"POST / HTTP/1.1\r\nContent-Length: -1\r\n\r\nabc".to_vec()));
    v.push(("content_length_huge_no_body", b"POST / HTTP/1.1\r\nContent-Length: 4000000000\r\n\r\n".to_vec()));
    v.push(("content_length_not_a_number", b"POST / HTTP/1.1\r\nContent-Length: abc\r\n\r\nxyz".to_vec()));
    v.push(("duplicate_content_length", b"POST / HTTP/1.1\r\nContent-Length: 3\r\nContent-Length: 99999\r\n\r\nabc".to_vec()));
    v.push(("chunked_huge_size", b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nFFFFFFFFFFFFFFFF\r\nabc\r\n0\r\n\r\n".to_vec()));
    v.push(("chunked_garbage", b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n\r\n".to_vec()));
    v.push(("percent_truncated", b"GET /% HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("percent_bad_hex", b"GET /%zz%4 HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("percent_invalid_utf8", b"GET /%ff%fe%c0%80 HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("path_traversal", b"GET /../../../../etc/passwd HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("raw_invalid_utf8_path", b"GET /\xff\xfe HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("nul_in_path", b"GET /a\0b HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("absolute_form", b"GET http://example.com/ HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("missing_path", b"GET  HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("missing_version", b"GET /\r\n\r\n".to_vec()));
    v.push(("weird_method", b"\xc3\xbf\xc3\xbf / HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("huge_method", {
        let mut b = vec![b'M'; 100_000];
        b.extend_from_slice(b" / HTTP/1.1\r\n\r\n");
        b
    }));
    v.push(("http_9_9", b"GET / HTTP/9.9\r\n\r\n".to_vec()));
    v.push(("invalid_utf8_header", b"GET / HTTP/1.1\r\nX: \xff\xfe\xfd\r\n\r\n".to_vec()));
    v.push(("header_without_colon", b"GET / HTTP/1.1\r\nnocolonhere\r\n\r\n".to_vec()));
    v.push(("header_empty_name", b"GET / HTTP/1.1\r\n: value\r\n\r\n".to_vec()));
    v.push(("bare_lf", b"GET / HTTP/1.1\nHost: x\n\n".to_vec()));
    v.push(("bare_cr", b"GET / HTTP/1.1\rHost: x\r\r".to_vec()));
    v.push(("expect_continue", b"POST / HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 5\r\n\r\nhello".to_vec()));
    v.push(("ws_upgrade_garbage", b"GET / HTTP/1.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: !!!\r\nSec-WebSocket-Version: 99\r\n\r\n".to_vec()));
    v.push(("query_garbage", b"GET /?a=%&b=%zz&=&&&a=1&a=2&%=%&==== HTTP/1.1\r\n\r\n".to_vec()));
    v.push(("query_many_params", {
        let mut b = b"GET /?".to_vec();
        for i in 0..200_000 {
            b.extend_from_slice(format!("p{i}=v&").as_bytes());
        }
        b.extend_from_slice(b" HTTP/1.1\r\n\r\n");
        b
    }));
    v.push(("pipelined_100", b"GET / HTTP/1.1\r\n\r\n".repeat(100)));
    v.push(("json_body_deep", {
        let body = format!("{}1{}", "[".repeat(200_000), "]".repeat(200_000));
        format!("POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }));
    v.push(("body_invalid_utf8", b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\n\xff\xfe\xfd\xfc".to_vec()));
    v.push(("half_request_then_close", b"POST / HTTP/1.1\r\nContent-Length: 100\r\n\r\nonly-part".to_vec()));
    v
}

#[test]
fn hostile_http_requests_do_not_kill_the_server() {
    let dir = std::env::temp_dir().join(format!("killer_net_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let port = pick_port();
    let script = dir.join("srv.killer");
    std::fs::write(
        &script,
        format!(
            "fn handler(req) {{\n  return {{\"status\": 200, \"body\": \"ok\"}}\n}}\nserve({port}, handler, {{\"max_requests\": 100000, \"read_timeout\": 2, \"idle_timeout\": 600}})\n"
        ),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer"))
        .arg(&script)
        .env("KILLER_ALLOW_PRIVATE_NET", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server");
    // drain pipes so the server never blocks on a full pipe
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let h1 = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = so.read_to_end(&mut b);
        b
    });
    let h2 = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = se.read_to_end(&mut b);
        b
    });

    // wait for the listener
    let t0 = Instant::now();
    loop {
        if TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(300)).is_ok() {
            break;
        }
        if let Some(st) = child.try_wait().unwrap() {
            panic!("server exited early: {st:?}");
        }
        assert!(t0.elapsed() < Duration::from_secs(60), "server did not start");
        std::thread::sleep(Duration::from_millis(100));
    }

    let mut dead_after: Option<&str> = None;
    for (name, bytes) in hostile_requests() {
        let _ = send(port, &bytes, true);
        if child.try_wait().unwrap().is_some() {
            dead_after = Some(name);
            break;
        }
    }

    // the server must still serve an honest request
    let mut alive = false;
    if dead_after.is_none() {
        for _ in 0..10 {
            let r = send(port, b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n", true);
            if String::from_utf8_lossy(&r).starts_with("HTTP/1.1 200") {
                alive = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let out = String::from_utf8_lossy(&h1.join().unwrap()).into_owned();
    let err = String::from_utf8_lossy(&h2.join().unwrap()).into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(dead_after.is_none(), "server died after hostile request '{}':\n{}{}", dead_after.unwrap_or(""), out, err);
    assert!(!err.contains("panicked at") && !out.contains("panicked at"), "server panicked:\n{err}\n{out}");
    assert!(alive, "server stopped answering normal requests:\n{err}");
}
