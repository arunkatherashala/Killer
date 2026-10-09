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

// ---------------------------------------------------------------- vector store and math

#[test]
fn vector_math_builtins() {
    let out = run("println(dot([1, 2, 3], [4, 5, 6]))\nprintln(norm([3, 4]))\nprintln(cosine([1, 0], [1, 0]))\nprintln(cosine([1, 0], [0, 1]))\nprintln(top_k([0.1, 0.9, 0.5, 0.7], 2))\ntry {\n  dot([1], [1, 2])\n} catch e {\n  println(\"mismatch\")\n}\n");
    assert_eq!(out, "32\n5\n1\n0\n[1, 3]\nmismatch");
}

#[test]
fn vector_store_add_search_filter_delete_save_load() {
    let path = std::env::temp_dir().join(format!("killer_na_store_{}.kvec", std::process::id()));
    let p = path.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let src = format!(
        r#"
s = vec_new(3)
vec_add(s, "a", [1, 0, 0], {{"lang": "en", "n": 1}})
vec_add(s, "b", [0.9, 0.1, 0], {{"lang": "de", "n": 2}})
vec_add(s, "c", [0, 1, 0], {{"lang": "en", "n": 3}})
println(vec_len(s))
hits = vec_search(s, [1, 0, 0], 2)
println(hits[0]["id"], hits[1]["id"], hits[0]["meta"]["n"])
only_de = vec_search(s, [1, 0, 0], 5, {{"lang": "de"}})
println(len(only_de), only_de[0]["id"])
vec_add(s, "a", [0, 0, 1])
println(vec_len(s))
println(vec_delete(s, "b"), vec_delete(s, "b"), vec_len(s))
vec_save(s, "{p}")
t = vec_load("{p}")
println(vec_len(t))
println(vec_search(t, [0, 1, 0], 1)[0]["id"])
println(vec_search(t, [0, 0, 1], 1)[0]["id"])
try {{
  vec_add(t, "bad", [1, 2])
}} catch e {{
  println("dim error")
}}
println(vec_free(s))
try {{
  vec_len(s)
}} catch e {{
  println("freed")
}}
"#
    );
    let out = run(&src);
    let _ = std::fs::remove_file(&path);
    assert_eq!(out, "3\na b 1\n1 b\n3\ntrue false 2\n2\nc\na\ndim error\ntrue\nfreed");
}

#[test]
fn vec_load_rejects_garbage_files() {
    let path = std::env::temp_dir().join(format!("killer_na_garbage_{}.kvec", std::process::id()));
    std::fs::write(&path, b"KVEC1\n\x03\x00\x00\x00\xff\xff\xff\xff\xff\xff\xff\xffjunk").unwrap();
    let p = path.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let out = run(&format!("try {{\n  vec_load(\"{}\")\n}} catch e {{\n  println(\"rejected\")\n}}\n", p));
    let _ = std::fs::remove_file(&path);
    assert_eq!(out, "rejected");
}

// ---------------------------------------------------------------- chunking, tokens, prompts

#[test]
fn chunking_tokens_and_prompt() {
    let src = r#"
chunks = chunk_text("the quick brown fox jumps over the lazy dog", 16, 0)
println(chunks)
println(len(chunk_text("abcdefghijklmnopqrstuvwxyz", 10, 3)))
println(chunk_by_sentence("One. Two! Three? Four.", 11))
println(count_tokens("Hello, world!"))
println(count_tokens(""))
p = build_rag_prompt("What is Killer?", ["Killer is a language.", {"text": "It has a VM."}])
println(p)
try {
  chunk_text("abc", 5, 5)
} catch e {
  println("bad overlap")
}
"#;
    assert_eq!(
        run(src),
        "[the quick brown, fox jumps over, the lazy dog]\n4\n[One. Two!, Three?, Four.]\n6\n0\nAnswer the question using only the context below. If the context does not contain the answer, say you do not know.\n\nContext:\n[1] Killer is a language.\n[2] It has a VM.\n\nQuestion: What is Killer?\nAnswer:\nbad overlap"
    );
}

// ---------------------------------------------------------------- embeddings

#[test]
fn mock_embed_is_deterministic_and_retrieval_works() {
    let src = r#"
a = mock_embed("reset your password in settings", 128)
b = mock_embed("reset your password in settings", 128)
println(len(a), a == b)
println(round(norm(a)))
q = mock_embed("how do I reset my password", 128)
near = cosine(q, a)
far = cosine(q, mock_embed("lunch is served at noon", 128))
println(near > far)
"#;
    assert_eq!(run(src), "128 true\n1\ntrue");
}

