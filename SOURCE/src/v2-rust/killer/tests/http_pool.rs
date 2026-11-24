//! `serve` with a bounded worker pool and a bounded queue: slow clients do not block fast ones, a full
//! queue answers 503 at once, handler globals are per worker, `server_stop` drains, `max_requests` counts
//! across workers, and replies stay correct under concurrency.
//!
//! Every test runs a server script in a child process and talks raw HTTP/1.1 to it from Rust threads.
//! Ports come from 20000..32000; timeouts are generous (debug builds on loaded CI runners).
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn free_port() -> u16 {
    static USED: std::sync::Mutex<Vec<u16>> = std::sync::Mutex::new(Vec::new());
    let mut used = USED.lock().unwrap();
    let start = 20000 + (std::process::id() as u16).wrapping_mul(53) % 11000;
    for step in 0..12000u16 {
        let port = 20000 + (start - 20000 + step) % 12000;
        if used.contains(&port) {
            continue;
        }
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            used.push(port);
            return port;
        }
    }
    panic!("no free port in 20000..32000")
}

struct Server {
    child: Option<Child>,
    path: std::path::PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

fn start(src: &str) -> Server {
    let path = std::env::temp_dir().join(format!("killer_pool_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_killer"))
        .arg("run")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    Server { child: Some(child), path }
}

impl Server {
    /// Wait up to `secs` for the script to exit; returns stdout+stderr (panics on a hang).
    fn finish(mut self, secs: u64) -> String {
        let mut child = self.child.take().unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(secs) {
                let _ = child.kill();
                let o = child.wait_with_output().unwrap();
                panic!("server script hung for more than {} s: {}{}", secs, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let o = child.wait_with_output().unwrap();
        format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).replace(&crlf(), "\n").trim().to_string()
    }
}

fn crlf() -> String {
    [13u8 as char, 10u8 as char].iter().collect()
}

/// One GET; returns (status, body). Status 0 means the connection failed or was closed without a reply.
fn get(port: u16, path: &str, timeout_s: u64) -> (u16, String) {
    let mut s = match TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(_) => return (0, String::new()),
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(timeout_s)));
    let nl = crlf();
    let req = format!("GET {} HTTP/1.1{nl}Host: localhost{nl}Connection: close{nl}{nl}", path);
    if s.write_all(req.as_bytes()).is_err() {
        return (0, String::new());
    }
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf).to_string();
    let status = text.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    let sep = format!("{nl}{nl}");
    let body = text.find(&sep).map(|i| text[i + 4..].to_string()).unwrap_or_default();
    (status, body)
}

/// Retry `GET /ping` until the server answers 200 (it is a debug build starting while the suite runs).
fn wait_ready(port: u16) {
    let started = Instant::now();
    loop {
        if get(port, "/ping", 30).0 == 200 {
            return;
        }
        assert!(started.elapsed() < Duration::from_secs(120), "server never came up on port {}", port);
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn twenty_slow_clients_do_not_block_a_fast_client() {
    let port = free_port();
    let srv = start(&format!(
        r#"fn h(req) {{
  if req["path"] == "/slow" {{
    sleep_ms(8000)
    return "slow"
  }}
  return "fast"
}}
n = serve({port}, h, {{"workers": 24, "max_requests": 22}})
println("served", n)
"#
    ));
    wait_ready(port);
    let slow: Vec<_> = (0..20).map(|_| std::thread::spawn(move || get(port, "/slow", 120))).collect();
    std::thread::sleep(Duration::from_millis(1000));
    let t = Instant::now();
    let fast = get(port, "/fast", 60);
    let took = t.elapsed();
    assert_eq!(fast, (200, "fast".to_string()));
    assert!(took < Duration::from_secs(5), "the fast request waited {:?} behind slow ones", took);
    for s in slow {
        assert_eq!(s.join().unwrap(), (200, "slow".to_string()));
    }
    assert_eq!(srv.finish(60), "served 22");
}

