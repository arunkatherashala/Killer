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
