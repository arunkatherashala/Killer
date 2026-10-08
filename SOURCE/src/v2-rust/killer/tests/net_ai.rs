//! Network and AI building blocks: HTTP server/client, vector store, chunking, embeddings, logging.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run_with_env(src: &str, env: &[(&str, &str)]) -> String {
    let path = std::env::temp_dir().join(format!("killer_na_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer_super"));
    cmd.arg(&path).arg("--run");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

fn run(src: &str) -> String {
    run_with_env(src, &[])
}

// ---------------------------------------------------------------- HTTP server

/// The client runs on an async_spawn thread and retries until the server (started right after on
/// the main VM thread) is accepting connections.
fn client_prelude() -> &'static str {
    "fn fetch(opts) {\n  opts[\"retries\"] = 6\n  opts[\"timeout\"] = 10\n  return http_request(opts)\n}\n"
}

#[test]
fn server_handles_get_post_json_and_status_codes() {
    let src = format!("{}{}", client_prelude(), r#"
port = free_port()
base = "http://127.0.0.1:" + str(port)
fn handler(req) {
  if req["path"] == "/hello" {
    return "hi " + req["query"]["name"]
  }
  if req["path"] == "/echo" {
    return {"method": req["method"], "len": len(req["body"]), "ct": req["headers"]["content-type"]}
  }
  if req["path"] == "/made" {
    return {"status": 201, "headers": {"X-Test": "yes"}, "body": {"ok": true}}
  }
  return {"status": 404, "body": "nope"}
}
fn clients(base) {
  a = fetch({"url": base + "/hello?name=bob%20smith"})
  b = fetch({"method": "POST", "url": base + "/echo", "body": "{\"a\":1}", "headers": {"Content-Type": "application/json"}})
  c = fetch({"url": base + "/made"})
  d = fetch({"url": base + "/zzz"})
  return [a, b, c, d]
}
h = async_spawn(clients, base)
n = serve(port, handler, {"max_requests": 4})
r = async_await(h)
println(n)
println(r[0]["status"], r[0]["body"])
println(r[0]["headers"]["content-type"])
println(json_parse(r[1]["body"])["method"], json_parse(r[1]["body"])["len"], json_parse(r[1]["body"])["ct"])
println(r[1]["headers"]["content-type"])
println(r[2]["status"], r[2]["headers"]["x-test"], r[2]["body"])
println(r[3]["status"], r[3]["body"])
"#);
    assert_eq!(
        run(&src),
        "4\n200 hi bob smith\ntext/plain; charset=utf-8\nPOST 7 application/json\napplication/json\n201 yes {\"ok\":true}\n404 nope"
    );
}

#[test]
fn route_helper_with_params_and_verbs() {
    let src = format!("{}{}", client_prelude(), r#"
port = free_port()
base = "http://127.0.0.1:" + str(port)
fn show(req) {
  return {"id": req["params"]["id"], "kind": req["params"]["kind"]}
}
fn create(req) {
  d = json_parse(req["body"])
  return {"status": 201, "body": {"got": d["name"]}}
}
route("GET", "/items/:kind/:id", show)
route("POST", "/items", create)
fn clients(base) {
  a = fetch({"url": base + "/items/book/42"})
  b = fetch({"method": "POST", "url": base + "/items", "body": {"name": "pen"}})
  c = fetch({"url": base + "/items"})
  return [a, b, c]
}
h = async_spawn(clients, base)
serve(port, null, {"max_requests": 3})
r = async_await(h)
println(r[0]["body"])
println(r[1]["status"], r[1]["body"])
println(r[2]["status"])
routes_clear()
"#);
    assert_eq!(run(&src), "{\"id\":\"42\",\"kind\":\"book\"}\n201 {\"got\":\"pen\"}\n404");
}

#[test]
fn handler_errors_become_500_and_server_keeps_going_and_stops_gracefully() {
    let src = format!("{}{}", client_prelude(), r#"
port = free_port()
base = "http://127.0.0.1:" + str(port)
fn handler(req) {
  if req["path"] == "/boom" {
    throw "kaboom"
  }
  if req["path"] == "/stop" {
    server_stop()
    return "bye"
  }
  return "fine"
}
fn clients(base) {
  b = fetch({"url": base + "/ok"})
  a = http_request({"url": base + "/boom", "timeout": 10})
  c = fetch({"url": base + "/stop"})
  return [a, b, c]
}
h = async_spawn(clients, base)
n = serve(port, handler)
r = async_await(h)
println(n)
println(r[0]["status"], contains(r[0]["body"], "kaboom"))
println(r[1]["body"], r[2]["body"])
"#);
    assert_eq!(run(&src), "3\n500 true\nfine bye");
}

#[test]
fn serve_once_and_idle_timeout_and_client_headers() {
    let src = format!("{}{}", client_prelude(), r#"
port = free_port()
fn handler(req) {
  return "auth=" + req["headers"]["authorization"]
}
fn client(port) {
  return http_get("http://127.0.0.1:" + str(port) + "/", {"Authorization": "Bearer abc"}, 10)
}
h = async_spawn(client, port)
println(serve_once(port, handler))
println(async_await(h))
p2 = free_port()
println(serve(p2, handler, {"idle_timeout": 0.2}))
"#);
    assert_eq!(run(&src), "1\nauth=Bearer abc\n0");
}

#[test]
fn http_client_policy_errors() {
    assert!(run("println(http_get(\"http://example.com/\"))").contains("only allowed for localhost"));
    assert!(run("println(http_get(\"ftp://example.com/\"))").contains("Invalid URL"));
    assert!(run("println(http_get(\"https://x.com/\", {\"A\": \"b\" + chr(13) + chr(10) + \"Evil: 1\"}))").contains("invalid header"));
}

#[test]
fn serve_without_handler_or_routes_is_an_error() {
    let out = run("serve(free_port(), null)");
    assert!(out.contains("pass a handler function"), "{}", out);
}
