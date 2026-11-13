//! LLM provider tests.
//!
//! Two groups live here:
//!
//! 1. `offline_*`: always run. A fake provider server on 127.0.0.1 (a `TcpListener` in this file)
//!    speaks the OpenAI-compatible, Anthropic and Ollama request/response shapes (JSON and SSE/NDJSON
//!    streaming). It records every request, so request building, header handling, response parsing,
//!    streaming and error handling are checked for real, with no network and no key.
//! 2. `live_*`: opt-in. Each runs only when its provider is configured (`OPENAI_API_KEY`,
//!    `ANTHROPIC_API_KEY`, or a reachable Ollama at `OLLAMA_HOST` / localhost:11434) and otherwise
//!    prints `skipped: <reason>` and passes. Keys are read from the environment by the library
//!    itself; this file never copies a key into a variable, an argument or an output line.
//!    See docs/LLM_TESTING.md.
//!
//! Run with `cargo test --test llm_live -- --nocapture` to see the `skipped:` lines.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use killer_native::llm::{self, LlmConfig, LlmMessage};
use killer_native::llm_stream::{build_plan, StreamSource};
use killer_native::value::{SharedDict, Value};

// ---------------------------------------------------------------------------------------------
// fake provider server
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Req {
    method: String,
    path: String,
    /// header names are lower-cased
    headers: HashMap<String, String>,
    body: String,
}

struct Reply {
    status: u16,
    content_type: &'static str,
    parts: Vec<String>,
    /// pause before each part after the first (lets a streaming client see separate reads)
    delay_ms: u64,
    /// `Transfer-Encoding: chunked` (each part is a chunk), else `Content-Length` for one part, or
    /// close-delimited when there are several parts
    chunked: bool,
}

impl Reply {
    fn json(status: u16, body: &str) -> Reply {
        Reply { status, content_type: "application/json", parts: vec![body.to_string()], delay_ms: 0, chunked: false }
    }
    fn sse(parts: Vec<String>) -> Reply {
        Reply { status: 200, content_type: "text/event-stream", parts, delay_ms: 20, chunked: false }
    }
}

struct Fake {
    base: String,
    seen: Arc<Mutex<Vec<Req>>>,
}

impl Fake {
    fn start(handler: impl Fn(&Req) -> Reply + Send + Sync + 'static) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake server");
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        let handler = Arc::new(handler);
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { continue };
                let seen = seen2.clone();
                let handler = handler.clone();
                std::thread::spawn(move || serve(conn, &*handler, &seen));
            }
        });
        Fake { base: format!("http://127.0.0.1:{}", port), seen }
    }

    fn requests(&self) -> Vec<Req> {
        self.seen.lock().unwrap().clone()
    }

    fn only_request(&self) -> Req {
        let r = self.requests();
        assert_eq!(r.len(), 1, "expected exactly one request, saw {:?}", r);
        r.into_iter().next().unwrap()
    }
}

fn read_request(conn: &mut TcpStream) -> Option<Req> {
    conn.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        let n = conn.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let mut headers = HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let want: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body: Vec<u8> = buf[head_end + 4..].to_vec();
    while body.len() < want {
        let n = conn.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    Some(Req { method, path, headers, body: String::from_utf8_lossy(&body).to_string() })
}

fn serve(mut conn: TcpStream, handler: &(dyn Fn(&Req) -> Reply + Send + Sync), seen: &Mutex<Vec<Req>>) {
    let Some(req) = read_request(&mut conn) else { return };
    seen.lock().unwrap().push(req.clone());
    let r = handler(&req);
    let reason = match r.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Status",
    };
    let mut head = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nConnection: close\r\n", r.status, reason, r.content_type);
    if r.chunked {
        head.push_str("Transfer-Encoding: chunked\r\n");
    } else if r.parts.len() == 1 {
        head.push_str(&format!("Content-Length: {}\r\n", r.parts[0].len()));
    }
    head.push_str("\r\n");
    if conn.write_all(head.as_bytes()).is_err() {
        return;
    }
    for (i, p) in r.parts.iter().enumerate() {
        if i > 0 && r.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(r.delay_ms));
        }
        let ok = if r.chunked {
            conn.write_all(format!("{:x}\r\n{}\r\n", p.len(), p).as_bytes())
        } else {
            conn.write_all(p.as_bytes())
        };
        if ok.is_err() || conn.flush().is_err() {
            return;
        }
    }
    if r.chunked {
        let _ = conn.write_all(b"0\r\n\r\n");
    }
    let _ = conn.flush();
}