#[test]
fn a_full_queue_answers_503_at_once() {
    let port = free_port();
    let srv = start(&format!(
        r#"fn h(req) {{
  if req["path"] == "/slow" {{
    sleep_ms(3000)
    return "slow"
  }}
  return "ok"
}}
n = serve({port}, h, {{"workers": 1, "queue": 1, "max_requests": 3}})
println("served", n)
"#
    ));
    wait_ready(port);
    // A takes the only worker, B waits in the queue (size 1), C finds the queue full
    let a = std::thread::spawn(move || get(port, "/slow", 120));
    std::thread::sleep(Duration::from_millis(600));
    let b = std::thread::spawn(move || get(port, "/b", 120));
    std::thread::sleep(Duration::from_millis(600));
    let t = Instant::now();
    let c = get(port, "/c", 60);
    assert_eq!(c.0, 503, "queue full: {:?}", c);
    assert!(t.elapsed() < Duration::from_millis(2000), "the 503 took {:?}", t.elapsed());
    assert_eq!(a.join().unwrap(), (200, "slow".to_string()));
    assert_eq!(b.join().unwrap(), (200, "ok".to_string()));
    assert_eq!(srv.finish(60), "served 3");
}

#[test]
fn handler_globals_are_per_worker_and_mutex_cells_are_shared() {
    let port = free_port();
    let srv = start(&format!(
        r#"hits = []
total = mutex_new(0)
fn h(req) {{
  if req["path"] == "/ping" {{
    return "pong"
  }}
  push(hits, 1)
  mutex_add(total, 1)
  sleep_ms(1500)
  return str(len(hits))
}}
n = serve({port}, h, {{"workers": 2, "max_requests": 3}})
println("served", n, "main sees", len(hits), "mutex", mutex_get(total))
"#
    ));
    wait_ready(port);
    // the ping went to one worker; two concurrent requests occupy both workers, each with its own `hits`
    let a = std::thread::spawn(move || get(port, "/a", 120));
    let b = std::thread::spawn(move || get(port, "/b", 120));
    assert_eq!(a.join().unwrap(), (200, "1".to_string()));
    assert_eq!(b.join().unwrap(), (200, "1".to_string()));
    // the main script's copy is untouched; the mutex cell is shared by every worker
    assert_eq!(srv.finish(60), "served 3 main sees 0 mutex 2");
}

#[test]
fn server_stop_answers_at_once_and_drains_in_flight_requests() {
    let port = free_port();
    let srv = start(&format!(
        r#"fn h(req) {{
  if req["path"] == "/slow" {{
    sleep_ms(2500)
    return "slow done"
  }}
  if req["path"] == "/stop" {{
    server_stop()
    return "stopping"
  }}
  return "pong"
}}
n = serve({port}, h, {{"workers": 4}})
println("served", n)
"#
    ));
    wait_ready(port);
    let slow = std::thread::spawn(move || get(port, "/slow", 120));
    std::thread::sleep(Duration::from_millis(500));
    let t = Instant::now();
    assert_eq!(get(port, "/stop", 60), (200, "stopping".to_string()));
    assert!(t.elapsed() < Duration::from_millis(1500), "/stop waited {:?} behind /slow", t.elapsed());
    // the in-flight request still gets its full answer
    assert_eq!(slow.join().unwrap(), (200, "slow done".to_string()));
    assert_eq!(srv.finish(60), "served 3");
}

#[test]
fn server_stop_gives_up_on_a_stuck_handler_after_the_drain_timeout() {
    let port = free_port();
    let srv = start(&format!(
        r#"fn h(req) {{
  if req["path"] == "/stuck" {{
    sleep_ms(30000)
    return "late"
  }}
  if req["path"] == "/stop" {{
    server_stop()
    return "stopping"
  }}
  return "pong"
}}
n = serve({port}, h, {{"workers": 2, "drain_timeout": 1}})
println("served", n)
"#
    ));
    wait_ready(port);
    let _stuck = std::thread::spawn(move || get(port, "/stuck", 120));
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(get(port, "/stop", 60).0, 200);
    let t = Instant::now();
    // ping and stop completed; the stuck handler is abandoned after about 1 s
    assert_eq!(srv.finish(60), "served 2");
    assert!(t.elapsed() < Duration::from_secs(10), "serve waited {:?} for the stuck handler", t.elapsed());
}

