//! Streaming LLM output (`llm_stream`, `llm_stream_cb`, SSE server replies) and the profiler.
//! Fake providers are Killer servers in the same script; no real network or key is used.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_sp_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

// ---------------------------------------------------------------- mock provider

#[test]
fn mock_stream_yields_chunks_lazily_and_break_stops() {
    let src = r#"
parts = []
for c in llm_stream({"provider": "mock", "reply": "the quick brown fox"}, "hi") {
  push(parts, c)
}
println(parts)
n = 0
for c in llm_stream({"provider": "mock", "reply": "a b c d e f"}, "hi") {
  n = n + 1
  if n == 3 {
    break
  }
}
println(n)
g = llm_stream({"provider": "mock", "chunks": ["x", "y", "z"]}, "q")
println(next(g), has_next(g), next(g))
gen_close(g)
println(next(g, "closed"))
println(next(llm_stream("mock", "hello there")))
"#;
    assert_eq!(run(src), "[the,  quick,  brown,  fox]\n3\nx true y\nclosed\nMock");
}

#[test]
fn mock_stream_callback_form_collects_and_can_stop() {
    let src = r#"
seen = []
fn on_chunk(c) {
  print("[" + c + "]")
}
full = llm_stream_cb({"provider": "mock", "reply": "one two three"}, "p", on_chunk)
println(full)
fn stop_at_two(c) {
  if c == " two" {
    return false
  }
}
println(llm_stream_cb({"provider": "mock", "reply": "one two three"}, "p", stop_at_two))
"#;
    assert_eq!(run(src), "[one]\n[ two]\n[ three]\none two three\none two");
}

#[test]
fn mock_stream_errors_are_catchable_and_arguments_are_validated() {
    let src = r#"
try {
  for c in llm_stream({"provider": "mock", "reply": "a b c", "fail_after": 2}, "p") {
    print(c)
  }
} catch e {
  println("caught", e)
}
try {
  llm_stream({"provider": "nope"}, "p")
} catch e {
  println(e)
}
try {
  llm_stream({"provider": "mock"}, 5)
} catch e {
  println(e)
}
"#;
    let out = run(src);
    assert!(out.starts_with("a\n b\ncaught"), "{}", out);
    assert!(out.contains("mock stream failed"), "{}", out);
    assert!(out.contains("provider nope is not supported"), "{}", out);
    assert!(out.contains("prompt must be a string"), "{}", out);
}

// ---------------------------------------------------------------- providers against fake servers

/// A Killer server speaking each provider's streaming dialect, and a client on an async thread.
fn provider_script(provider_opts: &str, server: &str) -> String {
    format!(
        r#"
port = free_port()
{server}
fn client(port) {{
  opts = {provider_opts}
  opts["base_url"] = "http://127.0.0.1:" + str(port) + "/v1"
  parts = []
  for c in llm_stream(opts, "hello") {{
    push(parts, c)
  }}
  return parts
}}
h = async_spawn(client, port)
serve(port, api, {{"max_requests": 1}})
println(async_await(h))
"#
    )
}