// ---------------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------------

fn opts(pairs: Vec<(&str, Value)>) -> SharedDict {
    SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

/// Run a stream to the end and return (chunks, error).
fn drain(o: &SharedDict, prompt: &str) -> (Vec<String>, Option<String>) {
    let plan = build_plan(o, &[("user".to_string(), prompt.to_string())]).expect("build_plan");
    let mut src = StreamSource::start(plan).expect("start stream (is curl installed?)");
    let mut chunks = Vec::new();
    loop {
        match src.next_chunk() {
            Ok(Some(c)) => chunks.push(c),
            Ok(None) => return (chunks, None),
            Err(e) => return (chunks, Some(e)),
        }
    }
}

macro_rules! skip {
    ($($arg:tt)*) => {{
        println!("skipped: {}", format!($($arg)*));
        return;
    }};
}

// ---------------------------------------------------------------------------------------------
// offline tests against the fake server
// ---------------------------------------------------------------------------------------------

const OPENAI_OK: &str = r#"{"id":"chatcmpl-1","object":"chat.completion","model":"gpt-fake-2024","choices":[{"index":0,"message":{"role":"assistant","content":"caf\u00e9 \"ok\"\nline two \ud83d\ude00"},"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}}"#;

#[test]
fn offline_openai_complete_builds_request_and_parses_reply() {
    let fake = Fake::start(|_| Reply::json(200, OPENAI_OK));
    let cfg = LlmConfig::openai("sk-fake-key-AAA", "gpt-fake").with_base_url(&fake.base).with_temperature(0.25).with_max_tokens(77);
    let msgs = vec![LlmMessage::system("be brief"), LlmMessage::user("offline openai complete \"quoted\"\nsecond line")];
    let resp = llm::complete(&cfg, &msgs).expect("complete");

    assert_eq!(resp.content, "caf\u{e9} \"ok\"\nline two \u{1F600}");
    assert_eq!(resp.model, "gpt-fake-2024");
    assert_eq!((resp.prompt_tokens, resp.completion_tokens, resp.total_tokens()), (11, 7, 18));

    let r = fake.only_request();
    assert_eq!(r.method, "POST");
    assert_eq!(r.path, "/chat/completions");
    assert_eq!(r.headers.get("authorization").map(String::as_str), Some("Bearer sk-fake-key-AAA"));
    assert_eq!(r.headers.get("content-type").map(String::as_str), Some("application/json"));
    assert!(r.headers.get("x-api-key").is_none());
    assert!(r.body.contains(r#""model":"gpt-fake""#), "{}", r.body);
    assert!(r.body.contains(r#""max_tokens":77"#), "{}", r.body);
    assert!(r.body.contains(r#""temperature":0.25"#), "{}", r.body);
    assert!(r.body.contains(r#"{"role":"system","content":"be brief"}"#), "{}", r.body);
    // quotes and newlines in the prompt are JSON-escaped, so the body is one valid line
    assert!(r.body.contains(r#"offline openai complete \"quoted\"\nsecond line"#), "{}", r.body);
}

#[test]
fn offline_openai_cache_does_not_cross_endpoints_or_keys() {
    let a = Fake::start(|_| Reply::json(200, r#"{"choices":[{"message":{"role":"assistant","content":"from A"}}]}"#));
    let b = Fake::start(|_| Reply::json(200, r#"{"choices":[{"message":{"role":"assistant","content":"from B"}}]}"#));
    let msgs = vec![LlmMessage::user("offline cache isolation prompt")];
    let ra = llm::complete(&LlmConfig::openai("k1", "m").with_base_url(&a.base), &msgs).unwrap();
    let rb = llm::complete(&LlmConfig::openai("k1", "m").with_base_url(&b.base), &msgs).unwrap();
    assert_eq!((ra.content.as_str(), rb.content.as_str()), ("from A", "from B"));
    // an identical request is served from the cache: no second request reaches the server
    let again = llm::complete(&LlmConfig::openai("k1", "m").with_base_url(&a.base), &msgs).unwrap();
    assert_eq!(again.content, "from A");
    assert_eq!(a.requests().len(), 1);
    // a different (bad) key must not be answered from the cache of a good one
    let bad = Fake::start(|_| Reply::json(401, r#"{"error":{"message":"Incorrect API key provided: sk-...","type":"invalid_request_error"}}"#));
    let err = llm::complete(&LlmConfig::openai("wrong", "m").with_base_url(&bad.base), &msgs);
    assert!(err.is_err(), "a bad key got a cached success");
}

#[test]
fn offline_openai_bad_key_is_a_clear_error_not_a_panic() {
    let fake = Fake::start(|_| {
        Reply::json(401, r#"{"error":{"message":"Incorrect API key provided: sk-bad.","type":"invalid_request_error","code":"invalid_api_key"}}"#)
    });
    let cfg = LlmConfig::openai("sk-bad-key-ZZZ", "gpt-fake").with_base_url(&fake.base);
    let err = llm::complete(&cfg, &[LlmMessage::user("offline openai bad key")]).unwrap_err();
    assert!(err.contains("OpenAI API error"), "{}", err);
    assert!(err.contains("Incorrect API key"), "{}", err);
    assert!(!err.contains("sk-bad-key-ZZZ"), "the key leaked into the error: {}", err);
}

#[test]
fn offline_openai_garbage_reply_is_an_error() {
    let fake = Fake::start(|_| Reply::json(200, "<html>gateway timeout</html>"));
    let cfg = LlmConfig::openai("k", "m").with_base_url(&fake.base);
    let err = llm::complete(&cfg, &[LlmMessage::user("offline openai garbage")]).unwrap_err();
    assert!(err.contains("Cannot parse"), "{}", err);
}

#[test]
fn offline_connection_refused_is_an_error() {
    // grab a free port, then close it again so nothing listens there
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let base = format!("http://127.0.0.1:{}", port);
    let cfg = LlmConfig::openai("k", "m").with_base_url(&base);
    assert!(llm::complete(&cfg, &[LlmMessage::user("offline refused openai")]).is_err());
    let cfg = LlmConfig::ollama("m").with_base_url(&base);
    let err = llm::complete(&cfg, &[LlmMessage::user("offline refused ollama")]).unwrap_err();
    assert!(err.contains("Cannot connect to Ollama"), "{}", err);
}

#[test]
fn offline_openai_embeddings_use_base_url_and_bearer_key() {
    let fake = Fake::start(|_| Reply::json(200, r#"{"object":"list","data":[{"object":"embedding","index":0,"embedding":[0.5,-1.25,3e-2]}],"model":"text-embedding-3-small","usage":{"prompt_tokens":3,"total_tokens":3}}"#));
    let cfg = LlmConfig::openai("sk-embed-key", "text-embedding-3-small").with_base_url(&fake.base);
    let v = llm::embed(&cfg, "offline embed text").expect("embed");
    assert_eq!(v, vec![0.5, -1.25, 0.03]);
    let r = fake.only_request();
    assert_eq!(r.path, "/embeddings");
    assert_eq!(r.headers.get("authorization").map(String::as_str), Some("Bearer sk-embed-key"));
    assert!(r.body.contains(r#""input":"offline embed text""#), "{}", r.body);
    assert!(r.body.contains(r#""model":"text-embedding-3-small""#), "{}", r.body);

    let bad = Fake::start(|_| Reply::json(401, r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error"}}"#));
    let err = llm::embed(&LlmConfig::openai("nope", "text-embedding-3-small").with_base_url(&bad.base), "x").unwrap_err();
    assert!(err.contains("Incorrect API key"), "{}", err);
}

#[test]
fn offline_llm_embed_batch_options_path() {
    use killer_native::embed_builtins::embed_many;
    let fake = Fake::start(|req| {
        let n = req.body.matches("\"t").count();
        let items: Vec<String> = (0..n).rev().map(|i| format!(r#"{{"index":{},"embedding":[{}.0,1.0]}}"#, i, i)).collect();
        Reply::json(200, &format!(r#"{{"data":[{}]}}"#, items.join(",")))
    });
    let o = opts(vec![("provider", s("openai_compatible")), ("base_url", s(&fake.base)), ("key", s("sk-batch")), ("model", s("emb"))]);
    let rows = embed_many(&o, &["t0".to_string(), "t1".to_string(), "t2".to_string()]).expect("embed_many");
    // the server answered out of order; rows come back sorted by index
    assert_eq!(rows, vec![vec![0.0, 1.0], vec![1.0, 1.0], vec![2.0, 1.0]]);
    let r = fake.only_request();
    assert_eq!(r.headers.get("authorization").map(String::as_str), Some("Bearer sk-batch"));
    assert_eq!(r.path, "/embeddings");
}

#[test]
fn offline_anthropic_complete_builds_request_and_parses_reply() {
    let fake = Fake::start(|_| {
        Reply::json(200, r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-fake-1","content":[{"type":"text","text":"Bonjour caf\u00e9, \"ami\"\nfin"}],"stop_reason":"end_turn","usage":{"input_tokens":12,"output_tokens":5}}"#)
    });
    let cfg = LlmConfig::anthropic("sk-ant-fake-key", "claude-fake").with_base_url(&fake.base).with_max_tokens(64).with_temperature(0.0);
    let msgs = vec![
        LlmMessage::system("first rule"),
        LlmMessage::user("offline anthropic complete"),
        LlmMessage::system("second rule"),
    ];
    let resp = llm::complete(&cfg, &msgs).expect("complete");
    assert_eq!(resp.content, "Bonjour caf\u{e9}, \"ami\"\nfin");
    assert_eq!(resp.model, "claude-fake-1");
    assert_eq!((resp.prompt_tokens, resp.completion_tokens), (12, 5));

    let r = fake.only_request();
    assert_eq!(r.method, "POST");
    assert_eq!(r.path, "/messages");
    assert_eq!(r.headers.get("x-api-key").map(String::as_str), Some("sk-ant-fake-key"));
    assert_eq!(r.headers.get("anthropic-version").map(String::as_str), Some("2023-06-01"));
    assert!(r.headers.get("authorization").is_none());
    assert!(r.body.contains(r#""max_tokens":64"#), "{}", r.body);
    // system messages become the top-level "system" field (all of them), never a message role
    assert!(r.body.contains(r#""system":"first rule\nsecond rule""#), "{}", r.body);
    assert!(!r.body.contains(r#""role":"system""#), "{}", r.body);
    assert!(r.body.contains(r#"{"role":"user","content":"offline anthropic complete"}"#), "{}", r.body);
}

#[test]
fn offline_anthropic_bad_key_is_a_clear_error() {
    let fake = Fake::start(|_| {
        Reply::json(401, r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#)
    });
    let cfg = LlmConfig::anthropic("sk-ant-bad-KEY", "claude-fake").with_base_url(&fake.base);
    let err = llm::complete(&cfg, &[LlmMessage::user("offline anthropic bad key")]).unwrap_err();
    assert!(err.contains("Anthropic API error") && err.contains("invalid x-api-key"), "{}", err);
    assert!(!err.contains("sk-ant-bad-KEY"), "the key leaked into the error: {}", err);
}

#[test]
fn offline_ollama_chat_and_embeddings() {
    let fake = Fake::start(|req| match req.path.as_str() {
        "/api/chat" => {
            // chunked transfer encoding, as Ollama may send it
            let body = r#"{"model":"llama-fake","message":{"role":"assistant","content":"pong \u00e9"},"done":true,"prompt_eval_count":9,"eval_count":2}"#;
            let (a, b) = body.split_at(40);
            Reply { status: 200, content_type: "application/json", parts: vec![a.to_string(), b.to_string()], delay_ms: 0, chunked: true }
        }
        "/api/embeddings" => Reply::json(200, r#"{"embedding":[0.1,0.2,0.3]}"#),
        _ => Reply::json(404, r#"{"error":"not found"}"#),
    });
    let cfg = LlmConfig::ollama("llama-fake").with_base_url(&fake.base).with_temperature(0.5).with_max_tokens(33);
    let resp = llm::complete(&cfg, &[LlmMessage::user("offline ollama chat")]).expect("ollama chat");
    assert_eq!(resp.content, "pong \u{e9}");
    assert_eq!((resp.prompt_tokens, resp.completion_tokens), (9, 2));
    let v = llm::embed(&cfg, "offline ollama embed").expect("ollama embed");
    assert_eq!(v, vec![0.1, 0.2, 0.3]);

    let reqs = fake.requests();
    assert_eq!(reqs.len(), 2);
    let chat = reqs.iter().find(|r| r.path == "/api/chat").unwrap();
    assert!(chat.body.contains(r#""stream":false"#), "{}", chat.body);
    assert!(chat.body.contains(r#""num_predict":33"#), "{}", chat.body);
    assert!(chat.body.contains(r#""model":"llama-fake""#), "{}", chat.body);
    // Ollama needs no key, so none may be sent
    assert!(chat.headers.get("authorization").is_none() && chat.headers.get("x-api-key").is_none());
}

#[test]
fn offline_ollama_error_reply_is_a_clear_error() {
    let fake = Fake::start(|_| Reply::json(404, r#"{"error":"model 'nope' not found, try pulling it first"}"#));
    let cfg = LlmConfig::ollama("nope").with_base_url(&fake.base);
    let err = llm::complete(&cfg, &[LlmMessage::user("offline ollama missing model")]).unwrap_err();
    assert!(err.contains("Ollama error") && err.contains("not found"), "{}", err);
    let err = llm::embed(&cfg, "x").unwrap_err();
    assert!(err.contains("Ollama error") && err.contains("not found"), "{}", err);
}

#[test]
fn offline_ollama_endpoint_parsing() {
    use killer_native::llm::parse_ollama_endpoint as p;
    assert_eq!(p(None), ("127.0.0.1".to_string(), 11434));
    assert_eq!(p(Some("")), ("127.0.0.1".to_string(), 11434));
    assert_eq!(p(Some("127.0.0.1:9999")), ("127.0.0.1".to_string(), 9999));
    assert_eq!(p(Some("http://127.0.0.1:9999/")), ("127.0.0.1".to_string(), 9999));
    assert_eq!(p(Some("gpu-box")), ("gpu-box".to_string(), 11434));
    assert_eq!(p(Some("http://gpu-box:8080/v1")), ("gpu-box".to_string(), 8080));
    assert_eq!(p(Some("0.0.0.0:11500")), ("127.0.0.1".to_string(), 11500));
    assert_eq!(p(Some("localhost")), ("127.0.0.1".to_string(), 11434));
    assert_eq!(p(Some("[::1]:7")), ("::1".to_string(), 7));
}

#[test]
fn offline_missing_key_names_the_variable() {
    if std::env::var("GROQ_API_KEY").is_ok() {
        skip!("GROQ_API_KEY is set, so the missing-key path cannot be tested");
    }
    let cfg = LlmConfig::groq("", "m");
    let err = llm::complete(&cfg, &[LlmMessage::user("offline missing groq key")]).unwrap_err();
    assert!(err.contains("GROQ_API_KEY"), "{}", err);
}

// --- streaming ---

fn openai_sse_events() -> Vec<String> {
    let ev = |t: &str| format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{}\"}}}}]}}\n\n", t);
    vec![
        "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n".to_string(),
        ev("Hel"),
        // one event split across two network writes
        "data: {\"choices\":[{\"delta\":{\"content\":\"lo ".to_string(),
        "wor\"}}]}\n\n".to_string(),
        ev("ld\\n"),
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n".to_string(),
        "data: [DONE]\n\n".to_string(),
        // nothing after [DONE] may count
        ev("IGNORED"),
    ]
}

#[test]
fn offline_openai_streaming_yields_deltas_in_order() {
    let fake = Fake::start(|_| Reply::sse(openai_sse_events()));
    let o = opts(vec![("provider", s("openai_compatible")), ("base_url", s(&fake.base)), ("key", s("sk-stream-key")), ("model", s("m"))]);
    let (chunks, err) = drain(&o, "offline openai stream");
    assert_eq!(err, None);
    assert_eq!(chunks.concat(), "Hello world\n");
    assert!(chunks.len() >= 3, "expected incremental chunks, got {:?}", chunks);
    let r = fake.only_request();
    assert_eq!(r.path, "/chat/completions");
    assert_eq!(r.headers.get("authorization").map(String::as_str), Some("Bearer sk-stream-key"));
    assert_eq!(r.headers.get("accept").map(String::as_str), Some("text/event-stream"));
    assert!(r.body.contains(r#""stream":true"#), "{}", r.body);
}

#[test]
fn offline_openai_streaming_bad_key_and_mid_stream_error() {
    let bad = Fake::start(|_| Reply::json(401, r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error"}}"#));
    let o = opts(vec![("provider", s("openai_compatible")), ("base_url", s(&bad.base)), ("key", s("sk-nope-KEY"))]);
    let (chunks, err) = drain(&o, "offline stream bad key");
    assert!(chunks.is_empty());
    let err = err.expect("a bad key must be an error");
    assert!(err.contains("Incorrect API key"), "{}", err);
    assert!(!err.contains("sk-nope-KEY"), "{}", err);

    let mid = Fake::start(|_| {
        Reply::sse(vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_string(),
            "data: {\"error\":{\"message\":\"upstream overloaded\"}}\n\n".to_string(),
        ])
    });
    let o = opts(vec![("provider", s("openai_compatible")), ("base_url", s(&mid.base))]);
    let (chunks, err) = drain(&o, "offline stream mid error");
    assert_eq!(chunks, vec!["partial".to_string()]);
    assert!(err.unwrap().contains("upstream overloaded"));
}

#[test]
fn offline_anthropic_streaming_uses_content_block_deltas() {
    let fake = Fake::start(|_| {
        let ev = |name: &str, data: &str| format!("event: {}\ndata: {}\n\n", name, data);
        Reply::sse(vec![
            ev("message_start", r#"{"type":"message_start","message":{"id":"m","role":"assistant","content":[]}}"#),
            ev("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#),
            ev("ping", r#"{"type":"ping"}"#),
            ev("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi "}}"#),
            ev("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"there"}}"#),
            ev("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
            ev("message_delta", r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}"#),
            ev("message_stop", r#"{"type":"message_stop"}"#),
        ])
    });
    let o = opts(vec![
        ("provider", s("anthropic")),
        ("base_url", s(&fake.base)),
        ("key", s("sk-ant-stream")),
        ("model", s("claude-fake")),
        ("system", s("be nice")),
    ]);
    let (chunks, err) = drain(&o, "offline anthropic stream");
    assert_eq!(err, None);
    assert_eq!(chunks.concat(), "Hi there");
    let r = fake.only_request();
    assert_eq!(r.path, "/messages");
    assert_eq!(r.headers.get("x-api-key").map(String::as_str), Some("sk-ant-stream"));
    assert_eq!(r.headers.get("anthropic-version").map(String::as_str), Some("2023-06-01"));
    assert!(r.body.contains(r#""system":"be nice""#) && r.body.contains(r#""stream":true"#), "{}", r.body);

    let bad = Fake::start(|_| Reply::json(401, r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#));
    let o = opts(vec![("provider", s("anthropic")), ("base_url", s(&bad.base)), ("key", s("bad"))]);
    let (_, err) = drain(&o, "offline anthropic stream bad key");
    assert!(err.expect("error").contains("invalid x-api-key"));
}

#[test]
fn offline_ollama_streaming_reads_ndjson() {
    let fake = Fake::start(|_| {
        Reply::sse(vec![
            "{\"message\":{\"role\":\"assistant\",\"content\":\"Ol\"},\"done\":false}\n".to_string(),
            "{\"message\":{\"role\":\"assistant\",\"content\":\"la\"},\"done\":false}\n{\"message\":{\"content\":\"!\"},".to_string(),
            "\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true}\n".to_string(),
        ])
    });
    let o = opts(vec![("provider", s("ollama")), ("base_url", s(&fake.base)), ("model", s("llama-fake"))]);
    let (chunks, err) = drain(&o, "offline ollama stream");
    assert_eq!(err, None);
    assert_eq!(chunks.concat(), "Olla!");
    let r = fake.only_request();
    assert_eq!(r.path, "/api/chat");
    assert!(r.headers.get("authorization").is_none());
}

// ---------------------------------------------------------------------------------------------
// live tests (opt in through the environment)
// ---------------------------------------------------------------------------------------------

fn env_nonempty(name: &str) -> bool {
    std::env::var(name).map(|v| !v.trim().is_empty()).unwrap_or(false)
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string())
}

fn assert_no_secret_in(text: &str, env_name: &str) {
    if let Ok(k) = std::env::var(env_name) {
        if k.len() >= 8 {
            assert!(!text.contains(&k), "an API key leaked into output ({})", env_name);
        }
    }
}

const LIVE_PROMPT: &str = "Reply with exactly one word: pong";

/// Distinct text per run so the in-process inference cache cannot answer for the provider.
fn live_prompt(tag: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{} ({} {})", LIVE_PROMPT, tag, nanos)
}

// --- OpenAI-compatible (OPENAI_API_KEY; OPENAI_BASE_URL for a compatible server; OPENAI_TEST_MODEL) ---

fn openai_live_cfg() -> LlmConfig {
    // an empty key makes the library read OPENAI_API_KEY itself; the key is never copied here
    let mut cfg = LlmConfig::openai("", &env_or("OPENAI_TEST_MODEL", "gpt-4o-mini")).with_max_tokens(16).with_temperature(0.0);
    if env_nonempty("OPENAI_BASE_URL") {
        cfg = cfg.with_base_url(env_or("OPENAI_BASE_URL", "").trim_end_matches('/'));
    }
    cfg
}

#[test]
fn live_openai_completion() {
    if !env_nonempty("OPENAI_API_KEY") {
        skip!("OPENAI_API_KEY is not set");
    }
    let resp = llm::complete(&openai_live_cfg(), &[LlmMessage::user(&live_prompt("openai"))]).expect("OpenAI completion failed");
    println!("openai completion ok: model={} tokens={}", resp.model, resp.total_tokens());
    assert!(!resp.content.trim().is_empty(), "empty completion");
    assert!(resp.total_tokens() > 0, "no token usage reported");
    assert_no_secret_in(&resp.content, "OPENAI_API_KEY");
}

#[test]
fn live_openai_streaming() {
    if !env_nonempty("OPENAI_API_KEY") {
        skip!("OPENAI_API_KEY is not set");
    }
    let mut pairs = vec![("provider", s("openai")), ("model", s(&env_or("OPENAI_TEST_MODEL", "gpt-4o-mini"))), ("max_tokens", Value::Number(16.0))];
    if env_nonempty("OPENAI_BASE_URL") {
        pairs.push(("base_url", s(&env_or("OPENAI_BASE_URL", ""))));
    }
    let (chunks, err) = drain(&opts(pairs), &live_prompt("openai stream"));
    assert_eq!(err, None, "stream error");
    let text = chunks.concat();
    println!("openai stream ok: {} chunks, {} chars", chunks.len(), text.len());
    assert!(!text.trim().is_empty(), "empty stream");
    assert_no_secret_in(&text, "OPENAI_API_KEY");
}

#[test]
fn live_openai_embeddings() {
    if !env_nonempty("OPENAI_API_KEY") {
        skip!("OPENAI_API_KEY is not set");
    }
    let mut cfg = openai_live_cfg();
    cfg.model = env_or("OPENAI_EMBED_MODEL", "text-embedding-3-small");
    let a = llm::embed(&cfg, "the cat sat on the mat").expect("OpenAI embedding failed");
    let b = llm::embed(&cfg, "a kitten rests on a rug").expect("OpenAI embedding failed");
    println!("openai embeddings ok: dim={}", a.len());
    assert!(a.len() >= 64 && a.len() == b.len());
    assert!(a.iter().all(|x| x.is_finite()) && a.iter().any(|x| *x != 0.0));
    assert!(a != b);
}

#[test]
fn live_openai_bad_key_is_an_error_not_a_panic() {
    // needs the provider to be configured (so the machine has network access to it), but sends a bogus key
    if !env_nonempty("OPENAI_API_KEY") {
        skip!("OPENAI_API_KEY is not set");
    }
    let mut cfg = openai_live_cfg();
    cfg.api_key = Some("sk-invalid-key-for-the-live-error-test".to_string());
    let err = llm::complete(&cfg, &[LlmMessage::user(&live_prompt("openai badkey"))]).expect_err("a bad key must fail");
    println!("openai bad key error ok: {}", err.chars().take(120).collect::<String>());
    assert!(!err.trim().is_empty());
    assert!(!err.contains("sk-invalid-key-for-the-live-error-test"), "the bad key was echoed: {}", err);
    assert_no_secret_in(&err, "OPENAI_API_KEY");
}

// --- Anthropic (ANTHROPIC_API_KEY; ANTHROPIC_TEST_MODEL) ---

fn anthropic_live_cfg() -> LlmConfig {
    LlmConfig::anthropic("", &env_or("ANTHROPIC_TEST_MODEL", "claude-3-5-haiku-latest")).with_max_tokens(16).with_temperature(0.0)
}

#[test]
fn live_anthropic_completion() {
    if !env_nonempty("ANTHROPIC_API_KEY") {
        skip!("ANTHROPIC_API_KEY is not set");
    }
    let msgs = [LlmMessage::system("Answer in one word."), LlmMessage::user(&live_prompt("anthropic"))];
    let resp = llm::complete(&anthropic_live_cfg(), &msgs).expect("Anthropic completion failed");
    println!("anthropic completion ok: model={} tokens={}", resp.model, resp.total_tokens());
    assert!(!resp.content.trim().is_empty(), "empty completion");
    assert!(resp.total_tokens() > 0, "no token usage reported");
    assert_no_secret_in(&resp.content, "ANTHROPIC_API_KEY");
}

#[test]
fn live_anthropic_streaming() {
    if !env_nonempty("ANTHROPIC_API_KEY") {
        skip!("ANTHROPIC_API_KEY is not set");
    }
    let o = opts(vec![
        ("provider", s("anthropic")),
        ("model", s(&env_or("ANTHROPIC_TEST_MODEL", "claude-3-5-haiku-latest"))),
        ("max_tokens", Value::Number(16.0)),
    ]);
    let (chunks, err) = drain(&o, &live_prompt("anthropic stream"));
    assert_eq!(err, None, "stream error");
    let text = chunks.concat();
    println!("anthropic stream ok: {} chunks, {} chars", chunks.len(), text.len());
    assert!(!text.trim().is_empty(), "empty stream");
    assert_no_secret_in(&text, "ANTHROPIC_API_KEY");
}

#[test]
fn live_anthropic_bad_key_is_an_error_not_a_panic() {
    if !env_nonempty("ANTHROPIC_API_KEY") {
        skip!("ANTHROPIC_API_KEY is not set");
    }
    let mut cfg = anthropic_live_cfg();
    cfg.api_key = Some("sk-ant-invalid-key-for-the-live-error-test".to_string());
    let err = llm::complete(&cfg, &[LlmMessage::user(&live_prompt("anthropic badkey"))]).expect_err("a bad key must fail");
    println!("anthropic bad key error ok: {}", err.chars().take(120).collect::<String>());
    assert!(!err.trim().is_empty());
    assert!(!err.contains("sk-ant-invalid-key-for-the-live-error-test"), "the bad key was echoed: {}", err);
    assert_no_secret_in(&err, "ANTHROPIC_API_KEY");
}

// --- Ollama (reachable at OLLAMA_HOST or localhost:11434; OLLAMA_TEST_MODEL, OLLAMA_EMBED_MODEL) ---

/// The installed models, or None when no Ollama answers.
fn ollama_models() -> Option<Vec<String>> {
    if !llm::ollama_is_running() {
        return None;
    }
    llm::ollama_list_models().ok()
}

fn ollama_chat_model(models: &[String]) -> Option<String> {
    if env_nonempty("OLLAMA_TEST_MODEL") {
        return Some(env_or("OLLAMA_TEST_MODEL", ""));
    }
    models.iter().find(|m| !m.contains("embed")).cloned()
}

#[test]
fn live_ollama_completion_and_streaming() {
    let Some(models) = ollama_models() else { skip!("no Ollama reachable at OLLAMA_HOST / localhost:11434") };
    let Some(model) = ollama_chat_model(&models) else { skip!("Ollama is running but has no chat model installed") };
    let cfg = LlmConfig::ollama(&model).with_max_tokens(32).with_temperature(0.0);
    let resp = llm::complete(&cfg, &[LlmMessage::user(&live_prompt("ollama"))]).expect("Ollama completion failed");
    println!("ollama completion ok: model={} tokens={}", model, resp.total_tokens());
    assert!(!resp.content.trim().is_empty(), "empty completion");

    let (host, port) = llm::ollama_endpoint(None);
    let o = opts(vec![
        ("provider", s("ollama")),
        ("model", s(&model)),
        ("base_url", s(&format!("http://{}:{}", host, port))),
        ("max_tokens", Value::Number(32.0)),
    ]);
    let (chunks, err) = drain(&o, &live_prompt("ollama stream"));
    assert_eq!(err, None, "stream error");
    println!("ollama stream ok: {} chunks", chunks.len());
    assert!(!chunks.concat().trim().is_empty(), "empty stream");
}

#[test]
fn live_ollama_embeddings() {
    let Some(models) = ollama_models() else { skip!("no Ollama reachable at OLLAMA_HOST / localhost:11434") };
    let want = env_or("OLLAMA_EMBED_MODEL", "nomic-embed-text");
    if !models.iter().any(|m| m == &want || m.starts_with(&format!("{}:", want))) {
        skip!("Ollama has no embedding model {} (run `ollama pull {}` or set OLLAMA_EMBED_MODEL)", want, want);
    }
    let cfg = LlmConfig::ollama(&want);
    let v = llm::embed(&cfg, "the cat sat on the mat").expect("Ollama embedding failed");
    println!("ollama embeddings ok: dim={}", v.len());
    assert!(v.len() >= 32 && v.iter().all(|x| x.is_finite()) && v.iter().any(|x| *x != 0.0));
}

#[test]
fn live_ollama_missing_model_is_an_error_not_a_panic() {
    if ollama_models().is_none() {
        skip!("no Ollama reachable at OLLAMA_HOST / localhost:11434");
    }
    let cfg = LlmConfig::ollama("killer-no-such-model-xyz");
    let err = llm::complete(&cfg, &[LlmMessage::user(&live_prompt("ollama nomodel"))]).expect_err("a missing model must fail");
    println!("ollama missing model error ok: {}", err.chars().take(120).collect::<String>());
    assert!(err.contains("Ollama error") || err.contains("not found"), "{}", err);
}