#[test]
fn max_requests_counts_completed_requests_across_workers() {
    let port = free_port();
    let srv = start(&format!(
        r#"calls = mutex_new(0)
fn h(req) {{
  if req["path"] == "/ping" {{
    return "pong"
  }}
  mutex_add(calls, 1)
  sleep_ms(700)
  return "done"
}}
n = serve({port}, h, {{"workers": 4, "max_requests": 9}})
println("served", n, "handler calls", mutex_get(calls) + 1)
"#
    ));
    wait_ready(port);
    let t = Instant::now();
    let clients: Vec<_> = (0..14).map(|_| std::thread::spawn(move || get(port, "/work", 120))).collect();
    let results: Vec<(u16, String)> = clients.into_iter().map(|c| c.join().unwrap()).collect();
    let ok = results.iter().filter(|r| **r == (200, "done".to_string())).count();
    // exactly 8 more requests reach a handler (1 ping + 8 = 9); every other client is refused (503 or closed)
    assert_eq!(ok, 8, "{:?}", results);
    assert!(results.iter().all(|r| r.0 == 200 || r.0 == 503 || r.0 == 0), "{:?}", results);
    // 8 handlers of 0.7 s on 4 workers take about 1.4 s; one at a time would take 5.6 s
    assert!(t.elapsed() < Duration::from_millis(4500), "took {:?}", t.elapsed());
    assert_eq!(srv.finish(60), "served 9 handler calls 9");
}

#[test]
fn replies_are_correct_under_concurrency_repeated_20_times() {
    let port = free_port();
    let rounds = 20;
    let per_round = 12;
    let total = 1 + rounds * per_round;
    let srv = start(&format!(
        r#"cache = {{}}
fn fib(n) {{
  if n < 2 {{
    return n
  }}
  return fib(n - 1) + fib(n - 2)
}}
fn h(req) {{
  if req["path"] == "/ping" {{
    return "pong"
  }}
  k = int(req["query"]["k"])
  cache[str(k)] = k
  parts = split(req["path"], "/")
  return {{"k": k, "double": k * 2, "fib": fib(k % 15), "tag": parts[len(parts) - 1], "names": keys(req["query"])}}
}}
n = serve({port}, h, {{"workers": 8, "queue": 256, "max_requests": {total}}})
println("served", n)
"#
    ));
    wait_ready(port);
    fn fib(n: u64) -> u64 {
        if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
    }
    for round in 0..rounds {
        let clients: Vec<_> = (0..per_round)
            .map(|i| {
                let k = (round * per_round + i) as u64;
                std::thread::spawn(move || (k, get(port, &format!("/item/t{}?k={}", k, k), 120)))
            })
            .collect();
        for c in clients {
            let (k, (status, body)) = c.join().unwrap();
            assert_eq!(status, 200, "round {} k {}: {}", round, k, body);
            let want = format!("{{\"k\":{},\"double\":{},\"fib\":{},\"tag\":\"t{}\",\"names\":[\"k\"]}}", k, k * 2, fib(k % 15), k);
            assert_eq!(body, want, "round {} k {}", round, k);
        }
    }
    assert_eq!(srv.finish(60), format!("served {}", total));
}

#[test]
fn bad_pool_options_are_errors() {
    for (opts, msg) in [("{\"workers\": 0}", "workers"), ("{\"queue\": -1}", "queue"), ("{\"workers\": \"x\"}", "workers")] {
        let srv = start(&format!("serve(0, fn(r) {{ return \"x\" }}, {})\n", opts));
        let out = srv.finish(60);
        assert!(out.contains(msg), "{} -> {}", opts, out);
    }
}