#[test]
fn openai_compatible_sse_stream_with_auth_and_keepalives() {
    let server = r#"
fn events(model) {
  yield {"event": "ping", "data": ""}
  for w in ["Hel", "lo", " wor", "ld"] {
    yield "{\"choices\":[{\"delta\":{\"content\":\"" + w + "\"}}]}"
  }
  yield "{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}"
  yield "[DONE]"
  yield "{\"choices\":[{\"delta\":{\"content\":\"AFTER DONE\"}}]}"
}
fn api(req) {
  body = json_parse(req["body"])
  if req["headers"]["authorization"] != "Bearer sk-test" or body["stream"] != true or req["path"] != "/v1/chat/completions" {
    return {"status": 401, "body": {"error": {"message": "bad request " + req["path"]}}}
  }
  if body["messages"][0]["content"] != "hello" {
    return {"status": 400, "body": "bad prompt"}
  }
  return {"stream": events(body["model"])}
}
"#;
    let out = run(&provider_script(r#"{"provider": "openai", "key": "sk-test", "model": "m"}"#, server));
    assert_eq!(out, "[Hel, lo,  wor, ld]");
}

#[test]
fn anthropic_sse_stream_uses_event_names_and_ignores_other_events() {
    let server = r#"
fn events() {
  yield {"event": "message_start", "data": "{\"type\":\"message_start\"}"}
  yield {"event": "content_block_start", "data": "{\"type\":\"content_block_start\"}"}
  yield {"event": "ping", "data": "{\"type\":\"ping\"}"}
  for w in ["Cl", "aude", " here"] {
    yield {"event": "content_block_delta", "data": {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": w}}}
  }
  yield {"event": "message_stop", "data": "{\"type\":\"message_stop\"}"}
}
fn api(req) {
  body = json_parse(req["body"])
  if req["headers"]["x-api-key"] != "ak" or req["headers"]["anthropic-version"] != "2023-06-01" or req["path"] != "/v1/messages" {
    return {"status": 401, "body": {"type": "error", "error": {"message": "auth"}}}
  }
  if body["system"] != "sys" or body["max_tokens"] != 1024 {
    return {"status": 400, "body": "bad body"}
  }
  return {"stream": events()}
}
"#;
    let out = run(&provider_script(r#"{"provider": "anthropic", "key": "ak", "model": "m", "system": "sys"}"#, server));
    assert_eq!(out, "[Cl, aude,  here]");
}

#[test]
fn ollama_ndjson_stream() {
    let server = r#"
fn api(req) {
  body = json_parse(req["body"])
  if req["path"] != "/v1/api/chat" or body["stream"] != true {
    return {"status": 404, "body": "nope"}
  }
  lines = []
  for w in ["Lla", "ma"] {
    push(lines, json_stringify({"message": {"role": "assistant", "content": w}, "done": false}))
  }
  push(lines, json_stringify({"message": {"role": "assistant", "content": ""}, "done": true}))
  return {"status": 200, "headers": {"Content-Type": "application/x-ndjson"}, "body": join(lines, "\n") + "\n"}
}
"#;
    let out = run(&provider_script(r#"{"provider": "ollama", "model": "m"}"#, server));
    assert_eq!(out, "[Lla, ma]");
}

#[test]
fn provider_http_errors_surface_as_catchable_errors() {
    let src = r#"
port = free_port()
fn api(req) {
  return {"status": 401, "body": {"error": {"message": "Incorrect API key"}}}
}
fn client(port) {
  try {
    for c in llm_stream({"provider": "compat", "base_url": "http://127.0.0.1:" + str(port) + "/v1"}, "x") {
      print(c)
    }
    return "no error"
  } catch e {
    return str(e)
  }
}
h = async_spawn(client, port)
serve(port, api, {"max_requests": 1})
println(async_await(h))
"#;
    let out = run(src);
    assert!(out.contains("Incorrect API key"), "{}", out);
}

#[test]
fn stream_arrives_incrementally_not_buffered_until_the_end() {
    // the server prints when it produces a piece, the client when it receives one: the two logs
    // must interleave, which they only do if the client reads while the server is still going
    let src = r#"
port = free_port()
fn pieces() {
  for w in ["a", "b", "c"] {
    println("S:" + w)
    yield "{\"choices\":[{\"delta\":{\"content\":\"" + w + "\"}}]}"
    sleep_ms(400)
  }
  yield "[DONE]"
}
fn api(req) {
  return {"stream": pieces()}
}
fn client(port) {
  for c in llm_stream({"provider": "compat", "base_url": "http://127.0.0.1:" + str(port) + "/v1"}, "x") {
    println("C:" + c)
  }
  return 0
}
h = async_spawn(client, port)
serve(port, api, {"max_requests": 1})
async_await(h)
"#;
    assert_eq!(run(src), "S:a\nC:a\nS:b\nC:b\nS:c\nC:c");
}

// ---------------------------------------------------------------- SSE server side

#[test]
fn server_streams_sse_to_a_raw_client_and_closes_generators_cleanly() {
    let src = r#"
port = free_port()
fn ticks() {
  for i in range(3) {
    yield {"event": "tick", "data": {"n": i}}
  }
  yield "multi\nline"
}
fn api(req) {
  if req["path"] == "/arr" {
    return {"stream": ["x", "y"], "headers": {"X-Test": "1"}}
  }
  return {"stream": ticks()}
}
fn client(port) {
  a = http_request({"url": "http://127.0.0.1:" + str(port) + "/gen", "retries": 6, "timeout": 10})
  b = http_request({"url": "http://127.0.0.1:" + str(port) + "/arr", "timeout": 10})
  return [a, b]
}
h = async_spawn(client, port)
serve(port, api, {"max_requests": 2})
r = async_await(h)
println(r[0]["status"], r[0]["headers"]["content-type"])
println(r[0]["body"])
println(r[1]["headers"]["x-test"], r[1]["body"])
"#;
    let out = run(src);
    assert!(out.starts_with("200 text/event-stream"), "{}", out);
    assert!(out.contains("event: tick\ndata: {\"n\":0}\n\nevent: tick\ndata: {\"n\":1}\n\nevent: tick\ndata: {\"n\":2}\n\ndata: multi\ndata: line\n\n"), "{}", out);
    assert!(out.contains("1 data: x\n\ndata: y"), "{}", out);
}

#[test]
fn generator_error_in_a_stream_becomes_an_error_event_and_server_survives() {
    let src = r#"
port = free_port()
fn bad() {
  yield "ok"
  throw "boom"
}
fn api(req) {
  if req["path"] == "/bad" {
    return {"stream": bad()}
  }
  return "alive"
}
fn client(port) {
  a = http_request({"url": "http://127.0.0.1:" + str(port) + "/bad", "retries": 6, "timeout": 10})
  b = http_request({"url": "http://127.0.0.1:" + str(port) + "/next", "timeout": 10})
  return [a["body"], b["body"]]
}
h = async_spawn(client, port)
serve(port, api, {"max_requests": 2})
r = async_await(h)
println(r[0])
println(r[1])
"#;
    let out = run(src);
    assert!(out.starts_with("data: ok\n\nevent: error\ndata: "), "{}", out);
    assert!(out.contains("boom"), "{}", out);
    assert!(out.ends_with("alive"), "{}", out);
}

// ---------------------------------------------------------------- profiler

/// Run a script and return stdout+stderr with the given extra arguments for killer_super.
fn run_super(src: &str, extra: &[&str]) -> String {
    let path = std::env::temp_dir().join(format!("killer_sp_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).args(extra).output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n")
}

const PROFILED: &str = r#"
fn heavy(n) {
  sleep_ms(25)
  return n
}
fn light(n) {
  return n + 1
}
fn driver() {
  for i in range(3) {
    heavy(i)
  }
  for i in range(5) {
    light(i)
  }
}
"#;

fn calls_of(rows_var: &str, name: &str) -> String {
    format!("for r in {rows_var} {{\n  if r[\"name\"] == \"{name}\" {{\n    println(\"{name}\", r[\"calls\"])\n  }}\n}}\n")
}

#[test]
fn profile_builtins_count_calls_and_order_by_self_time() {
    let src = format!(
        "{}profile_start()\ndriver()\nprofile_stop()\nrows = profile_report(\"functions\")\nprintln(rows[0][\"name\"], rows[0][\"calls\"])\n{}{}{}",
        PROFILED,
        calls_of("rows", "light"),
        calls_of("rows", "driver"),
        "println(rows[0][\"self_ms\"] >= 60, rows[0][\"total_ms\"] >= rows[0][\"self_ms\"])\n"
    );
    assert_eq!(run(&src), "heavy 3\nlight 5\ndriver 1\ntrue true");
}

#[test]
fn recursive_function_call_counts_are_exact() {
    let src = "fn fib(n) {\n  if n < 2 {\n    return n\n  }\n  return fib(n - 1) + fib(n - 2)\n}\nprofile_start()\nprintln(fib(10))\nprofile_stop()\nrows = profile_report(\"functions\")\nfor r in rows {\n  if r[\"name\"] == \"fib\" {\n    println(r[\"calls\"], r[\"total_ms\"] <= rows[0][\"total_ms\"] + 1000)\n  }\n}\n";
    assert_eq!(run(src), "55\n177 true");
}

#[test]
fn errors_unwinding_frames_do_not_corrupt_the_profile() {
    let src = r#"
fn bad() {
  throw "x"
}
fn wrap() {
  try {
    bad()
  } catch e {
    return 1
  }
}
fn after() {
  return 2
}
profile_start()
for i in range(3) {
  wrap()
}
after()
profile_stop()
rows = profile_report("functions")
for r in rows {
  println(r["name"], r["calls"])
}
"#;
    let out = run(src);
    let mut lines: Vec<&str> = out.lines().collect();
    lines.sort();
    assert_eq!(lines, vec!["<main> 1", "after 1", "bad 3", "wrap 3"]);
}

#[test]
fn line_hits_count_statement_executions() {
    let src = "profile_start()\ntotal = 0\nfor i in range(4) {\n  total = total + i\n}\nprofile_stop()\nfor r in profile_report(\"lines\") {\n  if r[\"line\"] == 4 {\n    println(\"body\", r[\"hits\"])\n  }\n  if r[\"line\"] == 2 {\n    println(\"init\", r[\"hits\"])\n  }\n}\n";
    let out = run(src);
    assert!(out.contains("body 4"), "{}", out);
    assert!(out.contains("init 1"), "{}", out);
}

#[test]
fn profile_report_text_and_html_formats_and_errors() {
    let src = format!(
        "{}profile_start()\ndriver()\nt = profile_report()\nh = profile_report(\"html\")\nprofile_stop()\nprintln(contains(t, \"self ms\"), contains(t, \"heavy\"), contains(t, \"hot lines\"))\nprintln(contains(h, \"<!DOCTYPE html>\"), contains(h, \"heavy\"), contains(h, \"https://\"))\ntry {{\n  profile_report(\"nope\")\n}} catch e {{\n  println(e)\n}}\n",
        PROFILED
    );
    let out = run(&src);
    assert!(out.starts_with("true true true\ntrue true false\n"), "{}", out);
    assert!(out.contains("unknown format"), "{}", out);
    let none = run("try {\n  profile_report()\n} catch e {\n  println(e)\n}\n");
    assert!(none.contains("call profile_start() first"), "{}", none);
}

#[test]
fn killer_profile_command_prints_a_table_sorted_by_self_time_and_writes_html() {
    let dir = std::env::temp_dir();
    let id = NEXT.fetch_add(1, Ordering::SeqCst);
    let script = dir.join(format!("killer_prof_{}_{}.killer", std::process::id(), id));
    let html = dir.join(format!("killer_prof_{}_{}.html", std::process::id(), id));
    std::fs::write(&script, format!("{}driver()\nprintln(\"done\")\n", PROFILED)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("profile").arg(&script).arg("--html").arg(&html).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    assert!(out.status.success(), "{}{}", text, String::from_utf8_lossy(&out.stderr));
    assert!(text.starts_with("done\n"), "{}", text);
    let heavy = text.find("\nheavy").expect(&text);
    let light = text.find("\nlight").expect(&text);
    let driver = text.find("\ndriver").expect(&text);
    assert!(heavy < light && heavy < driver, "{}", text);
    let heavy_row = text.lines().find(|l| l.starts_with("heavy")).unwrap();
    assert!(heavy_row.split_whitespace().nth(1) == Some("3"), "{}", heavy_row);
    let light_row = text.lines().find(|l| l.starts_with("light")).unwrap();
    assert!(light_row.split_whitespace().nth(1) == Some("5"), "{}", light_row);
    assert!(text.contains("hot lines"));
    let page = std::fs::read_to_string(&html).unwrap();
    let _ = std::fs::remove_file(&script);
    let _ = std::fs::remove_file(&html);
    assert!(page.starts_with("<!DOCTYPE html>"));
    assert!(page.contains("heavy") && page.contains("<script>") && page.contains("<style>"));
    assert!(!page.contains("http://") && !page.contains("https://") && !page.contains("src="), "must be self-contained");
    // help lists the command
    let help = Command::new(env!("CARGO_BIN_EXE_killer")).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("profile"));
}

#[test]
fn killer_super_profile_flag_and_normal_runs_are_unaffected() {
    let src = format!("{}driver()\nprintln(\"ok\")\n", PROFILED);
    let out = run_super(&src, &["--profile"]);
    assert!(out.starts_with("ok\n"), "{}", out);
    assert!(out.contains("heavy"), "{}", out);
    // without the flag there is no report
    let plain = run_super(&src, &["--run"]);
    assert_eq!(plain.trim(), "ok");
}

// ---------------------------------------------------------------- server robustness

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Stdio;
use std::time::{Duration, Instant};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// Start a script in the background (killer_super --run) and return the child.
fn spawn_script(src: &str) -> (std::process::Child, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("killer_sp_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&path)
        .arg("--run")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    (child, path)
}

/// Wait up to `secs` for the child; kills it and panics on a hang. Returns stdout+stderr.
fn finish(mut child: std::process::Child, path: std::path::PathBuf, secs: u64) -> String {
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(secs) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&path);
            panic!("script hung for more than {} s", secs);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = std::fs::remove_file(&path);
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    format!("{}{}", out, err).replace("\r\n", "\n").trim().to_string()
}

fn run_timeout(src: &str, secs: u64) -> String {
    let (c, p) = spawn_script(src);
    finish(c, p, secs)
}

fn connect_retry(port: u16) -> TcpStream {
    let start = Instant::now();
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => return s,
            Err(_) if start.elapsed() < Duration::from_secs(20) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => panic!("cannot connect: {}", e),
        }
    }
}

#[test]
fn idle_and_dribbling_clients_cannot_block_the_next_request() {
    let port = free_port();
    let src = format!(
        "fn h(req) {{\n  return \"pong\"\n}}\nn = serve({port}, h, {{\"max_requests\": 1, \"read_timeout\": 0.6}})\nprintln(\"served\", n)\n"
    );
    let (child, path) = spawn_script(&src);
    let idle = connect_retry(port); // sends nothing
    std::thread::sleep(Duration::from_millis(50));
    let dribble = std::thread::spawn(move || {
        let mut s = connect_retry(port);
        let mut cut_off = false;
        for b in b"GET / HTTP/1.1\r\nHost: x\r\nX-Slow: yes\r\n".iter() {
            if s.write_all(&[*b]).is_err() {
                cut_off = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(120));
        }
        // the server may reset the connection, so the 408 text is not guaranteed to be readable
        let mut reply = String::new();
        let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
        let read_failed = s.read_to_string(&mut reply).is_err();
        cut_off || read_failed || reply.contains("408")
    });
    std::thread::sleep(Duration::from_millis(100));
    let started = Instant::now();
    let mut good = connect_retry(port);
    good.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
    good.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
    let mut reply = String::new();
    let _ = good.read_to_string(&mut reply);
    assert!(reply.starts_with("HTTP/1.1 200 OK") && reply.ends_with("pong"), "{}", reply);
    assert!(started.elapsed() < Duration::from_secs(8), "took {:?}", started.elapsed());
    assert!(dribble.join().unwrap(), "the dribbling client was never cut off");
    drop(idle);
    assert_eq!(finish(child, path, 15), "served 1");
}

#[test]
fn server_stop_from_another_thread_ends_an_idle_server() {
    let port = free_port();
    let src = format!(
        "fn h(req) {{\n  return \"x\"\n}}\nfn stopper() {{\n  sleep_ms(400)\n  server_stop()\n  return 1\n}}\nt = async_spawn(stopper)\nn = serve({port}, h)\nprintln(\"stopped\", n)\nasync_await(t)\n"
    );
    assert_eq!(run_timeout(&src, 20), "stopped 0");
}

#[test]
fn server_stop_interrupts_a_connection_that_is_mid_request() {
    let port = free_port();
    let src = format!(
        "fn h(req) {{\n  return \"x\"\n}}\nfn stopper() {{\n  sleep_ms(500)\n  server_stop()\n  return 1\n}}\nt = async_spawn(stopper)\nn = serve({port}, h, {{\"read_timeout\": 600}})\nprintln(\"stopped\", n)\nasync_await(t)\n"
    );
    let (child, path) = spawn_script(&src);
    let mut half = connect_retry(port);
    half.write_all(b"GET / HTT").unwrap(); // never completed
    let out = finish(child, path, 20);
    drop(half);
    assert_eq!(out, "stopped 0");
}

#[test]
fn a_runaway_handler_gets_a_503_at_the_deadline_and_the_server_carries_on() {
    let port = free_port();
    let src = format!(
        r#"
port = {port}
fn handler(req) {{
  if req["path"] == "/slow" {{
    while true {{
      x = 1
    }}
  }}
  return "fast"
}}
fn client(port) {{
  base = "http://127.0.0.1:" + str(port)
  http_request({{"url": base + "/fast", "retries": 6, "timeout": 10}})
  a = http_request({{"url": base + "/slow", "timeout": 30}})
  b = http_request({{"url": base + "/fast", "timeout": 10}})
  return [a["status"], contains(a["body"], "deadline"), b["body"]]
}}
t = async_spawn(client, port)
serve(port, handler, {{"max_requests": 3, "request_timeout": 0.5}})
println(async_await(t))
"#
    );
    assert_eq!(run_timeout(&src, 40), "[503, true, fast]");
}

#[test]
fn a_slow_handler_cannot_hold_off_server_stop_beyond_its_deadline() {
    let port = free_port();
    let src = format!(
        r#"
port = {port}
fn handler(req) {{
  if req["path"] == "/ready" {{
    return "ok"
  }}
  while true {{
    x = 1
  }}
}}
fn client(port) {{
  base = "http://127.0.0.1:" + str(port)
  http_request({{"url": base + "/ready", "retries": 6, "timeout": 10}})
  return http_request({{"url": base + "/slow", "timeout": 30}})["status"]
}}
fn stopper() {{
  sleep_ms(1500)
  server_stop()
  return 1
}}
c = async_spawn(client, port)
s = async_spawn(stopper)
n = serve(port, handler, {{"request_timeout": 0.4}})
println("stopped", n, async_await(c))
async_await(s)
"#
    );
    let started = Instant::now();
    assert_eq!(run_timeout(&src, 40), "stopped 2 503");
    assert!(started.elapsed() < Duration::from_secs(30));
}