#[test]
fn llm_embed_batch_against_a_local_openai_compatible_server_with_key_from_env() {
    // a Killer server plays the embedding endpoint; the key must arrive as a Bearer header read
    // from the environment variable named by key_env
    let src = r#"
port = free_port()
fn api(req) {
  body = json_parse(req["body"])
  out = []
  i = 0
  for t in body["input"] {
    push(out, {"index": i, "embedding": [len(t), i, 1]})
    i = i + 1
  }
  if req["headers"]["authorization"] != "Bearer sk-from-env" {
    return {"status": 401, "body": {"error": {"message": "bad key"}}}
  }
  return {"data": out}
}
fn client(port) {
  opts = {"provider": "openai_compatible", "base_url": "http://127.0.0.1:" + str(port) + "/v1", "model": "m", "key_env": "MY_EMBED_KEY", "retries": 5, "timeout": 10}
  return llm_embed_batch(opts, ["a", "bcd"])
}
h = async_spawn(client, port)
serve(port, api, {"max_requests": 1})
println(async_await(h))
"#;
    assert_eq!(run_with_env(src, &[("MY_EMBED_KEY", "sk-from-env")]), "[[1, 0, 1], [3, 1, 1]]");
}

#[test]
fn llm_embed_opts_form_and_mock_provider_and_missing_key_env() {
    let out = run("v = llm_embed({\"provider\": \"mock\", \"dim\": 16}, \"hello world\")\nprintln(len(v))\nr = llm_embed_batch({\"provider\": \"mock\", \"dim\": 8}, [\"a\", \"b\"])\nprintln(len(r), len(r[0]))\ntry {\n  llm_embed({\"provider\": \"openai\", \"key_env\": \"KILLER_SURELY_UNSET_VAR\"}, \"x\")\n} catch e {\n  println(e)\n}\n");
    assert!(out.starts_with("16\n2 8\n"), "{}", out);
    assert!(out.contains("KILLER_SURELY_UNSET_VAR is not set"), "{}", out);
}

// ---------------------------------------------------------------- logging and config

#[test]
fn logging_levels_text_and_json() {
    let src = "log_debug(\"d\")\nlog_info(\"hello\", {\"user\": \"bob\"})\nlog_warn(\"w\")\nlog_error(\"e\")\n";
    let default = run(src);
    assert!(!default.contains("d\n") || !default.contains("DEBUG"), "{}", default);
    assert!(default.contains("INFO  hello user=bob") && default.contains("WARN  w") && default.contains("ERROR e"), "{}", default);
    assert!(!default.contains("DEBUG"));
    let dbg = run_with_env(src, &[("KILLER_LOG", "debug")]);
    assert!(dbg.contains("DEBUG d"), "{}", dbg);
    let err_only = run_with_env(src, &[("KILLER_LOG", "error")]);
    assert!(err_only.contains("ERROR e") && !err_only.contains("INFO") && !err_only.contains("WARN"), "{}", err_only);
    let json = run_with_env(src, &[("KILLER_LOG_FORMAT", "json")]);
    let line = json.lines().find(|l| l.contains("hello")).expect(&json);
    assert!(line.starts_with("{\"ts\":\"") && line.contains("\"level\":\"info\",\"msg\":\"hello\",\"user\":\"bob\"}"), "{}", line);
}

#[test]
fn load_env_does_not_override_existing_variables() {
    let path = std::env::temp_dir().join(format!("killer_na_env_{}.env", std::process::id()));
    std::fs::write(&path, "KNA_NEW=fresh\nKNA_KEEP=from-file\n# c\nexport KNA_Q=\"a b\"\n").unwrap();
    let p = path.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let src = format!("set = load_env(\"{}\")\nprintln(set)\nprintln(env_get(\"KNA_NEW\"), env_get(\"KNA_KEEP\"), env_get(\"KNA_Q\"))\n", p);
    let out = run_with_env(&src, &[("KNA_KEEP", "from-env")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(out, "[KNA_NEW, KNA_Q]\nfresh from-env a b");
}

// ---------------------------------------------------------------- RAG demo (separate processes)

#[test]
fn rag_demo_answers_from_the_right_chunk() {
    use std::net::TcpStream;
    use std::time::{Duration, Instant};
    let demo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("rag_demo.killer");
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut server = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&demo)
        .arg("--run")
        .env("RAG_PORT", port.to_string())
        .env("RAG_MAX_REQUESTS", "3")
        .env_remove("OPENAI_API_KEY")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(start.elapsed() < Duration::from_secs(60), "demo server never started");
        std::thread::sleep(Duration::from_millis(100));
    }
    // the probe connection above counts as a request that sends nothing; the demo ignores it
    let src = format!(
        r#"
base = "http://127.0.0.1:{port}"
r = json_parse(http_post_json(base + "/ask", {{"question": "how do I reset my password?"}}, 20))
println(r["sources"][0]["id"])
println(contains(r["answer"], "Reset password button"))
println(len(r["sources"]))
println(r["prompt_tokens"] > 20)
r2 = json_parse(http_post_json(base + "/ask", "{{\"question\": \"when does express delivery arrive\"}}", 20))
println(r2["sources"][0]["id"])
println(http_post_json(base + "/ask", "{{}}", 20))
"#,
        port = port
    );
    let out = run(&src);
    let _ = server.kill();
    let _ = server.wait();
    assert_eq!(out, "accounts#0\ntrue\n2\ntrue\nshipping#0\n{\"error\":\"missing question\"}");
}
