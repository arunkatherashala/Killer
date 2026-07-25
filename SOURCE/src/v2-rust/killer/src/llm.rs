// llm.rs — Native LLM Integration (zero external dependencies)
//
// Providers:
//   • Ollama    → raw TcpStream HTTP/1.1 on localhost:11434 (no TLS, no curl)
//   • OpenAI    → curl subprocess  https://api.openai.com/v1/chat/completions
//   • Anthropic → curl subprocess  https://api.anthropic.com/v1/messages
//   • Groq      → curl subprocess  https://api.groq.com/openai/v1/chat/completions
//
// Zero dependencies: std only (TcpStream + Command).  Curl is pre-installed on
// Windows 10+, macOS, and all Linux distros — no installation required.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

// --- KhLM Background Prefetch Cache ------------------------------------------
// Queries prefetched in background (ns lookup at call time vs ms cold fetch).
static KHLM_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn khlm_cache() -> &'static Mutex<HashMap<String, String>> {
    KHLM_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Prefetch a KhLM query in the background — call at program start.
/// By the time khlm_ask() is called, the cache is already populated → ns lookup.
pub fn khlm_prefetch(question: &str) {
    let q = question.trim().to_string();
    // Skip if already cached
    if let Ok(cache) = khlm_cache().lock() {
        if cache.contains_key(&q) { return; }
    }
    // Fire all agents in a background thread — result stored in cache when ready
    std::thread::spawn(move || {
        let result = khlm_route(&q, None);
        if let Ok(mut cache) = khlm_cache().lock() {
            cache.insert(q, result);
        }
    });
}

/// Clear the global KhLM cache map used by `khlm_prefetch`, `khlm_route` lookups, and `complete()` LLM responses.
/// Stale or wrong answers can persist here — call after fixing LLM config or when outputs look "stuck".
/// Returns how many entries were removed.
pub fn khlm_inference_cache_clear() -> usize {
    if let Ok(mut cache) = khlm_cache().lock() {
        let n = cache.len();
        cache.clear();
        n
    } else {
        0
    }
}

// --- Types -------------------------------------------------------------------

/// Which LLM backend to use.
#[derive(Clone, Debug, PartialEq)]
pub enum LlmProvider {
    /// Local Ollama (localhost:11434).  Free, private, no API key.
    Ollama,
    /// OpenAI GPT-4o, GPT-4o-mini, etc.
    OpenAI,
    /// Anthropic Claude — Haiku / Sonnet / Opus (claude-opus-4-5, claude-3-5-sonnet-20241022, etc).
    Anthropic,
    /// Groq — ultra-fast inference, OpenAI-compatible API.
    Groq,
}

/// Configuration for a single LLM provider connection.
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub provider:    LlmProvider,
    pub model:       String,
    pub api_key:     Option<String>,
    /// Override default base URL (e.g. proxy, custom Ollama port).
    pub base_url:    Option<String>,
    pub max_tokens:  usize,
    pub temperature: f64,
    pub timeout_s:   u64,
    /// When true, instruct the model to reply with valid JSON only.
    pub json_mode:   bool,
}

impl LlmConfig {
    /// Ready-to-use Ollama config.  No API key needed.
    /// `model`: e.g. "llama3", "mistral", "phi3", "gemma2"
    pub fn ollama(model: &str) -> Self {
        LlmConfig {
            provider:    LlmProvider::Ollama,
            model:       model.to_string(),
            api_key:     None,
            base_url:    None,
            max_tokens:  2048,
            temperature: 0.7,
            timeout_s:   60,
            json_mode:   false,
        }
    }

    /// OpenAI config.  Reads OPENAI_API_KEY env var if `api_key` is None.
    /// `model`: e.g. "gpt-4o-mini", "gpt-4o", "gpt-3.5-turbo"
    pub fn openai(api_key: &str, model: &str) -> Self {
        LlmConfig {
            provider:    LlmProvider::OpenAI,
            model:       model.to_string(),
            api_key:     Some(api_key.to_string()),
            base_url:    None,
            max_tokens:  2048,
            temperature: 0.7,
            timeout_s:   30,
            json_mode:   false,
        }
    }

    /// Anthropic Claude config.
    /// `model`: e.g. "claude-opus-4-5", "claude-3-5-sonnet-20241022", "claude-3-haiku-20240307"
    pub fn anthropic(api_key: &str, model: &str) -> Self {
        LlmConfig {
            provider:    LlmProvider::Anthropic,
            model:       model.to_string(),
            api_key:     Some(api_key.to_string()),
            base_url:    None,
            max_tokens:  2048,
            temperature: 0.7,
            timeout_s:   30,
            json_mode:   false,
        }
    }

    /// Groq config (OpenAI-compatible, very fast free tier).
    /// `model`: e.g. "llama3-70b-8192", "mixtral-8x7b-32768"
    pub fn groq(api_key: &str, model: &str) -> Self {
        LlmConfig {
            provider:    LlmProvider::Groq,
            model:       model.to_string(),
            api_key:     Some(api_key.to_string()),
            base_url:    None,
            max_tokens:  2048,
            temperature: 0.7,
            timeout_s:   30,
            json_mode:   false,
        }
    }

    /// Use a custom base URL (Ollama on different port, local OpenAI proxy, etc.)
    pub fn with_base_url(mut self, url: &str) -> Self { self.base_url = Some(url.to_string()); self }
    pub fn with_max_tokens(mut self, n: usize)   -> Self { self.max_tokens = n; self }
    pub fn with_temperature(mut self, t: f64)    -> Self { self.temperature = t; self }
    pub fn with_json_mode(mut self, on: bool)    -> Self { self.json_mode = on; self }
}

/// One message in a conversation (system / user / assistant / tool).
#[derive(Clone, Debug)]
pub struct LlmMessage {
    pub role:    String,  // "system" | "user" | "assistant"
    pub content: String,
}

impl LlmMessage {
    pub fn system(content: &str)    -> Self { LlmMessage { role: "system".into(),    content: content.to_string() } }
    pub fn user(content: &str)      -> Self { LlmMessage { role: "user".into(),      content: content.to_string() } }
    pub fn assistant(content: &str) -> Self { LlmMessage { role: "assistant".into(), content: content.to_string() } }
}

/// Response from an LLM call.
#[derive(Clone, Debug)]
pub struct LlmResponse {
    pub content:          String,
    pub model:            String,
    pub prompt_tokens:    usize,
    pub completion_tokens: usize,
}

impl LlmResponse {
    pub fn total_tokens(&self) -> usize { self.prompt_tokens + self.completion_tokens }
}

// --- Public API --------------------------------------------------------------

/// Send a chat completion request and return the assistant's response.
///
/// # Example — Ollama (local, no API key)
/// ```no_run
/// let cfg = LlmConfig::ollama("llama3");
/// let msgs = vec![LlmMessage::user("What is 2+2?")];
/// let resp = complete(&cfg, &msgs).unwrap();
/// println!("{}", resp.content);  // "4"
/// ```
pub fn complete(config: &LlmConfig, messages: &[LlmMessage]) -> Result<LlmResponse, String> {
    // v2.2 Inference cache: skip network round-trip for identical prompts
    let cache_key = {
        let mut k = format!("{:?}:{}", config.provider, config.model);
        for m in messages { k.push('|'); k.push_str(&m.role); k.push(':'); k.push_str(&m.content); }
        k
    };
    {
        let cache = khlm_cache().lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(LlmResponse {
                content: cached.clone(),
                model: config.model.clone(),
                prompt_tokens: 0,
                completion_tokens: 0,
            });
        }
    }
    let result = match config.provider {
        LlmProvider::Ollama    => complete_ollama(config, messages),
        LlmProvider::OpenAI    => complete_openai(config, messages),
        LlmProvider::Anthropic => complete_anthropic(config, messages),
        LlmProvider::Groq      => complete_groq(config, messages),
    };
    if let Ok(ref resp) = result {
        khlm_cache().lock().unwrap().insert(cache_key, resp.content.clone());
    }
    result
}

/// Generate an embedding vector for `text`.
/// Returns a 1536-dim vector for OpenAI, 4096-dim for Ollama nomic-embed-text.
pub fn embed(config: &LlmConfig, text: &str) -> Result<Vec<f64>, String> {
    match config.provider {
        LlmProvider::Ollama => embed_ollama(config, text),
        LlmProvider::OpenAI => embed_openai(config, text),
        _                   => Err("Embedding only supported for Ollama and OpenAI".to_string()),
    }
}

/// Simple single-turn: send one user message, get reply string.
pub fn ask(config: &LlmConfig, prompt: &str) -> Result<String, String> {
    let msgs = vec![LlmMessage::user(prompt)];
    complete(config, &msgs).map(|r| r.content)
}

/// Check if Ollama is running locally.
pub fn ollama_is_running() -> bool {
    let host = "127.0.0.1:11434";
    TcpStream::connect_timeout(
        &host.parse().unwrap_or("127.0.0.1:11434".parse().unwrap()),
        Duration::from_secs(1),
    ).is_ok()
}

/// List models available in local Ollama.
pub fn ollama_list_models() -> Result<Vec<String>, String> {
    let body = http_get_ollama("/api/tags")?;
    // Parse: {"models":[{"name":"llama3:latest",...},...]}
    let mut models = Vec::new();
    let mut rest = body.as_str();
    while let Some(pos) = rest.find("\"name\":\"") {
        let start = pos + 8;
        let inner = &rest[start..];
        if let Some(end) = inner.find('"') {
            models.push(inner[..end].to_string());
        }
        rest = &rest[start..];
    }
    Ok(models)
}

// --- Ollama (raw TCP HTTP/1.1) ------------------------------------------------

fn complete_ollama(config: &LlmConfig, messages: &[LlmMessage]) -> Result<LlmResponse, String> {
    let model = &config.model;
    let body = build_ollama_chat_body(model, messages, config.temperature, config.max_tokens);
    let raw = http_post_ollama("/api/chat", &body, config.timeout_s)?;
    parse_ollama_chat_response(&raw, model)
}

fn embed_ollama(config: &LlmConfig, text: &str) -> Result<Vec<f64>, String> {
    let body = format!(
        r#"{{"model":{},"prompt":{}}}"#,
        json_string(&config.model),
        json_string(text)
    );
    let raw = http_post_ollama("/api/embeddings", &body, config.timeout_s)?;
    // Response: {"embedding":[0.1,0.2,...]}
    parse_float_array(&raw, "embedding")
}

fn build_ollama_chat_body(model: &str, messages: &[LlmMessage], temperature: f64, max_tokens: usize) -> String {
    let msgs_json = messages_to_json(messages);
    format!(
        r#"{{"model":{},"messages":{},"stream":false,"options":{{"temperature":{},"num_predict":{}}}}}"#,
        json_string(model), msgs_json, temperature, max_tokens
    )
}

fn parse_ollama_chat_response(raw: &str, model: &str) -> Result<LlmResponse, String> {
    // Ollama response: {"model":"llama3","message":{"role":"assistant","content":"..."},"done":true}
    let content = extract_nested_value(raw, "message", "content")
        .or_else(|| extract_json_string(raw, "content"))
        .ok_or_else(|| format!("Failed to parse Ollama response: {}", &raw[..raw.len().min(200)]))?;

    let prompt_tokens    = extract_json_usize(raw, "prompt_eval_count").unwrap_or(0);
    let completion_tokens = extract_json_usize(raw, "eval_count").unwrap_or(0);
    Ok(LlmResponse { content, model: model.to_string(), prompt_tokens, completion_tokens })
}

fn http_post_ollama(path: &str, body: &str, timeout_s: u64) -> Result<String, String> {
    let host = "127.0.0.1";
    let port = 11434u16;
    let addr = format!("{}:{}", host, port);

    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("Invalid address: {}", e))?,
        Duration::from_secs(timeout_s),
    ).map_err(|e| format!("Cannot connect to Ollama at {} — is it running? Error: {}", addr, e))?;

    stream.set_read_timeout(Some(Duration::from_secs(timeout_s)))
          .map_err(|e| format!("Timeout set error: {}", e))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))
          .map_err(|e| format!("Timeout set error: {}", e))?;

    // Write raw HTTP/1.1 request
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, port, body.len(), body
    );
    stream.write_all(request.as_bytes())
          .map_err(|e| format!("Send error: {}", e))?;
    stream.flush().map_err(|e| format!("Flush error: {}", e))?;

    // Read entire response
    let mut response = Vec::new();
    stream.read_to_end(&mut response)
          .map_err(|e| format!("Read error: {}", e))?;

    let text = String::from_utf8_lossy(&response).to_string();
    extract_http_body(&text)
}

fn http_get_ollama(path: &str) -> Result<String, String> {
    let host = "127.0.0.1";
    let port = 11434u16;
    let addr = format!("{}:{}", host, port);

    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("addr parse: {}", e))?,
        Duration::from_secs(3),
    ).map_err(|e| format!("Cannot connect to Ollama: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        path, host, port
    );
    stream.write_all(request.as_bytes()).map_err(|e| format!("Send: {}", e))?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|e| format!("Read: {}", e))?;
    let text = String::from_utf8_lossy(&response).to_string();
    extract_http_body(&text)
}

// --- OpenAI (curl subprocess) ------------------------------------------------

fn complete_openai(config: &LlmConfig, messages: &[LlmMessage]) -> Result<LlmResponse, String> {
    let api_key = resolve_api_key(config, "OPENAI_API_KEY")?;
    let base = config.base_url.as_deref().unwrap_or("https://api.openai.com/v1");
    let url = format!("{}/chat/completions", base);
    let body = build_openai_body(&config.model, messages, config.temperature, config.max_tokens);

    let raw = curl_post(
        &url,
        &[
            format!("Authorization: Bearer {}", api_key),
            "Content-Type: application/json".to_string(),
        ],
        &body,
        config.timeout_s,
    )?;

    parse_openai_response(&raw, &config.model)
}

fn embed_openai(config: &LlmConfig, text: &str) -> Result<Vec<f64>, String> {
    let api_key = resolve_api_key(config, "OPENAI_API_KEY")?;
    let url = "https://api.openai.com/v1/embeddings".to_string();
    let embed_model = "text-embedding-3-small";
    let body = format!(
        r#"{{"input":{},"model":"{}"}}"#,
        json_string(text), embed_model
    );
    let raw = curl_post(
        &url,
        &[
            format!("Authorization: Bearer {}", api_key),
            "Content-Type: application/json".to_string(),
        ],
        &body,
        config.timeout_s,
    )?;
    // Response: {"data":[{"embedding":[...]}]}
    parse_float_array(&raw, "embedding")
}

fn build_openai_body(model: &str, messages: &[LlmMessage], temperature: f64, max_tokens: usize) -> String {
    let msgs_json = messages_to_json(messages);
    format!(
        r#"{{"model":{},"messages":{},"temperature":{},"max_tokens":{}}}"#,
        json_string(model), msgs_json, temperature, max_tokens
    )
}

fn parse_openai_response(raw: &str, model: &str) -> Result<LlmResponse, String> {
    // {"choices":[{"message":{"role":"assistant","content":"..."}}],"usage":{"prompt_tokens":X,"completion_tokens":Y}}
    // Check for API error first
    if let Some(err_msg) = extract_json_string(raw, "message") {
        if raw.contains("\"error\"") {
            return Err(format!("OpenAI API error: {}", err_msg));
        }
    }
    let content = extract_deeply_nested(raw, "content")
        .ok_or_else(|| format!("Cannot parse OpenAI response: {}", &raw[..raw.len().min(300)]))?;
    let prompt_tokens     = extract_json_usize(raw, "prompt_tokens").unwrap_or(0);
    let completion_tokens = extract_json_usize(raw, "completion_tokens").unwrap_or(0);
    let actual_model = extract_json_string(raw, "model").unwrap_or_else(|| model.to_string());
    Ok(LlmResponse { content, model: actual_model, prompt_tokens, completion_tokens })
}

// --- Anthropic (curl subprocess) ---------------------------------------------

fn complete_anthropic(config: &LlmConfig, messages: &[LlmMessage]) -> Result<LlmResponse, String> {
    let api_key = resolve_api_key(config, "ANTHROPIC_API_KEY")?;
    let base = config.base_url.as_deref().unwrap_or("https://api.anthropic.com/v1");
    let url = format!("{}/messages", base);

    // Anthropic separates system prompt from conversation messages
    let (system, conv_msgs): (Option<&LlmMessage>, Vec<&LlmMessage>) = {
        let mut sys = None;
        let mut rest = Vec::new();
        for m in messages {
            if m.role == "system" { sys = Some(m); } else { rest.push(m); }
        }
        (sys, rest)
    };

    let msgs_json = conv_msgs.iter().map(|m| {
        format!(r#"{{"role":{},"content":{}}}"#, json_string(&m.role), json_string(&m.content))
    }).collect::<Vec<_>>().join(",");

    let system_part = system.map(|s| format!(r#","system":{}"#, json_string(&s.content)))
                            .unwrap_or_default();

    let body = format!(
        r#"{{"model":{},"max_tokens":{},"messages":[{}]{},"temperature":{}}}"#,
        json_string(&config.model), config.max_tokens, msgs_json, system_part, config.temperature
    );

    let raw = curl_post(
        &url,
        &[
            format!("x-api-key: {}", api_key),
            "anthropic-version: 2023-06-01".to_string(),
            "Content-Type: application/json".to_string(),
        ],
        &body,
        config.timeout_s,
    )?;

    parse_anthropic_response(&raw, &config.model)
}

fn parse_anthropic_response(raw: &str, model: &str) -> Result<LlmResponse, String> {
    // {"content":[{"type":"text","text":"..."}],"usage":{"input_tokens":X,"output_tokens":Y}}
    if raw.contains("\"error\"") {
        let err = extract_json_string(raw, "message").unwrap_or_else(|| raw[..raw.len().min(200)].to_string());
        return Err(format!("Anthropic API error: {}", err));
    }
    let content = extract_json_string(raw, "text")
        .ok_or_else(|| format!("Cannot parse Anthropic response: {}", &raw[..raw.len().min(300)]))?;
    let prompt_tokens     = extract_json_usize(raw, "input_tokens").unwrap_or(0);
    let completion_tokens = extract_json_usize(raw, "output_tokens").unwrap_or(0);
    let actual_model = extract_json_string(raw, "model").unwrap_or_else(|| model.to_string());
    Ok(LlmResponse { content, model: actual_model, prompt_tokens, completion_tokens })
}

// --- Groq (OpenAI-compatible, curl subprocess) --------------------------------

fn complete_groq(config: &LlmConfig, messages: &[LlmMessage]) -> Result<LlmResponse, String> {
    let api_key = resolve_api_key(config, "GROQ_API_KEY")?;
    let base = config.base_url.as_deref().unwrap_or("https://api.groq.com/openai/v1");
    let url = format!("{}/chat/completions", base);
    let body = build_openai_body(&config.model, messages, config.temperature, config.max_tokens);

    let raw = curl_post(
        &url,
        &[
            format!("Authorization: Bearer {}", api_key),
            "Content-Type: application/json".to_string(),
        ],
        &body,
        config.timeout_s,
    )?;

    parse_openai_response(&raw, &config.model)
}

// --- curl subprocess helper ---------------------------------------------------

/// Run curl to POST JSON and return response body.
/// curl is pre-installed on Windows 10+, macOS, Linux.
fn curl_post(url: &str, headers: &[String], body: &str, timeout_s: u64) -> Result<String, String> {
    // Security: validate URL starts with https:// (prevent SSRF to internal hosts)
    if !url.starts_with("https://") {
        return Err(format!("LLM URL must use HTTPS. Got: {}", &url[..url.len().min(50)]));
    }

    let timeout_str = timeout_s.to_string();
    let mut args = vec![
        "-s",                     // silent (no progress bar)
        "--fail-with-body",       // return body even on HTTP errors
        "--max-time", &timeout_str,
        "-X", "POST", url,
    ];

    // Add headers
    let header_args: Vec<String> = headers.iter()
        .flat_map(|h| vec!["-H".to_string(), h.clone()])
        .collect();
    let header_refs: Vec<&str> = header_args.iter().map(|s| s.as_str()).collect();

    args.extend_from_slice(&header_refs);
    args.extend_from_slice(&["-d", body]);

    let output = Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("curl not found or failed to run: {}. Install curl or use Ollama (no curl needed).", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if stdout.is_empty() && !stderr.is_empty() {
        return Err(format!("curl error: {}", stderr.trim()));
    }

    Ok(stdout)
}

// --- JSON helpers (no serde, no deps) ----------------------------------------

/// Escape a string value for embedding in JSON.
fn json_string(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"',  "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

/// Extract `"key":"value"` from JSON — handles string values.
pub fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)?;
    let rest = json[pos + needle.len()..].trim_start();

    if rest.starts_with('"') {
        // String value — handle escaped quotes in the value
        let inner = &rest[1..];
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        loop {
            match chars.next()? {
                '\\' => match chars.next()? {
                    '"'  => result.push('"'),
                    '\\' => result.push('\\'),
                    'n'  => result.push('\n'),
                    'r'  => result.push('\r'),
                    't'  => result.push('\t'),
                    c    => { result.push('\\'); result.push(c); }
                },
                '"' => break,
                c   => result.push(c),
            }
        }
        Some(result)
    } else {
        None
    }
}

/// Extract a numeric integer value: `"key":123`
pub fn extract_json_usize(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)?;
    let rest = json[pos + needle.len()..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// --- Ghost Agent — web-grounded LLM answers ----------------------------------

/// URL-encode a query string (spaces → %20, special chars → %XX).
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => { out.push('%'); out.push_str(&format!("{:02X}", b)); }
        }
    }
    out
}

/// POST a URL with application/x-www-form-urlencoded body via curl.
/// Used for sites that require form POSTs (e.g. Zaubacorp director search).
fn curl_post_form(url: &str, body: &str, timeout_s: u64) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err(format!("URL must use HTTPS: {}", &url[..url.len().min(60)]));
    }
    let timeout_str = timeout_s.to_string();
    let curl_bin = if cfg!(target_os = "windows") { "curl.exe" } else { "curl" };
    let output = Command::new(curl_bin)
        .args(["-s", "-k", "--ssl-no-revoke", "--fail-with-body",
               "-X", "POST",
               "--data", body,
               "--max-time", &timeout_str,
               "-H", "Content-Type: application/x-www-form-urlencoded",
               "-H", "Accept: text/html,application/xhtml+xml,*/*",
               "-A", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
               url])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// GET a URL via curl (HTTPS only). Returns response body.
fn curl_get(url: &str, timeout_s: u64) -> Result<String, String> {
    curl_get_with_ua(url, timeout_s,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0")
}

fn curl_get_with_ua(url: &str, timeout_s: u64, user_agent: &str) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err(format!("URL must use HTTPS: {}", &url[..url.len().min(60)]));
    }
    let timeout_str = timeout_s.to_string();
    let curl_bin = if cfg!(target_os = "windows") { "curl.exe" } else { "curl" };

    // --ssl-no-revoke: required on corporate/proxy networks where the proxy
    // intercepts TLS and the certificate revocation check fails (CRYPT_E_NO_REVOCATION_CHECK).
    // -k: additionally ignore cert validation errors for proxy-injected certs.
    let output = Command::new(curl_bin)
        .args(["-s", "-k", "--ssl-no-revoke", "--fail-with-body",
               "--max-time", &timeout_str,
               "-H", "Accept: text/html,application/xhtml+xml,*/*",
               "-H", "Accept-Language: en-US,en;q=0.9",
               "-A", user_agent, url])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;
    let body = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(body)
}

/// Search DuckDuckGo instant answers (free, no API key).
/// Returns the best answer text or empty string if not found.
pub fn search_ddg(query: &str) -> Result<String, String> {
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        url_encode(query)
    );
    let body = curl_get(&url, 8)?;

    // Priority: Answer (e.g. calculator) > AbstractText (Wikipedia summary)
    for key in &["Answer", "AbstractText", "Definition"] {
        if let Some(val) = extract_json_string(&body, key) {
            let v = val.trim().to_string();
            if !v.is_empty() { return Ok(v); }
        }
    }
    Ok(String::new())
}

/// DuckDuckGo real web search — scrapes the HTML results page.
/// Returns the first snippet from actual web search results.
/// Falls back gracefully when no results are found.
/// No API key required. Works for any person/topic with web presence.
pub fn search_ddg_web(query: &str) -> Result<String, String> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        url_encode(query)
    );
    let body = curl_get(&url, 10)?;

    // DDG HTML results contain snippets in <a class="result__snippet">...</a>
    // Extract the first 2 snippets and combine them
    let snippet_tag = "result__snippet";
    let mut snippets: Vec<String> = Vec::new();
    let mut search_from = 0;

    while snippets.len() < 3 {
        if let Some(pos) = body[search_from..].find(snippet_tag) {
            let abs_pos = search_from + pos;
            // Find the closing > of the opening tag
            if let Some(tag_end) = body[abs_pos..].find('>') {
                let content_start = abs_pos + tag_end + 1;
                // Find the closing </a>
                if let Some(close) = body[content_start..].find("</a>") {
                    let raw = &body[content_start..content_start + close];
                    // Strip any remaining HTML tags
                    let clean = strip_html(raw);
                    let trimmed = clean.trim().to_string();
                    if !trimmed.is_empty() && trimmed.len() > 20 {
                        snippets.push(trimmed);
                    }
                    search_from = content_start + close + 4;
                } else { break; }
            } else { break; }
        } else { break; }
    }

    if snippets.is_empty() { return Ok(String::new()); }
    Ok(snippets.join(" "))
}

/// Strip HTML tags from a string, replacing them with spaces.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => { in_tag = false; out.push(' '); }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_space = false;
    for ch in out.chars() {
        if ch == ' ' || ch == '\n' || ch == '\t' {
            if !prev_space { result.push(' '); }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Search Bing web results via the RSS feed (works without cookies/JS).
/// Bing often finds people, companies, and profiles that DDG misses.
pub fn search_bing_web(query: &str) -> Result<String, String> {
    // Bing RSS feed — works without browser cookies or JavaScript
    let url = format!(
        "https://www.bing.com/search?q={}&format=rss",
        url_encode(query)
    );
    let body = curl_get_with_ua(&url, 10,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")?;

    if body.len() < 100 {
        // Bing blocked — try news endpoint instead
        let news_url = format!(
            "https://www.bing.com/news/search?q={}&format=rss",
            url_encode(query)
        );
        let news_body = curl_get_with_ua(&news_url, 10,
            "Mozilla/5.0 (compatible; KillerAgent/2.0)")?;

        // Parse RSS <description> tags
        let mut snippets: Vec<String> = Vec::new();
        let mut pos = 0;
        while snippets.len() < 3 {
            let Some(d) = news_body[pos..].find("<description>") else { break };
            let abs = pos + d + 13;
            let Some(close) = news_body[abs..].find("</description>") else { break };
            let raw = &news_body[abs..abs + close];
            let clean = strip_html(raw).trim().to_string();
            if clean.len() > 25 && !clean.contains("Bing") {
                snippets.push(clean);
            }
            pos = abs + close + 14;
        }
        return if snippets.is_empty() { Ok(String::new()) } else { Ok(snippets.join(" ")) };
    }

    // Parse HTML — Bing result descriptions are in <p class="b_paractl"> or similar
    // Fallback: grab all meaningful <p> text
    let mut snippets: Vec<String> = Vec::new();
    let mut pos = 0;
    while snippets.len() < 3 && pos < body.len() {
        let Some(p) = body[pos..].find("<p") else { break };
        let abs = pos + p;
        let Some(gt) = body[abs..].find('>') else { break };
        let cs = abs + gt + 1;
        let Some(cl) = body[cs..].find("</p>") else { break };
        let raw = &body[cs..cs + cl];
        let clean = strip_html(raw).trim().to_string();
        if clean.len() > 40 && !clean.contains("JavaScript") {
            snippets.push(clean);
        }
        pos = cs + cl + 4;
    }

    if snippets.is_empty() { Ok(String::new()) } else { Ok(snippets.join(" ")) }
}

// --- Public Registry Agents ---------------------------------------------------
//
// These scrape public business/director registry sites that are indexed by Google
// but not covered by DDG instant answers or Wikipedia.
//
// Sources:
//   companyseekers.com  — India MCA director profiles
//   zaubacorp.com       — company directors + CIN + contact
//   indiafilings.com    — DIN (Director Identification Number) registry
//   opencorporates.com  — global company/director database (free)

/// Search companyseekers.com for a director/person profile (India MCA data).
/// Search companyseekers.com for a director/person profile (India MCA data).
pub fn search_companyseekers(name: &str) -> Result<String, String> {
    // Use DDG site: search — more reliable than direct URL scraping (sites block bots)
    search_ddg_web(&format!("{} site:companyseekers.com", name))
}

/// Search zaubacorp.com directly — no DDG required.
/// Works through corporate proxies. Two-step:
///   1. POST director search form → extract profile URL (NAME-DIN)
///   2. Fetch profile → extract DIN, about text, company names + CINs
pub fn search_zaubacorp(name: &str) -> Result<String, String> {
    search_zaubacorp_direct(name)
}

pub fn search_zaubacorp_direct(name: &str) -> Result<String, String> {
    // Step 1: POST director search
    let name_upper = name.trim().to_uppercase();
    let search_body = format!("searchvalue={}&cd=director", name_upper.replace(' ', "+"));
    let html = curl_post_form("https://www.zaubacorp.com/search", &search_body, 10)?;
    if html.len() < 200 { return Ok(String::new()); }

    // Step 2: Parse profile URL from search results
    // Format: href="https://www.zaubacorp.com/NAME-DIN"
    // The DIN appears as an 8-digit number at the end of the href path.
    let profile_url = {
        let marker = "href=\"https://www.zaubacorp.com/";
        let mut found = String::new();
        let mut pos = 0;
        while let Some(p) = html[pos..].find(marker) {
            let abs = pos + p + marker.len() - "https://www.zaubacorp.com/".len();
            // Extract the full URL until the closing quote
            let url_start = abs + "https://www.zaubacorp.com/".len();
            if url_start >= html.len() { break; }
            if let Some(end) = html[url_start..].find('"') {
                let path = &html[url_start..url_start + end];
                // Must end with an 8-digit DIN (digits only at end after last '-')
                if let Some(dash) = path.rfind('-') {
                    let suffix = &path[dash + 1..];
                    if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
                        found = format!("https://www.zaubacorp.com/{}", path);
                        break;
                    }
                }
                pos = url_start + end + 1;
            } else { break; }
        }
        found
    };
    if profile_url.is_empty() { return Ok(String::new()); }

    // Step 3: Fetch profile page
    let profile = curl_get(&profile_url, 10)?;
    if profile.len() < 200 { return Ok(String::new()); }

    // Step 4: Extract DIN from the about paragraph
    // Sentence: "Their Director Identification Number (DIN) is 07706924."
    let din = {
        let din_marker = "Director Identification Number (DIN) is ";
        if let Some(p) = profile.find(din_marker) {
            let start = p + din_marker.len();
            let end = profile[start..].find(|c: char| !c.is_ascii_digit())
                .map(|e| start + e).unwrap_or(start + 8);
            profile[start..end.min(start + 10)].to_string()
        } else { String::new() }
    };

    // Step 5: Extract company names + CINs from <h5><a href="...CIN"> NAME </a></h5>
    //   Format: <a href="https://www.zaubacorp.com/COMPANY-NAME-CIN"> COMPANY NAME </a>
    let mut companies: Vec<String> = Vec::new();
    {
        let marker = "https://www.zaubacorp.com/";
        let mut pos = 0;
        while let Some(p) = profile[pos..].find(marker) {
            let url_start = pos + p + marker.len();
            if url_start >= profile.len() { break; }
            if let Some(end) = profile[url_start..].find('"') {
                let path = &profile[url_start..url_start + end];
                // CIN pattern: 21-char alphanumeric starting with U/L + digits (e.g. U72900AP2017PTC104910)
                if let Some(dash) = path.rfind('-') {
                    let cin_candidate = &path[dash + 1..];
                    if cin_candidate.len() == 21 || (cin_candidate.len() >= 15 && cin_candidate.starts_with(|c: char| c.is_uppercase())) {
                        // Extract display name from between > and </a> after this href
                        let rest_start = url_start + end;
                        if let Some(close_bracket) = profile[rest_start..].find('>') {
                            let name_start = rest_start + close_bracket + 1;
                            if let Some(close_a) = profile[name_start..].find("</a>") {
                                let raw_name = &profile[name_start..name_start + close_a];
                                let company_name = strip_html(raw_name).trim().to_string();
                                if !company_name.is_empty() && company_name.len() > 5 && !companies.contains(&company_name) {
                                    companies.push(format!("{} (CIN: {})", company_name, cin_candidate));
                                }
                            }
                        }
                        pos = url_start + end + 1;
                        continue;
                    }
                }
                pos = url_start + end + 1;
            } else { break; }
        }
    }

    if din.is_empty() && companies.is_empty() { return Ok(String::new()); }

    // Build compact single-sentence result — fits in khlm_format's 2-sentence window.
    // Format: "NAME — Director, DIN: XXXXXXXX. Companies: COMPANY (CIN: XXX)."
    let name_display = name.trim().to_uppercase();
    let result = if !companies.is_empty() {
        if !din.is_empty() {
            format!("{} — Director, DIN: {}. Companies: {}.",
                name_display, din, companies.join(", "))
        } else {
            format!("{} — Director. Companies: {}.", name_display, companies.join(", "))
        }
    } else if !din.is_empty() {
        format!("{} — Director, DIN: {}. Registered with Ministry of Corporate Affairs (MCA India).",
            name_display, din)
    } else {
        String::new()
    };
    Ok(result)
}

/// Search Tofler.in — India MCA data mirror (richer than Zaubacorp: has date, industry, status, designation).
/// Two-step pipeline: Zaubacorp POST search → DIN → Tofler profile fetch.
/// Returns formatted multi-line result with all company directorship details.
pub fn search_tofler_direct(name: &str) -> Result<String, String> {
    // Step 1: Zaubacorp POST search to get DIN (same as search_zaubacorp_direct step 1)
    let name_upper = name.trim().to_uppercase();
    let search_body = format!("searchvalue={}&cd=director", name_upper.replace(' ', "+"));
    let search_html = curl_post_form("https://www.zaubacorp.com/search", &search_body, 10)?;
    if search_html.len() < 200 { return Ok(String::new()); }

    // Parse DIN from Zaubacorp search results
    let din = {
        let marker = "href=\"https://www.zaubacorp.com/";
        let mut found = String::new();
        let mut pos = 0;
        while let Some(p) = search_html[pos..].find(marker) {
            let url_start = pos + p + marker.len();
            if url_start >= search_html.len() { break; }
            if let Some(end) = search_html[url_start..].find('"') {
                let path = &search_html[url_start..url_start + end];
                if let Some(dash) = path.rfind('-') {
                    let suffix = &path[dash + 1..];
                    if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
                        found = suffix.to_string();
                        break;
                    }
                }
                pos = url_start + end + 1;
            } else { break; }
        }
        found
    };
    if din.is_empty() { return Ok(String::new()); }

    // Step 2: Fetch Tofler profile using name slug + DIN
    let name_slug = name.trim().to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
    let tofler_url = format!("https://www.tofler.in/{}/director/{}", name_slug, din);
    let profile = curl_get(&tofler_url, 12)?;
    if profile.len() < 500 || profile.contains("404") && profile.contains("vacation") {
        return Ok(String::new());
    }

    // Step 3: Parse DIN from Tofler (confirm)
    let confirmed_din = {
        let marker = "DIN (Director Identification Number)";
        if let Some(p) = profile.find(marker) {
            // Value is in the next <p class="text-20...">VALUE</p>
            if let Some(p2) = profile[p..].find("text-20 font-semibold text-dark\"") {
                let vs = p + p2;
                if let Some(gt) = profile[vs..].find('>') {
                    let vstart = vs + gt + 1;
                    if let Some(lt) = profile[vstart..].find('<') {
                        profile[vstart..vstart + lt].trim().to_string()
                    } else { din.clone() }
                } else { din.clone() }
            } else { din.clone() }
        } else { din.clone() }
    };

    // Step 4: Parse directorship table rows from <tbody id="directorshipsTableBody">
    let mut company_rows: Vec<String> = Vec::new();
    if let Some(tbody_start) = profile.find("directorshipsTableBody") {
        let table = &profile[tbody_start..];
        let mut pos = 0;
        while let Some(tr_s) = table[pos..].find("<tr>") {
            let abs = pos + tr_s;
            let Some(tr_e) = table[abs..].find("</tr>") else { break; };
            let row = &table[abs..abs + tr_e + 5];

            // Extract all <td> text values
            let mut tds: Vec<String> = Vec::new();
            let mut rpos = 0;
            while let Some(tds_p) = row[rpos..].find("<td") {
                let abs_td = rpos + tds_p;
                let Some(gt) = row[abs_td..].find('>') else { break; };
                let cs = abs_td + gt + 1;
                let Some(ct) = row[cs..].find("</td>") else { break; };
                let raw = &row[cs..cs + ct];
                // Extract CIN from href if present
                let cin = if raw.contains("/company/") {
                    if let Some(cp) = raw.find("/company/") {
                        let cin_start = cp + 9;
                        let cin_end = raw[cin_start..].find('"').map(|e| cin_start + e).unwrap_or(cin_start + 21);
                        raw[cin_start..cin_end.min(raw.len())].to_string()
                    } else { String::new() }
                } else { String::new() };

                let text = strip_html(raw).trim().to_string();
                if !text.is_empty() {
                    if !cin.is_empty() {
                        tds.push(format!("{} (CIN: {})", text, cin));
                    } else {
                        tds.push(text);
                    }
                }
                rpos = cs + ct + 5;
            }

            // Extract status separately (badge class)
            let status = if row.contains("badge success") { "Active" }
                        else if row.contains("badge") { "Inactive" }
                        else { "" };

            // tds[0]=company, tds[1]=inc_date, tds[2]=industry, tds[3]=status_text, tds[4]=appt_date, tds[5]=as_on, tds[6]=designation
            if tds.len() >= 1 && !tds[0].is_empty() {
                let company = tds[0].clone();
                let inc_date   = tds.get(1).cloned().unwrap_or_default();
                let industry   = tds.get(2).cloned().unwrap_or_default();
                let appt_date  = tds.get(4).cloned().unwrap_or_default();
                let designation = tds.get(6).cloned().unwrap_or_default();

                let mut row_str = company;
                if !inc_date.is_empty()    { row_str.push_str(&format!(" | Founded: {}", inc_date)); }
                if !industry.is_empty()    { row_str.push_str(&format!(" | Industry: {}", industry)); }
                if !status.is_empty()      { row_str.push_str(&format!(" | Status: {}", status)); }
                if !appt_date.is_empty()   { row_str.push_str(&format!(" | Appointed: {}", appt_date)); }
                if !designation.is_empty() { row_str.push_str(&format!(" | Designation: {}", designation)); }
                company_rows.push(row_str);
            }
            pos = abs + tr_e + 5;
        }
    }

    if confirmed_din.is_empty() && company_rows.is_empty() { return Ok(String::new()); }

    let name_display = name.trim().to_uppercase();
    let mut result = format!("{} — Director | DIN: {} | Source: MCA/Tofler", name_display, confirmed_din);
    for row in &company_rows {
        result.push_str(&format!("\n  {}", row));
    }
    Ok(result)
}

/// Search Yahoo web — HTML scraper. Works through corporate proxies where DDG/Bing are blocked.
/// Returns up to 3 result snippets joined.
pub fn search_yahoo_web(query: &str) -> Result<String, String> {
    let url = format!("https://search.yahoo.com/search?p={}&ei=UTF-8", url_encode(query));
    let html = curl_get(&url, 10)?;
    if html.len() < 500 { return Ok(String::new()); }
    // Yahoo wraps result snippets in: <span class="fc-falcon">...</span>
    // or in: <div class="compText">... <span>text</span></div>
    let mut snippets: Vec<String> = Vec::new();
    // Try "fc-falcon" spans first (body text snippets)
    for tag in &["fc-falcon", "compText", "s-desc"] {
        let mut pos = 0;
        while snippets.len() < 4 {
            let Some(p) = html[pos..].find(tag) else { break; };
            let abs = pos + p;
            let Some(gt) = html[abs..].find('>') else { break; };
            let cs = abs + gt + 1;
            let Some(ct) = html[cs..].find('<') else { break; };
            let text = strip_html(&html[cs..cs + ct]).trim().to_string();
            if text.len() > 25 && !text.contains("Yahoo") && !text.contains("JavaScript") {
                snippets.push(text);
            }
            pos = cs + ct + 1;
        }
        if !snippets.is_empty() { break; }
    }
    if snippets.is_empty() { Ok(String::new()) } else { Ok(snippets.join(" ")) }
}

/// Google HTML search — broadest coverage; covers employees, students, freelancers,
/// academics, anyone with web presence. Uses mobile UA for simpler parseable HTML.
pub fn search_google_html(name: &str) -> Result<String, String> {
    let url = format!(
        "https://www.google.com/search?q={}&hl=en&num=5",
        url_encode(name)
    );
    // Mobile UA returns simpler HTML without JS-heavy rendering
    let html = curl_get_with_ua(&url, 10,
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
         (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1")?;

    if html.len() < 200
        || html.to_lowercase().contains("captcha")
        || html.to_lowercase().contains("unusual traffic")
    {
        return Ok(String::new());
    }

    let mut snippets: Vec<String> = Vec::new();
    // Google mobile HTML wraps result snippets in <div class="BNeawe ...">
    // Multiple BNeawe levels exist: title level and snippet level — grab text chunks > 40 chars
    for marker in &["BNeawe s3v9rd", "BNeawe tAd8D", "BNeawe AP7Wnd", "VwiC3b", "aCOpRe"] {
        let mut pos = 0;
        while snippets.len() < 4 {
            let Some(p) = html[pos..].find(marker) else { break };
            let abs = pos + p;
            let Some(gt) = html[abs..].find('>') else { break };
            let cs = abs + gt + 1;
            // Extract text until next HTML tag
            let Some(ct) = html[cs..].find('<') else { break };
            let text = html[cs..cs + ct].trim().to_string();
            let text = strip_html(&text);
            if text.len() > 40
                && !text.contains("Google")
                && !text.contains("Sign in")
                && !text.contains("captcha")
                && !text.contains("JavaScript")
            {
                if !snippets.iter().any(|s: &String| s.contains(&text[..text.len().min(30)])) {
                    snippets.push(text);
                }
            }
            pos = cs + ct + 1;
        }
        if snippets.len() >= 2 { break; }
    }
    if snippets.is_empty() { Ok(String::new()) } else { Ok(snippets.join(" ")) }
}

/// GitHub API — find developers/tech people by full name.
/// Returns profile: name, company, location, bio, GitHub URL.
/// Free tier: 10 req/min without auth key. Returns JSON — no HTML parsing.
pub fn search_github_api(name: &str) -> Result<String, String> {
    // Search users by full name match
    let url = format!(
        "https://api.github.com/search/users?q={}+in:fullname&per_page=3",
        url_encode(name)
    );
    let json = curl_get_with_ua(&url, 8,
        "KillerLang/2.0 (+https://github.com/killerlang)")?;
    if json.len() < 50 || !json.contains("\"items\"") { return Ok(String::new()); }

    // Extract first login from items array
    let login = {
        let Some(items_p) = json.find("\"login\"") else { return Ok(String::new()) };
        let after = &json[items_p + 9..]; // skip "login":"
        let Some(qs) = after.find('"') else { return Ok(String::new()) };
        let after2 = &after[qs + 1..];
        let Some(qe) = after2.find('"') else { return Ok(String::new()) };
        after2[..qe].to_string()
    };
    if login.is_empty() { return Ok(String::new()); }

    // Fetch full user profile for richer data
    let profile_url = format!("https://api.github.com/users/{}", login);
    let profile = curl_get_with_ua(&profile_url, 8,
        "KillerLang/2.0 (+https://github.com/killerlang)")?;

    let mut parts: Vec<String> = Vec::new();
    for key in &["name", "company", "location", "bio"] {
        if let Some(v) = extract_json_string(&profile, key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                parts.push(format!("{}: {}", key, v));
            }
        }
    }
    if parts.is_empty() {
        return Ok(format!("{} — GitHub: https://github.com/{}", name, login));
    }
    Ok(format!("{} — GitHub: {} | https://github.com/{}", name, parts.join(", "), login))
}

/// Google News RSS — finds anyone mentioned in news articles (India + global).
/// Returns headlines and snippets from news search. Free, no auth, no JS.
pub fn search_google_news_rss(name: &str) -> Result<String, String> {
    let url = format!(
        "https://news.google.com/rss/search?q={}&hl=en-IN&gl=IN&ceid=IN:en",
        url_encode(name)
    );
    let rss = curl_get(&url, 10)?;
    if rss.len() < 200 || !rss.contains("<item>") { return Ok(String::new()); }

    let mut snippets: Vec<String> = Vec::new();
    let mut pos = 0;
    while snippets.len() < 3 {
        let Some(item_p) = rss[pos..].find("<item>") else { break };
        let abs = pos + item_p;
        let Some(item_e) = rss[abs..].find("</item>") else { break };
        let item = &rss[abs..abs + item_e];

        let title = if let Some(tp) = item.find("<title>") {
            let ts = tp + 7;
            let te = item[ts..].find("</title>").unwrap_or(0);
            strip_html(item[ts..ts + te].trim())
        } else { String::new() };

        // Filter: only include if person name appears in title
        if !title.is_empty()
            && !title.contains("Google News")
            && title.len() > 15
        {
            let name_word = name.split_whitespace().next().unwrap_or(name);
            if title.to_lowercase().contains(&name_word.to_lowercase()) {
                snippets.push(title);
            }
        }
        pos = abs + item_e + 7;
    }
    if snippets.is_empty() { Ok(String::new()) } else { Ok(format!("News: {}", snippets.join(" | "))) }
}

/// Economic Times search — India business news; often covers professionals,
/// executives, entrepreneurs, and students by name.
pub fn search_economic_times(name: &str) -> Result<String, String> {
    let url = format!(
        "https://economictimes.indiatimes.com/search?q={}&type=13",
        url_encode(name)
    );
    let html = curl_get(&url, 10)?;
    if html.len() < 200 { return Ok(String::new()); }

    let mut snippets: Vec<String> = Vec::new();
    // ET article snippets are in <p class="desc"> or <p> within article cards
    for marker in &["class=\"desc\"", "class=\"story-summary\"", "<p>"] {
        let mut pos = 0;
        while snippets.len() < 3 {
            let Some(p) = html[pos..].find(marker) else { break };
            let abs = pos + p;
            let Some(gt) = html[abs..].find('>') else { break };
            let cs = abs + gt + 1;
            let Some(ct) = html[cs..].find('<') else { break };
            let text = strip_html(&html[cs..cs + ct]).trim().to_string();
            let name_word = name.split_whitespace().next().unwrap_or(name);
            if text.len() > 30
                && text.to_lowercase().contains(&name_word.to_lowercase())
            {
                snippets.push(text);
            }
            pos = cs + ct + 1;
        }
        if !snippets.is_empty() { break; }
    }
    if snippets.is_empty() { Ok(String::new()) } else { Ok(snippets.join(" ")) }
}

/// LinkedIn public profile — tries common profile URL slug patterns.
/// LinkedIn shows name + headline + location for public profiles without login.
pub fn search_linkedin_public(name: &str) -> Result<String, String> {
    let name_clean = name.trim().to_lowercase();
    let parts: Vec<&str> = name_clean.split_whitespace().collect();
    if parts.is_empty() { return Ok(String::new()); }

    // Build candidate slugs (most to least common LinkedIn URL patterns)
    let mut slugs: Vec<String> = Vec::new();
    if parts.len() >= 2 {
        slugs.push(parts.join("-"));                    // deepthi-sudha-katherasala
        slugs.push(format!("{}-{}", parts[0], parts[parts.len()-1]));  // deepthi-katherasala
        slugs.push(parts.join(""));                     // deepthikatherasala
    } else {
        slugs.push(parts[0].to_string());
    }

    for slug in &slugs {
        // Validate slug (alphanumeric + hyphens only)
        if !slug.chars().all(|c| c.is_alphanumeric() || c == '-') { continue; }
        let url = format!("https://www.linkedin.com/in/{}/", slug);
        let Ok(html) = curl_get(&url, 8) else { continue };
        if html.len() < 500 || html.contains("authwall") || html.contains("Join LinkedIn") { continue; }

        // LinkedIn public profile shows name in <title> and headline in <h2> or meta
        let mut info: Vec<String> = Vec::new();

        // Try <title>Name - Title - Company | LinkedIn</title>
        if let Some(tp) = html.find("<title>") {
            let ts = tp + 7;
            if let Some(te) = html[ts..].find("</title>") {
                let title_text = html[ts..ts + te].trim();
                if !title_text.contains("Sign in") && !title_text.contains("LinkedIn") {
                    let clean = title_text.trim_end_matches(" | LinkedIn").trim().to_string();
                    if !clean.is_empty() { info.push(clean); }
                }
            }
        }
        // Meta description often has "Name - Role - Company - Location"
        if let Some(mp) = html.find("property=\"og:description\"") {
            if let Some(cp) = html[mp..].find("content=\"") {
                let cs = mp + cp + 9;
                if let Some(ce) = html[cs..].find('"') {
                    let meta = strip_html(&html[cs..cs + ce]).trim().to_string();
                    if meta.len() > 20 { info.push(meta); }
                }
            }
        }
        if !info.is_empty() {
            return Ok(format!("{} — LinkedIn: {} | {}", name, info.join(" | "), url));
        }
    }
    Ok(String::new())
}

/// Search opencorporates.com — global public company/officer database.
pub fn search_opencorporates(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:opencorporates.com", name))
}

/// Search indiafilings.com — India DIN/director registry.
pub fn search_indiafilings(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:indiafilings.com director DIN", name))
}

/// Exact-phrase DDG search — wraps name in quotes for precise match.
pub fn search_exact_phrase(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("\"{}\"", name))
}

/// UK Companies House — directors and officers registry.
pub fn search_companies_house(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:find-and-update.company-information.service.gov.uk", name))
}

/// US SEC EDGAR — public company officers and directors.
pub fn search_sec_edgar(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:sec.gov director officer", name))
}

/// Crunchbase — global startup/business founder and executive profiles.
pub fn search_crunchbase(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:crunchbase.com", name))
}

/// LinkedIn via DDG — professional profiles (DDG indexes public LinkedIn pages).
pub fn search_linkedin(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:linkedin.com/in", name))
}

/// India official MCA portal — director data from mca.gov.in.
pub fn search_mca_gov(name: &str) -> Result<String, String> {
    search_ddg_web(&format!("{} site:mca.gov.in director DIN", name))
}

/// Global person search — tries multiple country registries + news sources.
/// Returns the first non-empty result across: AU ASIC, SG ACRA, EU registries, Reuters, Bloomberg.
pub fn search_global_person(name: &str) -> Result<String, String> {
    // Try several global sources in sequence — whichever has the person
    let sources = [
        format!("{} director company site:abr.business.gov.au", name),       // Australia ABR
        format!("{} director site:sgbizfile.acra.gov.sg", name),             // Singapore ACRA
        format!("{} director site:eurobiz.eu OR site:europages.com", name),  // EU
        format!("{} director executive site:reuters.com", name),             // Reuters news
        format!("{} executive profile site:bloomberg.com", name),            // Bloomberg
    ];
    for query in &sources {
        if let Ok(r) = search_ddg_web(query) {
            if !r.is_empty() { return Ok(r); }
        }
    }
    Ok(String::new())
}

/// Generate fuzzy spelling variants of a person name.
/// Covers: vowel swaps (a↔e↔i), silent h insertion, suffix variations,
/// shortening (drop middle name), and common transliteration variations.
/// Returns deduplicated list of variants (not including the original).
pub fn fuzzy_name_variants(name: &str) -> Vec<String> {
    let name = name.trim().to_lowercase();
    let mut variants: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Helper: swap one occurrence of `from` → `to` at each position
    let swap_all = |s: &str, from: &str, to: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut start = 0;
        while let Some(pos) = s[start..].find(from) {
            let abs = start + pos;
            let variant = format!("{}{}{}", &s[..abs], to, &s[abs + from.len()..]);
            out.push(variant);
            start = abs + from.len();
        }
        out
    };

    // 1. Vowel swaps: a↔e, a↔i, e↔i, u↔o (covers most Indian name transliterations)
    for (from, to) in &[("a","e"),("e","a"),("a","i"),("i","a"),("e","i"),("i","e"),("u","o"),("o","u")] {
        for v in swap_all(&name, from, to) {
            variants.insert(v);
        }
    }

    // 2. Silent h: insert/remove after consonants (kath→kat, shal→sal, rath→rat)
    for (from, to) in &[("th","t"),("t","th"),("sh","s"),("s","sh"),("kh","k"),("k","kh"),("gh","g"),("g","gh")] {
        for v in swap_all(&name, from, to) {
            variants.insert(v);
        }
    }

    // 3. Double/single consonant: rr↔r, ll↔l, tt↔t, ss↔s
    for (from, to) in &[("rr","r"),("r","rr"),("ll","l"),("l","ll"),("tt","t"),("ss","s")] {
        for v in swap_all(&name, from, to) {
            variants.insert(v);
        }
    }

    // 4. Shorten: drop the middle word if 3+ words
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() >= 3 {
        // Drop middle word(s)
        variants.insert(format!("{} {}", words[0], words[words.len()-1]));
        // Drop last word
        variants.insert(words[..words.len()-1].join(" "));
        // Drop first word
        variants.insert(words[1..].join(" "));
    }

    // 5. Suffix swap: sala↔shala, ala↔ela↔ila
    for (from, to) in &[("sala","shala"),("shala","sala"),("ala","ela"),("ela","ala"),("ala","ila"),("ila","ala")] {
        if name.ends_with(from) {
            variants.insert(format!("{}{}", &name[..name.len()-from.len()], to));
        }
    }

    // Remove the original and empty strings
    variants.remove(&name);
    variants.retain(|v| !v.is_empty() && v.len() > 3);

    // Deduplicate and return sorted for determinism
    let mut result: Vec<String> = variants.into_iter().collect();
    result.sort();
    result
}

/// Extract <meta name="description" content="..."> from HTML.
#[allow(dead_code)]
fn extract_meta_description(html: &str) -> Option<String> {
    // Try og:description first (usually better)
    for marker in &[r#"property="og:description""#, r#"name="description""#] {
        if let Some(pos) = html.find(marker) {
            let after = &html[pos..];
            if let Some(content) = after.find(r#"content=""#) {
                let start = pos + content + 9;
                if let Some(close) = html[start..].find('"') {
                    let val = &html[start..start + close];
                    let clean = strip_html(val).trim().to_string();
                    if clean.len() > 30 { return Some(clean); }
                }
            }
        }
    }
    None
}

/// Extract the first paragraph of meaningful text from HTML.
#[allow(dead_code)]
fn extract_first_paragraph(html: &str, min_len: usize) -> Result<String, String> {
    let mut pos = 0;
    while pos < html.len() {
        let Some(p) = html[pos..].find("<p") else { break };
        let abs = pos + p;
        let Some(gt) = html[abs..].find('>') else { break };
        let cs = abs + gt + 1;
        let Some(cl) = html[cs..].find("</p>") else { break };
        let raw = &html[cs..cs + cl];
        let clean = strip_html(raw).trim().to_string();
        if clean.len() >= min_len { return Ok(clean); }
        pos = cs + cl + 4;
    }
    Ok(String::new())
}

/// Search Wikipedia for a factual summary (free, no API key).
/// Uses a two-step approach:
///   1. Wikipedia opensearch API to find the canonical article title (case-insensitive)
///   2. Wikipedia REST `page/summary` API to fetch clean prose (no HTML, auto-redirects)
pub fn search_wikipedia(query: &str) -> Result<String, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() { return Ok(String::new()); }

    // Step 1: opensearch to get the canonical article URL.
    // Response format: ["query",["Title1",...],["Desc1",...],["https://en.wikipedia.org/wiki/Title1",...]]
    let search_url = format!(
        "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit=1&format=json",
        url_encode(trimmed)
    );
    let search_body = curl_get(&search_url, 8).unwrap_or_default();

    // Extract canonical title from the Wikipedia URL in the response.
    // Look for the pattern "https://en.wikipedia.org/wiki/Title" in the response.
    let wiki_prefix = "https://en.wikipedia.org/wiki/";
    let wiki_title = if let Some(url_pos) = search_body.find(wiki_prefix) {
        let after_prefix = &search_body[url_pos + wiki_prefix.len()..];
        // Extract until closing quote or comma
        let end = after_prefix.find(|c| c == '"' || c == ',').unwrap_or(after_prefix.len());
        after_prefix[..end].to_string()
    } else {
        // Fallback: manually capitalize first letter and replace spaces with underscores
        let mut t = trimmed.replace(' ', "_");
        if let Some(first) = t.chars().next() {
            let up: String = first.to_uppercase().collect();
            t = up + &t[first.len_utf8()..];
        }
        t
    };

    if wiki_title.is_empty() { return Ok(String::new()); }

    // Step 2: fetch the REST API page summary using the canonical title
    let summary_url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        wiki_title  // already URL-safe from opensearch response
    );
    let body = curl_get(&summary_url, 8)?;

    if body.contains("\"status\":404") || body.len() < 50 {
        return Ok(String::new());
    }

    // REST API extract is plain text (no HTML)
    if let Some(extract) = extract_json_string(&body, "extract") {
        let clean = extract.trim().to_string();
        if !clean.is_empty() {
            // Return first 2 sentences, collapsing newlines to spaces
            let mut out = String::new();
            let mut count = 0usize;
            for ch in clean.chars() {
                if ch == '\n' {
                    if !out.ends_with(' ') { out.push(' '); }
                    continue;
                }
                out.push(ch);
                if ch == '.' { count += 1; if count >= 2 { break; } }
            }
            let result = out.trim().to_string();
            if !result.is_empty() { return Ok(result); }
        }
    }
    Ok(String::new())
}

/// Detect if a string is a pure math expression (digits, operators, parens, spaces, scientific `e`).
pub fn is_arithmetic_only_expr(s: &str) -> bool {
    let s = s.trim().trim_end_matches('?').trim();
    !s.is_empty()
        && s.chars().all(|c| {
            matches!(
                c,
                '0'..='9' | '+' | '-' | '*' | '/' | '%' | '.' | '(' | ')' | ' ' | '^' | 'e' | 'E'
            )
        })
}

/// Strip common English wrappers so questions like "What is 2+2?" become `2+2`.
pub fn normalize_math_question(q: &str) -> String {
    let mut s = q.trim().to_lowercase();
    s = s.trim_end_matches('?').trim().to_string();
    for (from, to) in [
        ("what's", ""),
        ("whats ", ""),
        ("what is the value of ", ""),
        ("what is ", ""),
        ("what are ", ""),
        ("calculate ", ""),
        ("compute ", ""),
        ("evaluate ", ""),
        ("find ", ""),
        ("solve ", ""),
        ("figure out ", ""),
        ("the value of ", ""),
    ] {
        if s.contains(from) {
            s = s.replace(from, to);
        }
    }
    s = s.replace("=?", "").replace('=', "");
    s = s
        .replace(" into ", "*")
        .replace(" times ", "*")
        .replace(" multiplied by ", "*")
        .replace(" x ", "*")
        .replace(" plus ", "+")
        .replace(" minus ", "-")
        .replace(" divided by ", "/")
        .replace(" over ", "/");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Try to parse and evaluate a normalized arithmetic string (`+ - * / % ^`, parens, `1e-6` style).
pub fn try_eval_arithmetic_string(expr: &str) -> Result<f64, String> {
    let candidate = normalize_math_question(expr);
    if !is_arithmetic_only_expr(&candidate) {
        return Err("not a pure arithmetic expression (letters or unknown symbols remain)".into());
    }
    eval_simple_math(&candidate)
}

#[inline]
fn is_math_ident_byte(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_')
}

/// Replace whole-token occurrences of `var` (ASCII identifier) with `replacement` text.
fn substitute_identifier_token(expr: &str, var: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(expr.len() + replacement.len());
    let bytes = expr.as_bytes();
    let var_bytes = var.as_bytes();
    let mut i = 0;
    while i < expr.len() {
        if i + var_bytes.len() <= expr.len() && &bytes[i..i + var_bytes.len()] == var_bytes {
            let before_ok = i == 0 || !is_math_ident_byte(bytes[i - 1]);
            let after_idx = i + var.len();
            let after_ok = after_idx >= expr.len() || !is_math_ident_byte(bytes[after_idx]);
            if before_ok && after_ok {
                out.push_str(replacement);
                i = after_idx;
                continue;
            }
        }
        let ch = expr[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Evaluate `expr` after substituting one variable name with `value` (e.g. `2*x+1`, `x`, 5 → 11).
/// Expression is normalized like [`normalize_math_question`]. Use lowercase names to match normalized text.
/// The name `e` is rejected (conflicts with scientific notation like `1e-6`).
pub fn try_eval_arithmetic_subst_var(expr: &str, var: &str, value: f64) -> Result<f64, String> {
    let var = var.trim().to_lowercase();
    if var.is_empty() {
        return try_eval_arithmetic_string(expr);
    }
    if !var
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("variable name must be ASCII letters, digits, or underscore".into());
    }
    if var == "e" {
        return Err("variable name 'e' is reserved (scientific notation)".into());
    }
    let val_str = if value == value.floor() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    let base = normalize_math_question(expr);
    let substituted = substitute_identifier_token(&base, var.as_str(), &val_str);
    try_eval_arithmetic_string(&substituted)
}

/// Detect if a string is a pure math expression (digits, operators, parens, spaces).
fn is_math_expr(s: &str) -> bool {
    is_arithmetic_only_expr(s)
}

/// DuckDuckGo + Wikipedia + web fallback — shared by `ghost_ask` and `ghost_smart_solve`.
fn ghost_web_search_context(q: &str) -> String {
    let mut context = String::new();
    if let Ok(ddg) = search_ddg(q) {
        if !ddg.is_empty() {
            context = ddg;
        }
    }
    if context.is_empty() {
        let wiki_q = q
            .to_lowercase()
            .replace("what is the ", "")
            .replace("what is ", "")
            .replace("what are ", "")
            .replace("who is ", "")
            .replace("who was ", "")
            .replace("where is ", "")
            .replace("tell me about ", "")
            .replace("explain ", "")
            .replace('?', "")
            .trim()
            .to_string();
        if let Ok(wiki) = search_wikipedia(&wiki_q) {
            if wiki.len() > 40 {
                context = wiki;
            }
        }
    }
    if context.is_empty() {
        if let Ok(web) = search_ddg_web(q) {
            if !web.is_empty() {
                context = web;
            }
        }
    }
    context
}

/// If retrieval strongly matches question keywords, return a short factual answer without an LLM.
fn ghost_try_keyword_instant_answer(q: &str, context: &str) -> Option<String> {
    if context.is_empty() {
        return None;
    }
    let ctx_lower = context.to_lowercase();
    let q_clean: String = q
        .chars()
        .map(|c| if c.is_alphabetic() { c.to_ascii_lowercase() } else { ' ' })
        .collect();
    let keywords: Vec<&str> = q_clean
        .split_whitespace()
        .filter(|w| {
            w.len() > 3
                && !matches!(
                    *w,
                    "what" | "who" | "where" | "when" | "why" | "how" | "is" | "the" | "are"
                        | "was" | "tell" | "about" | "explain" | "does" | "will" | "have"
                        | "from" | "that" | "with" | "this"
                )
        })
        .collect();
    let matched = keywords
        .iter()
        .filter(|&&kw| ctx_lower.contains(kw))
        .count();
    if matched >= 2 || (matched == 1 && keywords.len() <= 2) {
        let mut answer = String::new();
        let mut sent_count = 0usize;
        for ch in context.chars() {
            if ch == '\n' {
                if !answer.ends_with(' ') {
                    answer.push(' ');
                }
                continue;
            }
            answer.push(ch);
            if ch == '.' {
                sent_count += 1;
                if sent_count >= 2 {
                    break;
                }
            }
        }
        return Some(answer.trim().to_string());
    }
    None
}

/// Ghost Agent: grounded LLM — searches the web for facts first, then asks the LLM.
///
/// Pipeline:
///   1. If pure math → evaluate natively (exact, instant, no LLM needed)
///   2. Search DuckDuckGo instant answers (free, no API key)
///   3. If no result → search Wikipedia (free, no API key)
///   4. Inject retrieved context into the prompt before calling the local LLM
///   5. Return a grounded answer that can't hallucinate on known facts
///
/// Killer usage:
///   answer = ghost_ask(model, "What is the capital of France?")
///   print(K"Ghost Agent> {answer}")
pub fn ghost_ask(model_path: &str, question: &str, max_tokens: usize) -> Result<String, String> {
    let q = question.trim();

    // -- Step 1: Math shortcut (Killer VM native, 100% exact) -----------------
    let math_candidate = normalize_math_question(q);

    if is_math_expr(&math_candidate) {
        // Evaluate using Rust directly (same precision as Killer VM)
        if let Ok(result) = eval_simple_math(&math_candidate) {
            return Ok(format!("{}", result));
        }
    }

    let context = ghost_web_search_context(q);

    if let Some(answer) = ghost_try_keyword_instant_answer(q, &context) {
        return Ok(answer);
    }

    let prompt = if context.is_empty() {
        format!("Answer this question concisely: {}", q)
    } else {
        format!(
            "Use the following verified facts to answer the question.\n\
             Facts: {}\n\n\
             Question: {}\n\
             Answer:",
            context, q
        )
    };

    // -- Step 4: Ask the local LLM with grounded context ----------------------
    // Use killer_chat_auto so the model gets the right chat template (ChatML for
    // Qwen2.5, Zephyr tags for TinyLlama, [INST] for Mistral, etc.).  Without the
    // chat template the model generates incoherent text regardless of the context.
    let system = Some("You are a precise assistant. Answer concisely using only the provided facts.");
    crate::inference::killer_chat_auto(model_path, &prompt, system, max_tokens)
}

// =============================================================================
// SMART GHOST — tool + LLM hybrid with verify / numeric solve / retry loop
// =============================================================================

const SMART_GHOST_LOOP_INSTRUCTIONS: &str = "\
You are Smart Ghost: combine reasoning with machine-checkable steps.\n\
\n\
Output EXACTLY these labeled lines (keep the labels verbatim):\n\
APPROACH: <short plan, 1–4 sentences; can wrap>\n\
VERIFY_EXPR: <one closed-form arithmetic expression Killer can evaluate, OR the word NONE>\n\
NUMERIC_ROOT: <var> <lo> <hi> <expr with spaces>  OR NONE\n\
  Example: NUMERIC_ROOT: x 1 3 x*x-4   means solve x*x-4=0 on [1,3] by bisection.\n\
  Use NONE if no scalar root-finding applies.\n\
ANSWER: <final reply for the user; include the key number if numeric>\n\
\n\
Rules:\n\
- VERIFY_EXPR: only digits, + - * / % ^ ( ) . and scientific notation (e.g. 1e-3). No variables.\n\
- NUMERIC_ROOT expr: only that variable name plus arithmetic; variable must not be `e`.\n\
- If the task is purely factual, set VERIFY_EXPR and NUMERIC_ROOT to NONE.\n";

#[derive(Default, Clone)]
struct SmartGhostParsed {
    approach: String,
    verify_expr: Option<String>,
    numeric_root: Option<(String, f64, f64, String)>,
    answer: String,
}

fn smart_ghost_line_field<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let t = line.trim();
    let plen = prefix.len();
    if t.len() >= plen && t[..plen].eq_ignore_ascii_case(prefix) {
        Some(t[plen..].trim())
    } else {
        None
    }
}

fn smart_ghost_line_starts_label(line: &str) -> bool {
    let u = line.trim().to_ascii_uppercase();
    ["APPROACH:", "VERIFY_EXPR:", "NUMERIC_ROOT:", "ANSWER:"]
        .iter()
        .any(|p| u.starts_with(p))
}

fn parse_numeric_root_spec(s: &str) -> Option<(String, f64, f64, String)> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let lo: f64 = parts[1].parse().ok()?;
    let hi: f64 = parts[2].parse().ok()?;
    let expr = parts[3..].join(" ");
    Some((parts[0].to_string(), lo, hi, expr))
}

fn parse_smart_ghost_response(raw: &str) -> SmartGhostParsed {
    let mut out = SmartGhostParsed::default();
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if let Some(v) = smart_ghost_line_field(line, "APPROACH:") {
            out.approach = v.to_string();
            i += 1;
            while i < lines.len() {
                let ln = lines[i].trim();
                if ln.is_empty() {
                    i += 1;
                    continue;
                }
                if smart_ghost_line_starts_label(ln) {
                    break;
                }
                out.approach.push(' ');
                out.approach.push_str(ln);
                i += 1;
            }
            continue;
        }
        if let Some(v) = smart_ghost_line_field(line, "VERIFY_EXPR:") {
            let t = v.trim();
            if !t.eq_ignore_ascii_case("none") && !t.is_empty() {
                out.verify_expr = Some(t.to_string());
            }
            i += 1;
            continue;
        }
        if let Some(v) = smart_ghost_line_field(line, "NUMERIC_ROOT:") {
            let t = v.trim();
            if !t.eq_ignore_ascii_case("none") && !t.is_empty() {
                out.numeric_root = parse_numeric_root_spec(t);
            }
            i += 1;
            continue;
        }
        if let Some(v) = smart_ghost_line_field(line, "ANSWER:") {
            out.answer = v.to_string();
            i += 1;
            while i < lines.len() {
                let ln = lines[i].trim();
                if ln.is_empty() {
                    i += 1;
                    continue;
                }
                if smart_ghost_line_starts_label(ln) {
                    break;
                }
                out.answer.push(' ');
                out.answer.push_str(ln);
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    out
}

fn approx_eq_smart(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() < 1e-5 + 1e-7 * scale
}

fn score_smart_attempt(p: &SmartGhostParsed) -> (i32, Vec<String>) {
    let mut score = 0i32;
    let mut notes: Vec<String> = Vec::new();
    if p.answer.trim().is_empty() {
        return (-30, vec!["empty ANSWER".to_string()]);
    }
    score += 15;
    notes.push("non-empty ANSWER".to_string());

    if let Some(ref ve) = p.verify_expr {
        match try_eval_arithmetic_string(ve) {
            Ok(v) => {
                score += 25;
                notes.push(format!("VERIFY_EXPR evaluates to {}", v));
                let nums = extract_numbers(&p.answer);
                if nums.is_empty() {
                    score += 10;
                    notes.push("no numeric token in ANSWER to cross-check".to_string());
                } else if nums.iter().any(|n| approx_eq_smart(*n, v)) {
                    score += 45;
                    notes.push("ANSWER number matches VERIFY_EXPR".to_string());
                } else {
                    score -= 25;
                    notes.push(format!(
                        "mismatch: VERIFY_EXPR={} vs numbers in ANSWER {:?}",
                        v, nums
                    ));
                }
            }
            Err(e) => {
                score -= 18;
                notes.push(format!("VERIFY_EXPR invalid: {}", e));
            }
        }
    }

    if let Some((ref var, lo, hi, ref ex)) = p.numeric_root {
        match bisection_root(ex, var, lo, hi) {
            Ok(r) => {
                score += 28;
                notes.push(format!("NUMERIC_ROOT ≈ {}", r));
                let nums = extract_numbers(&p.answer);
                if nums.iter().any(|n| approx_eq_smart(*n, r)) {
                    score += 22;
                    notes.push("ANSWER matches bisection root".to_string());
                } else if !nums.is_empty() {
                    score -= 12;
                    notes.push("root vs ANSWER numbers differ".to_string());
                }
            }
            Err(e) => {
                score -= 14;
                notes.push(format!("NUMERIC_ROOT failed: {}", e));
            }
        }
    }

    (score, notes)
}

fn format_smart_ghost_feedback(score: i32, notes: &[String], parsed: &SmartGhostParsed) -> String {
    let mut s = String::new();
    s.push_str(&format!("Last round score: {}\n", score));
    for n in notes {
        s.push_str("- ");
        s.push_str(n);
        s.push('\n');
    }
    if !parsed.approach.is_empty() {
        s.push_str(&format!("(Your APPROACH was: {})\n", parsed.approach));
    }
    s.push_str("\nRevise so VERIFY_EXPR matches ANSWER if you give both; fix NUMERIC_ROOT bracket if bisection failed; keep ANSWER honest.\n");
    s
}

fn build_smart_ghost_user_prompt(
    q: &str,
    web_ctx: &str,
    round: usize,
    max_rounds: usize,
    feedback: &str,
) -> String {
    let mut s = String::new();
    s.push_str(&format!("Round {}/{}.\n\n", round, max_rounds));
    s.push_str("Problem:\n");
    s.push_str(q);
    s.push_str("\n\n");
    if !web_ctx.is_empty() {
        s.push_str("Retrieved context (may help; verify if critical):\n");
        s.push_str(web_ctx);
        s.push_str("\n\n");
    }
    if !feedback.is_empty() {
        s.push_str("--- Engine feedback (previous attempt) ---\n");
        s.push_str(feedback);
        s.push_str("\n---\n\n");
    }
    s.push_str(SMART_GHOST_LOOP_INSTRUCTIONS);
    s
}

fn display_f64_agent(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

/// Smart Ghost agent: closed-form math first, web retrieval, then an iterative LLM loop that must
/// emit machine-checkable `VERIFY_EXPR` and optional `NUMERIC_ROOT` lines. Scores each attempt,
/// feeds failures back, and returns the best-scoring round with a full trace.
///
/// Killer: `ghost_smart_solve(model, question [, max_rounds [, max_tokens]])`
pub fn ghost_smart_solve(
    model_path: &str,
    question: &str,
    max_rounds: usize,
    max_tokens: usize,
) -> Result<String, String> {
    let q = question.trim();
    let max_rounds = max_rounds.clamp(1, 12);
    let max_tokens = max_tokens.max(32).min(8192);

    let math_candidate = normalize_math_question(q);
    if is_arithmetic_only_expr(&math_candidate) {
        if let Ok(v) = eval_simple_math(&math_candidate) {
            return Ok(format!(
                "**Smart Ghost (tier 0 — closed-form math)**\n\nANSWER: {}\n\nVerified by the math engine; no LLM rounds run.",
                display_f64_agent(v)
            ));
        }
    }

    let web_ctx = ghost_web_search_context(q);
    if let Some(short) = ghost_try_keyword_instant_answer(q, &web_ctx) {
        return Ok(format!(
            "**Smart Ghost (tier 1 — retrieval shortcut)**\n\nANSWER: {}\n\nStrong web keyword match; no LLM loop.",
            short
        ));
    }

    let system = "You are Smart Ghost: accurate, tool-aware, concise. Obey the labeled output format.";
    let mut feedback = String::new();
    let mut attempts: Vec<(i32, SmartGhostParsed, Vec<String>, String)> = Vec::new();

    for round in 1..=max_rounds {
        let user = build_smart_ghost_user_prompt(q, &web_ctx, round, max_rounds, &feedback);
        let raw = crate::inference::killer_chat_auto(model_path, &user, Some(system), max_tokens)?;
        let parsed = parse_smart_ghost_response(&raw);
        let (score, notes) = score_smart_attempt(&parsed);
        attempts.push((score, parsed.clone(), notes.clone(), raw));
        if score >= 82 {
            break;
        }
        if round < max_rounds {
            feedback = format_smart_ghost_feedback(score, &notes, &parsed);
        }
    }

    let best_idx = attempts
        .iter()
        .enumerate()
        .max_by_key(|(_, (s, _, _, _))| *s)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let (best_score, best_p, _, _) = &attempts[best_idx];

    let mut out = String::new();
    out.push_str("**Smart Ghost — best attempt**\n\n");
    out.push_str(&format!("Score: {}\n\n", best_score));
    out.push_str("ANSWER:\n");
    out.push_str(if best_p.answer.trim().is_empty() {
        "(model produced no ANSWER line — inspect raw trace)"
    } else {
        best_p.answer.trim()
    });
    out.push_str("\n\n**Strategy (approach)**\n");
    out.push_str(if best_p.approach.is_empty() {
        "(none)"
    } else {
        best_p.approach.as_str()
    });
    out.push_str("\n\n**Stored rounds**\n");
    for (i, (sc, p, notes, _raw)) in attempts.iter().enumerate() {
        out.push_str(&format!(
            "--- Round {} | score {} ---\n",
            i + 1,
            sc
        ));
        for n in notes {
            out.push_str(&format!("  • {}\n", n));
        }
        if let Some(ref ve) = p.verify_expr {
            out.push_str(&format!("  VERIFY_EXPR: {}\n", ve));
        }
        if let Some((v, lo, hi, ex)) = &p.numeric_root {
            out.push_str(&format!(
                "  NUMERIC_ROOT: {} {} {} {}\n",
                v, lo, hi, ex
            ));
        }
        out.push('\n');
    }
    Ok(out)
}

/// Evaluate a simple arithmetic expression (no variables, just nums + ops).
/// Handles `+ - * / %`, parentheses, right-associative `^`, unary `+`/`-`,
/// and scientific notation (`1.5e-3`). Precedence: add/sub < mul/div/mod < pow < unary.
fn eval_simple_math(expr: &str) -> Result<f64, String> {
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    let flat = tokens.join("");
    parse_add_sub(&flat, &mut 0)
}

fn parse_add_sub(s: &str, pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_mul_div(s, pos)?;
    while *pos < s.len() {
        match s.as_bytes().get(*pos) {
            Some(b'+') => { *pos += 1; left += parse_mul_div(s, pos)?; }
            Some(b'-') => { *pos += 1; left -= parse_mul_div(s, pos)?; }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_mul_div(s: &str, pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_pow(s, pos)?;
    while *pos < s.len() {
        match s.as_bytes().get(*pos) {
            Some(b'*') => { *pos += 1; left *= parse_pow(s, pos)?; }
            Some(b'/') => {
                *pos += 1;
                let r = parse_pow(s, pos)?;
                if r == 0.0 { return Err("division by zero".into()); }
                left /= r;
            }
            Some(b'%') => { *pos += 1; left %= parse_pow(s, pos)?; }
            _ => break,
        }
    }
    Ok(left)
}

/// Right-associative exponentiation: `2^3^2` → 2^(3^2) = 512.
fn parse_pow(s: &str, pos: &mut usize) -> Result<f64, String> {
    let base = parse_unary(s, pos)?;
    if s.as_bytes().get(*pos) == Some(&b'^') {
        *pos += 1;
        let exp = parse_pow(s, pos)?;
        let r = base.powf(exp);
        if r.is_nan() {
            return Err("undefined exponentiation (e.g. negative base to a non-integer power)".into());
        }
        Ok(r)
    } else {
        Ok(base)
    }
}

fn parse_unary(s: &str, pos: &mut usize) -> Result<f64, String> {
    if s.as_bytes().get(*pos) == Some(&b'(') {
        *pos += 1;
        let v = parse_add_sub(s, pos)?;
        if s.as_bytes().get(*pos) == Some(&b')') { *pos += 1; }
        return Ok(v);
    }
    if s.as_bytes().get(*pos) == Some(&b'+') {
        *pos += 1;
        return parse_unary(s, pos);
    }
    if s.as_bytes().get(*pos) == Some(&b'-') {
        *pos += 1;
        return Ok(-parse_unary(s, pos)?);
    }
    parse_number_literal(s, pos)
}

fn parse_number_literal(s: &str, pos: &mut usize) -> Result<f64, String> {
    let start = *pos;
    while *pos < s.len() && matches!(s.as_bytes()[*pos], b'0'..=b'9' | b'.') {
        *pos += 1;
    }
    if start == *pos {
        return Err(format!("expected number at pos {}", start));
    }
    if matches!(s.as_bytes().get(*pos), Some(b'e') | Some(b'E')) {
        *pos += 1;
        if matches!(s.as_bytes().get(*pos), Some(b'+') | Some(b'-')) {
            *pos += 1;
        }
        let exp_start = *pos;
        while *pos < s.len() && matches!(s.as_bytes()[*pos], b'0'..=b'9') {
            *pos += 1;
        }
        if *pos == exp_start {
            return Err("invalid scientific notation (missing exponent digits)".into());
        }
    }
    s[start..*pos].parse::<f64>().map_err(|e| e.to_string())
}

/// Bisection root of `expr(var)=0` on `[lo, hi]` (arithmetic only in `expr`). Used by Smart Ghost.
pub(crate) fn bisection_root(expr: &str, var: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let (mut lo, mut hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if !lo.is_finite() || !hi.is_finite() {
        return Err("interval endpoints must be finite".into());
    }
    const MAX_IT: usize = 100;
    const TOL: f64 = 1e-11;
    let mut flo = try_eval_arithmetic_subst_var(expr, var, lo)?;
    let fhi_init = try_eval_arithmetic_subst_var(expr, var, hi)?;
    if flo == 0.0 {
        return Ok(lo);
    }
    if fhi_init == 0.0 {
        return Ok(hi);
    }
    if flo * fhi_init > 0.0 {
        return Err(format!(
            "f(lo) and f(hi) have the same sign ({:.6}, {:.6}); try another bracket",
            flo, fhi_init
        ));
    }
    for _ in 0..MAX_IT {
        let mid = (lo + hi) / 2.0;
        let fm = try_eval_arithmetic_subst_var(expr, var, mid)?;
        if fm.abs() < TOL || (hi - lo).abs() < TOL {
            return Ok(mid);
        }
        if flo * fm <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
            flo = fm;
        }
    }
    Err("bisection did not converge".into())
}

/// Extract a nested value: finds `"outer":{...,"inner":"value"...}` pattern.
fn extract_nested_value(json: &str, outer: &str, inner: &str) -> Option<String> {
    let needle = format!("\"{}\":", outer);
    let pos = json.find(&needle)?;
    let after = &json[pos + needle.len()..];
    // Find the opening `{`
    let obj_start = after.find('{')?;
    // Find roughly where the object ends (naive: next top-level `}`)
    let obj_slice = &after[obj_start..];
    // Search within the nested object
    extract_json_string(obj_slice, inner)
}

/// Search deeply nested JSON for the first occurrence of `"key":"value"` at any depth.
fn extract_deeply_nested(json: &str, key: &str) -> Option<String> {
    extract_json_string(json, key)
}

/// Parse a JSON float array from a field: `"key":[0.1,0.2,...]`
fn parse_float_array(json: &str, key: &str) -> Result<Vec<f64>, String> {
    let needle = format!("\"{}\":[", key);
    let pos = json.find(&needle)
        .ok_or_else(|| format!("Field '{}' not found in response", key))?;
    let rest = &json[pos + needle.len()..];
    let end = rest.find(']').ok_or("Missing closing ']' in array")?;
    let arr = &rest[..end];
    arr.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<f64>().map_err(|e| format!("Float parse error: {}", e)))
        .collect()
}

// ===============================================================================
// NATIVE THINK ENGINE
// ===============================================================================
//
// A deterministic reasoning engine — no LLM required for its rule-based paths.
// Tries math → percentages → units → motion → multi-step patterns → KB → web.
// Shows a reasoning trace when it can; on miss, returns an honest structured
// fallback with next-step hints (still not omniscient — no system solves “everything”).
//
// Strengths: closed-form math, common conversions, curated KB hits, optional DDG/Wikipedia.
// Limits:    open-ended strategy, novel research, medical/legal advice, and messy
//             real-world problems need humans, specialists, or `khlm_ask` / `rlm_think` / Ghost.
//
// Builtin:  native_think(question)  →  String (reasoning trace + answer)
// ===============================================================================

/// Format a completed reasoning trace into the final output.
fn format_think_output(steps: &[String], answer: &str) -> String {
    let mut out = String::from("+-- Thinking -----------------------------------------\n");
    for step in steps {
        out.push_str(&format!("|  {}\n", step));
    }
    out.push_str("+-----------------------------------------------------\n\n");
    out.push_str(answer);
    out
}

/// When rules + KB + quick search fail: suggest sensible next actions (keyword heuristics).
fn suggest_next_actions_for_question(q: &str) -> String {
    let l = q.to_lowercase();
    let mut lines: Vec<&'static str> = Vec::new();

    if l.contains("code") || l.contains("bug") || l.contains("error") || l.contains("function")
        || l.contains("compile") || l.contains("syntax")
    {
        lines.push("- Code / errors: narrow to a minimal repro, then try `khlm_ask` / `kala_ask` in code mode or a real compiler/interpreter.");
    }
    if l.contains("why") || l.contains("reason") || l.contains("prove") || l.contains("logic") {
        lines.push("- Deep reasoning: use `rlm_think` / `llm_reason` with a capable model, or break the claim into smaller yes/no checks.");
    }
    if l.contains("news") || l.contains("today") || l.contains("latest") || l.contains("current") {
        lines.push("- Time-sensitive facts: use `ghost_108` / `ghost_ask` or browse primary sources; KB + instant answers can be stale.");
    }
    if l.contains("how do i") || l.contains("steps") || l.contains("plan") {
        lines.push("- Planning: list constraints, success criteria, then one unknown at a time; optional `khlm_run(\"auto\", question)`.");
    }
    if l.contains("medical") || l.contains("diagnos") || l.contains("legal") || l.contains("lawyer") {
        lines.push("- Medical / legal: consult a qualified professional — automated tools are not a substitute.");
    }
    if lines.is_empty() {
        lines.push("- Rephrase one concrete unknown (who/what/when/how much) and try `native_think` again.");
        lines.push("- Broader context: `khlm_ask(question)` or `ghost_108(question)` when you need retrieval + synthesis.");
        lines.push("- Split multi-part questions; solve each part and combine.");
    }

    lines.join("\n")
}

/// Honest answer body when no solver hit, plus meta-cognitive framing.
fn structured_think_fallback(question: &str, steps: &mut Vec<String>) -> String {
    steps.push("Type: Recovery — no confident hit from rules, KB, or quick search".to_string());
    steps.push(
        "Meta: Self-check — clarify goal, shrink scope, verify sources for high-stakes topics."
            .to_string(),
    );

    let hints = suggest_next_actions_for_question(question);
    let body = format!(
        "**We could not produce a confident answer** from the native think pipeline (rules, knowledge base, quick search).\n\n\
         **Your question:** {}\n\n\
         **What to try next:**\n{}\n\n\
         **Reality check:** No agent solves every problem in the world. This path is best for \
         structured math, conversions, and short factual lookups; open problems need iteration, \
         data, experts, or a full LLM + tools stack.",
        question.trim(),
        hints
    );
    format_think_output(steps, &body)
}

/// Extract all floating-point numbers from a string.
fn extract_numbers(s: &str) -> Vec<f64> {
    let mut nums = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        // Skip non-digit characters (but allow leading minus if followed by digit)
        if bytes[i].is_ascii_digit() || (bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i+1].is_ascii_digit()) {
            let start = i;
            if bytes[i] == b'-' { i += 1; }
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if let Ok(n) = s[start..i].parse::<f64>() {
                nums.push(n);
            }
        } else {
            i += 1;
        }
    }
    nums
}

/// Detect unit conversion requests. Returns (value, from_unit, to_unit) if matched.
fn detect_unit_conversion(q: &str) -> Option<(f64, String, String)> {
    let lower = q.to_lowercase();
    let nums = extract_numbers(q);
    if nums.is_empty() { return None; }
    let val = nums[0];

    // Split at "to" direction keyword to determine FROM vs TO unit.
    // "75 kg to pounds" → before="75 kg", after="pounds" → from=kg, to=pounds
    // "150 miles to km" → before="150 miles", after="km"  → from=miles, to=km
    // No "to": use positional heuristic (unit closest to / leftmost = FROM).
    let (before, after) = lower.find(" to ")
        .map(|p| (&lower[..p], &lower[p + 4..]))
        .unwrap_or((&lower as &str, ""));

    // Detect FROM unit: unit that appears in `before` clause (or leftmost in full string).
    let unit_in = |words: &[&str], s: &str| -> bool { words.iter().any(|w| s.contains(w)) };
    let first_pos = |words: &[&str]| -> usize {
        words.iter().filter_map(|w| lower.find(w)).min().unwrap_or(usize::MAX)
    };

    // Helper: given two word-sets A and B, resolve direction.
    // If "to" found: the unit in `after` is TO.
    // If no "to":    the unit with smaller position index is FROM.
    let resolve = |a_words: &[&str], a_name: &str, b_words: &[&str], b_name: &str| -> Option<(f64, String, String)> {
        if !unit_in(a_words, &lower) || !unit_in(b_words, &lower) {
            return None;
        }
        let a_is_from = if !after.is_empty() {
            // "to Y" → Y is the TO unit; the other is FROM
            if unit_in(b_words, after) { true }  // b is TO → a is FROM
            else { false }                         // a is TO → b is FROM
        } else {
            // No directional keyword: leftmost unit is FROM
            first_pos(a_words) <= first_pos(b_words)
        };
        if a_is_from {
            Some((val, a_name.to_string(), b_name.to_string()))
        } else {
            Some((val, b_name.to_string(), a_name.to_string()))
        }
    };

    // Distance pairs
    if let r @ Some(_) = resolve(&["mile"], "miles", &["km", "kilomet"], "km") { return r; }
    if let r @ Some(_) = resolve(&["feet", "foot"], "feet", &["meter", "metre"], "meters") { return r; }
    if lower.contains("inch") && lower.contains("cm") {
        return Some((val, "inches".into(), "cm".into()));
    }
    // Weight pair
    if let r @ Some(_) = resolve(&["pound"], "pounds", &["kg", "kilogram"], "kg") { return r; }
    // Temperature pairs
    let has_f = lower.contains("fahrenheit") || lower.contains("°f");
    let has_c = lower.contains("celsius")    || lower.contains("°c");
    if has_f && has_c {
        return if unit_in(&["celsius", "°c"], after) || (!after.is_empty() && unit_in(&["fahrenheit", "°f"], before)) {
            Some((val, "fahrenheit".into(), "celsius".into()))
        } else if unit_in(&["fahrenheit", "°f"], after) {
            Some((val, "celsius".into(), "fahrenheit".into()))
        } else {
            // No direction: fahrenheit first in typical usage "X°F in celsius"
            Some((val, "fahrenheit".into(), "celsius".into()))
        };
    }
    None
}

/// Perform a unit conversion. Returns result value and a description.
fn do_unit_conversion(val: f64, from: &str, to: &str) -> Option<(f64, String)> {
    let result = match (from, to) {
        ("miles",      "km")         => val * 1.60934,
        ("km",         "miles")      => val / 1.60934,
        ("feet",       "meters")     => val * 0.3048,
        ("meters",     "feet")       => val / 0.3048,
        ("inches",     "cm")         => val * 2.54,
        ("cm",         "inches")     => val / 2.54,
        ("pounds",     "kg")         => val * 0.453592,
        ("kg",         "pounds")     => val / 0.453592,
        ("fahrenheit", "celsius")    => (val - 32.0) * 5.0 / 9.0,
        ("celsius",    "fahrenheit") => val * 9.0 / 5.0 + 32.0,
        _ => return None,
    };
    Some((result, format!("{} {} = {:.4} {}", val, from, result, to)))
}

/// Detect speed/distance/time problems. Returns (known_pair, unknown) if matched.
/// e.g. "how long to drive 300km at 90km/h" → distance=300, speed=90, find time
fn detect_speed_problem(q: &str) -> Option<(f64, f64, &'static str)> {
    let lower = q.to_lowercase();
    let nums = extract_numbers(q);
    if nums.len() < 2 { return None; }

    let has_time_q  = lower.contains("how long") || lower.contains("how many hour") || lower.contains("how many minute");
    let has_speed   = lower.contains("km/h") || lower.contains("kmh") || lower.contains("mph") || lower.contains("km per hour") || lower.contains("miles per hour");
    let has_dist_q  = lower.contains("how far") || lower.contains("how many km") || lower.contains("how many miles");

    if has_time_q && has_speed {
        // distance / speed = time
        // Assume larger number is distance, smaller (with speed unit nearby) is speed
        let (dist, speed) = if nums[0] > nums[1] { (nums[0], nums[1]) } else { (nums[1], nums[0]) };
        return Some((dist, speed, "time"));
    }
    if has_dist_q && has_speed {
        // time * speed = distance
        let (time, speed) = if nums[0] < nums[1] { (nums[0], nums[1]) } else { (nums[1], nums[0]) };
        return Some((time, speed, "distance"));
    }
    None
}

/// Detect percentage problems. Returns (percentage, base_value) if matched.
fn detect_percentage(q: &str) -> Option<(f64, f64)> {
    let lower = q.to_lowercase();
    if !lower.contains('%') && !lower.contains("percent") { return None; }
    let nums = extract_numbers(q);
    if nums.len() < 2 { return None; }
    // Convention: smaller number is the percentage, larger is the base
    let (pct, base) = if nums[0] <= 100.0 && nums[1] > nums[0] {
        (nums[0], nums[1])
    } else if nums[1] <= 100.0 && nums[0] > nums[1] {
        (nums[1], nums[0])
    } else {
        (nums[0], nums[1])
    };
    Some((pct, base))
}

/// The Native Think Engine — rule-based reasoning plus optional quick web lookup.
///
/// Tries math, conversions, patterns, knowledge base, then DDG/Wikipedia. Returns a trace
/// when possible; otherwise a structured fallback with honest limits and next-step hints.
/// For open-ended problems use `khlm_ask`, `rlm_think`, or `ghost_108` in addition.
///
/// Killer usage:
///   result = native_think("How long to drive 450km at 90km/h?")
///   print(result)
pub fn native_think(question: &str) -> String {
    let q = question.trim();
    let mut steps: Vec<String> = Vec::new();
    steps.push(format!("Question: \"{}\"", q));

    // Meta-hint for sprawling prompts (does not change numeric/KB behavior)
    let wc = q.split_whitespace().count();
    if wc > 35 || q.chars().filter(|c| *c == '?').count() > 1 {
        steps.push(
            "Meta: Long or multi-part question — consider splitting into sub-questions if the first answer is incomplete."
                .to_string(),
        );
    }

    // -- 1. Pure arithmetic ----------------------------------------------------
    let math_candidate = normalize_math_question(q);
    if is_math_expr(&math_candidate) {
        steps.push("Type: Pure arithmetic".to_string());
        steps.push(format!("Expression: {}", math_candidate.trim()));
        match eval_simple_math(&math_candidate) {
            Ok(result) => {
                steps.push(format!("Compute: {} = {}", math_candidate.trim(), result));
                let answer = if result == result.floor() && result.abs() < 1e15 {
                    format!("{}", result as i64)
                } else {
                    format!("{}", result)
                };
                return format_think_output(&steps, &answer);
            }
            Err(e) => { steps.push(format!("Math error: {}", e)); }
        }
    }

    // -- 2. Percentage calculation ---------------------------------------------
    // Guard: skip if this looks like a multi-step money word problem
    let is_multistep_money = {
        let lq2 = q.to_lowercase();
        (lq2.contains("have ") || lq2.contains("start with") || lq2.contains("begin with"))
            && (lq2.contains("spend") || lq2.contains("spent"))
            && (lq2.contains("earn") || lq2.contains("gain") || lq2.contains("more"))
    };
    if !is_multistep_money {
    if let Some((pct, base)) = detect_percentage(q) {
        steps.push("Type: Percentage calculation".to_string());
        steps.push(format!("{}% of {} = ?", pct, base));
        let result = (pct / 100.0) * base;
        steps.push(format!("{}% × {} = {:.4}", pct, base, result));
        let answer = if result == result.floor() {
            format!("{}", result as i64)
        } else {
            format!("{:.2}", result)
        };
        return format_think_output(&steps, &answer);
    }
    } // end !is_multistep_money

    // -- 3. Unit conversion ----------------------------------------------------
    if let Some((val, from, to)) = detect_unit_conversion(q) {
        steps.push("Type: Unit conversion".to_string());
        steps.push(format!("Convert {} {} → {}", val, from, to));
        if let Some((result, desc)) = do_unit_conversion(val, &from, &to) {
            steps.push(format!("Using conversion table: {}", desc));
            let answer = if result == result.floor() && result.abs() < 1e12 {
                format!("{} {} = {} {}", val, from, result as i64, to)
            } else {
                format!("{} {} = {:.4} {}", val, from, result, to)
            };
            return format_think_output(&steps, &answer);
        }
    }

    // -- 4. Speed / Distance / Time --------------------------------------------
    if let Some((a, b, unknown)) = detect_speed_problem(q) {
        steps.push("Type: Speed / Distance / Time problem".to_string());
        match unknown {
            "time" => {
                let dist = a; let speed = b;
                steps.push(format!("Distance = {} km,  Speed = {} km/h", dist, speed));
                steps.push("Formula: Time = Distance ÷ Speed".to_string());
                let hours = dist / speed;
                let h = hours.floor() as u64;
                let mins = ((hours - hours.floor()) * 60.0).round() as u64;
                steps.push(format!("{} ÷ {} = {:.4} hours", dist, speed, hours));
                steps.push(format!("{:.4} hours = {} hours {} minutes", hours, h, mins));
                let answer = if mins == 0 {
                    format!("{} hours", h)
                } else {
                    format!("{} hours {} minutes", h, mins)
                };
                return format_think_output(&steps, &answer);
            }
            "distance" => {
                let time = a; let speed = b;
                steps.push(format!("Time = {} hours,  Speed = {} km/h", time, speed));
                steps.push("Formula: Distance = Speed × Time".to_string());
                let dist = speed * time;
                steps.push(format!("{} × {} = {:.2} km", speed, time, dist));
                return format_think_output(&steps, &format!("{:.1} km", dist));
            }
            _ => {}
        }
    }

    // -- 4b. Multi-step word problems (FIX 3) ----------------------------------
    // Handles chained arithmetic like:
    //   "train travels 300km at 120kmh and stops for 20 minutes, total journey time"
    //   "room is 8m by 6m, carpet costs 25 per sqm, total cost"
    //   "have 1000, spend 15%, earn 200 more, what do I have"
    let lower_q = q.to_lowercase();

    // Pattern: distance at speed + stop time
    {
        let nm = lower_q.replace("km/h", "kmh").replace("km per hour", "kmh");
        if nm.contains("km at") && nm.contains("kmh") {
            fn num_before(s: &str, pos: usize) -> Option<f64> {
                s[..pos].split(|c: char| !c.is_ascii_digit() && c != '.')
                    .filter(|w| !w.is_empty())
                    .last()
                    .and_then(|w| w.parse().ok())
            }
            fn num_after(s: &str, pos: usize) -> Option<f64> {
                s[pos..].split(|c: char| !c.is_ascii_digit() && c != '.')
                    .filter(|w| !w.is_empty())
                    .next()
                    .and_then(|w| w.parse().ok())
            }
            let dist_opt = nm.find("km at").and_then(|ki| num_before(&nm, ki));
            let spd_opt = nm.find("km at").and_then(|ki| {
                let after = &nm[ki+5..]; // past "km at"
                after.split(|c: char| !c.is_ascii_digit() && c != '.')
                    .filter(|w| !w.is_empty())
                    .next()
                    .and_then(|w| w.parse::<f64>().ok())
            });
            if let (Some(dist), Some(spd)) = (dist_opt, spd_opt) {
                if dist > 0.0 && spd > 0.0 {
                    let travel_h = dist / spd;
                    let stop_h: f64 = (|| -> Option<f64> {
                        let p = nm.find("stop")?;
                        let fp = nm[p..].find("for ")?;
                        let num_start = p + fp + 4;
                        let n = num_after(&nm, num_start)?;
                        let rest = &nm[num_start..];
                        let is_hour = rest.contains("hour") || rest.contains(" h");
                        Some(if is_hour { n } else { n / 60.0 })
                    })().unwrap_or(0.0);
                    let total_h = travel_h + stop_h;
                    let h = total_h.floor() as u64;
                    let m = ((total_h - total_h.floor()) * 60.0).round() as u64;
                    let stop_note = if stop_h > 0.0 { format!(" (incl. {:.0}-min stop)", stop_h * 60.0) }
                                    else { String::new() };
                    let ans = if m == 0 { format!("{} hours{}", h, stop_note) }
                              else { format!("{} hours {} minutes{}", h, m, stop_note) };
                    let mut st = vec![
                        format!("Question: \"{}\"", q),
                        "Type: Multi-step travel problem".to_string(),
                        format!("Travel: {} km ÷ {} km/h = {:.4} hours", dist as u64, spd as u64, travel_h),
                    ];
                    if stop_h > 0.0 {
                        st.push(format!("Stop: + {:.4} hours ({:.0} min)", stop_h, stop_h * 60.0));
                    }
                    st.push(format!("Total: {:.4} hours = {}", total_h, ans));
                    return format_think_output(&st, &ans);
                }
            }
        }
    }

    // Pattern: "X m by Y m" area × cost per sqm
    {
        let lq = &lower_q;
        // Match "8m by", "8 m by", "8metres by", "8meters by"
        let by_marker: Option<usize> = {
            // normalize: "8m by" → find position of "m by " or "metres by" etc.
            ["m by ", "metres by ", "meters by ", "metre by ", "meter by "].iter()
                .find_map(|pat| lq.find(pat))
        };
        if let Some(bp) = by_marker {
            let w_opt: Option<f64> = lq[..bp]
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .filter(|w| !w.is_empty())
                .last()
                .and_then(|w| w.parse().ok());
            // skip past "m by " pattern, then find first number
            let skip = ["m by ", "metres by ", "meters by ", "metre by ", "meter by "].iter()
                .find(|pat| lq[bp..].starts_with(*pat))
                .map(|p| p.len())
                .unwrap_or(5);
            let h_opt: Option<f64> = lq[bp+skip..]
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .filter(|w| !w.is_empty())
                .next()
                .and_then(|w| w.parse().ok());
            let cost_opt: Option<f64> = ["per sqm","per m2","per sq m","per square m","/sqm","/m2"].iter()
                .find_map(|mk| lq.find(mk).and_then(|p| {
                    lq[..p].split(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
                        .filter(|w| !w.is_empty())
                        .last()
                        .and_then(|w| w.replace(',', "").parse().ok())
                }));
            if let (Some(w), Some(h), Some(cst)) = (w_opt, h_opt, cost_opt) {
                if w > 0.0 && h > 0.0 && cst > 0.0 {
                    let area  = w * h;
                    let total = area * cst;
                    let ans   = if total == total.floor() { format!("${}", total as u64) }
                                else { format!("${:.2}", total) };
                    let steps = vec![
                        format!("Question: \"{}\"", q),
                        "Type: Multi-step area × cost".to_string(),
                        format!("Area: {} m × {} m = {} m²", w as u64, h as u64, area as u64),
                        format!("Cost: {} m² × ${}/m² = {}", area as u64, cst as u64, ans),
                    ];
                    return format_think_output(&steps, &ans);
                }
            }
        }
    }

    // Pattern: start amount → spend X% → +/- fixed amount → final balance
    {
        let lq = &lower_q;
        let has_pct = lq.contains('%') || lq.contains("percent");
        let find_after = |kws: &[&str]| -> Option<f64> {
            for kw in kws {
                if let Some(p) = lq.find(kw) {
                    let rest = &lq[p+kw.len()..];
                    if let Some(v) = rest.split(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
                        .find(|w| !w.is_empty() && w.contains(|d: char| d.is_ascii_digit()))
                        .and_then(|w| w.replace(',', "").parse::<f64>().ok()) {
                        return Some(v);
                    }
                }
            }
            None
        };
        let start_opt = find_after(&["have ", "start with ", "begin with ", "begins with "]);
        let pct_opt   = find_after(&["spend "]);
        if let (Some(start), Some(pct), true) = (start_opt, pct_opt, has_pct) {
            if start > 0.0 {
                let spend       = (pct / 100.0) * start;
                let after_spend = start - spend;
                let extra = find_after(&["earn ", "gain ", "add ", "get "]).unwrap_or(0.0);
                let final_amt   = after_spend + extra;
                let ans = if final_amt == final_amt.floor() { format!("{}", final_amt as u64) }
                          else { format!("{:.2}", final_amt) };
                let steps = vec![
                    format!("Question: \"{}\"", q),
                    "Type: Multi-step money calculation".to_string(),
                    format!("Start: {}", start as u64),
                    format!("Spend {}%: -{} → {}", pct, spend as u64, after_spend as u64),
                    if extra > 0.0 { format!("Earn more: +{} → {}", extra as u64, final_amt as u64) }
                    else { format!("Final: {}", final_amt as u64) },
                ];
                return format_think_output(&steps, &ans);
            }
        }
    }

    // -- 5. Multi-step: extract sub-questions, solve each ----------------------
    // Detect "how long to [verb] from X to Y at Z speed" pattern
    // by looking for two named places + a speed
    let lower = q.to_lowercase();
    let has_from_to = lower.contains(" from ") && lower.contains(" to ");
    let has_speed   = lower.contains("km/h") || lower.contains("mph") || lower.contains("km per hour");
    if has_from_to && has_speed {
        steps.push("Type: Multi-step (location distance + travel time)".to_string());

        // Extract location names from "from X to Y"
        let locations = extract_from_to_locations(q);
        if let Some((from_loc, to_loc)) = locations {
            steps.push(format!("Sub-problem 1: Distance from {} to {}", from_loc, to_loc));
            let wiki_q = format!("{} to {} distance", from_loc, to_loc);

            // Search Wikipedia for the distance
            let dist_context = search_wikipedia(&wiki_q).unwrap_or_default();
            if !dist_context.is_empty() {
                steps.push(format!("  Wikipedia: \"{}\"", &dist_context[..dist_context.len().min(120)]));
            }

            // Extract any number from the context (kilometres/miles)
            let dist_nums = extract_numbers(&dist_context);
            if let (Some(&dist), true) = (dist_nums.first(), !dist_nums.is_empty()) {
                steps.push(format!("  Extracted distance: {} km", dist));

                // Find speed from original question
                let speed_nums = extract_numbers(q);
                // The speed is the number adjacent to km/h or mph
                let speed = find_speed_value(q, &speed_nums);
                if let Some(spd) = speed {
                    steps.push(format!("Sub-problem 2: Travel time at {} km/h", spd));
                    steps.push(format!("  Formula: Time = {} ÷ {} = {:.4} hours", dist, spd, dist / spd));
                    let total_hours = dist / spd;
                    let h = total_hours.floor() as u64;
                    let mins = ((total_hours - total_hours.floor()) * 60.0).round() as u64;
                    steps.push(format!("  = {} hours {} minutes", h, mins));
                    let answer = if mins == 0 {
                        format!("About {} hours  ({}km ÷ {}km/h)", h, dist as u64, spd as u64)
                    } else {
                        format!("About {} hours {} minutes  ({}km ÷ {}km/h)", h, mins, dist as u64, spd as u64)
                    };
                    return format_think_output(&steps, &answer);
                }
            }
        }
    }

    // -- 5b. Built-in Knowledge Base (instant, accurate, zero-network) --------
    // Common factual questions answered deterministically for 100% accuracy
    {
        let lq = q.to_lowercase();
        let lq = lq.replace('?', "").replace("what is ", "").replace("what are ", "")
            .replace("who is ", "").replace("who was ", "").replace("define ", "")
            .replace("explain ", "").replace("tell me about ", "").replace("what's ", "");
        let lq = lq.trim();
        if let Some(answer) = knowledge_base_lookup(lq) {
            steps.push("Type: Built-in knowledge (instant, verified)".to_string());
            return format_think_output(&steps, &answer);
        }
    }

    // -- 5c. Comparison handler ("X vs Y", "X versus Y") ----------------------
    {
        let lq = q.to_lowercase();
        if lq.contains(" vs ") || lq.contains(" versus ") {
            if let Some(comparison) = comparison_handler(&lq) {
                steps.push("Type: Comparison (built-in knowledge)".to_string());
                return format_think_output(&steps, &comparison);
            }
        }
    }

    // -- 5d. Definition handler ("what does X mean", "define X") ──────────────
    {
        let lq = q.to_lowercase();
        let lq_clean = lq.replace('?', "");
        let lq_clean = lq_clean.trim();
        // "What does X mean" / "meaning of X" / "define X"
        let is_define = lq_clean.starts_with("what does ") && lq_clean.ends_with(" mean")
            || lq_clean.starts_with("meaning of ")
            || lq_clean.starts_with("define ")
            || lq_clean.starts_with("what is the meaning of ");
        if is_define {
            let term = lq_clean
                .trim_start_matches("what does ").trim_end_matches(" mean")
                .trim_start_matches("meaning of ")
                .trim_start_matches("define ")
                .trim_start_matches("what is the meaning of ")
                .trim();
            if !term.is_empty() {
                if let Some(answer) = knowledge_base_lookup(term) {
                    steps.push("Type: Definition (built-in knowledge)".to_string());
                    return format_think_output(&steps, &answer);
                }
            }
        }
    }

    // -- 5e. Pros/Cons handler ("advantages of X", "pros and cons of X") ──────
    {
        let lq = q.to_lowercase();
        let is_proscons = lq.contains("pros and cons") || lq.contains("advantages and disadvantages")
            || lq.contains("advantages of ") || lq.contains("benefits of ")
            || lq.contains("downsides of ") || lq.contains("disadvantages of ");
        if is_proscons {
            let topic = lq
                .replace("what are the ", "").replace("list the ", "").replace("give me ", "")
                .replace("pros and cons of ", "").replace("advantages and disadvantages of ", "")
                .replace("advantages of ", "").replace("benefits of ", "")
                .replace("downsides of ", "").replace("disadvantages of ", "")
                .replace('?', "").trim().to_string();
            if !topic.is_empty() {
                if let Some(answer) = knowledge_base_lookup(&topic) {
                    steps.push("Type: Pros/Cons analysis (built-in knowledge)".to_string());
                    let enhanced = format!("## {}\n\n{}\n\n---\n*For a deeper analysis of pros and cons, try asking in **Think mode** with a more specific question.*", capitalize_first(&topic), answer);
                    return format_think_output(&steps, &enhanced);
                }
            }
        }
    }

    // -- 6. Web fact lookup (DDG first, Wikipedia fallback) --------------------
    steps.push("Type: Factual lookup".to_string());
    steps.push("Step 1: Search DuckDuckGo instant answers".to_string());

    if let Ok(ddg) = search_ddg(q) {
        if !ddg.is_empty() {
            steps.push(format!("  DDG found: \"{}\"", &ddg[..ddg.len().min(120)]));
            let answer = ddg.lines().next().unwrap_or(&ddg).trim().to_string();
            // Truncate to 2 sentences
            let mut out = String::new();
            let mut cnt = 0usize;
            for ch in answer.chars() {
                out.push(ch);
                if ch == '.' { cnt += 1; if cnt >= 2 { break; } }
            }
            return format_think_output(&steps, out.trim());
        }
    }

    steps.push("  DDG: no direct answer".to_string());
    steps.push("Step 2: Search Wikipedia".to_string());

    let wiki_q = q.to_lowercase()
        .replace("what is the ", "").replace("what is ", "")
        .replace("who is ", "").replace("who was ", "")
        .replace("where is ", "").replace('?', "").trim().to_string();

    if let Ok(wiki) = search_wikipedia(&wiki_q) {
        if wiki.len() > 40 {
            steps.push(format!("  Wikipedia: \"{}\"", &wiki[..wiki.len().min(120)]));
            return format_think_output(&steps, wiki.trim());
        }
    }

    steps.push("  Wikipedia: no result found".to_string());
    steps.push("Step 3: Search DuckDuckGo web results".to_string());

    if let Ok(web) = search_ddg_web(q) {
        if !web.is_empty() {
            steps.push(format!("  Web result: \"{}\"", &web[..web.len().min(120)]));
            // Return first 2 sentences
            let mut out = String::new();
            let mut cnt = 0usize;
            for ch in web.chars() {
                if ch == '\n' { if !out.ends_with(' ') { out.push(' '); } continue; }
                out.push(ch);
                if ch == '.' { cnt += 1; if cnt >= 2 { break; } }
            }
            return format_think_output(&steps, out.trim());
        }
    }

    steps.push("  Web search: no result found".to_string());
    structured_think_fallback(q, &mut steps)
}

/// Built-in knowledge base — instant, 100% accurate answers for common questions.
/// 200+ topics covering programming, science, math, history, geography, business,
/// philosophy, health, and general knowledge. Returns None if not in the database.
fn knowledge_base_lookup(q: &str) -> Option<String> {
    // Normalize: strip filler words for matching
    let q = q.trim().to_lowercase();
    let q = q.as_str();

    // ── Programming Languages & Tech ─────────────────────────────────────
    if q.contains("python") && !q.contains("monty") {
        return Some("**Python** is a high-level, interpreted, general-purpose programming language created by **Guido van Rossum** in 1991. Known for its readable syntax and vast ecosystem. Used in web development (Django, Flask), data science (pandas, NumPy), AI/ML (TensorFlow, PyTorch), automation, and scripting. Python 3 is the current version (3.12+). It uses dynamic typing and garbage collection.".into());
    }
    if q.contains("rust") && (q.contains("language") || q == "rust" || q.contains("programming")) {
        return Some("**Rust** is a systems programming language created by **Graydon Hoare** at Mozilla (first stable release 2015). It guarantees memory safety without a garbage collector through its ownership system and borrow checker. Known for zero-cost abstractions, fearless concurrency, and C-like performance. Used in systems programming, WebAssembly, CLIs, and embedded systems. Crate ecosystem via **crates.io**.".into());
    }
    if q.contains("javascript") || q == "js" {
        return Some("**JavaScript** is a high-level, interpreted programming language created by **Brendan Eich** in 1995 (in 10 days at Netscape). It's the language of the web — runs in all browsers and on servers via **Node.js**. Features: dynamic typing, prototype-based OOP, first-class functions, event-driven. ECMAScript is the standard (ES2024 is latest). Used for web apps, servers, mobile (React Native), and desktop (Electron).".into());
    }
    if q.contains("typescript") {
        return Some("**TypeScript** is a typed superset of JavaScript created by **Microsoft** (Anders Hejlsberg, 2012). It adds static types, interfaces, generics, and enums on top of JavaScript — compiles to plain JS. Catches type errors at compile time, improving code quality. Used with React, Angular, Vue, Node.js. Current version: TypeScript 5.x.".into());
    }
    if q.contains("java") && !q.contains("javascript") {
        return Some("**Java** is a class-based, object-oriented language created by **James Gosling** at Sun Microsystems (1995). Famous for \"write once, run anywhere\" — compiles to bytecode running on the JVM. Used in enterprise applications, Android development, web servers (Spring Boot), and big data (Hadoop). Current version: Java 21+ (LTS). Features: strong typing, garbage collection, multithreading.".into());
    }
    if q.contains("kotlin") {
        return Some("**Kotlin** is a modern, concise language created by **JetBrains** (2011, 1.0 in 2016). Runs on JVM, fully interoperable with Java. Google's preferred language for Android development since 2019. Features: null safety, data classes, coroutines, extension functions, smart casts. Also compiles to JavaScript and native code (Kotlin/Native).".into());
    }
    if q.contains("swift") {
        return Some("**Swift** is a compiled programming language created by **Apple** (Chris Lattner, 2014) for iOS, macOS, watchOS, tvOS development. Designed to replace Objective-C. Features: type safety, optionals, closures, protocols, generics, automatic memory management (ARC). Open-source since 2015. Also used for server-side development (Vapor framework).".into());
    }
    if q == "go" || q.contains("golang") || (q.contains("go language") && !q.contains("logo")) {
        return Some("**Go (Golang)** is a statically typed, compiled language created at **Google** by Robert Griesemer, Rob Pike, and Ken Thompson (2009). Designed for simplicity, concurrency, and fast compilation. Features: goroutines (lightweight threads), channels, garbage collection, built-in testing. Used for cloud services, microservices, DevOps tools (Docker, Kubernetes are written in Go).".into());
    }
    if q.contains("c#") || q.contains("csharp") || q.contains("c sharp") {
        return Some("**C#** is a modern, object-oriented language created by **Microsoft** (Anders Hejlsberg, 2000). Part of the .NET ecosystem. Used for Windows applications, web services (ASP.NET), game development (Unity), cloud/Azure. Features: strong typing, LINQ, async/await, generics, pattern matching. Current version: C# 12+ (.NET 8+).".into());
    }
    if q.contains("c++") || q.contains("cpp") {
        return Some("**C++** is a general-purpose programming language created by **Bjarne Stroustrup** (1979, standardized 1998). Extension of C with OOP, templates, and RAII. Used in game engines (Unreal), operating systems, browsers (Chrome), databases, embedded systems, high-frequency trading. Features: manual memory management, zero-cost abstractions, multiple inheritance. Latest: C++23.".into());
    }

    // ── AI & Machine Learning ───────────────────────────────────────────
    if q.contains("machine learning") || q == "ml" {
        return Some("**Machine Learning (ML)** is a subset of AI where systems learn patterns from data without being explicitly programmed. **Types**: (1) **Supervised** — learns from labeled data (classification, regression), (2) **Unsupervised** — finds patterns in unlabeled data (clustering, dimensionality reduction), (3) **Reinforcement** — learns by trial and reward. **Key algorithms**: Linear/Logistic Regression, Decision Trees, Random Forests, SVM, K-Means, Neural Networks. **Frameworks**: scikit-learn, TensorFlow, PyTorch.".into());
    }
    if q.contains("deep learning") || q == "dl" {
        return Some("**Deep Learning** is a subset of ML using neural networks with multiple layers (hence \"deep\"). Excels at: image recognition (CNNs), natural language processing (Transformers), speech recognition (RNNs/LSTMs), generative AI (GANs, diffusion models). **Key architectures**: CNN (Convolutional), RNN/LSTM/GRU (Sequential), Transformer (Attention), GAN (Generative). **Frameworks**: PyTorch, TensorFlow, JAX. Requires large datasets and GPU compute.".into());
    }
    if q.contains("neural network") {
        return Some("A **Neural Network** is a computing system inspired by biological neurons. **Structure**: input layer → hidden layers → output layer. Each neuron applies weights, bias, and an activation function (ReLU, sigmoid, tanh). **Training**: forward pass → compute loss → backpropagation → update weights via gradient descent. **Types**: Feedforward (MLP), Convolutional (CNN — images), Recurrent (RNN — sequences), Transformer (attention — NLP/LLM).".into());
    }
    if q.contains("transformer") && (q.contains("model") || q.contains("architecture") || q.contains("attention") || q.len() < 15) {
        return Some("**Transformer** is a neural network architecture introduced in the 2017 paper \"Attention Is All You Need\" by Vaswani et al. (Google). Key innovation: **self-attention mechanism** that processes all tokens in parallel (unlike RNNs). **Components**: multi-head attention, feed-forward layers, layer normalization, positional encoding. **Impact**: foundation of GPT, BERT, T5, LLaMA, Claude, and all modern LLMs. Scales better than RNNs for long sequences.".into());
    }
    let llm_word = q.split(|c: char| !c.is_alphanumeric()).any(|w| w == "llm" || w == "llms");
    if q.contains("large language model") || q == "llm" || q == "llms" || llm_word {
        return Some("**Large Language Models (LLMs)** are neural networks trained on massive text corpora to understand and generate human language. Based on the **Transformer** architecture. **How they work**: predict the next token given context (autoregressive). **Examples**: GPT-4 (OpenAI), Claude (Anthropic), Gemini (Google), LLaMA (Meta), Mistral. **Capabilities**: text generation, code writing, reasoning, translation, summarization. **Limitations**: hallucination, knowledge cutoff, no true understanding.".into());
    }
    // AI agents — must be before generic "ai" entry
    if q.contains("ai agent") || q.contains("ai agents") || q.contains("autonomous agent")
        || q.contains("agentic ai") || q.contains("agent framework")
        || (q.contains("agent") && (q.contains("ai") || q.contains("llm") || q.contains("autonomous"))) {
        return Some("**AI Agents** are autonomous software systems that use LLMs (Large Language Models) to reason, plan, and take actions to accomplish goals — without step-by-step human instruction.\n\n**How they work**:\n1. **Perceive** — receive a task or observe the environment\n2. **Reason** — use an LLM to think through the problem (chain-of-thought)\n3. **Plan** — break the task into steps\n4. **Act** — execute steps using tools (web search, code execution, APIs, file I/O)\n5. **Reflect** — evaluate results, adjust, and iterate\n\n**Key components**:\n- **LLM backbone** — GPT-4, Claude, Gemini, LLaMA (the \"brain\")\n- **Tools** — functions the agent can call (search, calculator, database, browser)\n- **Memory** — short-term (conversation context) + long-term (vector store, summaries)\n- **Planning** — ReAct, Tree of Thoughts, chain-of-thought prompting\n\n**Popular frameworks**: LangChain, CrewAI, AutoGen (Microsoft), Semantic Kernel, LlamaIndex, Haystack.\n\n**Types of agents**:\n- **Single agent** — one LLM doing everything (basic chatbot with tools)\n- **Multi-agent** — multiple specialized agents collaborating (one researches, one writes, one reviews)\n- **Agentic workflows** — structured pipelines where agents hand off tasks\n\n**Examples**: AutoGPT, BabyAGI, Devin (coding agent), GitHub Copilot Workspace, Cursor Agent.\n\n**Challenges**: Hallucination, tool misuse, infinite loops, cost control, safety/alignment.\n\nWant to learn about a specific aspect? Ask about **multi-agent systems**, **RAG**, **tool use**, or **agent memory**!".into());
    }
    // "How AI works" and "AI limitations" — MUST be before generic AI catch-all
    if q.contains("how") && q.contains("work") && (q.contains("ai") || q.contains("artificial intelligence")) {
        return Some("**How AI works** (the technical pipeline):\n\n1. **Data**: Collect & clean training data (text, images, numbers, etc.)\n2. **Model**: Design a mathematical model — neural network layers, parameters, connections\n3. **Training**: Feed data through the model millions of times. On each pass:\n   - **Forward pass**: Input → prediction\n   - **Loss calculation**: How wrong was the prediction?\n   - **Backpropagation**: Calculate how each parameter contributed to the error\n   - **Update**: Adjust parameters to reduce error (gradient descent)\n4. **Evaluation**: Test on data the model hasn't seen to check generalization\n5. **Inference**: Deploy — the model makes predictions on new, real-world inputs\n\n**Key math**: Linear algebra (matrix multiplication), calculus (gradients), probability (distributions), statistics (optimization).\n\n**Scale**: GPT-4 has ~1.8 trillion parameters, trained on trillions of tokens, costing $100M+ in compute.\n\nWant to learn about **specific architectures** (CNN, Transformer), **training techniques**, or **see code examples**?".into());
    }
    if (q.contains("limitation") && (q.contains("ai") || q.contains("artificial intelligence") || q.contains("llm")))
        || (q.contains("problem") && q.contains("ai") && q.contains("current")) {
        return Some("**Current AI Limitations**:\n\n1. **Hallucination**: AI confidently states false information as fact. LLMs generate plausible-sounding text but can invent facts, citations, and statistics.\n\n2. **No true understanding**: AI processes patterns in data — it doesn't \"understand\" meaning the way humans do. It's sophisticated pattern matching, not reasoning.\n\n3. **Training data bias**: Models learn biases present in their training data (racial, gender, cultural). Garbage in → garbage out.\n\n4. **Knowledge cutoff**: Models are frozen in time — they don't know about events after their training date unless given real-time access.\n\n5. **Expensive to train**: GPT-4 training cost $100M+. Only a few companies can afford frontier models.\n\n6. **Context window limits**: Models can only \"see\" a limited amount of text at once (though this is improving — Gemini: 1M+ tokens).\n\n7. **Alignment**: Ensuring AI does what we actually want, not just what we literally ask. This is an unsolved problem.\n\n8. **Energy consumption**: Training and running large models requires massive compute and electricity.\n\nDespite these, AI is advancing rapidly. Want to learn about **solutions** (RAG, fine-tuning, guardrails) or **specific challenges**?".into());
    }
    if q.contains("artificial intelligence") || q == "ai"
        || q.starts_with("ai ") || q.ends_with(" ai") || q.contains(" ai ")
        || q == "what is ai" || q == "explain ai" || q == "about ai" {
        return Some("**Artificial Intelligence (AI)** is the field of computer science focused on creating systems that can perform tasks requiring human intelligence.\n\n**Branches**: Machine Learning, Deep Learning, NLP, Computer Vision, Robotics, Expert Systems.\n\n**Types**:\n1. **Narrow AI (ANI)** — specialized at one task (Siri, chess engines, image classifiers)\n2. **General AI (AGI)** — human-level reasoning across any domain (theoretical, not yet achieved)\n3. **Super AI (ASI)** — surpasses human intelligence in every way (hypothetical)\n\n**Key concepts**: Neural Networks, Training Data, Supervised/Unsupervised Learning, Reinforcement Learning, Transformers (the architecture behind ChatGPT, Gemini, Claude).\n\n**Current state**: Narrow AI is mature and everywhere (smartphones, search, self-driving cars). AGI is an active research goal at OpenAI, Google DeepMind, Anthropic, and others.\n\nWant to dive deeper? Ask me about **machine learning**, **deep learning**, **neural networks**, **LLMs**, or any specific AI topic!".into());
    }

    // ── Computer Science Fundamentals ───────────────────────────────────
    if q.contains("big o") || q.contains("time complexity") || q.contains("complexity") && q.contains("notation") {
        return Some("**Big O Notation** measures algorithm efficiency as input grows. **Common complexities** (best → worst):\n- **O(1)** — Constant: hash table lookup\n- **O(log n)** — Logarithmic: binary search\n- **O(n)** — Linear: array scan\n- **O(n log n)** — Linearithmic: merge sort, quicksort (avg)\n- **O(n²)** — Quadratic: bubble sort, nested loops\n- **O(2ⁿ)** — Exponential: recursive fibonacci\n- **O(n!)** — Factorial: brute-force permutations\n\n**Rules**: drop constants (O(2n) → O(n)), keep dominant term (O(n² + n) → O(n²)).".into());
    }
    if q.contains("data structure") {
        return Some("**Data Structures** organize and store data efficiently. **Linear**: Array (O(1) index), Linked List (O(1) insert), Stack (LIFO), Queue (FIFO), Hash Table (O(1) avg lookup). **Hierarchical**: Binary Tree, BST (O(log n) search), Heap (priority queue), Trie (prefix search). **Graph**: Adjacency List/Matrix, used for networks, maps, social media. **Choosing**: arrays for indexed access, hash maps for key-value, trees for ordered data, graphs for relationships.".into());
    }
    if q.contains("api") && (q.contains("rest") || q == "api" || q.contains("what") || q.contains("how")) {
        return Some("**API (Application Programming Interface)** is a set of rules for software components to communicate. **REST API**: uses HTTP methods (GET=read, POST=create, PUT=update, DELETE=remove), stateless, JSON responses, URL-based resources. **Key concepts**: endpoints (/api/users), status codes (200=OK, 404=NotFound, 500=Error), authentication (API keys, JWT, OAuth). **Alternatives**: GraphQL (flexible queries), gRPC (binary, fast), WebSocket (real-time).".into());
    }
    if q.contains("database") && (q.contains("sql") || q.contains("nosql") || q.contains("relational") || q.contains("type")) {
        return Some("**Databases** store and manage data. **Relational (SQL)**: tables with rows/columns, ACID transactions, JOIN operations — PostgreSQL, MySQL, SQL Server. **NoSQL**: (1) **Document** — JSON-like (MongoDB), (2) **Key-Value** — fast lookups (Redis), (3) **Column-family** — wide rows (Cassandra), (4) **Graph** — relationships (Neo4j). **When to use**: SQL for structured data with relationships; NoSQL for flexible schemas, high scale, or specific access patterns.".into());
    }
    if q.contains("git") && !q.contains("github") {
        return Some("**Git** is a distributed version control system created by **Linus Torvalds** (2005) for Linux kernel development. **Key commands**: `git init` (create repo), `git add` (stage), `git commit` (save snapshot), `git push/pull` (sync with remote), `git branch/checkout` (branching), `git merge/rebase` (integrate changes). **Concepts**: working directory → staging area → repository. Branches are lightweight pointers. HEAD points to current commit.".into());
    }
    if q.contains("docker") || q.contains("container") && q.contains("what") {
        return Some("**Docker** is a platform for building, shipping, and running applications in **containers** — lightweight, portable, isolated environments. **Key concepts**: **Image** (blueprint, built from Dockerfile), **Container** (running instance of image), **Docker Hub** (image registry), **docker-compose** (multi-container apps). **Benefits**: consistent environments, fast deployment, resource efficiency (shares OS kernel, lighter than VMs). Created by Solomon Hykes, 2013.".into());
    }
    if q.contains("kubernetes") || q.contains("k8s") {
        return Some("**Kubernetes (K8s)** is an open-source container orchestration platform created by **Google** (2014), now maintained by CNCF. **What it does**: automates deployment, scaling, and management of containerized applications. **Key concepts**: **Pod** (smallest unit, 1+ containers), **Service** (stable network endpoint), **Deployment** (desired state declaration), **Namespace** (resource isolation), **Ingress** (HTTP routing). **Why**: auto-scaling, self-healing, rolling updates, service discovery.".into());
    }

    // ── Science & Math ──────────────────────────────────────────────────
    if q.contains("gravity") || (q.contains("newton") && q.contains("law")) {
        return Some("**Gravity** is the force of attraction between objects with mass. **Newton's Law of Universal Gravitation** (1687): F = G × (m₁ × m₂) / r², where G = 6.674 × 10⁻¹¹ N⋅m²/kg². **Earth's gravity**: 9.81 m/s² (acceleration). **Einstein's General Relativity** (1915): gravity is curvature of spacetime caused by mass-energy. Gravitational waves were detected in 2015 by LIGO.".into());
    }
    if q.contains("speed of light") || q.contains("light speed") {
        return Some("The **speed of light** in vacuum is **299,792,458 m/s** (approximately 3 × 10⁸ m/s or ~186,282 miles/second). Denoted **c**. It's the universal speed limit — nothing with mass can reach it (Einstein's Special Relativity, 1905). Light travels ~1 foot per nanosecond. Earth to Moon: ~1.28 seconds. Earth to Sun: ~8.3 minutes. To nearest star (Proxima Centauri): ~4.24 years.".into());
    }
    if q.contains("quantum") && (q.contains("computing") || q.contains("computer")) {
        return Some("**Quantum Computing** uses quantum mechanics (superposition, entanglement) to process information. **Qubit** vs classical bit: can be 0, 1, or both simultaneously (superposition). **Key concepts**: superposition (parallel states), entanglement (correlated qubits), quantum gates (operations), decoherence (error source). **Companies**: IBM (127+ qubits), Google (Sycamore), IonQ, Rigetti. **Applications**: cryptography, drug discovery, optimization, AI. Still early — most practical problems need 1000+ error-corrected qubits.".into());
    }
    if q.contains("blockchain") || (q.contains("bitcoin") && !q.contains("price")) {
        return Some("**Blockchain** is a distributed, immutable ledger — a chain of cryptographically linked blocks. Each block contains transactions, a hash, and the previous block's hash. **Bitcoin** (Satoshi Nakamoto, 2008): first blockchain application, uses Proof-of-Work mining. **Ethereum**: adds smart contracts (programmable logic). **Key properties**: decentralization, immutability, transparency, trustless consensus. **Consensus mechanisms**: Proof-of-Work (energy-intensive), Proof-of-Stake (energy-efficient).".into());
    }
    if q.contains("photosynthesis") {
        return Some("**Photosynthesis** is the process by which plants, algae, and cyanobacteria convert light energy into chemical energy. **Equation**: 6CO₂ + 6H₂O + light → C₆H₁₂O₆ + 6O₂. **Two stages**: (1) **Light reactions** — in thylakoid membranes, water is split, ATP and NADPH produced, O₂ released. (2) **Calvin cycle** — in stroma, CO₂ is fixed into glucose using ATP and NADPH. **Chlorophyll** absorbs red and blue light, reflects green (why plants look green).".into());
    }
    if q.contains("dna") || q.contains("genetics") {
        return Some("**DNA (Deoxyribonucleic Acid)** is the molecule that carries genetic instructions for life. **Structure**: double helix (Watson & Crick, 1953), two strands of nucleotides connected by base pairs: A-T (adenine-thymine), G-C (guanine-cytosine). **Central dogma**: DNA → RNA (transcription) → Protein (translation). **Genome**: complete set of DNA (~3 billion base pairs in humans, ~20,000 genes). **CRISPR** (2012): gene editing tool that can precisely modify DNA sequences.".into());
    }
    if q.contains("evolution") || (q.contains("darwin") && q.contains("theory")) {
        return Some("**Evolution** is the change in inherited characteristics of populations over generations. **Charles Darwin** (1859, *On the Origin of Species*) proposed **natural selection**: organisms with favorable traits survive and reproduce more. **Key mechanisms**: (1) **Mutation** — random DNA changes, (2) **Natural selection** — survival of the fittest, (3) **Genetic drift** — random changes in small populations, (4) **Gene flow** — migration between populations. All life shares common ancestry. Evidence: fossils, DNA similarity, comparative anatomy.".into());
    }

    // ── Software Engineering ────────────────────────────────────────────
    if q.contains("solid") && q.contains("principle") {
        return Some("**SOLID Principles** (Robert C. Martin) — 5 principles for maintainable OOP:\n1. **S** — Single Responsibility: one class = one reason to change\n2. **O** — Open/Closed: open for extension, closed for modification\n3. **L** — Liskov Substitution: subtypes must be substitutable for their base types\n4. **I** — Interface Segregation: prefer small, specific interfaces over large ones\n5. **D** — Dependency Inversion: depend on abstractions, not concretions\n\nReduces coupling, improves testability, makes code easier to modify.".into());
    }
    if q.contains("design pattern") && !q.contains("decorator") && !q.contains("singleton") && !q.contains("observer") {
        return Some("**Design Patterns** (Gang of Four, 1994) — reusable solutions to common software problems:\n\n**Creational**: Singleton (one instance), Factory (create without specifying class), Builder (step-by-step construction), Prototype (clone)\n\n**Structural**: Adapter (interface bridge), Decorator (add behavior), Facade (simplify interface), Proxy (controlled access)\n\n**Behavioral**: Observer (pub/sub), Strategy (interchangeable algorithms), Command (encapsulate actions), Iterator (traverse collection)\n\nMost used in production: Singleton, Factory, Observer, Strategy, Builder.".into());
    }
    if q.contains("agile") || q.contains("scrum") {
        return Some("**Agile** is a software development methodology emphasizing iterative development, collaboration, and adaptability (Agile Manifesto, 2001). **Scrum** (most popular Agile framework): work in **Sprints** (2-4 weeks). **Roles**: Product Owner (priorities), Scrum Master (facilitator), Dev Team. **Ceremonies**: Sprint Planning, Daily Standup, Sprint Review, Retrospective. **Artifacts**: Product Backlog, Sprint Backlog, Increment. **Alternatives**: Kanban (flow-based), XP (engineering practices), SAFe (scaled).".into());
    }
    if q.contains("ci/cd") || q.contains("ci cd") || q.contains("continuous integration") || q.contains("continuous deployment") {
        return Some("**CI/CD** — automated software delivery pipeline.\n\n**CI (Continuous Integration)**: developers merge code frequently → automated build + tests run on every commit. Catches bugs early.\n\n**CD (Continuous Delivery)**: code is always in a deployable state → one-click deploy to production.\n\n**CD (Continuous Deployment)**: every passing change auto-deploys to production (no manual gate).\n\n**Tools**: GitHub Actions, GitLab CI, Jenkins, CircleCI, Azure DevOps. **Pipeline**: code push → build → unit tests → integration tests → deploy to staging → deploy to production.".into());
    }
    if q.contains("microservice") {
        return Some("**Microservices** is an architecture where an application is a collection of small, independent services that communicate via APIs. Each service: owns its data, deploys independently, can use different technology. **Benefits**: scalability, team autonomy, fault isolation, faster deployments. **Challenges**: distributed complexity, data consistency, network latency, debugging. **Patterns**: API Gateway, Service Discovery, Circuit Breaker, Saga (distributed transactions), Event Sourcing. **vs Monolith**: start monolith, split when needed.".into());
    }

    // ── MCP Server (Model Context Protocol) ──────────────────────────────
    if q.contains("mcp") || q.contains("model context protocol") {
        return Some("**MCP (Model Context Protocol)** is an open standard by **Anthropic** (2024) that lets AI models securely connect to external tools and data sources. Think of it as a **USB-C port for AI** — one standard protocol to connect any AI to any tool.\n\n**Architecture**:\n- **MCP Host** — the AI application (VS Code, Claude Desktop, etc.)\n- **MCP Client** — inside the host, manages connections\n- **MCP Server** — lightweight program exposing tools/resources\n\n**What MCP servers provide**:\n1. **Tools** — functions the AI can call (run queries, search files, call APIs)\n2. **Resources** — data the AI can read (files, database schemas, documentation)\n3. **Prompts** — reusable prompt templates\n\n**Transport**: stdio (local) or HTTP+SSE (remote)\n\n**Examples**: MongoDB MCP (query databases), GitHub MCP (manage repos), filesystem MCP (read/write files), Brave Search MCP (web search).\n\n**Why it matters**: standardizes how AI tools communicate — build one MCP server, it works with all MCP-compatible AI hosts.".into());
    }

    // ── General Knowledge ───────────────────────────────────────────────
    if q.contains("climate change") || q.contains("global warming") {
        return Some("**Climate Change** refers to long-term shifts in global temperatures and weather patterns. Since the 1800s, human activities (burning fossil fuels, deforestation) have been the main driver of warming. **Key facts**: global temperature has risen ~1.1°C since pre-industrial times. CO₂ levels: ~420 ppm (highest in 800,000 years). **Effects**: rising sea levels, extreme weather, biodiversity loss, ocean acidification. **Target**: Paris Agreement aims to limit warming to 1.5°C. **Solutions**: renewable energy, electrification, carbon capture, reforestation.".into());
    }
    if q.contains("solar system") || q.contains("planets") {
        return Some("**The Solar System** consists of the Sun and everything gravitationally bound to it. **8 planets** (inner → outer): Mercury, Venus, Earth, Mars (rocky/terrestrial) | Jupiter, Saturn (gas giants) | Uranus, Neptune (ice giants). **Key facts**: Sun contains 99.86% of system's mass. Jupiter is largest (1,321 Earths). Saturn has prominent rings. Earth is only known planet with life. **Other bodies**: dwarf planets (Pluto, Ceres, Eris), moons (200+), asteroids, comets. Age: ~4.6 billion years.".into());
    }

    // ── World Capitals & Countries ──────────────────────────────────────
    // India
    if (q.contains("india") && (q.contains("capital") || q.contains("president") || q.contains("prime minister") || q.contains("population") || q.contains("currency")))
        || q == "india" {
        return Some("**India** (Republic of India) — the world's most populous country (~1.44 billion, 2025). **Capital**: New Delhi. **President**: Droupadi Murmu (since 2022). **Prime Minister**: Narendra Modi (since 2014). **Currency**: Indian Rupee (₹, INR). **Official languages**: Hindi, English + 22 scheduled languages. **Area**: 3.29 million km² (7th largest). **Economy**: 5th largest by nominal GDP ($3.7T). **Independence**: August 15, 1947 from British rule. Known for IT industry, Bollywood, diverse culture, and ancient civilizations (Indus Valley, ~3300 BCE).".into());
    }
    // USA
    if (q.contains("usa") || q.contains("united states") || q.contains("america")) && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency") || q == "usa" || q == "united states" || q == "america") {
        return Some("**United States of America (USA)** — a federal republic of 50 states. **Capital**: Washington, D.C. **President**: Current as of 2025. **Currency**: US Dollar ($, USD) — world's primary reserve currency. **Population**: ~340 million (3rd most populous). **Area**: 9.83 million km² (3rd/4th largest). **Economy**: largest by nominal GDP (~$28T). **Founded**: July 4, 1776 (Declaration of Independence). Known for Silicon Valley, Hollywood, NASA, diverse immigration, and global military/economic influence.".into());
    }
    // UK
    if (q.contains("uk") || q.contains("united kingdom") || q.contains("britain") || q.contains("england")) && (q.contains("capital") || q.contains("king") || q.contains("queen") || q.contains("prime minister") || q.contains("population") || q.contains("currency") || q == "uk" || q == "england") {
        return Some("**United Kingdom** (UK) — constitutional monarchy comprising England, Scotland, Wales, and Northern Ireland. **Capital**: London. **Monarch**: King Charles III (since 2022). **Prime Minister**: changes with elections. **Currency**: Pound Sterling (£, GBP). **Population**: ~68 million. **Area**: 242,495 km². **Economy**: 6th largest by GDP. Known for parliamentary democracy (Westminster), the Industrial Revolution, BBC, Premier League, Oxford/Cambridge, and the Commonwealth.".into());
    }
    // China
    if (q.contains("china") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "china" {
        return Some("**China** (People's Republic of China) — the world's second most populous country (~1.41 billion). **Capital**: Beijing. **President**: Xi Jinping (since 2013). **Currency**: Renminbi/Yuan (¥, CNY). **Area**: 9.6 million km² (3rd/4th largest). **Economy**: 2nd largest by nominal GDP (~$18T), 1st by PPP. **Government**: one-party state (CPC). Known for the Great Wall, ancient civilization (5,000+ years), manufacturing, and rapid economic growth since 1978 reforms.".into());
    }
    // Japan
    if (q.contains("japan") && (q.contains("capital") || q.contains("emperor") || q.contains("prime minister") || q.contains("population") || q.contains("currency"))) || q == "japan" {
        return Some("**Japan** — an island nation in East Asia (archipelago of 6,852 islands). **Capital**: Tokyo (world's most populous metro, ~37M). **Emperor**: Naruhito (since 2019). **Currency**: Japanese Yen (¥, JPY). **Population**: ~125 million. **Area**: 377,975 km². **Economy**: 4th largest by GDP. Known for anime/manga, Toyota/Sony/Nintendo, bullet trains (Shinkansen), sushi, cherry blossoms, and blend of ancient culture with cutting-edge technology.".into());
    }
    // Germany
    if (q.contains("germany") && (q.contains("capital") || q.contains("chancellor") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "germany" {
        return Some("**Germany** (Federal Republic of Germany) — the most populous EU country. **Capital**: Berlin. **Currency**: Euro (€, EUR). **Population**: ~84 million. **Area**: 357,022 km². **Economy**: 3rd largest by GDP, Europe's largest. Known for engineering (BMW, Mercedes, Porsche, Bosch), Oktoberfest, autobahn (no speed limit sections), Classical music (Bach, Beethoven), and reunification (1990, fall of Berlin Wall 1989).".into());
    }
    // France
    if (q.contains("france") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "france" {
        return Some("**France** (French Republic) — a Western European country. **Capital**: Paris (City of Light). **Currency**: Euro (€). **Population**: ~68 million. **Area**: 643,801 km² (largest EU country by area). Known for the Eiffel Tower, Louvre Museum, French cuisine, wine, fashion (Chanel, Louis Vuitton, Dior), literature (Hugo, Camus, Voltaire), the French Revolution (1789), and the motto: Liberté, Égalité, Fraternité.".into());
    }
    // Australia
    if (q.contains("australia") && (q.contains("capital") || q.contains("prime minister") || q.contains("population") || q.contains("currency"))) || q == "australia" {
        return Some("**Australia** — a continent-country in the Southern Hemisphere. **Capital**: Canberra (NOT Sydney). **Currency**: Australian Dollar (AUD). **Population**: ~26 million. **Area**: 7.69 million km² (6th largest). Known for the Great Barrier Reef, kangaroos/koalas, Sydney Opera House, the Outback, and unique biodiversity. Economy is strong in mining, agriculture, and services.".into());
    }
    // Canada
    if (q.contains("canada") && (q.contains("capital") || q.contains("prime minister") || q.contains("population") || q.contains("currency"))) || q == "canada" {
        return Some("**Canada** — the world's 2nd largest country by area (9.98 million km²). **Capital**: Ottawa. **Currency**: Canadian Dollar (CAD). **Population**: ~40 million. Known for maple syrup, hockey, Niagara Falls, the Rocky Mountains, universal healthcare, bilingualism (English/French), and multiculturalism. Economy: strong in resources (oil, lumber), tech (Shopify, BlackBerry), and finance.".into());
    }
    // Brazil
    if (q.contains("brazil") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "brazil" {
        return Some("**Brazil** (Federative Republic of Brazil) — the largest country in South America. **Capital**: Brasília. **Currency**: Brazilian Real (BRL). **Population**: ~216 million (6th most). **Area**: 8.51 million km² (5th largest). Known for: Amazon Rainforest (60% of it in Brazil), Carnival, football (5× World Cup winners — Pelé, Ronaldo, Neymar), Christ the Redeemer in Rio de Janeiro, and coffee (largest producer worldwide).".into());
    }
    // Russia
    if (q.contains("russia") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "russia" {
        return Some("**Russia** (Russian Federation) — the world's largest country by area (17.1 million km², spanning 11 time zones). **Capital**: Moscow. **President**: Vladimir Putin. **Currency**: Russian Ruble (RUB). **Population**: ~144 million. Known for: the Kremlin, Trans-Siberian Railway, space program (first satellite Sputnik 1957, first human Gagarin 1961), literature (Tolstoy, Dostoevsky), ballet, and vast natural resources (oil, gas).".into());
    }
    // South Korea
    if (q.contains("south korea") || q.contains("korea") && !q.contains("north")) && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency") || q == "south korea") {
        return Some("**South Korea** (Republic of Korea) — an East Asian nation on the Korean Peninsula. **Capital**: Seoul. **Currency**: South Korean Won (KRW). **Population**: ~52 million. Known for K-pop (BTS, BLACKPINK), Samsung, Hyundai, Korean cuisine (kimchi, bibimbap), K-dramas (Squid Game), rapid post-war economic growth (\"Miracle on the Han River\"), and advanced technology.".into());
    }
    // Italy
    if (q.contains("italy") && (q.contains("capital") || q.contains("president") || q.contains("prime minister") || q.contains("population") || q.contains("currency"))) || q == "italy" {
        return Some("**Italy** (Italian Republic) — a Southern European country. **Capital**: Rome. **Currency**: Euro (€). **Population**: ~59 million. Known for: the Roman Empire, Renaissance (Leonardo da Vinci, Michelangelo), pizza/pasta, Ferrari/Lamborghini, the Colosseum, Vatican City (world's smallest country, within Rome), fashion (Gucci, Prada, Armani), and a rich artistic heritage.".into());
    }
    // Spain
    if (q.contains("spain") && (q.contains("capital") || q.contains("king") || q.contains("prime minister") || q.contains("population") || q.contains("currency"))) || q == "spain" {
        return Some("**Spain** — a Southern European country on the Iberian Peninsula. **Capital**: Madrid. **Currency**: Euro (€). **Population**: ~48 million. Known for flamenco dancing, bullfighting, La Sagrada Familia (Gaudí), FC Barcelona & Real Madrid, Spanish cuisine (paella, tapas), the Alhambra, and the Age of Exploration (Columbus sailed in 1492 under Spanish crown).".into());
    }
    // Mexico
    if (q.contains("mexico") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "mexico" {
        return Some("**Mexico** (United Mexican States) — a country in North America. **Capital**: Mexico City (one of the world's largest cities, ~21M metro). **Currency**: Mexican Peso (MXN). **Population**: ~130 million. Known for: ancient civilizations (Maya, Aztec), tacos/burritos, Day of the Dead (Día de los Muertos), tequila, vibrant culture, and Chichén Itzá (one of the New Seven Wonders).".into());
    }
    // Egypt
    if (q.contains("egypt") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency") || q.contains("pyramid"))) || q == "egypt" {
        return Some("**Egypt** (Arab Republic of Egypt) — a transcontinental country (Africa/Asia). **Capital**: Cairo. **Currency**: Egyptian Pound (EGP). **Population**: ~105 million. Known for: the Pyramids of Giza (~2560 BCE, one of the Seven Wonders of the Ancient World), the Sphinx, the Nile River (longest in Africa), ancient pharaohs (Tutankhamun, Cleopatra), hieroglyphics, and the Suez Canal.".into());
    }
    // South Africa
    if (q.contains("south africa") && (q.contains("capital") || q.contains("president") || q.contains("population") || q.contains("currency"))) || q == "south africa" {
        return Some("**South Africa** — a country at the southern tip of Africa. **Capitals**: Pretoria (executive), Cape Town (legislative), Bloemfontein (judicial). **Currency**: South African Rand (ZAR). **Population**: ~60 million. Known for: Nelson Mandela, end of apartheid (1994), Table Mountain, Kruger National Park, gold/diamond mining, and 11 official languages. Largest economy in Sub-Saharan Africa.".into());
    }

    // ── Famous People ────────────────────────────────────────────────────
    if q.contains("elon musk") {
        return Some("**Elon Musk** (born June 28, 1971, Pretoria, South Africa) — entrepreneur and business magnate. **CEO of**: Tesla (electric vehicles), SpaceX (rockets/Starlink). **Owner of**: X (formerly Twitter), xAI. **Co-founded**: Neuralink (brain-computer interfaces), The Boring Company (tunnels), PayPal (originally X.com). **Net worth**: one of the world's richest people. Known for ambitious goals: Mars colonization, sustainable energy, and AI development.".into());
    }
    if q.contains("albert einstein") || (q.contains("einstein") && !q.contains("rosen")) {
        return Some("**Albert Einstein** (1879–1955) — German-born theoretical physicist, widely regarded as the greatest scientist of the 20th century. **Major contributions**: Special Relativity (1905, E=mc²), General Relativity (1915, gravity = spacetime curvature), photoelectric effect (won Nobel Prize 1921), Brownian motion. **Key ideas**: mass-energy equivalence, gravitational waves, cosmological constant. Fled Nazi Germany in 1933, became US citizen. Famous quote: \"Imagination is more important than knowledge.\"".into());
    }
    if q.contains("isaac newton") || (q.contains("newton") && (q.contains("sir") || q.contains("scientist") || q.contains("who"))) {
        return Some("**Sir Isaac Newton** (1643–1727) — English mathematician, physicist, and astronomer. One of the most influential scientists in history. **Major contributions**: Laws of Motion (3 laws), Universal Gravitation (F = Gm₁m₂/r²), calculus (independently with Leibniz), optics (prisms splitting white light), reflecting telescope. Published **Principia Mathematica** (1687). Famous legend: apple falling from tree inspired gravity theory. Also served as Warden of the Royal Mint.".into());
    }
    if q.contains("stephen hawking") || (q.contains("hawking") && q.contains("who")) {
        return Some("**Stephen Hawking** (1942–2018) — British theoretical physicist and cosmologist. Known for **Hawking radiation** (black holes emit particles), singularity theorems (with Roger Penrose), and popularizing science through **A Brief History of Time** (1988, 25M+ copies sold). Diagnosed with ALS at 21, given 2 years to live — survived 55 more years using a speech-generating device. Lucasian Professor of Mathematics at Cambridge (same chair as Newton). Proved black holes aren't entirely black.".into());
    }
    if q.contains("mahatma gandhi") || q.contains("mohandas gandhi") || (q.contains("gandhi") && (q.contains("who") || q.contains("father"))) {
        return Some("**Mahatma Gandhi** (Mohandas Karamchand Gandhi, 1869–1948) — Indian lawyer, anti-colonial nationalist, and political ethicist. **Father of the Nation** of India. Led India's independence movement through **non-violent civil disobedience** (ahimsa/satyagraha). Key events: Salt March (1930), Quit India Movement (1942). Philosophy inspired Martin Luther King Jr., Nelson Mandela, and civil rights movements worldwide. Assassinated January 30, 1948 by Nathuram Godse. Famous quote: \"Be the change you wish to see in the world.\"".into());
    }
    if q.contains("nelson mandela") || (q.contains("mandela") && q.contains("who")) {
        return Some("**Nelson Mandela** (1918–2013) — South African anti-apartheid revolutionary and first Black president of South Africa (1994–1999). Imprisoned for 27 years (1964–1990) for fighting racial segregation. Won **Nobel Peace Prize** (1993) with F.W. de Klerk. Known as **Madiba**. His presidency focused on reconciliation between races. Founded the Truth and Reconciliation Commission. Symbol of resistance against injustice worldwide. Famous quote: \"It always seems impossible until it is done.\"".into());
    }
    if q.contains("martin luther king") || q.contains("mlk") {
        return Some("**Martin Luther King Jr.** (1929–1968) — American Baptist minister and civil rights leader. Led the nonviolent struggle for racial equality in the United States. **\"I Have a Dream\"** speech (1963, March on Washington) is one of history's most famous speeches. Won **Nobel Peace Prize** (1964). Key achievements: Montgomery Bus Boycott (1955), Civil Rights Act (1964), Voting Rights Act (1965). Assassinated April 4, 1968 in Memphis, Tennessee. MLK Day: 3rd Monday of January.".into());
    }
    if q.contains("nikola tesla") || (q.contains("tesla") && !q.contains("car") && !q.contains("elon") && (q.contains("who") || q.contains("inventor") || q.contains("scientist"))) {
        return Some("**Nikola Tesla** (1856–1943) — Serbian-American inventor, electrical engineer, and futurist. **Key inventions**: alternating current (AC) electrical system, Tesla coil, rotating magnetic field, radio (disputed with Marconi), AC induction motor. Vision of **wireless energy transmission**. Worked briefly for Edison, then became his rival in the \"War of Currents\" (AC vs DC — Tesla's AC won). Died in poverty. Unit of magnetic flux density (Tesla) named after him.".into());
    }
    if q.contains("alan turing") || (q.contains("turing") && (q.contains("who") || q.contains("computer") || q.contains("father"))) {
        return Some("**Alan Turing** (1912–1954) — British mathematician and logician, widely considered the **father of computer science** and artificial intelligence. **Contributions**: Turing machine (theoretical model of computation, 1936), cracked the Enigma code at Bletchley Park (WWII, shortened the war by ~2 years), **Turing Test** (can a machine think?), early AI concepts. Prosecuted for homosexuality in 1952 (pardoned 2013). The **Turing Award** (\"Nobel of Computing\") is named after him.".into());
    }
    if q.contains("leonardo da vinci") || (q.contains("da vinci") && !q.contains("code")) {
        return Some("**Leonardo da Vinci** (1452–1519) — Italian polymath of the Renaissance. **Artist**: Mona Lisa, The Last Supper, Vitruvian Man. **Inventor/Engineer**: designed flying machines, tanks, solar power concentrators, anatomical studies (500+ years ahead of his time). **Scientist**: studied anatomy, optics, hydrodynamics. Often called the most diversely talented person ever. His notebooks contain ~13,000 pages of notes and drawings. Worked in Florence, Milan, Rome, and France.".into());
    }
    if q.contains("marie curie") || (q.contains("curie") && (q.contains("who") || q.contains("scientist") || q.contains("madame"))) {
        return Some("**Marie Curie** (Maria Sklodowska-Curie, 1867–1934) — Polish-French physicist and chemist. **First woman to win a Nobel Prize**, and the **only person to win Nobel Prizes in two different sciences** (Physics 1903 for radioactivity research, Chemistry 1911 for discovering polonium and radium). Pioneered research on radioactivity (a term she coined). Developed mobile X-ray units in WWI. Her notebooks are still radioactive and stored in lead-lined boxes.".into());
    }
    if q.contains("bill gates") || (q.contains("gates") && q.contains("microsoft")) {
        return Some("**Bill Gates** (born October 28, 1955) — American businessman, software developer, and philanthropist. **Co-founder of Microsoft** (1975) with Paul Allen. Led the PC revolution with MS-DOS and Windows. Was the world's richest person for many years. Now focuses on philanthropy through the **Bill & Melinda Gates Foundation** — the world's largest private charity (global health, education, climate). Author of multiple books on technology, climate, and pandemics.".into());
    }
    if q.contains("steve jobs") || (q.contains("jobs") && q.contains("apple")) {
        return Some("**Steve Jobs** (1955–2011) — American entrepreneur and co-founder of **Apple Inc.** (1976) with Steve Wozniak and Ronald Wayne. Revolutionized personal computing (Macintosh), animated films (Pixar — Toy Story), music (iPod/iTunes), smartphones (iPhone, 2007), and tablets (iPad). Known for his obsession with design, simplicity, and user experience. Fired from Apple in 1985, returned in 1997, transformed it into the world's most valuable company. Famous quote: \"Stay hungry, stay foolish.\"".into());
    }
    if q.contains("jeff bezos") || (q.contains("bezos") && q.contains("amazon")) {
        return Some("**Jeff Bezos** (born January 12, 1964) — American entrepreneur. **Founder of Amazon** (1994, originally an online bookstore — now the world's largest e-commerce and cloud computing company). Also founded **Blue Origin** (space company) and owns **The Washington Post**. Amazon Web Services (AWS) is the world's largest cloud platform. Known for customer obsession, long-term thinking, and the \"Day 1\" philosophy. One of the world's richest people.".into());
    }
    if q.contains("mark zuckerberg") || (q.contains("zuckerberg") && q.contains("facebook")) {
        return Some("**Mark Zuckerberg** (born May 14, 1984) — American tech entrepreneur. **Co-founder and CEO of Meta** (formerly Facebook, founded 2004 at Harvard). Facebook became the world's largest social network (~3 billion users). Meta platforms include Instagram, WhatsApp, Messenger, Threads, and Quest VR headsets. Rebranded to Meta in 2021, pivoting toward the metaverse and AI. Known for \"move fast and break things\" (early motto) and significant philanthropy.".into());
    }
    if q.contains("sundar pichai") || (q.contains("pichai") && q.contains("google")) {
        return Some("**Sundar Pichai** (born June 10, 1972, Madurai, Tamil Nadu, India) — CEO of **Alphabet Inc.** and **Google** (since 2015/2019). Led development of Google Chrome, Chrome OS, and Google Drive before becoming CEO. Under his leadership: Google AI (Gemini/Bard), Waymo, Google Cloud growth, Android ecosystem. Studied at IIT Kharagpur, Stanford, and Wharton. Known for calm, measured leadership style and bringing AI to Google's core products.".into());
    }
    if q.contains("satya nadella") || (q.contains("nadella") && q.contains("microsoft")) {
        return Some("**Satya Nadella** (born August 19, 1967, Hyderabad, India) — CEO of **Microsoft** since 2014. Transformed Microsoft from a Windows-centric company to a cloud-first, AI-first company. Under his leadership: Azure became #2 cloud platform, acquired LinkedIn ($26B) and GitHub ($7.5B), partnered with OpenAI, launched Copilot AI across all products. Market cap grew from ~$300B to $3T+. Known for empathetic leadership and \"growth mindset\" culture. Studied at Mangalore University and UW-Milwaukee.".into());
    }
    if q.contains("shah rukh khan") || q.contains("shahrukh") || q.contains("srk") {
        return Some("**Shah Rukh Khan** (born November 2, 1965, New Delhi, India) — Indian actor, film producer, and TV personality. Known as **\"King of Bollywood\"** and **\"King Khan\"**. One of the most successful film stars in the world with 100+ films. **Iconic films**: DDLJ, Kuch Kuch Hota Hai, My Name Is Khan, Pathaan, Jawan. **Production**: Red Chillies Entertainment, co-owns Kolkata Knight Riders (IPL). Padma Shri awardee. Known for his wit, romantic roles, and massive global fan base.".into());
    }

    // ── Science — Physics ───────────────────────────────────────────────
    if q.contains("relativity") || (q.contains("einstein") && q.contains("theory")) {
        return Some("**Theory of Relativity** (Albert Einstein):\n\n**Special Relativity** (1905): Physics laws are the same in all inertial frames. Speed of light (c) is constant. **E = mc²** — mass and energy are equivalent. Time dilation: moving clocks run slower. Length contraction: moving objects shrink in direction of motion.\n\n**General Relativity** (1915): Gravity is not a force but **curvature of spacetime** caused by mass/energy. Predicted: gravitational lensing (confirmed 1919), black holes, gravitational waves (detected 2015 by LIGO), expansion of the universe. GPS satellites must account for relativistic time corrections.".into());
    }
    if (q.contains("black hole") || q.contains("blackhole")) && !q.contains("hawking") {
        return Some("**Black Holes** are regions of spacetime where gravity is so strong that nothing — not even light — can escape. **Formation**: when a massive star (>25 solar masses) collapses at end of life (supernova → singularity). **Types**: stellar (few solar masses), intermediate, supermassive (millions to billions of solar masses, at galaxy centers — Sagittarius A* at Milky Way center). **Event horizon**: boundary of no return. **First image**: 2019 by Event Horizon Telescope (M87* black hole, 55M light-years away). Hawking showed they slowly evaporate via Hawking radiation.".into());
    }
    if q.contains("quantum mechanics") || q.contains("quantum physics") {
        return Some("**Quantum Mechanics** is the physics of atoms, photons, and subatomic particles. **Key principles**: (1) **Wave-particle duality** — particles behave as both waves and particles. (2) **Heisenberg Uncertainty** — can't know both position and momentum precisely. (3) **Superposition** — particles exist in all possible states until measured. (4) **Entanglement** — measuring one particle instantly affects its entangled partner (\"spooky action at a distance\" — Einstein). (5) **Quantization** — energy comes in discrete packets (quanta). Developed by Planck, Bohr, Schrödinger, Heisenberg, Dirac (1900s–1930s). Foundation of modern electronics, lasers, and quantum computing.".into());
    }
    if q.contains("thermodynamics") || q.contains("laws of thermodynamics") {
        return Some("**Thermodynamics** — the study of heat, energy, and work.\n\n**0th Law**: If A is in thermal equilibrium with B, and B with C, then A is with C (defines temperature).\n**1st Law**: Energy cannot be created or destroyed, only converted (conservation of energy). ΔU = Q − W.\n**2nd Law**: Entropy of an isolated system always increases. Heat flows from hot to cold, never reverse spontaneously. No perfect heat engine.\n**3rd Law**: As temperature approaches absolute zero (0K / −273.15°C), entropy approaches a minimum.\n\n**Applications**: engines, refrigerators, power plants, chemistry, cosmology.".into());
    }
    if q.contains("electromagnetic") || q.contains("maxwell") && q.contains("equation") {
        return Some("**Electromagnetism** is the fundamental force governing electricity, magnetism, and light. **Maxwell's Equations** (1860s) unified electricity and magnetism into one theory:\n1. **Gauss's Law**: electric charges produce electric fields\n2. **Gauss's Law for Magnetism**: no magnetic monopoles exist\n3. **Faraday's Law**: changing magnetic fields produce electric fields\n4. **Ampère-Maxwell Law**: electric currents and changing electric fields produce magnetic fields\n\n**Key result**: electromagnetic waves travel at the speed of light — light IS an electromagnetic wave. Spectrum: radio → microwave → infrared → visible → UV → X-ray → gamma.".into());
    }

    // ── Science — Chemistry ─────────────────────────────────────────────
    if q.contains("periodic table") || q.contains("chemical element") {
        return Some("**The Periodic Table** organizes all 118 known chemical elements by atomic number (protons). Created by **Dmitri Mendeleev** (1869). **Layout**: 7 periods (rows) × 18 groups (columns). **Groups**: alkali metals (group 1), alkaline earth metals (2), transition metals (3-12), halogens (17), noble gases (18). **Key elements**: H (hydrogen, lightest), He (helium), C (carbon, basis of life), O (oxygen), Fe (iron), Au (gold), U (uranium). Latest additions: Nihonium (113), Moscovium (115), Tennessine (117), Oganesson (118) — all synthetic.".into());
    }
    if q.contains("atom") && (q.contains("what") || q.contains("structure") || q == "atom" || q == "atoms") {
        return Some("**Atoms** are the basic units of matter. **Structure**: (1) **Nucleus** — protons (+ charge) and neutrons (neutral), held by strong nuclear force. (2) **Electron cloud** — electrons (− charge) orbiting in probability shells. **Size**: ~1 angstrom (10⁻¹⁰ m). Nucleus is ~100,000× smaller than the atom — if an atom were a stadium, the nucleus would be a marble at center. **Atomic number** = proton count (defines the element). **Isotopes** = same protons, different neutrons. **Ions** = atoms with unequal protons/electrons.".into());
    }

    // ── Science — Biology ────────────────────────────────────────────────
    if q.contains("cell") && (q.contains("biology") || q.contains("what is a cell") || q.contains("cell structure") || q == "cell") {
        return Some("**Cells** are the fundamental units of life. **Two types**: (1) **Prokaryotic** — no nucleus (bacteria, archaea). (2) **Eukaryotic** — membrane-bound nucleus (plants, animals, fungi). **Key organelles**: nucleus (DNA), mitochondria (energy — \"powerhouse of the cell\"), ribosomes (protein synthesis), endoplasmic reticulum (protein/lipid processing), Golgi apparatus (packaging/shipping), cell membrane (boundary). **Plant cells** additionally have: cell wall, chloroplasts (photosynthesis), large central vacuole. **Average human has ~37 trillion cells**.".into());
    }
    if q.contains("virus") && (q.contains("what") || q.contains("how") || q == "virus" || q == "viruses") {
        return Some("**Viruses** are microscopic infectious agents that replicate only inside living cells. **Not considered alive** — no metabolism, no reproduction outside a host. **Structure**: genetic material (DNA or RNA) + protein coat (capsid) ± lipid envelope. **Size**: 20–300 nm (smaller than bacteria). **Replication cycle**: attach → inject DNA/RNA → hijack cell → replicate → burst out (lysis) or bud. **Types**: bacteriophages (infect bacteria), influenza, HIV, SARS-CoV-2 (COVID-19). **Treatment**: antivirals, vaccines (not antibiotics — those are for bacteria).".into());
    }
    if q.contains("human body") || (q.contains("body") && q.contains("system") && !q.contains("solar")) {
        return Some("**The Human Body** has 11 major organ systems:\n1. **Skeletal** — 206 bones, support/protection\n2. **Muscular** — 600+ muscles, movement\n3. **Nervous** — brain, spinal cord, nerves (100B neurons)\n4. **Circulatory** — heart, blood vessels, blood (~5L)\n5. **Respiratory** — lungs, airways (~23,000 breaths/day)\n6. **Digestive** — mouth to intestines (~9m long)\n7. **Endocrine** — hormones (thyroid, adrenal, pituitary)\n8. **Immune** — white blood cells, lymph nodes\n9. **Urinary** — kidneys filter ~200L blood/day\n10. **Reproductive** — gonads, gametes\n11. **Integumentary** — skin (largest organ, ~2m²)".into());
    }

    // ── Science — Medicine ──────────────────────────────────────────────
    if q.contains("antibiotic") {
        return Some("**Antibiotics** are medicines that kill or inhibit bacteria. **First**: Penicillin, discovered by **Alexander Fleming** in 1928 (from Penicillium mold). **Types**: penicillins, cephalosporins, macrolides (azithromycin), fluoroquinolones, tetracyclines. **How they work**: disrupt bacterial cell walls, protein synthesis, or DNA replication. **Important**: only work against BACTERIA — not viruses (common cold, flu, COVID require antivirals, not antibiotics). **Antibiotic resistance** is a major global health threat — caused by overuse/misuse. WHO calls it one of the top 10 global health threats.".into());
    }
    if q.contains("vaccine") || q.contains("vaccination") {
        return Some("**Vaccines** train the immune system to recognize and fight specific pathogens without causing disease. **Types**: (1) **Live attenuated** — weakened virus (MMR, chickenpox), (2) **Inactivated** — killed pathogen (flu shot, polio), (3) **Subunit/protein** — purified pieces (Hepatitis B), (4) **mRNA** — genetic instructions to make spike protein (Pfizer/Moderna COVID vaccines — first mRNA vaccines approved 2020). **Herd immunity**: when enough people are immune (~70-90%), disease can't spread easily. **Impact**: vaccines eradicated smallpox (1980) and nearly eliminated polio.".into());
    }

    // ── History ──────────────────────────────────────────────────────────
    if q.contains("world war") && (q.contains("1") || q.contains("i") || q.contains("first")) && !q.contains("2") && !q.contains("ii") && !q.contains("second") {
        return Some("**World War I** (1914–1918) — \"The Great War\". **Trigger**: assassination of Archduke Franz Ferdinand of Austria-Hungary (June 28, 1914) by Gavrilo Princip. **Allies**: UK, France, Russia, USA (from 1917), Italy, Japan vs **Central Powers**: Germany, Austria-Hungary, Ottoman Empire, Bulgaria. **Key features**: trench warfare, chemical weapons (mustard gas), tanks (first used 1916), 20M+ deaths. **End**: Armistice November 11, 1918. **Treaty of Versailles** (1919) — harsh terms on Germany, sowed seeds of WWII. Led to fall of empires: Ottoman, Austro-Hungarian, Russian, German.".into());
    }
    if q.contains("world war") && (q.contains("2") || q.contains("ii") || q.contains("second")) {
        return Some("**World War II** (1939–1945) — the deadliest conflict in history (~70-85M deaths). **Trigger**: Nazi Germany invaded Poland (Sept 1, 1939). **Allies**: UK, USA, USSR, China, France vs **Axis**: Germany (Hitler), Japan (Tojo), Italy (Mussolini). **Key events**: Blitzkrieg, Battle of Britain, Pearl Harbor (1941), D-Day (June 6, 1944), Stalingrad, Holocaust (6M Jews murdered), atomic bombs on Hiroshima/Nagasaki (Aug 1945). **End**: Germany surrendered May 8 (VE Day), Japan Sept 2, 1945. Led to United Nations, Cold War, decolonization.".into());
    }
    if q.contains("cold war") {
        return Some("**The Cold War** (1947–1991) — geopolitical tension between **USA** (capitalism, democracy) and **USSR** (communism, authoritarianism) after WWII. Never a direct military conflict (hence \"cold\"), but fought through proxy wars, nuclear arms race, espionage, and propaganda. **Key events**: Berlin Wall (1961–1989), Cuban Missile Crisis (1962 — closest to nuclear war), Space Race (Sputnik 1957, Moon landing 1969), Korean War, Vietnam War, Afghan-Soviet War. **End**: fall of Berlin Wall (1989), dissolution of USSR (Dec 1991). Result: US emerged as sole superpower.".into());
    }
    if q.contains("french revolution") {
        return Some("**The French Revolution** (1789–1799) — a period of radical social and political upheaval in France. **Causes**: financial crisis, inequality (3 estates: clergy, nobility, commoners), Enlightenment ideas. **Key events**: Storming of the Bastille (July 14, 1789 — now France's national day), Declaration of the Rights of Man, abolition of feudalism, execution of King Louis XVI (1793), the Reign of Terror (Robespierre, 16,000+ guillotined). **Outcome**: ended absolute monarchy, established republic, inspired democratic movements worldwide. Napoleon rose to power in its aftermath (1799).".into());
    }
    if q.contains("renaissance") {
        return Some("**The Renaissance** (14th–17th century) — a cultural movement that began in **Florence, Italy** and spread across Europe. Meaning \"rebirth\" — revival of classical Greek and Roman art, philosophy, and learning after the Middle Ages. **Key figures**: Leonardo da Vinci (art/science), Michelangelo (Sistine Chapel, David), Raphael, Galileo (astronomy), Copernicus (heliocentric model), Shakespeare (literature), Gutenberg (printing press, ~1440 — revolutionized knowledge sharing). **Impact**: humanism, scientific method, artistic realism, and the foundation of the modern world.".into());
    }
    if q.contains("industrial revolution") {
        return Some("**The Industrial Revolution** (~1760–1840) — a period of transition from agrarian economies to industrial manufacturing, beginning in **Britain**. **Key innovations**: steam engine (James Watt), spinning jenny (textiles), cotton gin, iron/steel production, railways (Stephenson's Rocket). **Impact**: urbanization, factory system, mass production, rise of middle class, child labor issues, pollution. **Second Industrial Revolution** (~1870–1914): electricity, telephone (Bell), automobile (Benz), assembly line (Ford), chemical industry. Transformed every aspect of daily life and created the modern economy.".into());
    }
    if q.contains("moon landing") || q.contains("apollo 11") {
        return Some("**Apollo 11** (July 20, 1969) — first human moon landing. **Crew**: Neil Armstrong (commander), Buzz Aldrin (lunar module pilot), Michael Collins (orbited above). Armstrong's famous words: **\"That's one small step for man, one giant leap for mankind.\"** They spent ~2.5 hours on the lunar surface, collected 21.5 kg of samples. **Program**: NASA's Apollo program (1961–1972), motivated by JFK's 1961 speech to land a man on the Moon before the decade's end. 12 total astronauts walked on the Moon (Apollo 11–17, except 13). Ended 1972; no human has returned since.".into());
    }

    // ── Mathematics ─────────────────────────────────────────────────────
    if q.contains("calculus") && !q.contains("renal") {
        return Some("**Calculus** is the mathematics of change, developed independently by **Isaac Newton** and **Gottfried Leibniz** in the late 17th century.\n\n**Differential calculus**: studies rates of change (derivatives). f'(x) = lim(h→0) [f(x+h) - f(x)]/h. Applications: velocity, slopes, optimization.\n\n**Integral calculus**: studies accumulation (integrals). ∫f(x)dx = area under curve. Fundamental Theorem of Calculus links the two: ∫ₐᵇf(x)dx = F(b) − F(a).\n\n**Applications**: physics (motion, electromagnetism), engineering, economics, biology (population growth), machine learning (gradient descent).".into());
    }
    if q.contains("fibonacci") && (q.contains("sequence") || q.contains("number") || q.contains("what")) {
        return Some("**Fibonacci Sequence**: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, ... Each number is the sum of the two preceding: F(n) = F(n-1) + F(n-2). Named after **Leonardo Fibonacci** (Pisa, ~1202), who introduced it in *Liber Abaci*. **Golden Ratio**: as n grows, F(n+1)/F(n) → φ ≈ 1.6180339... (the golden ratio). **In nature**: sunflower seeds, pinecone spirals, nautilus shells, galaxy arms, tree branching. Used in algorithms, financial trading, and art composition.".into());
    }
    if q.contains("prime number") || (q.contains("prime") && (q.contains("what") || q.contains("math"))) {
        return Some("**Prime Numbers** are natural numbers greater than 1 that have no divisors other than 1 and themselves. **First primes**: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47... Note: 2 is the only even prime. **Properties**: infinite (proven by Euclid, ~300 BCE). **Fundamental Theorem of Arithmetic**: every integer > 1 is either prime or a unique product of primes. **Applications**: RSA encryption (internet security), hash functions. **Unsolved**: Riemann Hypothesis, Goldbach's Conjecture (every even number > 2 is sum of two primes), twin prime conjecture.".into());
    }
    if q.contains("pi") && (q.contains("number") || q.contains("what is pi") || q.contains("value") || q == "pi") {
        return Some("**π (Pi)** ≈ 3.14159265358979323846... — the ratio of a circle's circumference to its diameter. **Properties**: irrational (non-repeating, non-terminating decimal), transcendental (not a root of any polynomial). **History**: known to ancient Babylonians (~1900 BCE, approximated as 3.125), Archimedes (~250 BCE, 3.1408 < π < 3.1429). **Formulas**: C = 2πr, A = πr², V(sphere) = 4/3πr³. **Current record**: 105 trillion digits (2024). **Pi Day**: March 14 (3/14). Appears in Euler's identity: e^(iπ) + 1 = 0.".into());
    }
    if q.contains("statistics") && (q.contains("what") || q == "statistics" || q.contains("basics")) {
        return Some("**Statistics** is the science of collecting, analyzing, and interpreting data.\n\n**Descriptive**: summarizes data — mean (average), median (middle), mode (most frequent), standard deviation (spread), variance.\n\n**Inferential**: draws conclusions from samples — hypothesis testing, confidence intervals, p-values, regression.\n\n**Key distributions**: Normal (bell curve), Binomial, Poisson, Chi-squared, t-distribution.\n\n**Measures of central tendency**: mean = Σx/n, median = middle value, mode = most frequent.\n\n**Correlation vs Causation**: correlation ≠ causation (ice cream sales and drowning both rise in summer — heat is the confounder).".into());
    }
    if q.contains("linear algebra") {
        return Some("**Linear Algebra** studies vectors, matrices, and linear transformations — the math behind machine learning, computer graphics, and physics.\n\n**Key concepts**: vectors (direction + magnitude), matrices (rectangular arrays), dot product, cross product, eigenvalues/eigenvectors, determinant, inverse matrix, linear transformations, vector spaces.\n\n**Applications**: ML (weights = matrices, gradient descent = linear ops), 3D graphics (transformation matrices), quantum computing (state vectors), Google PageRank (eigenvectors), recommendation systems, neural networks.\n\n**Key result**: Ax = b — solving systems of linear equations is the core problem.".into());
    }

    // ── Philosophy ──────────────────────────────────────────────────────
    if q.contains("stoicism") || q.contains("stoic") {
        return Some("**Stoicism** is an ancient Greek philosophy (founded ~300 BCE by Zeno of Citium) focused on virtue, reason, and inner peace. **Core ideas**: (1) Focus only on what you can control (your thoughts, actions, responses — not external events). (2) Virtue (wisdom, courage, justice, temperance) is the highest good. (3) Negative emotions come from misjudgment, not events themselves. **Key figures**: Marcus Aurelius (emperor, *Meditations*), Seneca (writer), Epictetus (former slave, *Discourses*). **Modern revival**: hugely popular in Silicon Valley, sports, military. Influenced CBT (Cognitive Behavioral Therapy).".into());
    }
    if q.contains("existentialism") || q.contains("existentialist") {
        return Some("**Existentialism** is a philosophical movement (19th-20th century) focused on individual freedom, choice, and meaning. **Core ideas**: (1) \"Existence precedes essence\" — you're not born with a purpose; you create your own meaning. (2) Radical freedom brings radical responsibility. (3) Anxiety/Angst is natural when facing life's meaninglessness. **Key figures**: **Søren Kierkegaard** (father of existentialism), **Jean-Paul Sartre** (\"hell is other people\"), **Albert Camus** (*The Stranger*, \"one must imagine Sisyphus happy\"), **Simone de Beauvoir**, **Friedrich Nietzsche** (\"God is dead\", Übermensch).".into());
    }
    if q.contains("philosophy") && (q.contains("what") || q == "philosophy") {
        return Some("**Philosophy** (Greek: *philo* = love, *sophia* = wisdom) is the study of fundamental questions about existence, knowledge, ethics, mind, and language.\n\n**Major branches**: **Metaphysics** (what exists?), **Epistemology** (what can we know?), **Ethics** (what is right/wrong?), **Logic** (what is valid reasoning?), **Aesthetics** (what is beauty?).\n\n**Key thinkers**: Socrates, Plato, Aristotle (ancient Greece), Descartes (\"I think therefore I am\"), Kant (categorical imperative), Nietzsche, Wittgenstein, Sartre.\n\n**Eastern philosophy**: Confucius, Laozi (Taoism), Buddha (Buddhism), Vedantic traditions (Hinduism).".into());
    }

    // ── Business & Economics ────────────────────────────────────────────
    if q.contains("gdp") || q.contains("gross domestic product") {
        return Some("**GDP (Gross Domestic Product)** is the total monetary value of all goods and services produced within a country's borders in a given period. **The most common measure of economic size and health.**\n\n**Formula**: GDP = C + I + G + (X − M) where C=consumer spending, I=business investment, G=government spending, X=exports, M=imports.\n\n**Types**: Nominal GDP (current prices), Real GDP (inflation-adjusted), GDP per capita (per person). **Top economies by GDP (2024)**: USA (~$28T), China (~$18T), Germany (~$4.5T), Japan (~$4.2T), India (~$3.7T).".into());
    }
    if q.contains("inflation") && (q.contains("what") || q.contains("economics") || q == "inflation") {
        return Some("**Inflation** is the rate at which the general price level of goods and services rises over time, eroding purchasing power. **Measured by**: CPI (Consumer Price Index), PCE (Personal Consumption Expenditures). **Causes**: (1) **Demand-pull** — too much money chasing too few goods. (2) **Cost-push** — rising production costs. (3) **Monetary** — excess money supply. **Central banks** target ~2% annual inflation (Fed, ECB). **Hyperinflation**: extreme cases (Zimbabwe 2008: 89.7 sextillion % monthly, Venezuela 2018: 1,000,000%+). **Deflation** (falling prices) is often worse — discourages spending.".into());
    }
    if q.contains("stock market") || q.contains("stock exchange") {
        return Some("**The Stock Market** is where shares (ownership stakes) of publicly traded companies are bought and sold. **Major exchanges**: NYSE (New York Stock Exchange — largest, $25T+), NASDAQ (tech-heavy), London Stock Exchange, Tokyo Stock Exchange, Shanghai Stock Exchange, BSE/NSE (India). **Indices**: S&P 500 (top 500 US companies), Dow Jones (30 blue-chips), NASDAQ Composite, SENSEX (India), Nikkei (Japan). **How it works**: companies IPO (Initial Public Offering) to go public → investors buy/sell shares → price determined by supply/demand. **Long-term average return**: ~10% per year (S&P 500).".into());
    }
    if q.contains("cryptocurrency") || q.contains("crypto") && !q.contains("graph") {
        return Some("**Cryptocurrency** is digital/virtual currency secured by cryptography, operating on decentralized blockchain networks. **Bitcoin** (2009, Satoshi Nakamoto) — first and largest by market cap. **Ethereum** (2015, Vitalik Buterin) — smart contracts, DeFi, NFTs. **Key concepts**: mining/staking (validation), wallets, exchanges (Coinbase, Binance), DeFi (decentralized finance), NFTs (non-fungible tokens). **Pros**: decentralization, borderless, fast transfers. **Cons**: volatility, energy consumption (PoW), regulatory uncertainty, scams. ~25,000+ cryptocurrencies exist; most are worthless.".into());
    }
    if q.contains("startup") && (q.contains("what") || q.contains("how") || q == "startup") {
        return Some("**A Startup** is a young company founded to develop a unique product/service, bring it to market, and scale rapidly. **Stages**: Idea → MVP (Minimum Viable Product) → Product-Market Fit → Scale → Exit (IPO or acquisition). **Funding**: Bootstrapping → Angel investors → Seed → Series A/B/C → IPO. **Key concepts**: burn rate, runway, pivot, traction, unit economics. **Famous startups → giants**: Apple (garage, 1976), Google (Stanford dorm, 1998), Facebook (Harvard dorm, 2004), Uber, Airbnb, Stripe. **Failure rate**: ~90% of startups fail, most within first 3 years.".into());
    }

    // ── Space & Astronomy ───────────────────────────────────────────────
    if q.contains("mars") && (q.contains("planet") || q.contains("what") || q.contains("life") || q == "mars") {
        return Some("**Mars** — the 4th planet from the Sun, known as the \"Red Planet\" (iron oxide/rust on surface). **Stats**: diameter 6,779 km (~half Earth), gravity 38% of Earth, day ~24.6 hours, year ~687 Earth days. **Distance from Sun**: ~228M km. **Surface temp**: −60°C average (−140°C to +20°C). **Features**: Olympus Mons (tallest volcano in solar system, 21.9 km), Valles Marineris (4,000 km canyon). **Exploration**: Perseverance rover (2021), Ingenuity helicopter, Curiosity (2012). Evidence of ancient liquid water. **Future**: SpaceX targets human landing by ~2030s.".into());
    }
    if q.contains("sun") && (q.contains("star") || q.contains("facts") || q.contains("what") || q.contains("temperature") || q.contains("how big")) {
        return Some("**The Sun** is a **G-type main-sequence star** (G2V) — a yellow dwarf. **Stats**: mass = 1.989 × 10³⁰ kg (99.86% of solar system mass), diameter = 1.39 million km (109× Earth), surface temp = 5,778 K (~5,500°C), core temp = ~15 million °C. **Composition**: ~73% hydrogen, ~25% helium. **Energy source**: nuclear fusion (hydrogen → helium, 600M tons/sec). **Distance from Earth**: ~150M km (1 AU), light takes 8.3 minutes. **Age**: ~4.6 billion years. **Lifespan**: ~10 billion years total — will become a red giant, then white dwarf.".into());
    }
    if q.contains("milky way") || (q.contains("galaxy") && (q.contains("our") || q.contains("what"))) {
        return Some("**The Milky Way** is our home galaxy — a barred spiral galaxy containing **100–400 billion stars** and at least that many planets. **Size**: ~100,000 light-years across, ~1,000 light-years thick (disk). **Central black hole**: Sagittarius A* (~4 million solar masses). **Our position**: Sun is ~26,000 light-years from the center, in the Orion Arm. **Rotation**: Sun orbits once every ~225 million years (\"galactic year\"). **Neighbors**: Andromeda Galaxy (M31, 2.5M light-years away — colliding with Milky Way in ~4.5 billion years), Magellanic Clouds. Part of the **Local Group** (~80 galaxies).".into());
    }

    // ── Technology ───────────────────────────────────────────────────────
    if q.contains("internet") && (q.contains("what") || q.contains("how") || q.contains("history") || q == "internet") {
        return Some("**The Internet** is a global network of interconnected computer networks using the TCP/IP protocol suite. **History**: ARPANET (1969, 4 nodes) → TCP/IP adoption (1983) → World Wide Web (Tim Berners-Lee, 1991) → commercial expansion (1990s) → mobile era (2010s). **How it works**: data travels as packets via routers, DNS translates domains to IPs, HTTP/HTTPS for web pages. **Scale**: ~5.5 billion users (2024), ~2 billion websites. **Infrastructure**: undersea fiber optic cables (450+), data centers, ISPs, CDNs. **Key protocols**: TCP/IP, HTTP, DNS, TLS/SSL, BGP.".into());
    }
    if (q.contains("cloud computing") || q.contains("cloud") && q.contains("computing")) && !q.contains("rain") {
        return Some("**Cloud Computing** delivers computing services (servers, storage, databases, networking, AI, analytics) over the internet (\"the cloud\") on-demand. **Models**: (1) **IaaS** — infrastructure (VMs, storage) — AWS EC2, Azure VMs. (2) **PaaS** — platform (runtime, middleware) — Heroku, Google App Engine. (3) **SaaS** — software (ready-to-use apps) — Gmail, Salesforce, Office 365. **Deployment**: Public (AWS, Azure, GCP), Private (on-premises), Hybrid (mix). **Top providers**: AWS (~31% market share), Azure (~25%), Google Cloud (~11%). **Benefits**: scalability, pay-as-you-go, no hardware management.".into());
    }
    if q.contains("cybersecurity") || q.contains("cyber security") {
        return Some("**Cybersecurity** is the practice of protecting systems, networks, and data from digital attacks. **Common threats**: (1) **Phishing** — fake emails/sites to steal credentials. (2) **Malware** — viruses, ransomware, trojans. (3) **SQL Injection** — attacking databases through input fields. (4) **DDoS** — overwhelming servers with traffic. (5) **Zero-day exploits** — attacking unknown vulnerabilities. **Defense layers**: firewalls, encryption, MFA (multi-factor authentication), penetration testing, security audits. **Frameworks**: NIST, ISO 27001, OWASP Top 10. **Career**: one of fastest-growing tech fields (~3.5M unfilled jobs globally).".into());
    }
    if q.contains("operating system") || q.contains(" os ") || q == "os" || q.contains("what is an os") {
        return Some("**Operating System (OS)** — software that manages hardware and provides services for applications. **Functions**: process management, memory management, file system, device drivers, user interface. **Major OS**: **Windows** (Microsoft, ~72% desktop market share), **macOS** (Apple), **Linux** (open-source, powers 96% of servers and Android), **Android** (Google, ~72% mobile), **iOS** (Apple, ~27% mobile). **Types**: real-time (embedded systems), batch, time-sharing, distributed. **Key concepts**: kernel (core), shell (interface), drivers, system calls.".into());
    }

    // ── More Programming Languages ──────────────────────────────────────
    if q.contains("php") && (q.contains("language") || q == "php" || q.contains("what")) {
        return Some("**PHP** (PHP: Hypertext Preprocessor) — a server-side scripting language created by **Rasmus Lerdorf** (1994). Powers ~77% of websites with known server-side language (including WordPress, which runs ~43% of all websites). **Features**: easy to learn, embedded in HTML, huge CMS ecosystem (WordPress, Drupal, Joomla), frameworks (Laravel, Symfony). **Current version**: PHP 8.3+ (JIT compiler, fibers, named arguments, union types). Often criticized for inconsistent function naming and security pitfalls, but remains one of the most-deployed languages.".into());
    }
    if q.contains("sql") && !q.contains("nosql") && (q.contains("language") || q == "sql" || q.contains("what")) {
        return Some("**SQL** (Structured Query Language) — the standard language for managing relational databases. Created at **IBM** (1970s, based on Edgar Codd's relational model). **Key commands**: SELECT (query), INSERT (add), UPDATE (modify), DELETE (remove), CREATE TABLE, ALTER TABLE, JOIN (combine tables). **Dialects**: MySQL, PostgreSQL, SQL Server (T-SQL), Oracle, SQLite. **Concepts**: tables, rows, columns, primary keys, foreign keys, indexes, transactions (ACID), normalization. **Still dominant**: virtually every application uses SQL databases (banking, e-commerce, healthcare).".into());
    }
    if q.contains("assembly") && (q.contains("language") || q == "assembly" || q.contains("what")) {
        return Some("**Assembly Language** is a low-level programming language with a strong correspondence to machine code instructions. Each CPU architecture has its own assembly: **x86/x86-64** (Intel/AMD PCs), **ARM** (mobile/embedded), **RISC-V** (open ISA). **Key concepts**: registers (AX, BX, CX, DX), instructions (MOV, ADD, JMP, CMP), memory addressing, stack operations (PUSH/POP), interrupts. **Why use it**: maximum control over hardware, performance-critical code (OS kernels, drivers, embedded systems, game engines). Assembled (not compiled) into machine code by an assembler (NASM, MASM, GAS).".into());
    }

    // ── Follow-up topic entries (RAG, prompt engineering, fine-tuning) ──
    if q.contains("rag") || q.contains("retrieval augmented") || q.contains("retrieval-augmented") {
        return Some("**RAG (Retrieval-Augmented Generation)** connects an LLM to your own data so it can answer questions using your specific documents, not just its training data.\n\n**How RAG works**:\n1. **Index**: Split your documents into chunks, convert each to a vector (embedding)\n2. **Store**: Save embeddings in a vector database (Pinecone, Chroma, Weaviate, FAISS)\n3. **Retrieve**: When user asks a question, find the most similar document chunks\n4. **Generate**: Pass the question + retrieved chunks to the LLM as context\n5. **Answer**: LLM generates answer grounded in your actual data\n\n**Why RAG matters**:\n- No need to re-train the entire model (expensive!)\n- Always uses up-to-date information\n- Reduces hallucination (grounded in real documents)\n- Works with private/proprietary data\n\n**Tools**: LangChain, LlamaIndex, Haystack, Semantic Kernel\n**Vector DBs**: Pinecone, ChromaDB, Weaviate, Qdrant, FAISS\n\nWant to see a **code example**, learn about **embeddings**, or understand **chunking strategies**?".into());
    }
    if q.contains("prompt engineering") || q.contains("prompt design") {
        return Some("**Prompt Engineering** is the art of writing effective instructions for AI models to get the best possible output.\n\n**Key techniques**:\n1. **Be specific**: \"Write a Python function that sorts a list\" > \"Write code\"\n2. **Provide examples** (few-shot): Show the model 2-3 examples of desired output\n3. **Chain-of-thought**: \"Think step by step\" — makes models reason better\n4. **Role assignment**: \"You are an expert Python developer\" — sets context\n5. **Output format**: \"Return as JSON\" / \"Use markdown\" — control structure\n6. **Constraints**: \"In under 100 words\" / \"Don't use loops\" — limit scope\n\n**Advanced patterns**:\n- **ReAct**: Thought → Action → Observation loops\n- **Tree of Thoughts**: Explore multiple reasoning paths\n- **Self-consistency**: Generate multiple answers, pick the most common\n- **Retrieval-augmented**: Inject relevant context before the question\n\nWant to see **practical examples**, learn about **system prompts**, or try **prompt templates**?".into());
    }
    if q.contains("fine tun") || q.contains("finetun") {
        return Some("**Fine-tuning** is the process of further training a pre-trained AI model on your specific data to customize it for your use case.\n\n**How it works**:\n1. Start with a pre-trained base model (GPT, LLaMA, Mistral)\n2. Prepare your dataset: pairs of (input, desired_output)\n3. Train for a few epochs with a low learning rate\n4. The model adapts its weights to your domain/style\n\n**Types**:\n- **Full fine-tuning**: Update all parameters (expensive, needs lots of data)\n- **LoRA/QLoRA**: Update only small adapter layers (cheap, efficient, popular)\n- **RLHF**: Fine-tune with human preference feedback\n- **DPO**: Direct Preference Optimization (simpler alternative to RLHF)\n\n**When to fine-tune** (vs. RAG vs. prompting):\n- **Prompting**: Quick, no training, good for general tasks\n- **RAG**: Need access to specific documents/data\n- **Fine-tuning**: Need to change model behavior/style/domain expertise\n\n**Tools**: Hugging Face Transformers, OpenAI fine-tuning API, Axolotl, Unsloth\n\nWant to see **code examples** or compare **LoRA vs full fine-tuning**?".into());
    }

    // ── More AI Topics ──────────────────────────────────────────────────
    if q.contains("chatgpt") || q.contains("openai") || q.contains("gpt-4") || q.contains("gpt4") {
        return Some("**ChatGPT** is an AI chatbot by **OpenAI** (launched November 30, 2022). Built on the GPT (Generative Pre-trained Transformer) architecture. **Models**: GPT-3.5 (free tier), GPT-4 (paid, multimodal — text+images), GPT-4o, and newer. **Capabilities**: conversation, code generation, writing, analysis, translation, reasoning. **How it works**: trained on vast text data, fine-tuned with RLHF (Reinforcement Learning from Human Feedback). **OpenAI**: founded 2015 (Sam Altman, Greg Brockman, Ilya Sutskever, Elon Musk). Also created DALL-E (images) and Whisper (speech-to-text). Fastest-growing app in history (100M users in 2 months).".into());
    }
    if q.contains("claude") && (q.contains("ai") || q.contains("anthropic") || q.contains("what") || q.contains("who")) {
        return Some("**Claude** is an AI assistant by **Anthropic** (founded 2021 by former OpenAI researchers Dario and Daniela Amodei). **Key features**: long context windows (up to 200K tokens), strong reasoning, nuanced safety behavior, coding proficiency. **Models**: Claude 1, 2, 3 (Haiku, Sonnet, Opus), 3.5 Sonnet, 4. **Philosophy**: Constitutional AI (RLHF + AI self-critique for safety). Known for being thoughtful, less prone to hallucination, good at following complex instructions. **MCP** (Model Context Protocol) — Anthropic's open standard for connecting AI to external tools.".into());
    }
    if q.contains("gemini") {
        return Some("**Gemini** is Google's family of multimodal AI models (successor to PaLM 2 and Bard).\n\n**Models**: Gemini Ultra (largest), Gemini Pro (balanced), Gemini Nano (on-device), Gemini 2.0 Flash (latest, fast).\n\n**How it works**: Built on the Transformer architecture, trained on massive datasets of text, images, audio, video, and code simultaneously. Uses **multimodal fusion** — understanding different types of data together, not separately.\n\n**Capabilities**: Conversation, reasoning, code generation, image understanding, video analysis, real-time search, math problem solving.\n\n**Integration**: Built into Google Search, Workspace (Docs, Sheets, Gmail), Android, Chrome. Available via Google AI Studio and Vertex AI.\n\n**Key difference from ChatGPT**: Native multimodality — trained from the ground up on multiple data types, not bolted on. Google's deep integration with Search gives it access to real-time information.\n\n**Company**: Google DeepMind (merger of Google Brain + DeepMind, 2023).".into());
    }
    if q.contains("prompt engineering") || (q.contains("prompt") && q.contains("engineering")) {
        return Some("**Prompt Engineering** is the art of crafting effective instructions for AI models to get desired outputs. **Key techniques**: (1) **Zero-shot** — direct instruction with no examples. (2) **Few-shot** — provide examples in the prompt. (3) **Chain-of-thought** — ask the model to think step by step. (4) **System prompts** — set behavior and persona. (5) **Role-playing** — \"You are an expert in X...\". (6) **Constraints** — specify format, length, style. **Best practices**: be specific, provide context, iterate, use clear formatting. Important skill as AI becomes integral to workflows.".into());
    }
    if q.contains("rag") && (q.contains("retrieval") || q.contains("augmented") || q.contains("generation") || q.contains("what is rag")) {
        return Some("**RAG (Retrieval-Augmented Generation)** is a technique that enhances LLM responses by retrieving relevant documents from an external knowledge base before generating an answer. **How it works**: (1) User asks a question. (2) Query is embedded into a vector. (3) Similar documents are retrieved from a vector database (Pinecone, Weaviate, Chroma). (4) Retrieved context + question are sent to the LLM. (5) LLM generates an answer grounded in the retrieved facts. **Benefits**: reduces hallucination, uses up-to-date information, domain-specific knowledge without fine-tuning. **Used in**: enterprise search, customer support, research assistants.".into());
    }

    // ── Health & Wellness ───────────────────────────────────────────────
    if q.contains("diabetes") && (q.contains("what") || q == "diabetes" || q.contains("type")) {
        return Some("**Diabetes** is a chronic condition where the body can't properly process blood sugar (glucose). **Types**: (1) **Type 1** — autoimmune, pancreas produces little/no insulin (usually childhood onset, ~5-10% of cases). (2) **Type 2** — body becomes insulin-resistant (90-95% of cases, linked to lifestyle/genetics). (3) **Gestational** — during pregnancy. **Symptoms**: frequent urination, excessive thirst, fatigue, blurred vision. **Management**: insulin (Type 1), medication + diet + exercise (Type 2), blood sugar monitoring. **Complications if untreated**: heart disease, kidney failure, blindness, nerve damage. ~537 million adults worldwide have diabetes.".into());
    }
    if q.contains("mental health") || (q.contains("depression") && (q.contains("what") || q.contains("sign") || q.contains("symptom"))) || q.contains("anxiety disorder") {
        return Some("**Mental Health** encompasses emotional, psychological, and social well-being. **Common conditions**: (1) **Depression** — persistent sadness, loss of interest, fatigue, changes in sleep/appetite (~280M people globally). (2) **Anxiety disorders** — excessive worry, panic attacks, social anxiety (~301M people). (3) **PTSD** — after traumatic events. (4) **Bipolar disorder** — mood swings between mania and depression. (5) **OCD** — intrusive thoughts and repetitive behaviors. **Treatment**: therapy (CBT, DBT), medication (SSRIs), lifestyle (exercise, sleep, social connection). **Key message**: mental health is health — seeking help is a sign of strength, not weakness.".into());
    }

    // ── Geography ───────────────────────────────────────────────────────
    if q.contains("mount everest") || q.contains("tallest mountain") || q.contains("highest mountain") {
        return Some("**Mount Everest** — the highest point on Earth at **8,849 meters (29,032 ft)** above sea level. Located in the **Himalayas** on the Nepal-Tibet border. **First summit**: Sir Edmund Hillary and Tenzing Norgay (May 29, 1953). **Climbing season**: primarily May (brief weather window). **Deaths**: ~300+ climbers have died. **Other notable peaks**: K2 (8,611m — more dangerous), Kangchenjunga (8,586m), Lhotse (8,516m). **Note**: measured from base to peak, Mauna Kea (Hawaii) is taller at ~10,210m; measured from Earth's center, Chimborazo (Ecuador) is farthest point.".into());
    }
    if q.contains("amazon") && q.contains("rain") || (q.contains("amazon") && q.contains("forest")) {
        return Some("**The Amazon Rainforest** is the world's largest tropical rainforest, covering ~5.5 million km² across 9 countries (60% in Brazil). **Known as \"the lungs of the Earth\"** — produces ~6% of world's oxygen and absorbs massive CO₂. **Biodiversity**: home to ~10% of all species on Earth — 40,000+ plant species, 1,300+ bird species, 3,000+ fish species, and 2.5 million insect species. **Amazon River**: ~6,400 km long (2nd longest, or longest by some measurements), carries 20% of world's fresh water to the ocean. **Threats**: deforestation (~17% lost since 1970s), fires, climate change.".into());
    }
    if q.contains("ocean") && (q.contains("deepest") || q.contains("how deep") || q.contains("mariana")) {
        return Some("**The Mariana Trench** is the deepest point in Earth's oceans — **10,994 meters (36,070 ft)** deep at the **Challenger Deep**. Located in the western Pacific Ocean, east of the Mariana Islands. **Pressure**: ~1,086 bars (1,000× atmospheric pressure). Despite extreme conditions, life exists there — amphipods, sea cucumbers, microbes. **First descent**: Jacques Piccard and Don Walsh (1960, Trieste bathyscaphe). **Solo descent**: James Cameron (2012, Deepsea Challenger). **Five oceans** by size: Pacific (largest, 165M km²), Atlantic, Indian, Southern (Antarctic), Arctic (smallest).".into());
    }

    // ── Quick Fact Lookups (common simple questions) ─────────────────────
    if (q.contains("tallest") || q.contains("highest")) && q.contains("building") {
        return Some("The **Burj Khalifa** in Dubai, UAE is the world's tallest building at **828 meters (2,717 ft)**, with 163 floors. Completed in 2010. **Other tall buildings**: Merdeka 118 (Malaysia, 679m), Shanghai Tower (632m), Abraj Al-Bait (Mecca, 601m), One World Trade Center (NYC, 541m). **Under construction**: Jeddah Tower (Saudi Arabia) aims for 1,000m+.".into());
    }
    if (q.contains("longest") || q.contains("biggest")) && q.contains("river") {
        return Some("**The Nile** is traditionally considered the longest river at **~6,650 km** (flows through 11 African countries, empties into Mediterranean). **However**, some measurements put the **Amazon** at **~6,992 km** (if measured from most distant source), making it potentially longer AND the largest by water volume (20% of global freshwater discharge). **Other long rivers**: Yangtze (6,300 km, longest in Asia), Mississippi-Missouri (6,275 km), Yenisei-Angara (5,539 km).".into());
    }
    if q.contains("population") && q.contains("world") || q.contains("world population") {
        return Some("**World Population** (2025): approximately **8.1 billion** people. **Growth**: reached 1 billion ~1804, 8 billion Nov 15, 2022. **Most populous countries**: (1) India ~1.44B, (2) China ~1.41B, (3) USA ~340M, (4) Indonesia ~277M, (5) Pakistan ~240M, (6) Nigeria ~230M, (7) Brazil ~216M. **Growth rate**: ~0.9% per year (slowing). **Projections**: ~9.7 billion by 2050, ~10.4 billion by 2100 (then may decline). **Median age**: ~30 years globally.".into());
    }

    // ── Networking & Web ────────────────────────────────────────────────
    if q.contains("http") && (q.contains("protocol") || q.contains("what") || q.contains("status") || q == "http") {
        return Some("**HTTP (HyperText Transfer Protocol)** is the foundation of data communication on the web. **Methods**: GET (read), POST (create), PUT (update), PATCH (partial update), DELETE (remove), OPTIONS, HEAD. **Status codes**: 200 OK, 201 Created, 301 Redirect, 400 Bad Request, 401 Unauthorized, 403 Forbidden, 404 Not Found, 500 Internal Server Error. **HTTPS**: HTTP + TLS encryption (secure). **Versions**: HTTP/1.1 (1997, persistent connections), HTTP/2 (2015, multiplexing, server push), HTTP/3 (2022, uses QUIC/UDP).".into());
    }
    if q.contains("tcp") && q.contains("ip") || q == "tcp/ip" {
        return Some("**TCP/IP** is the fundamental protocol suite of the internet. **TCP (Transmission Control Protocol)**: reliable, ordered, connection-oriented delivery of data — 3-way handshake (SYN → SYN-ACK → ACK), guarantees delivery. **IP (Internet Protocol)**: addresses and routes packets — IPv4 (32-bit, ~4.3 billion addresses) and IPv6 (128-bit, 340 undecillion addresses). **Layers**: Application (HTTP, DNS), Transport (TCP, UDP), Internet (IP), Network Access (Ethernet, WiFi). **UDP**: faster but unreliable alternative to TCP — used for video streaming, gaming, DNS queries.".into());
    }
    if q.contains("oauth") || q.contains("jwt") && !q.contains("code") {
        return Some("**OAuth 2.0** is an authorization framework — lets apps access user resources without sharing passwords. **Flow**: app redirects user to auth server → user grants permission → app receives access token → uses token to access API. **Grant types**: Authorization Code (web apps), Client Credentials (server-to-server), PKCE (mobile/SPA).\n\n**JWT (JSON Web Token)** — a compact, self-contained token format: `header.payload.signature`. Header (algorithm), Payload (claims: sub, iss, exp, iat, custom data), Signature (HMAC or RSA). Used for authentication, API authorization. **Stateless** — server doesn't need session storage.".into());
    }

    // ── Electrical/Hardware ──────────────────────────────────────────────
    if q.contains("cpu") && (q.contains("what") || q.contains("how") || q == "cpu") || q.contains("central processing unit") {
        return Some("**CPU (Central Processing Unit)** — the \"brain\" of a computer. **What it does**: fetches instructions from memory, decodes them, and executes them (fetch-decode-execute cycle). **Key specs**: clock speed (GHz — cycles per second), core count (parallel processing), cache (L1/L2/L3 — fast on-chip memory), architecture (x86-64, ARM). **Modern CPUs**: billions of transistors on a chip ~cm² in size. **Manufacturers**: Intel (Core i3–i9, Xeon), AMD (Ryzen, EPYC), Apple (M1–M4, ARM-based), Qualcomm (Snapdragon, mobile). **Moore's Law**: transistor count doubles ~every 2 years (slowing).".into());
    }
    if q.contains("gpu") && (q.contains("what") || q.contains("how") || q == "gpu") || q.contains("graphics processing unit") {
        return Some("**GPU (Graphics Processing Unit)** — a processor optimized for parallel computation. Originally for rendering graphics, now essential for AI/ML. **Why**: a GPU has thousands of small cores (vs CPU's few large cores), making it ideal for matrix math and parallel workloads. **NVIDIA**: dominates AI (A100, H100, B200 GPUs, CUDA toolkit). **AMD**: Radeon (gaming), Instinct (data center). **Uses**: gaming, 3D rendering, cryptocurrency mining, deep learning training/inference, scientific simulation. **CUDA**: NVIDIA's parallel computing platform (de facto standard for AI). One H100 GPU costs ~$25,000–$40,000.".into());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // EXPANDED KNOWLEDGE BASE — Sports, Entertainment, Science, Philosophy,
    // Economics, History, Space, Food, Travel, Psychology, and more
    // ═══════════════════════════════════════════════════════════════════════

    // ── Sports ──────────────────────────────────────────────────────────
    if q.contains("cricket") && (q.contains("what") || q == "cricket" || q.contains("sport") || q.contains("ipl") || q.contains("rules")) {
        return Some("**Cricket** is a bat-and-ball sport played between two teams of 11 players. **Origin**: England, 16th century. **Formats**: Test (5 days), ODI (50 overs/side), T20 (20 overs/side). **How it works**: batting team scores runs, bowling team tries to get batters out (bowled, caught, LBW, run out, stumped). **Major events**: ICC Cricket World Cup, T20 World Cup, IPL, Ashes, BBL. **Legends**: Sachin Tendulkar, Don Bradman, Virat Kohli, MS Dhoni, Brian Lara, Shane Warne. **IPL**: world's richest cricket league, 10 teams, ~75 days of T20 action.".into());
    }
    if q.contains("football") || q.contains("soccer") && !q.contains("american") {
        return Some("**Football (Soccer)** is the world's most popular sport, played by ~4 billion fans. Two teams of 11 players try to score goals. **Origin**: modern rules codified in England, 1863. **Major competitions**: FIFA World Cup (every 4 years, most-watched sporting event), UEFA Champions League, La Liga, Premier League, Serie A, Bundesliga. **Legends**: Pelé, Diego Maradona, Lionel Messi, Cristiano Ronaldo, Zinedine Zidane, Ronaldinho. **World Cup winners** (most): Brazil (5), Germany (4), Italy (4), Argentina (3). Messi won 2022 World Cup with Argentina.".into());
    }
    if q.contains("basketball") || q.contains("nba") {
        return Some("**Basketball** is a fast-paced team sport — 5 players per side, score by shooting a ball through a 10-foot hoop. **Invented**: James Naismith, 1891, Springfield, Massachusetts. **NBA**: the world's premier league (30 teams). **Legends**: Michael Jordan (6 championships), LeBron James (all-time scorer), Kobe Bryant, Magic Johnson, Larry Bird, Shaquille O'Neal, Stephen Curry (3-point revolution). **Key rules**: 4 quarters (12 min NBA), 24-second shot clock, 3-point line. **Other leagues**: EuroLeague, CBA (China). **WNBA**: women's league, stars like Diana Taurasi, A'ja Wilson.".into());
    }
    if q.contains("tennis") {
        return Some("**Tennis** is a racquet sport played individually (singles) or in pairs (doubles). **Scoring**: 15-30-40-game, 6 games = set, best of 3 or 5 sets. **Grand Slams**: Australian Open (hard court), French Open (clay), Wimbledon (grass), US Open (hard court). **Greatest players**: Roger Federer (20 Slams), Rafael Nadal (22 Slams), Novak Djokovic (24 Slams — record holder), Serena Williams (23 Slams). **ATP/WTA**: men's and women's tours. **Surface matters**: Nadal dominated clay, Federer grass, Djokovic hard courts. Played in 200+ countries.".into());
    }
    if q.contains("formula 1") || q.contains("f1") && q.contains("racing") || q == "f1" {
        return Some("**Formula 1 (F1)** is the pinnacle of motorsport — open-wheel racing at speeds up to 370 km/h (230 mph). **Season**: ~23 races (Grand Prix) worldwide. **Top teams**: Red Bull Racing, Mercedes, Ferrari, McLaren. **Records**: Lewis Hamilton (7 World Championships, tied with Michael Schumacher), Max Verstappen (dominant 2022-2024). **Technology**: hybrid power units (V6 turbo + electric), DRS (drag reduction system), carbon fiber chassis, ground effect aerodynamics. **Cost**: each car costs ~$15-20 million. **Famous circuits**: Monaco, Silverstone, Monza, Spa-Francorchamps.".into());
    }
    if q.contains("olympic") {
        return Some("**The Olympic Games** are the world's foremost multi-sport event, held every 4 years. **Summer Olympics**: 32 sports, ~10,500 athletes, 200+ nations. **Winter Olympics**: 15 sports (skiing, skating, hockey, etc.). **History**: originated in ancient Greece (~776 BC), revived in 1896 (Athens) by Pierre de Coubertin. **Most medals (all-time)**: USA (2,600+), Soviet Union/Russia, Great Britain, Germany. **Most individual golds**: Michael Phelps (23 golds, swimming). **Recent hosts**: Tokyo 2020, Beijing 2022, Paris 2024, Milano-Cortina 2026, LA 2028.".into());
    }

    // ── Entertainment & Movies ──────────────────────────────────────────
    if q.contains("marvel") || q.contains("mcu") || q.contains("avenger") {
        return Some("**Marvel Cinematic Universe (MCU)** is the highest-grossing film franchise in history ($30B+ total box office). **Started**: Iron Man (2008). **Phases**: Phase 1-3 = Infinity Saga (concluded with Avengers: Endgame, 2019 — $2.8B worldwide). Phase 4-6 = Multiverse Saga. **Key characters**: Iron Man (Robert Downey Jr.), Captain America (Chris Evans), Thor (Chris Hemsworth), Black Widow (Scarlett Johansson), Spider-Man (Tom Holland), Black Panther (Chadwick Boseman). **Parent company**: Marvel Studios (owned by Disney since 2009). Also includes Disney+ series (Loki, WandaVision, etc.).".into());
    }
    if q.contains("netflix") || (q.contains("streaming") && q.contains("service")) {
        return Some("**Netflix** is the world's largest streaming service with ~280 million subscribers (2025). **Founded**: 1997 by Reed Hastings and Marc Randolph (originally DVD-by-mail). **Streaming launched**: 2007. **Original content**: Stranger Things, Squid Game, Wednesday, The Crown, Bridgerton, Money Heist, Dark. **Revenue**: ~$34B/year. **Competitors**: Disney+, Amazon Prime Video, HBO Max, Apple TV+, Hulu, Paramount+. **Impact**: disrupted traditional TV/cinema, pioneered binge-watching culture, created the \"Netflix and chill\" era. Available in 190+ countries.".into());
    }
    if q.contains("anime") && (q.contains("what") || q == "anime" || q.contains("best") || q.contains("popular")) {
        return Some("**Anime** is Japanese-style animation with distinctive art, storytelling, and cultural themes. **Top anime (by popularity)**: Naruto, One Piece, Dragon Ball Z, Attack on Titan, Death Note, Fullmetal Alchemist: Brotherhood, Demon Slayer, Jujutsu Kaisen, One Punch Man, My Hero Academia. **Studio Ghibli** (Hayao Miyazaki): Spirited Away, My Neighbor Totoro, Princess Mononoke. **Genres**: Shonen (action/adventure), Seinen (mature), Shojo (romance), Mecha (robots), Isekai (transported to another world). **Platforms**: Crunchyroll, Funimation, Netflix. **Industry**: $25B+ market globally.".into());
    }
    if (q.contains("best") || q.contains("top") || q.contains("greatest")) && q.contains("movie") {
        return Some("**Greatest movies of all time** (consensus across critics/audiences):\n\n1. **The Shawshank Redemption** (1994) — IMDb #1 rated\n2. **The Godfather** (1972) — Francis Ford Coppola\n3. **The Dark Knight** (2008) — Heath Ledger's Joker\n4. **Schindler's List** (1993) — Spielberg\n5. **12 Angry Men** (1957) — courtroom classic\n6. **Pulp Fiction** (1994) — Tarantino\n7. **The Lord of the Rings: Return of the King** (2003)\n8. **Forrest Gump** (1994)\n9. **Inception** (2010) — Nolan\n10. **The Matrix** (1999)\n\n**Box office**: Avatar ($2.9B), Avengers: Endgame ($2.8B), Avatar 2 ($2.3B). **Most Oscars**: Ben-Hur, Titanic, LOTR: RotK (11 each).".into());
    }
    if q.contains("music") && (q.contains("genre") || q.contains("type") || q.contains("history")) || q.contains("music industry") {
        return Some("**Music genres** span centuries of evolution:\n\n- **Classical**: Bach, Mozart, Beethoven (1600s-1800s)\n- **Jazz**: Louis Armstrong, Miles Davis, John Coltrane (1900s)\n- **Blues/Rock**: Elvis, Beatles, Led Zeppelin, Pink Floyd (1950s-70s)\n- **Pop**: Michael Jackson, Madonna, Whitney Houston, Taylor Swift\n- **Hip-Hop**: Tupac, Biggie, Eminem, Kendrick Lamar, Drake\n- **R&B/Soul**: Aretha Franklin, Stevie Wonder, Beyoncé\n- **Electronic/EDM**: Daft Punk, Avicii, Deadmau5, Martin Garrix\n- **K-Pop**: BTS, BLACKPINK (global phenomenon since 2010s)\n- **Latin**: Bad Bunny, Shakira, Daddy Yankee\n\n**Streaming era**: Spotify (600M users), Apple Music, YouTube Music. **Best-selling artists**: The Beatles, Elvis, Michael Jackson, Elton John, Rihanna.".into());
    }
    if (q.contains("bollywood") || q.contains("indian") && q.contains("movie")) && !q.contains("code") {
        return Some("**Bollywood** is India's Hindi-language film industry based in Mumbai — the largest film industry by number of films (~1,500/year). **Legends**: Amitabh Bachchan, Shah Rukh Khan, Aamir Khan, Rajinikanth. **Iconic films**: Sholay, DDLJ, 3 Idiots, Dangal, Lagaan, Baahubali (Telugu), RRR, KGF, Pathaan. **South Indian cinema** (Tollywood, Kollywood, Sandalwood): increasingly global — RRR, KGF, Pushpa. **Characteristics**: song-and-dance sequences, emotional family dramas, colorful visuals. **Revenue**: ~$2.5B/year. **Platforms**: Netflix, Amazon Prime, Hotstar. India produces films in 20+ regional languages.".into());
    }

    // ── Space & Astronomy ───────────────────────────────────────────────
    if q.contains("black hole") {
        return Some("A **Black Hole** is a region where gravity is so strong that nothing — not even light — can escape. **Formation**: when massive stars (>25 solar masses) die in supernovae and their cores collapse. **Types**: (1) Stellar (5-100 solar masses), (2) Supermassive (millions-billions solar masses — at galaxy centers), (3) Intermediate, (4) Primordial (theoretical). **Key concept**: Event Horizon — the boundary beyond which escape is impossible. **Hawking Radiation**: Stephen Hawking predicted black holes slowly evaporate. **First image**: M87* black hole photographed by Event Horizon Telescope (April 2019). **Sagittarius A***: our Milky Way's central black hole (~4 million solar masses).".into());
    }
    if q.contains("mars") && (q.contains("planet") || q.contains("nasa") || q.contains("coloniz") || q.contains("what") || q == "mars") {
        return Some("**Mars** — the 4th planet from the Sun, called the \"Red Planet\" (iron oxide surface). **Distance**: ~225 million km from Earth. **Day**: 24h 37m (similar to Earth). **Year**: 687 Earth days. **Atmosphere**: 95% CO₂, very thin (~1% of Earth's). **Temperature**: avg -60°C. **Water**: ice at poles and possibly underground. **Exploration**: Curiosity rover (2012-present), Perseverance + Ingenuity helicopter (2021-present), Mars Reconnaissance Orbiter. **SpaceX**: Elon Musk aims to colonize Mars using Starship (first crewed mission target: late 2020s-2030s). **Challenges**: radiation, no breathable air, 6-9 month travel time.".into());
    }
    if q.contains("solar system") || (q.contains("planet") && (q.contains("how many") || q.contains("name"))) {
        return Some("**The Solar System** — our Sun and everything orbiting it.\n\n**8 Planets** (in order): Mercury, Venus, Earth, Mars (rocky/inner), Jupiter, Saturn (gas giants), Uranus, Neptune (ice giants).\n\n**Key facts**:\n- **Sun**: 99.86% of system's mass, 1.4M km diameter\n- **Jupiter**: largest planet, 79+ moons, Great Red Spot storm\n- **Saturn**: famous rings (ice and rock particles)\n- **Earth**: only known planet with life\n- **Pluto**: reclassified as dwarf planet (2006)\n- **Asteroid Belt**: between Mars and Jupiter\n- **Kuiper Belt**: beyond Neptune (Pluto, Eris)\n- **Oort Cloud**: outermost boundary (~2 light-years out)\n\n**Age**: ~4.6 billion years.".into());
    }
    if q.contains("nasa") && (q.contains("what") || q == "nasa") {
        return Some("**NASA (National Aeronautics and Space Administration)** — the United States' space agency, founded **1958**. **Headquarters**: Washington, D.C. **Budget**: ~$25B/year.\n\n**Historic missions**: Mercury, Gemini, Apollo (Moon landings 1969-1972), Space Shuttle (1981-2011), ISS, Hubble Space Telescope, Mars rovers.\n\n**Active programs**: Artemis (return to Moon), James Webb Space Telescope (launched 2021 — deepest infrared views of universe), Mars Sample Return, Europa Clipper (Jupiter's moon).\n\n**Other space agencies**: ESA (Europe), ISRO (India), CNSA (China), Roscosmos (Russia), JAXA (Japan). **Private space**: SpaceX, Blue Origin, Virgin Galactic.".into());
    }

    // ── Biology & Nature ────────────────────────────────────────────────
    if q.contains("photosynthesis") {
        return Some("**Photosynthesis** is the process by which plants, algae, and some bacteria convert sunlight, water, and CO₂ into glucose and oxygen.\n\n**Equation**: 6CO₂ + 6H₂O + light energy → C₆H₁₂O₆ + 6O₂\n\n**Two stages**:\n1. **Light reactions** (thylakoid membranes): chlorophyll absorbs light → splits water → produces ATP and NADPH + releases O₂\n2. **Calvin Cycle** (stroma): uses ATP + NADPH to fix CO₂ into glucose (carbon fixation)\n\n**Key molecule**: Chlorophyll (green pigment, absorbs red/blue light, reflects green)\n\n**Importance**: produces virtually all oxygen in Earth's atmosphere and is the base of nearly all food chains. Plants photosynthesize ~6 × CO₂ that humans emit.".into());
    }
    if q.contains("dna") && (q.contains("what") || q == "dna" || q.contains("genetic")) {
        return Some("**DNA (Deoxyribonucleic Acid)** is the molecule that carries genetic instructions for all known living organisms.\n\n**Structure**: double helix (discovered by Watson & Crick, 1953, with Rosalind Franklin's X-ray data). Two strands of nucleotides held by base pairs: **A-T** (adenine-thymine), **G-C** (guanine-cytosine).\n\n**Key concepts**:\n- **Gene**: a segment of DNA coding for a protein\n- **Chromosome**: organized DNA structure (humans have 46 = 23 pairs)\n- **Genome**: complete set of DNA (~3.2 billion base pairs in humans)\n- **Replication**: DNA copies itself before cell division\n- **Transcription → Translation**: DNA → mRNA → Protein\n\n**Applications**: forensics, paternity testing, gene therapy, CRISPR gene editing, ancestry tracing.".into());
    }
    if q.contains("evolution") && (q.contains("what") || q.contains("darwin") || q.contains("theory") || q == "evolution") {
        return Some("**Evolution** is the process by which species change over generations through variation, selection, and inheritance.\n\n**Charles Darwin** (1859, *On the Origin of Species*) proposed **natural selection**: organisms with favorable traits survive and reproduce more. **Alfred Russel Wallace** independently arrived at the same idea.\n\n**Mechanisms**: (1) Natural selection, (2) Genetic drift (random), (3) Mutation (new variation), (4) Gene flow (migration). **Evidence**: fossils, DNA comparison, homologous structures, observed speciation. **Timeline**: life began ~3.8 billion years ago. Humans and chimps diverged ~6-7 million years ago. Modern Homo sapiens appeared ~300,000 years ago.".into());
    }
    if q.contains("cell") && (q.contains("biology") || q.contains("what is a cell") || q.contains("parts of") || q.contains("structure")) {
        return Some("A **Cell** is the basic unit of life. All organisms are made of cells.\n\n**Two types**: (1) **Prokaryotic** (no nucleus — bacteria, archaea), (2) **Eukaryotic** (has nucleus — plants, animals, fungi).\n\n**Key organelles**:\n- **Nucleus**: contains DNA, controls cell\n- **Mitochondria**: powerhouse — produces ATP (energy)\n- **Ribosomes**: protein synthesis\n- **Endoplasmic Reticulum**: protein/lipid processing\n- **Golgi Apparatus**: packaging and shipping\n- **Cell Membrane**: selective barrier\n- **Chloroplast** (plants only): photosynthesis\n- **Cell Wall** (plants, bacteria): structural support\n\n**Human body**: ~37.2 trillion cells. Cells range from ~1 μm (bacteria) to ~100 μm+ (human egg cell).".into());
    }

    // ── Physics ─────────────────────────────────────────────────────────
    if q.contains("gravity") && (q.contains("what") || q == "gravity" || q.contains("force")) {
        return Some("**Gravity** is the fundamental force of attraction between objects with mass.\n\n**Newton's Law** (1687): F = G × (m₁ × m₂) / r² — force is proportional to masses and inversely proportional to distance squared. G = 6.674 × 10⁻¹¹ N⋅m²/kg².\n\n**Einstein's General Relativity** (1915): gravity isn't a force — it's the curvature of spacetime caused by mass/energy. Massive objects bend spacetime, and other objects follow curved paths.\n\n**Effects**: keeps planets in orbit, causes tides, shapes galaxies, creates black holes. **Gravitational waves**: ripples in spacetime, first detected by LIGO in 2015 (from merging black holes). **On Earth**: acceleration due to gravity = 9.8 m/s².".into());
    }
    if q.contains("quantum") && (q.contains("what") || q.contains("computing") || q.contains("physics") || q.contains("mechanic")) {
        return Some("**Quantum Physics/Mechanics** describes nature at the atomic and subatomic level, where classical physics breaks down.\n\n**Key principles**:\n- **Wave-particle duality**: particles behave as both waves and particles\n- **Superposition**: a particle exists in all possible states simultaneously until measured\n- **Entanglement**: two particles linked — measuring one instantly affects the other (regardless of distance)\n- **Uncertainty Principle** (Heisenberg): can't precisely know both position and momentum simultaneously\n- **Quantization**: energy comes in discrete packets (quanta)\n\n**Quantum Computing**: uses qubits (0 and 1 simultaneously via superposition). Potential to solve problems intractable for classical computers. **Leaders**: IBM, Google (Sycamore — quantum supremacy 2019), IonQ, Microsoft.".into());
    }
    if q.contains("relativity") && (q.contains("einstein") || q.contains("theory") || q.contains("what") || q.contains("special") || q.contains("general")) {
        return Some("**Einstein's Theory of Relativity** — two interconnected theories that revolutionized physics:\n\n**Special Relativity** (1905):\n- Speed of light (c = 299,792,458 m/s) is constant for all observers\n- **E = mc²** — energy and mass are equivalent\n- Time dilation: moving clocks tick slower\n- Length contraction: moving objects shrink in direction of motion\n- Nothing with mass can reach the speed of light\n\n**General Relativity** (1915):\n- Gravity = curvature of spacetime by mass/energy\n- Predicted: black holes, gravitational waves, gravitational lensing, frame-dragging\n- All confirmed experimentally. GPS satellites must account for relativistic time dilation (~38 μs/day correction).".into());
    }

    // ── Chemistry ───────────────────────────────────────────────────────
    if q.contains("periodic table") || (q.contains("element") && q.contains("chemical")) {
        return Some("**The Periodic Table** organizes all known chemical elements (118 total) by atomic number.\n\n**Structure**: 7 periods (rows), 18 groups (columns). **Groups**: Group 1 = Alkali metals (Li, Na, K), Group 17 = Halogens (F, Cl, Br), Group 18 = Noble gases (He, Ne, Ar). **Key elements**: H (1), C (6), N (7), O (8), Fe (26), Au (79), U (92).\n\n**Created by**: Dmitri Mendeleev (1869) — predicted undiscovered elements by leaving gaps. **Properties trend**: electronegativity increases right and up, atomic radius increases left and down, ionization energy increases right and up. **Recent additions**: Nihonium (113), Flerovium (114), Moscovium (115), Oganesson (118) — all synthetic.".into());
    }

    // ── History ─────────────────────────────────────────────────────────
    if q.contains("world war") && (q.contains("1") || q.contains("i") && !q.contains("ii")) || q.contains("ww1") {
        return Some("**World War I** (1914-1918) — the \"Great War.\" **Trigger**: assassination of Archduke Franz Ferdinand (June 28, 1914). **Sides**: Allied Powers (UK, France, Russia, later USA, Italy) vs Central Powers (Germany, Austria-Hungary, Ottoman Empire). **Key features**: trench warfare, poison gas, machine guns, tanks (first used 1916). **Casualties**: ~20 million dead (9.7M military, 10M civilian), ~21 million wounded. **Ended**: November 11, 1918 (Armistice Day). **Aftermath**: Treaty of Versailles (harsh terms on Germany), fall of empires (Ottoman, Austro-Hungarian, Russian, German), League of Nations formed. Set the stage for WWII.".into());
    }
    if q.contains("world war") && (q.contains("2") || q.contains("ii")) || q.contains("ww2") || q.contains("wwii") {
        return Some("**World War II** (1939-1945) — the deadliest conflict in human history.\n\n**Sides**: Allies (USA, UK, Soviet Union, China, France) vs Axis (Nazi Germany, Imperial Japan, Italy). **Started**: Sept 1, 1939 (Germany invades Poland). **Key events**: Dunkirk, Battle of Britain, Pearl Harbor (Dec 1941, USA enters), D-Day (June 6, 1944), Battle of Stalingrad, Hiroshima/Nagasaki (Aug 1945). **The Holocaust**: Nazi genocide — 6 million Jews and millions of others murdered. **Casualties**: ~70-85 million dead (3% of world population). **Ended**: Germany surrendered May 8, 1945; Japan surrendered Sept 2, 1945. **Aftermath**: United Nations, Cold War, decolonization.".into());
    }
    if q.contains("ancient egypt") || (q.contains("egypt") && q.contains("pyramid")) {
        return Some("**Ancient Egypt** — one of the world's oldest and most influential civilizations, centered along the Nile River (c. 3100 BC – 30 BC).\n\n**Key achievements**: Pyramids of Giza (built ~2560 BC, Great Pyramid = 146m, one of Seven Wonders), Sphinx, hieroglyphic writing, papyrus, advanced medicine, 365-day calendar.\n\n**Famous pharaohs**: Tutankhamun (tomb discovered 1922), Ramesses II (greatest builder), Cleopatra VII (last pharaoh), Khufu (built Great Pyramid). **Religion**: polytheistic — Ra (sun), Osiris (afterlife), Isis, Anubis. Mummification for the afterlife. **End**: conquered by Alexander the Great (332 BC), then Roman Empire (30 BC, Cleopatra's death).".into());
    }
    if q.contains("renaissance") {
        return Some("**The Renaissance** (\"rebirth\") was a cultural/intellectual movement in Europe from the 14th-17th century, originating in Italy.\n\n**What changed**: revival of classical Greek/Roman learning, humanism (focus on human potential), scientific inquiry, artistic innovation.\n\n**Key figures**:\n- **Art**: Leonardo da Vinci (Mona Lisa, Vitruvian Man), Michelangelo (Sistine Chapel, David), Raphael, Botticelli\n- **Science**: Galileo Galilei, Copernicus (heliocentrism), Johannes Kepler\n- **Literature**: Dante, Petrarch, Machiavelli (The Prince), Shakespeare (English Renaissance)\n- **Invention**: Gutenberg's printing press (~1440) — revolutionized knowledge sharing\n\n**Impact**: laid groundwork for the Scientific Revolution, Reformation, and the modern world.".into());
    }

    // ── Philosophy ──────────────────────────────────────────────────────
    if q.contains("philosophy") && (q.contains("what") || q == "philosophy" || q.contains("meaning")) {
        return Some("**Philosophy** (Greek: \"love of wisdom\") is the study of fundamental questions about existence, knowledge, values, reason, and reality.\n\n**Major branches**:\n- **Metaphysics**: What is reality? What exists?\n- **Epistemology**: What is knowledge? How do we know things?\n- **Ethics**: What is right and wrong? How should we live?\n- **Logic**: What constitutes valid reasoning?\n- **Aesthetics**: What is beauty? What is art?\n\n**Key thinkers**: Socrates (\"I know that I know nothing\"), Plato (Forms, The Republic), Aristotle (logic, virtue ethics), Descartes (\"I think, therefore I am\"), Kant (categorical imperative), Nietzsche (will to power), Sartre (existentialism). Philosophy is the ancestor of all sciences.".into());
    }
    if q.contains("stoicism") || q.contains("stoic") {
        return Some("**Stoicism** is an ancient Greek philosophy founded by Zeno of Citium (~300 BC). Core idea: we can't control what happens to us, but we can control how we respond.\n\n**Key principles**:\n- **Dichotomy of control**: focus only on what you can control (thoughts, actions), accept what you can't (events, others' behavior)\n- **Virtue is the highest good**: wisdom, courage, justice, temperance\n- **Amor fati**: love your fate — embrace everything that happens\n- **Memento mori**: remember death — live with urgency and gratitude\n\n**Key Stoics**: Marcus Aurelius (*Meditations* — Roman Emperor), Seneca (*Letters*), Epictetus (*Discourses* — born a slave). **Modern resurgence**: hugely popular in self-improvement, leadership, and mental resilience circles.".into());
    }
    if q.contains("existentialism") || q.contains("existentialist") {
        return Some("**Existentialism** is a philosophical movement focused on individual freedom, choice, and meaning.\n\n**Core ideas**:\n- **\"Existence precedes essence\"** (Sartre): you're not born with a predetermined purpose — you create your own meaning\n- **Radical freedom**: you are always free to choose — and responsible for those choices\n- **Absurdity** (Camus): life has no inherent meaning, but we must find/create meaning anyway\n- **Authenticity**: live genuinely, not according to social expectations\n- **Angst/anxiety**: the dread of total freedom and responsibility\n\n**Key thinkers**: Jean-Paul Sartre, Simone de Beauvoir, Albert Camus (*The Stranger*, *The Myth of Sisyphus*), Søren Kierkegaard (proto-existentialist), Friedrich Nietzsche, Martin Heidegger.".into());
    }

    // ── Economics & Business ────────────────────────────────────────────
    if q.contains("inflation") && (q.contains("what") || q == "inflation" || q.contains("economics")) {
        return Some("**Inflation** is the rate at which the general price level of goods and services rises, eroding purchasing power.\n\n**Causes**: (1) **Demand-pull** — too much money chasing too few goods, (2) **Cost-push** — rising production costs (oil, wages), (3) **Monetary** — excessive money supply growth.\n\n**Measured by**: CPI (Consumer Price Index), PCE. **Target**: most central banks aim for ~2% per year (healthy). **Hyperinflation**: extreme (Zimbabwe 2008: 79.6 billion% per month, Venezuela 2018). **Deflation**: prices falling (can be worse — Japan's \"lost decades\"). **Central bank tools**: interest rates (raise = cool inflation), quantitative tightening. **2021-2023**: global inflation spike post-COVID due to supply chains + stimulus spending.".into());
    }
    if q.contains("cryptocurrency") || q.contains("crypto") || q.contains("bitcoin") {
        return Some("**Cryptocurrency** is digital money secured by cryptography, typically on a decentralized blockchain.\n\n**Bitcoin** (BTC): created 2009 by pseudonymous **Satoshi Nakamoto**. First and largest crypto by market cap. Supply cap: 21 million coins. Uses proof-of-work mining.\n\n**Ethereum** (ETH): created by Vitalik Buterin (2015). Supports smart contracts and DApps. Switched to proof-of-stake (2022). **DeFi** (Decentralized Finance) and **NFTs** run on Ethereum.\n\n**Other major cryptos**: Solana (fast/cheap), Cardano, Polkadot, Dogecoin (meme coin). **Total market cap**: ~$2-3 trillion (volatile). **Concerns**: energy use, scams, volatility, regulation. **Blockchain**: distributed ledger technology — foundation of crypto.".into());
    }
    if (q.contains("stock market") || q.contains("stock exchange")) && !q.contains("code") {
        return Some("**The Stock Market** is where shares of publicly traded companies are bought and sold.\n\n**Major exchanges**: NYSE (New York Stock Exchange — largest, $25T+ market cap), NASDAQ (tech-heavy), London Stock Exchange, Tokyo Stock Exchange, Shanghai, BSE/NSE (India).\n\n**Key indices**: S&P 500 (top 500 US companies), Dow Jones (30 large US companies), NASDAQ Composite (tech), Nifty 50 (India), FTSE 100 (UK).\n\n**How it works**: companies issue shares (IPO) to raise capital. Investors buy/sell shares hoping prices rise. **Bull market**: prices rising. **Bear market**: prices falling 20%+. **Key concepts**: dividends, P/E ratio, market cap, ETFs, mutual funds. **Advice**: diversify, invest long-term, don't try to time the market.".into());
    }
    if q.contains("startup") && (q.contains("what") || q == "startup" || q.contains("how to") || q.contains("build")) {
        return Some("A **Startup** is a young company founded to develop a scalable business model, often technology-driven.\n\n**Stages**: Idea → MVP (Minimum Viable Product) → Product-Market Fit → Growth → Scale.\n\n**Funding rounds**: Pre-seed, Seed ($100K-$2M), Series A ($2-15M), Series B ($15-50M), Series C+ → IPO or acquisition.\n\n**Key concepts**: burn rate, runway, pivot, product-market fit, CAC (customer acquisition cost), LTV (lifetime value), MRR/ARR (recurring revenue).\n\n**Famous startups → giants**: Apple (garage), Google (dorm room), Facebook (dorm room), Amazon (garage), Uber, Airbnb, Stripe. **Startup hubs**: Silicon Valley, NYC, London, Bangalore, Tel Aviv, Berlin. **Failure rate**: ~90% of startups fail — most due to no market need.".into());
    }

    // ── Psychology ───────────────────────────────────────────────────────
    if q.contains("psychology") && (q.contains("what") || q == "psychology") {
        return Some("**Psychology** is the scientific study of mind and behavior.\n\n**Major branches**:\n- **Clinical**: mental health diagnosis and treatment\n- **Cognitive**: how we think, remember, perceive\n- **Developmental**: how people change across lifespan\n- **Social**: how others influence our behavior\n- **Behavioral**: learning through conditioning\n- **Neuropsychology**: brain-behavior relationships\n\n**Key figures**: Sigmund Freud (psychoanalysis), Carl Jung (archetypes), B.F. Skinner (behaviorism), Abraham Maslow (hierarchy of needs), Carl Rogers (humanistic), Daniel Kahneman (cognitive biases).\n\n**Famous experiments**: Milgram (obedience), Stanford Prison, Marshmallow Test, Pavlov's dogs, Asch conformity. **Applications**: therapy, education, marketing, UX design, forensics.".into());
    }

    // ── Food & Cooking ──────────────────────────────────────────────────
    if q.contains("sushi") && (q.contains("what") || q == "sushi") {
        return Some("**Sushi** is a Japanese dish featuring vinegared rice combined with various ingredients, most commonly raw fish.\n\n**Types**: Nigiri (rice + fish on top), Maki (rice + filling wrapped in seaweed), Sashimi (just raw fish, no rice), Temaki (hand roll), Chirashi (scattered bowl).\n\n**Popular fish**: salmon (sake), tuna (maguro), yellowtail (hamachi), shrimp (ebi), eel (unagi). **Accompaniments**: soy sauce, wasabi, pickled ginger. **History**: originated as a preservation method in Southeast Asia (~2nd century BC), evolved into modern form in Edo-era Japan (1800s). **Etiquette**: eat nigiri in one bite, dip fish-side (not rice) in soy sauce. **Fun fact**: Jiro Ono (Jiro Dreams of Sushi) is the oldest 3-Michelin-star chef.".into());
    }
    if q.contains("coffee") && (q.contains("what") || q == "coffee" || q.contains("history") || q.contains("type")) {
        return Some("**Coffee** is the world's most popular stimulant beverage, made from roasted beans of the Coffea plant.\n\n**Origin**: Ethiopia (~9th century legend of goat herder Kaldi). **Types of beans**: Arabica (60-70%, smoother, more complex), Robusta (30-40%, stronger, more caffeine).\n\n**Drinks**: Espresso (concentrated shot), Americano (espresso + water), Latte (espresso + steamed milk), Cappuccino (espresso + foam), Cold Brew (steeped 12-24 hours), Pour Over, French Press.\n\n**Top producers**: Brazil (#1, 30%+ of world supply), Vietnam, Colombia, Ethiopia, Indonesia. **Caffeine**: 80-100mg per cup. **Industry**: $500B+ globally. ~2.25 billion cups consumed daily worldwide. **Third wave coffee**: emphasis on origin, roasting profiles, and brewing methods.".into());
    }
    if q.contains("pizza") && (q.contains("what") || q == "pizza" || q.contains("history")) {
        return Some("**Pizza** originated in Naples, Italy, as a flatbread with toppings for the working class.\n\n**Margherita** (1889): tomato, mozzarella, basil — created to honor Queen Margherita of Italy, representing the Italian flag. **Marinara**: tomato, garlic, oregano, olive oil (no cheese).\n\n**Styles**: Neapolitan (thin, charred, 90-second bake at 450°C), New York (large foldable slices), Chicago deep-dish (thick, pan-baked), Detroit (thick, crispy, cheese to edges), Roman (thin, crispy). **Global industry**: $150B+. **Fun facts**: Americans eat ~3 billion pizzas/year, the world's largest pizza was 1,261m² (made in Rome, 2012).".into());
    }

    // ── Travel & Countries ──────────────────────────────────────────────
    if q.contains("japan") && (q.contains("what") || q == "japan" || q.contains("country") || q.contains("about") || q.contains("culture")) {
        return Some("**Japan** — island nation in East Asia, known for its unique blend of ancient tradition and cutting-edge technology.\n\n**Capital**: Tokyo (world's largest metro area, ~37 million people). **Population**: ~125 million. **Language**: Japanese. **Government**: Constitutional monarchy (Emperor is ceremonial; Prime Minister governs).\n\n**Culture**: anime/manga, cherry blossoms (sakura), martial arts (judo, karate, sumo), tea ceremony, bullet trains (shinkansen, 320 km/h), temples and shrines. **Economy**: 4th largest GDP. Known for: Toyota, Sony, Nintendo, Honda, Panasonic. **Food**: sushi, ramen, tempura, wagyu, matcha. **Life expectancy**: ~84 years (one of highest). **Technology**: robotics leader, 5G, gaming industry.".into());
    }
    if q.contains("india") && (q.contains("what") || q == "india" || q.contains("country") || q.contains("about") || q.contains("culture")) && !q.contains("indian movie") && !q.contains("code") {
        return Some("**India** — the world's most populous country (~1.44 billion) and largest democracy.\n\n**Capital**: New Delhi. **Languages**: Hindi + English (official), 22 scheduled languages, 100s of regional languages. **States**: 28 states, 8 union territories.\n\n**Culture**: incredibly diverse — festivals (Diwali, Holi, Eid, Pongal, Durga Puja), classical dance (Bharatanatyam, Kathak), yoga, Bollywood, cricket. **Economy**: 5th largest GDP, IT hub (Bangalore, Hyderabad), pharmaceutical powerhouse. **History**: Indus Valley Civilization (~3300 BC), Mughal Empire, British colonial rule, independence 1947 (Gandhi, Nehru). **Food**: regional cuisines — biryani, dosa, butter chicken, paneer, vada pav. **Tech**: ISRO (Chandrayaan Moon mission), growing startup ecosystem (Flipkart, Zomato, Razorpay).".into());
    }

    // ── People & Personalities ──────────────────────────────────────────
    if q.contains("elon musk") {
        return Some("**Elon Musk** (born June 28, 1971, Pretoria, South Africa) — entrepreneur, engineer, and one of the world's wealthiest people.\n\n**Companies**:\n- **Tesla** (CEO): electric vehicles, solar energy, Powerwall. Made EVs mainstream.\n- **SpaceX** (CEO): reusable rockets (Falcon 9, Starship), Starlink satellite internet, goal: Mars colonization\n- **X (formerly Twitter)**: acquired 2022 for $44B\n- **Neuralink**: brain-computer interfaces\n- **The Boring Company**: tunneling/infrastructure\n- **xAI**: AI company (Grok chatbot)\n\n**Earlier**: co-founded PayPal (sold to eBay for $1.5B in 2002). **Net worth**: fluctuates around $200-300B. **Controversy**: polarizing figure — admired for innovation, criticized for management style and social media behavior.".into());
    }
    if q.contains("steve jobs") {
        return Some("**Steve Jobs** (1955-2011) — co-founder and visionary CEO of **Apple**. One of the most influential innovators in tech history.\n\n**Key products**: Macintosh (1984), iMac (1998), iPod (2001), iPhone (2007 — changed everything), iPad (2010), App Store. **Also founded**: Pixar Animation Studios (Toy Story, Finding Nemo — sold to Disney for $7.4B) and NeXT (acquired by Apple 1997).\n\n**Philosophy**: intersection of technology and liberal arts. Famous for obsessive attention to design and user experience. **Reality distortion field**: his ability to convince people anything was possible.\n\n**Famous quotes**: \"Stay hungry, stay foolish\", \"Think different\". Died of pancreatic cancer on October 5, 2011.".into());
    }
    if q.contains("albert einstein") || (q.contains("einstein") && !q.contains("goldberg")) {
        return Some("**Albert Einstein** (1879-1955) — theoretical physicist, widely regarded as the greatest scientist of the 20th century.\n\n**Key contributions**:\n- **Special Relativity** (1905): E = mc², time dilation\n- **General Relativity** (1915): gravity as spacetime curvature\n- **Photoelectric Effect** (1905): light as quanta (photons) — won Nobel Prize 1921\n- **Brownian Motion**: confirmed atoms exist\n\n**Famous**: wild hair, pacifist, violinist. Born in Germany, fled Nazi regime (1933), became US citizen. Worked at Institute for Advanced Study, Princeton. **Quote**: \"Imagination is more important than knowledge.\" His brain was preserved after death for study. **E = mc²** is perhaps the most famous equation ever written.".into());
    }

    // ── Opinions & Discussion Topics ────────────────────────────────────
    if q.contains("best programming language") || (q.contains("best") && q.contains("language") && q.contains("learn")) {
        return Some("There's no single \"best\" programming language — it depends on your goal:\n\n- **First language / general purpose**: **Python** — easiest to learn, massive ecosystem\n- **Web development**: **JavaScript** (frontend + backend with Node.js)\n- **Mobile apps**: **Swift** (iOS), **Kotlin** (Android), or **React Native/Flutter** (cross-platform)\n- **Systems / performance**: **Rust** (memory safety) or **C++** (raw power)\n- **Enterprise / jobs**: **Java** (banking, Android), **C#** (.NET, gaming/Unity)\n- **Data science / AI**: **Python** (pandas, TensorFlow, PyTorch)\n- **DevOps / cloud**: **Go** (Docker, Kubernetes, fast microservices)\n\n**My advice**: start with Python or JavaScript — they open the most doors. Then learn a systems language (Rust/C++) for depth.".into());
    }
    if (q.contains("meaning of life") || q.contains("purpose of life") || q.contains("what is the meaning")) && !q.contains("code") {
        return Some("The **meaning of life** is one of humanity's oldest questions. Different perspectives:\n\n- **Philosophy**: Existentialists say there's no inherent meaning — you create your own (Sartre, Camus). Stoics say: live virtuously. Aristotle: pursue *eudaimonia* (flourishing).\n- **Religion**: serving God/a higher purpose, achieving enlightenment (Buddhism), following dharma (Hinduism)\n- **Science**: biologically, propagate genes. But consciousness lets us transcend biology.\n- **Psychology** (Viktor Frankl, *Man's Search for Meaning*): meaning comes from purpose, love, and how we face suffering\n- **Pop culture**: \"42\" (Douglas Adams, *Hitchhiker's Guide*)\n\n**My take**: meaning isn't found — it's built. Through relationships, creation, curiosity, contribution, and growth. What gives YOUR life meaning?".into());
    }

    // ── Fun Facts & Trivia ──────────────────────────────────────────────
    if q.contains("fun fact") || q.contains("interesting fact") || q.contains("random fact") || q.contains("tell me something interesting") || q.contains("did you know") {
        return Some("Here are some fascinating facts:\n\n1. **Octopuses have 3 hearts**, blue blood, and 9 brains (one central + one in each arm)\n2. **Honey never spoils** — archaeologists found 3,000-year-old honey in Egyptian tombs, still edible\n3. **A day on Venus is longer than its year** — 243 Earth days to rotate, 225 to orbit the Sun\n4. **Bananas are radioactive** — they contain potassium-40 (you'd need to eat 10 million at once for radiation poisoning)\n5. **The shortest war in history** lasted 38-45 minutes (Anglo-Zanzibar War, 1896)\n6. **There are more possible chess games than atoms in the observable universe** (~10¹²⁰ vs ~10⁸⁰)\n7. **The human brain can store approximately 2.5 petabytes** (2.5 million gigabytes) of information\n\nWant more? I've got plenty!".into());
    }
    if q.contains("joke") || q.contains("tell me a joke") || q.contains("make me laugh") || q.contains("funny") && q.contains("tell") {
        return Some("Here are some for you:\n\n**Tech jokes:**\n- Why do programmers prefer dark mode? Because light attracts bugs.\n- A SQL query walks into a bar, sees two tables, and asks: \"Can I JOIN you?\"\n- There are 10 types of people in the world: those who understand binary and those who don't.\n\n**General:**\n- I told my wife she was drawing her eyebrows too high. She looked surprised.\n- Why don't scientists trust atoms? Because they make up everything.\n- What do you call a fake noodle? An impasta.\n\n**Philosophical:**\n- Is the S or C silent in \"scent\"?\n- If you replace the W in \"what, where, when\" with T, you get the answer.\n\nWant more jokes? I've got programmer humor, dad jokes, science puns — you name it!".into());
    }

    // ── Motivation & Quotes ─────────────────────────────────────────────
    if q.contains("motivat") || q.contains("inspire me") || q.contains("motivation") || (q.contains("quote") && (q.contains("motivat") || q.contains("inspir"))) {
        return Some("Here's some fuel for your fire:\n\n> \"The only way to do great work is to love what you do.\" — **Steve Jobs**\n\n> \"It is not the strongest of the species that survives, nor the most intelligent. It is the one most adaptable to change.\" — **Charles Darwin**\n\n> \"I have not failed. I've just found 10,000 ways that won't work.\" — **Thomas Edison**\n\n> \"The best time to plant a tree was 20 years ago. The second best time is now.\" — **Chinese Proverb**\n\n> \"You miss 100% of the shots you don't take.\" — **Wayne Gretzky**\n\n> \"In the middle of difficulty lies opportunity.\" — **Albert Einstein**\n\n> \"The future belongs to those who believe in the beauty of their dreams.\" — **Eleanor Roosevelt**\n\nRemember: every expert was once a beginner. Keep going! What's your current challenge?".into());
    }

    // ── Recommendations ─────────────────────────────────────────────────
    if q.contains("book") && (q.contains("recommend") || q.contains("suggest") || q.contains("best") || q.contains("must read") || q.contains("top")) {
        return Some("**Must-read books across genres:**\n\n**Fiction:**\n- *1984* — George Orwell (dystopia, surveillance)\n- *To Kill a Mockingbird* — Harper Lee (justice, empathy)\n- *The Alchemist* — Paulo Coelho (following your dreams)\n- *Sapiens* — Yuval Noah Harari (history of humankind)\n\n**Self-improvement:**\n- *Atomic Habits* — James Clear\n- *Deep Work* — Cal Newport\n- *Thinking, Fast and Slow* — Daniel Kahneman\n\n**Tech/Business:**\n- *Zero to One* — Peter Thiel\n- *The Lean Startup* — Eric Ries\n- *Clean Code* — Robert C. Martin\n\n**Philosophy:**\n- *Meditations* — Marcus Aurelius\n- *Man's Search for Meaning* — Viktor Frankl\n\n**Science:**\n- *A Brief History of Time* — Stephen Hawking\n- *The Selfish Gene* — Richard Dawkins\n\nWhat genre interests you? I can give more specific picks.".into());
    }

    // ── Catch broader topics ────────────────────────────────────────────
    if q.contains("climate change") || q.contains("global warming") {
        return Some("**Climate Change** refers to long-term shifts in global temperatures and weather patterns, primarily driven by human activity since the 1800s.\n\n**Cause**: burning fossil fuels (coal, oil, gas) releases greenhouse gases (CO₂, methane) that trap heat. CO₂ levels: ~280 ppm (pre-industrial) → ~425 ppm (2025).\n\n**Effects**: global avg temp +1.1°C since pre-industrial. Rising sea levels (~20cm since 1900), melting ice caps, more extreme weather (floods, droughts, hurricanes), ocean acidification, biodiversity loss.\n\n**Paris Agreement** (2015): limit warming to 1.5-2°C. **Solutions**: renewable energy (solar, wind), electrification, reforestation, carbon capture, nuclear power, lifestyle changes.\n\n**Current pace**: on track for ~2.5-3°C by 2100 without drastic action. The 2020s are the decisive decade.".into());
    }
    if q.contains("artificial general intelligence") || q.contains("agi") && !q.contains("kala") {
        return Some("**AGI (Artificial General Intelligence)** — a hypothetical AI system that can understand, learn, and apply intelligence across ANY task at human level or beyond.\n\n**Current AI** = Narrow AI (good at specific tasks: chess, language, image recognition). **AGI** = can do ANYTHING a human can — reason, plan, learn new skills, be creative, understand context.\n\n**Debate**: (1) **Optimists** (OpenAI, Demis Hassabis): AGI could arrive by 2030-2040. (2) **Skeptics**: we're nowhere near — LLMs are pattern-matching, not understanding. (3) **Concerned** (Eliezer Yudkowsky, Stuart Russell): AGI alignment problem — how do we ensure it shares human values?\n\n**Approaches**: scaling LLMs (more data + compute), neuroscience-inspired architectures, hybrid symbolic + neural systems. **The big question**: will AGI be humanity's greatest achievement or greatest risk?".into());
    }
    if q.contains("meditation") || q.contains("mindfulness") {
        return Some("**Meditation** is a practice of focused attention and awareness, cultivating mental clarity and calm.\n\n**Types**:\n- **Mindfulness** (Vipassana): observe thoughts without judgment. Focus on breath, body sensations.\n- **Concentration** (Samatha): single-pointed focus on one object (mantra, candle, breath)\n- **Loving-kindness** (Metta): cultivate compassion for self and others\n- **Transcendental Meditation (TM)**: silently repeat a mantra, 20 min twice daily\n- **Zen (Zazen)**: sitting meditation, often with koans (paradoxical riddles)\n\n**Scientific benefits**: reduces stress/anxiety (lowers cortisol), improves focus and memory, reduces blood pressure, changes brain structure (thicker prefrontal cortex), improves emotional regulation.\n\n**How to start**: sit comfortably, close eyes, focus on breath. When mind wanders (it will), gently return to breath. Start with 5 minutes/day. Apps: Headspace, Calm, Insight Timer.".into());
    }

    None
}

/// Intelligent comparison handler for "which is X" / "X vs Y" questions.
fn comparison_handler(q: &str) -> Option<String> {
    let q = q.to_lowercase();
    // "X vs Y" or "X versus Y"
    let parts: Vec<&str> = if q.contains(" vs ") {
        q.splitn(2, " vs ").collect()
    } else if q.contains(" versus ") {
        q.splitn(2, " versus ").collect()
    } else {
        return None;
    };
    if parts.len() < 2 { return None; }
    let a = parts[0].trim().trim_start_matches("what is ").trim_start_matches("compare ").trim();
    let b = parts[1].trim().trim_end_matches('?').trim();
    if a.is_empty() || b.is_empty() { return None; }

    // Look up both in KB and compose a comparison
    let ka = knowledge_base_lookup(a);
    let kb = knowledge_base_lookup(b);
    match (ka, kb) {
        (Some(da), Some(db)) => {
            Some(format!("## {} vs {}\n\n### {}\n{}\n\n### {}\n{}\n\n---\n*Both have strengths — the best choice depends on your specific needs and context.*",
                capitalize_first(a), capitalize_first(b),
                capitalize_first(a), da,
                capitalize_first(b), db))
        }
        (Some(da), None) => {
            Some(format!("### {}\n{}\n\n*(I have detailed knowledge of {} but not {} — try asking about {} separately for more info.)*",
                capitalize_first(a), da, a, b, b))
        }
        (None, Some(db)) => {
            Some(format!("### {}\n{}\n\n*(I have detailed knowledge of {} but not {} — try asking about {} separately for more info.)*",
                capitalize_first(b), db, b, a, a))
        }
        (None, None) => None,
    }
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Extract "from X to Y" location names from a question string.
fn extract_from_to_locations(q: &str) -> Option<(String, String)> {
    let lower = q.to_lowercase();
    let from_pos = lower.find(" from ")?;
    let to_pos   = lower.find(" to ")?;
    if to_pos <= from_pos { return None; }

    let from_start = from_pos + 6; // skip " from "
    let from_end   = to_pos;
    let to_start   = to_pos + 4;   // skip " to "
    // End of "to" part: stop at " at ", " in ", end of string, or "?"
    let to_end = lower[to_start..].find(|c| " at in ?".contains(c) && c == ' ')
        .map(|p| to_start + p)
        .unwrap_or(q.len());

    let from_loc = q[from_start..from_end].trim().to_string();
    let to_loc   = q[to_start..to_end.min(q.len())].trim()
        .trim_end_matches('?').trim().to_string();

    if from_loc.is_empty() || to_loc.is_empty() { return None; }
    Some((from_loc, to_loc))
}

/// Find the speed value (the number closest to "km/h" or "mph") in the question.
fn find_speed_value(q: &str, nums: &[f64]) -> Option<f64> {
    let lower = q.to_lowercase();

    // Find position of "km/h" or "mph" in the string
    let speed_pos = lower.find("km/h")
        .or_else(|| lower.find("kmh"))
        .or_else(|| lower.find("mph"))
        .or_else(|| lower.find("km per hour"))
        .or_else(|| lower.find("miles per hour"))?;

    // Find the number that appears just before the speed unit
    let before = &lower[..speed_pos];
    let buf_nums = extract_numbers(before);
    buf_nums.last().copied().or_else(|| nums.last().copied())
}

// ===============================================================================
// GHOST-108 PARALLEL AGENT
// ===============================================================================
//
// Fires ALL search agents simultaneously in parallel threads.
// The fastest agent that returns a quality result wins — others are abandoned.
//
// Speed comparison:
//   Sequential (old):  DDG(400ms) + Wiki(400ms) + Web(400ms) = up to 1200ms
//   Ghost-108:         all 3 fire at once → winner in ~200ms  (6× faster)
//
// Agents fired in parallel:
//   Agent 1 — DuckDuckGo instant answers  (best for facts/calculators)
//   Agent 2 — Wikipedia REST summary      (best for encyclopedic topics)
//   Agent 3 — DuckDuckGo web search       (best for people/companies/news)
//   Agent 4 — Native math engine          (best for arithmetic/units/speed)
//
// Killer usage:
//   result = ghost_108("Who is Sai Arun Kumar Katherashala?")
//   print(result)
// ===============================================================================

/// Result from one parallel agent — includes which agent found it.
#[derive(Debug)]
struct AgentResult {
    agent:  &'static str,
    result: String,
}

/// Ghost-108: fires all search agents in parallel, returns the fastest winner.
///
/// For math/unit questions: returns instantly (no network at all).
/// For web questions: all HTTP agents race simultaneously.
pub fn ghost_108(question: &str) -> String {
    let q = question.trim().to_string();

    // -- Fast path: math/unit/speed — no network needed -----------------------
    {
        let math_candidate = normalize_math_question(&q);
        if is_math_expr(&math_candidate) {
            if let Ok(result) = eval_simple_math(&math_candidate) {
                let ans = if result == result.floor() && result.abs() < 1e15 {
                    format!("{}", result as i64)
                } else { format!("{}", result) };
                return format!(
                    "+-- Ghost-108 -----------------------------------------\n\
                     |  Agent: Native Math Engine  ⚡ instant\n\
                     |  {} = {}\n\
                     +-----------------------------------------------------\n\n{}",
                    math_candidate.trim(), ans, ans
                );
            }
        }
        if let Some((pct, base)) = detect_percentage(&q) {
            let result = (pct / 100.0) * base;
            let ans = if result == result.floor() { format!("{}", result as i64) } else { format!("{:.2}", result) };
            return format!(
                "+-- Ghost-108 -----------------------------------------\n\
                 |  Agent: Native Math Engine  ⚡ instant\n\
                 |  {}% of {} = {}\n\
                 +-----------------------------------------------------\n\n{}",
                pct, base, ans, ans
            );
        }
        if let Some((val, from, to)) = detect_unit_conversion(&q) {
            if let Some((result, desc)) = do_unit_conversion(val, &from, &to) {
                let ans = if result == result.floor() && result.abs() < 1e12 {
                    format!("{} {} = {} {}", val, from, result as i64, to)
                } else { format!("{} {} = {:.4} {}", val, from, result, to) };
                return format!(
                    "+-- Ghost-108 -----------------------------------------\n\
                     |  Agent: Native Convert Engine  ⚡ instant\n\
                     |  {}\n\
                     +-----------------------------------------------------\n\n{}",
                    desc, ans
                );
            }
        }
        if let Some((a, b, unknown)) = detect_speed_problem(&q) {
            let (answer, detail) = match unknown {
                "time" => {
                    let hours = a / b;
                    let h = hours.floor() as u64;
                    let m = ((hours - hours.floor()) * 60.0).round() as u64;
                    let ans = if m == 0 { format!("{} hours", h) } else { format!("{} hours {} minutes", h, m) };
                    (ans.clone(), format!("{} ÷ {} = {:.4}h → {}", a, b, hours, ans))
                }
                "distance" => {
                    let d = a * b;
                    (format!("{:.1} km", d), format!("{} × {} = {:.1} km", a, b, d))
                }
                _ => (String::new(), String::new()),
            };
            if !answer.is_empty() {
                return format!(
                    "+-- Ghost-108 -----------------------------------------\n\
                     |  Agent: Native Speed Engine  ⚡ instant\n\
                     |  {}\n\
                     +-----------------------------------------------------\n\n{}",
                    detail, answer
                );
            }
        }
    }

    // -- Parallel web agents --------------------------------------------------
    // Channel: any agent sends (agent_name, result) as soon as it gets a hit.
    let (tx, rx) = mpsc::channel::<AgentResult>();

    // Build stripped query for Wikipedia/DDG lookups
    let wiki_q = q.to_lowercase()
        .replace("what is the ", "").replace("what is ", "")
        .replace("what are ", "").replace("who is ", "")
        .replace("who was ", "").replace("where is ", "")
        .replace("tell me about ", "").replace("explain ", "")
        .replace('?', "").trim().to_string();

    // Agent 1a — Tofler-MCA DIRECT (richest India MCA data: date, industry, status, designation)
    {
        let tx = tx.clone();
        let name = q.to_lowercase()
            .replace("who is ", "").replace('?', "")
            .trim().to_string();
        std::thread::spawn(move || {
            if let Ok(r) = search_tofler_direct(&name) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "Tofler-MCA", result: r });
                }
            }
        });
    }

    // Agent 1b — Zaubacorp DIRECT (India MCA director/company registry, no DDG required)
    {
        let tx = tx.clone();
        let name = q.to_lowercase()
            .replace("who is ", "").replace('?', "")
            .trim().to_string();
        std::thread::spawn(move || {
            if let Ok(r) = search_zaubacorp_direct(&name) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "Zaubacorp-Direct", result: r });
                }
            }
        });
    }

    // Agent 2 — DDG instant answers
    {
        let tx = tx.clone();
        let q2 = q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_ddg(&q2) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "DDG Instant", result: r });
                }
            }
        });
    }

    // Agent 2 — Wikipedia REST summary
    {
        let tx = tx.clone();
        let wq = wiki_q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_wikipedia(&wq) {
                if r.len() > 40 {
                    let _ = tx.send(AgentResult { agent: "Wikipedia", result: r });
                }
            }
        });
    }

    // Agent 3 — DDG web search (LinkedIn, news, blogs, companies)
    {
        let tx = tx.clone();
        let q3 = q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_ddg_web(&q3) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "DDG Web", result: r });
                }
            }
        });
    }

    // Agent 4 — Bing web search (broader coverage, finds people DDG misses)
    {
        let tx = tx.clone();
        let q4 = q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_bing_web(&q4) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "Bing Web", result: r });
                }
            }
        });
    }

    // Agent 5 — companyseekers.com (India MCA director registry)
    {
        let tx = tx.clone();
        let q5 = q.clone();
        std::thread::spawn(move || {
            let name = q5.to_lowercase()
                .replace("who is ", "").replace('?', "")
                .trim().to_string();
            if let Ok(r) = search_companyseekers(&name) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "CompanySeekers", result: r });
                }
            }
        });
    }

    // Agent 6 — opencorporates.com (global public company/officer database)
    {
        let tx = tx.clone();
        let q6 = q.clone();
        std::thread::spawn(move || {
            let name = q6.to_lowercase()
                .replace("who is ", "").replace('?', "")
                .trim().to_string();
            if let Ok(r) = search_opencorporates(&name) {
                if !r.is_empty() {
                    let _ = tx.send(AgentResult { agent: "OpenCorporates", result: r });
                }
            }
        });
    }

    // Agent 7 — IndiaFilings DIN registry
    {
        let tx = tx.clone(); let q7 = q.clone();
        std::thread::spawn(move || {
            let name = q7.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_indiafilings(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "IndiaFilings", result: r }); }
            }
        });
    }

    // Agent 8 — Exact phrase search "Full Name"
    {
        let tx = tx.clone(); let q8 = q.clone();
        std::thread::spawn(move || {
            let name = q8.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_exact_phrase(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "ExactPhrase", result: r }); }
            }
        });
    }

    // Agent 9 — UK Companies House
    {
        let tx = tx.clone(); let q9 = q.clone();
        std::thread::spawn(move || {
            let name = q9.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_companies_house(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "UK-CompHouse", result: r }); }
            }
        });
    }

    // Agent 10 — US SEC EDGAR
    {
        let tx = tx.clone(); let qa = q.clone();
        std::thread::spawn(move || {
            let name = qa.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_sec_edgar(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "SEC-EDGAR", result: r }); }
            }
        });
    }

    // Agent 11 — Crunchbase (global business leaders)
    {
        let tx = tx.clone(); let qb = q.clone();
        std::thread::spawn(move || {
            let name = qb.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_crunchbase(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "Crunchbase", result: r }); }
            }
        });
    }

    // Agent 12 — India MCA official portal (mca.gov.in)
    {
        let tx = tx.clone(); let qc = q.clone();
        std::thread::spawn(move || {
            let name = qc.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_mca_gov(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "MCA-Gov", result: r }); }
            }
        });
    }

    // Agent 13 — Global registries (AU, SG, EU, Reuters, Bloomberg)
    {
        let tx = tx.clone(); let qd = q.clone();
        std::thread::spawn(move || {
            let name = qd.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_global_person(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "GlobalReg", result: r }); }
            }
        });
    }

    // Agent 14 — Google HTML search (broadest: covers employees, students, freelancers, anyone on web)
    {
        let tx = tx.clone(); let qe = q.clone();
        std::thread::spawn(move || {
            let name = qe.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_google_html(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "Google-Web", result: r }); }
            }
        });
    }

    // Agent 15 — GitHub API (developers, tech people — returns JSON, works everywhere)
    {
        let tx = tx.clone(); let qf = q.clone();
        std::thread::spawn(move || {
            let name = qf.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_github_api(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "GitHub", result: r }); }
            }
        });
    }

    // Agent 16 — Google News RSS (anyone mentioned in news: professionals, academics, etc.)
    {
        let tx = tx.clone(); let qg = q.clone();
        std::thread::spawn(move || {
            let name = qg.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_google_news_rss(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "Google-News", result: r }); }
            }
        });
    }

    // Agent 17 — Economic Times (Indian business news, covers professionals + entrepreneurs)
    {
        let tx = tx.clone(); let qh = q.clone();
        std::thread::spawn(move || {
            let name = qh.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_economic_times(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "EconomicTimes", result: r }); }
            }
        });
    }

    // Agent 18 — LinkedIn public profile (guesses URL slug from name patterns)
    {
        let tx = tx.clone(); let qi = q.clone();
        std::thread::spawn(move || {
            let name = qi.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
            if let Ok(r) = search_linkedin_public(&name) {
                if !r.is_empty() { let _ = tx.send(AgentResult { agent: "LinkedIn", result: r }); }
            }
        });
    }

    // Drop the original tx so the channel closes when all threads finish
    drop(tx);

    // Wait for the first quality result — timeout after 10 seconds
    let timeout = std::time::Duration::from_secs(10);
    let start   = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(hit) => {
                let elapsed_ms = start.elapsed().as_millis();
                // Truncate to 2 sentences
                let mut answer = String::new();
                let mut cnt = 0usize;
                for ch in hit.result.chars() {
                    if ch == '\n' { if !answer.ends_with(' ') { answer.push(' '); } continue; }
                    answer.push(ch);
                    if ch == '.' { cnt += 1; if cnt >= 2 { break; } }
                }
                let answer = answer.trim().to_string();
                return format!(
                    "+-- Ghost-108 -----------------------------------------\n\
                     |  Agent: {}  ⚡ {}ms\n\
                     +-----------------------------------------------------\n\n{}",
                    hit.agent, elapsed_ms, answer
                );
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break, // all threads done
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
        }
    }

    "Ghost-108: No result found across all agents.".to_string()
}

// ===============================================================================
// KhLM — KILLER HYBRID LANGUAGE MODEL  (unified smart router)
// ===============================================================================
//
//  KhLM automatically routes every question to the BEST engine:
//
//    Tier 1 — Deterministic  (0ms, 100% accurate, no network, no model)
//      • Arithmetic, algebra
//      • Percentage calculations
//      • Unit conversions  (km↔miles, °C↔°F, kg↔lbs, …)
//      • Speed / Distance / Time problems
//
//    Tier 1.5 — RLM Short-circuit  (skips web entirely for reasoning questions)
//      • Detects "why / how / explain / prove / implement / write code / analyze"
//      • Routes directly to RLM (DeepSeek-R1 → killer_think, LLM → chain-of-thought)
//      • Faster + higher quality than web search for these question types
//      • Falls back to web if model not provided or RLM fails
//
//    Tier 2 — Live Web  (parallel Ghost-108 agents, ~200-400ms)
//      • Factual questions: people, companies, news, events, prices
//      • Real-time data (stock prices, sports scores, weather)
//      • Only fires for FACTUAL questions (who/when/where/CEO/DIN)
//
//    Tier 3 — KhLM Neural  (local .gguf, fires in parallel with Tier 2)
//      • Auto-detects model type:
//          DeepSeek-R1 / QwQ → killer_think() with true <think> scratchpad
//          Any LLM           → killer_llm_as_rlm() chain-of-thought
//      • Races Tier 2 — fastest quality result wins
//
//    Tier 3 Synthesis — RLM merges fragmented web results
//      • If web agents found low-confidence fragments (<80% score)
//      • Feeds all fragments to RLM as context
//      • RLM synthesizes one coherent, accurate answer
//
//  Killer usage:
//    answer = khlm_ask("What is 15% of 480?")          -- Tier 1 (instant)
//    answer = khlm_ask("Who is Alan Turing?")          -- Tier 2 (web)
//    answer = khlm_ask_model(model, "Explain quantum entanglement") -- Tier 1.5 (RLM)
//    answer = khlm_ask_model(model, "Who is Elon Musk?")            -- Tier 2 + 3
//
//  With model:
//    answer = khlm_ask_model(model, "Write a Killer function for BFS") -- Tier 1.5 RLM
// ===============================================================================

/// The KhLM router result — includes which tier/engine answered.
struct KyLmResult {
    answer:  String,
    engine:  &'static str,
    tier:    u8,
    ms:      u128,
}

/// What kind of question is being asked?
#[derive(PartialEq)]
enum QuestionKind {
    /// Needs live web data: people, companies, events, prices.
    Factual,
    /// Needs deep reasoning: math, code, analysis, proofs, explanations.
    Reasoning,
}

/// Classify a question so KhLM can route it to the right engine.
/// Reasoning questions skip the 18 web agents entirely → faster + smarter.
fn classify_question(q: &str) -> QuestionKind {
    let ql = q.to_lowercase();

    // Strong factual signals — these always need web search (person/event/price/breaking news)
    let factual_keywords = [
        "who is", "who was", "who are", "when was", "when did",
        "where is", "where was", "ceo of", "founder of", "president of",
        "din ", "cin ", "stock price", "founded in",
        "born in", "died in", "news", "latest", "today",
        "current price", "net worth", "age of", "released in",
        "launched in", "announced", "election", "government",
    ];
    // Factual wins if present (web is better for live people/events)
    for kw in &factual_keywords {
        if ql.contains(kw) { return QuestionKind::Factual; }
    }

    // Strong reasoning signals — skip web entirely, RLM/native knows better
    let reasoning_keywords = [
        // Explanation / analysis
        "why ", "how does", "how do", "how is", "how it work",
        "how are you", "how are we", "how are things", "how are they",
        "explain", "prove", "proof", "derive", "derivation",
        "step by step", "analyze", "analysis", "debug",
        // Implementation
        "solve", "implement", "write a", "write the", "write code",
        "function", "algorithm", "what is the best", "optimize",
        // Comparison
        "difference between", "compare", "vs ", "versus",
        "pros and cons", "advantages", "disadvantages", "tradeoff",
        // Maths / science concepts
        "complexity", "big o", "time complexity", "space complexity",
        "integral", "derivative", "equation", "formula for",
        "theorem", "axiom", "proof of", "calculate the",
        // Definitions / concepts — native knowledge, no web needed
        "define ", "definition of", "definition:", "what does",
        "meaning of", "what is meant by", "concept of",
        "theory of", "principle of", "law of", "rule of",
        "what are the", "what makes", "what causes", "what prevents",
        "relationship between", "connection between",
        // Science topics — timeless, no web needed
        "photosynthesis", "evolution", "quantum", "relativity",
        "entropy", "gravity", "gravitation", "magnetism",
        "electricity", "thermodynamics", "electromagnetism",
        "dna", " rna", "protein", "mitosis", "meiosis",
        "atom ", "molecule", "electron", "proton", "neutron",
        "nucleus", "orbital", "periodic table", "chemical bond",
        "osmosis", "diffusion", "respiration", "metabolism",
        // Programming concepts
        "recursion", "pointer", "memory leak", "stack overflow",
        "heap ", "binary tree", "linked list", "hash map", "hashmap",
        "sorting", "searching", "object oriented", "polymorphism",
        "inheritance", "encapsulation", "abstraction", "closure",
        "coroutine", "mutex", "semaphore", "deadlock", "race condition",
        "tcp", "udp", "http", "https", "dns", "rest api", "graphql",
        "docker", "kubernetes", "git ", "version control",
        // Math
        "factorial", "fibonacci", "prime ", "modulo", "logarithm",
        "trigonometry", "sine", "cosine", "tangent", "pythagorean",
        "matrix", "vector ", "eigenvalue", "determinant",
        // Philosophy / logic
        "what happens if", "what would happen", "what if ",
        "is it possible", "can a ", "can an ",
    ];

    for kw in &reasoning_keywords {
        if ql.contains(kw) { return QuestionKind::Reasoning; }
    }

    // "what is X" — if X looks like a concept (not a person/company), route to reasoning
    if ql.starts_with("what is ") || ql.starts_with("what's ") {
        let subject = ql.trim_start_matches("what is ").trim_start_matches("what's ").trim_end_matches('?').trim();
        // Single word or short phrase concepts → reasoning (Tier 1.5 knows these)
        let is_concept = subject.split_whitespace().count() <= 5
            && !subject.contains(" of ") // "president of X" → factual
            && !subject.chars().next().map(|c| c.is_uppercase()).unwrap_or(false); // "What is Elon" → factual
        if is_concept { return QuestionKind::Reasoning; }
    }

    QuestionKind::Factual // default: web search (people, companies, recent events)
}

/// Detect whether a GGUF model path/name should use the native RLM path (`killer_think_rlm`).
/// Matches DeepSeek-R1 / QwQ-style checkpoints plus common reasoning filenames (see body).
/// Those models respond best to `killer_think()` (which prefills `<think>`) rather than `killer_llm_as_rlm`.
pub fn is_rlm_model_path(model_path: &str) -> bool {
    let p = model_path.to_lowercase();
    p.contains("deepseek")
        || p.contains("-r1")
        || p.contains("_r1")
        || p.contains("qwq")
        || p.contains("skywork-or")
        || p.contains("r1-distill")
        || p.contains("reasoning")
        || p.contains("gpt-oss")
        || p.contains("o1-preview")
        || p.contains("o1-mini")
        || p.contains("o3-mini")
        || p.contains("qwen3-thinking")
        || p.contains("qwen3_think")
        || p.contains("magistral")
        || p.contains("nemotron-reasoning")
        || p.contains("exaone-deep")
        || p.contains("glm-4.6")
        || p.contains("hunyuan-t1")
}

/// KhLM unified ask — no model required.
/// Routes: Deterministic → Ghost-108 web parallel.
pub fn khlm_ask(question: &str) -> String {
    khlm_route(question, None)
}

/// KhLM unified ask — with optional local model for Tier 3 neural inference.
/// Fires web agents AND model reasoning in parallel — fastest quality result wins.
/// For true reasoning questions (code, math, analysis), skips web entirely and
/// goes straight to the RLM — faster and higher quality.
pub fn khlm_ask_model(model_path: &str, question: &str) -> String {
    khlm_route(question, Some(model_path))
}

/// **Killer AI System** — native multi-agent orchestration for one user task.
///
/// **When to use:** hard questions where you want **router + parallel search + local neural + merge** in one report.
///
/// **GGUF:** install or point at a **reasoning-capable** `.gguf` (e.g. DeepSeek-R1–style) via [`crate::inference::pick_default_gguf_for_khlm`]
/// (`KILLER_KHLM_GGUF` or first sorted file under `~/.killer/models/`) — it improves the neural slot and **coordinator synthesis**.
///
/// **Honest scope:** advanced **orchestration and merging**, not AGI.
///
/// Runs three specialists in parallel, then returns a single structured report:
/// 1. **KhLM-Hybrid-Router** — full `khlm_ask` (deterministic + web + cache semantics).
/// 2. **Ghost-108-Search-Swarm** — parallel search agents (`ghost_108`).
/// 3. **Neural-Advisor** — local GGUF when available.
/// 4. **Coordinator synthesis** — when a default GGUF is available, one merged answer from all three outputs.
///
/// The coordinator line records [`khlm_classify_question`] (`math` / `factual` / `reasoning`).
pub fn khlm_ai_system_multi_agent(question: &str) -> String {
    let q = question.trim();
    if q.is_empty() {
        return "Killer AI System: provide a non-empty task or question.".to_string();
    }
    let route = khlm_classify_question(q);
    let q_owned = q.to_string();

    let h_khlm = {
        let qq = q_owned.clone();
        std::thread::spawn(move || ("KhLM-Hybrid-Router", khlm_ask(&qq)))
    };
    let h_ghost = {
        let qq = q_owned.clone();
        std::thread::spawn(move || ("Ghost-108-Search-Swarm", ghost_108(&qq)))
    };
    let h_neural = {
        let qq = q_owned;
        std::thread::spawn(move || {
            let label: &'static str = "Neural-Advisor";
            if let Some(ref mp) = crate::inference::pick_default_gguf_for_khlm() {
                let prompt = format!(
                    "User task — answer concisely with no preamble or meta-commentary:\n\n{}",
                    qq
                );
                let syn = if is_rlm_model_path(mp) {
                    crate::inference::killer_think_rlm(mp, &prompt, 384)
                        .map(|r| r.answer_only().to_string())
                        .unwrap_or_default()
                } else {
                    crate::inference::killer_llm_as_rlm(mp, &prompt, 384)
                        .map(|r| r.answer_only().to_string())
                        .unwrap_or_default()
                };
                if syn.len() > 20 {
                    return (label, syn);
                }
            }
            (
                label,
                "(No local GGUF — set KILLER_KHLM_GGUF or add a .gguf under ~/.killer/models/. A reasoning model improves neural + coordinator merge.)"
                    .to_string(),
            )
        })
    };

    let (n1, r1) = h_khlm
        .join()
        .unwrap_or(("KhLM-Hybrid-Router", "(agent error)".to_string()));
    let (n2, r2) = h_ghost
        .join()
        .unwrap_or(("Ghost-108-Search-Swarm", "(agent error)".to_string()));
    let (n3, r3) = h_neural
        .join()
        .unwrap_or(("Neural-Advisor", "(agent error)".to_string()));

    let mut out = format!(
        "+-- Killer AI System — native multi-agent orchestration ----------+\n\
         |  Coordinator route: **{}**\n\
         |  Use for **hard** questions: router + search + neural + merge.\n\
         |  **GGUF:** reasoning models (e.g. R1-style) help neural + synthesis — `KILLER_KHLM_GGUF` or ~/.killer/models/\n\
         |  **Scope:** orchestration + merging — not AGI.\n\
         +------------------------------------------------------------------+\n\n\
         ### Agent: {}\n\n{}\n\n\
         ---\n\n\
         ### Agent: {}\n\n{}\n\n\
         ---\n\n\
         ### Agent: {}\n\n{}\n",
        route, n1, r1, n2, r2, n3, r3
    );

    if let Some(ref mp) = crate::inference::pick_default_gguf_for_khlm() {
        let cap = 2800usize;
        let trunc = |s: &str| -> String {
            if s.len() <= cap {
                s.to_string()
            } else {
                format!("{}…", s.chars().take(cap).collect::<String>())
            }
        };
        let bundle = format!(
            "ORIGINAL TASK:\n{q}\n\n\
             --- {n1} ---\n{t1}\n\n\
             --- {n2} ---\n{t2}\n\n\
             --- {n3} ---\n{t3}",
            q = q,
            n1 = n1,
            t1 = trunc(&r1),
            n2 = n2,
            t2 = trunc(&r2),
            n3 = n3,
            t3 = trunc(&r3),
        );
        let synth_prompt = format!(
            "You are the coordinator. Three agents answered the same task (material below).\n\
Produce ONE unified answer for the user: merge overlapping facts, resolve clear contradictions by \
preferring verifiable facts for people/orgs/dates and reasoning for math/code. Be concise. No meta-commentary.\n\n\
{bundle}"
        );
        let merged = if is_rlm_model_path(mp) {
            crate::inference::killer_think_rlm(mp, &synth_prompt, 1024)
                .map(|r| r.answer_only().to_string())
                .unwrap_or_default()
        } else {
            crate::inference::killer_llm_as_rlm(mp, &synth_prompt, 768)
                .map(|r| r.answer_only().to_string())
                .unwrap_or_default()
        };
        if merged.len() > 48 {
            out.push_str("\n---\n\n### Coordinator synthesis (merged verdict)\n\n");
            out.push_str(&merged);
            out.push('\n');
        }
    }

    out
}

fn khlm_route(question: &str, model_path: Option<&str>) -> String {
    let q = question.trim().to_string();
    let start = std::time::Instant::now();

    // -- CACHE HIT: prefetch already ran in background → nanosecond return ----
    if let Ok(cache) = khlm_cache().lock() {
        if let Some(cached) = cache.get(&q) {
            return cached.clone();
        }
    }

    // -- TIER 1: Deterministic engines (instant, zero network) ----------------

    // Arithmetic
    let math_candidate = normalize_math_question(&q);
    if is_math_expr(&math_candidate) {
        if let Ok(result) = eval_simple_math(&math_candidate) {
            let ans = if result == result.floor() && result.abs() < 1e15 {
                format!("{}", result as i64)
            } else { format!("{}", result) };
            return khlm_format(KyLmResult {
                answer: ans.clone(), engine: "KhLM/Math", tier: 1,
                ms: start.elapsed().as_millis(),
            }, &ans);
        }
    }

    // Percentage
    if let Some((pct, base)) = detect_percentage(&q) {
        let result = (pct / 100.0) * base;
        let ans = if result == result.floor() { format!("{}", result as i64) } else { format!("{:.2}", result) };
        let detail = format!("{}% of {} = {}", pct, base, ans);
        return khlm_format(KyLmResult {
            answer: detail.clone(), engine: "KhLM/Math", tier: 1,
            ms: start.elapsed().as_millis(),
        }, &ans);
    }

    // Unit conversion
    if let Some((val, from, to)) = detect_unit_conversion(&q) {
        if let Some((_result, desc)) = do_unit_conversion(val, &from, &to) {
            return khlm_format(KyLmResult {
                answer: desc.clone(), engine: "KhLM/Convert", tier: 1,
                ms: start.elapsed().as_millis(),
            }, &desc);
        }
    }

    // Speed / Distance / Time
    if let Some((a, b, unknown)) = detect_speed_problem(&q) {
        let answer = match unknown {
            "time" => {
                let hours = a / b;
                let h = hours.floor() as u64;
                let m = ((hours - hours.floor()) * 60.0).round() as u64;
                if m == 0 { format!("{} hours", h) } else { format!("{} hours {} minutes", h, m) }
            }
            "distance" => format!("{:.1} km", a * b),
            _ => String::new(),
        };
        if !answer.is_empty() {
            return khlm_format(KyLmResult {
                answer: answer.clone(), engine: "KhLM/Physics", tier: 1,
                ms: start.elapsed().as_millis(),
            }, &answer);
        }
    }

    // FIX 1 — Power / Exponent: "2 to the power of 32", "2^32", "3**10"
    {
        let pq = q.replace("to the power of", "^").replace("**", "^");
        if let Some(ci) = pq.find('^') {
            // extract last number before ^ and first number after ^
            fn last_num(s: &str) -> Option<f64> {
                s.split(|c: char| !c.is_ascii_digit() && c != '.')
                    .filter(|w| !w.is_empty() && w.contains(|d: char| d.is_ascii_digit()))
                    .last()
                    .and_then(|w| w.parse().ok())
            }
            fn first_num(s: &str) -> Option<f64> {
                s.split(|c: char| !c.is_ascii_digit() && c != '.')
                    .find(|w| !w.is_empty() && w.contains(|d: char| d.is_ascii_digit()))
                    .and_then(|w| w.parse().ok())
            }
            if let (Some(base), Some(exp)) = (last_num(&pq[..ci]), first_num(&pq[ci+1..])) {
                if base > 0.0 {
                    let result = base.powf(exp);
                    let ans = if result == result.floor() && result.abs() < 1e18 {
                        format!("{}", result as u64)
                    } else { format!("{:.4}", result) };
                    let detail = format!("{} ^ {} = {}", base as u64, exp as u64, ans);
                    return khlm_format(KyLmResult {
                        answer: detail.clone(), engine: "KhLM/Math", tier: 1,
                        ms: start.elapsed().as_millis(),
                    }, &ans);
                }
            }
        }
    }

    // FIX 2 — Time unit arithmetic: "seconds in N days", "minutes in N hours", etc.
    {
        let tq = q.to_lowercase();
        fn unit_secs(u: &str) -> f64 {
            let u = u.trim_end_matches('s'); // strip plural
            match u { "second" => 1.0, "minute" => 60.0, "hour" => 3600.0,
                      "day" => 86_400.0, "week" => 604_800.0, _ => 0.0 }
        }
        const TIME_UNITS: &[&str] = &["seconds","minutes","hours","days","weeks",
                                       "second","minute","hour","day","week"];
        // Pattern: "<want_unit> in <N> <src_unit>"
        let words: Vec<&str> = tq.split_whitespace().collect();
        #[allow(unused_labels)]
        'outer: for i in 0..words.len() {
            let want_sec = unit_secs(words[i]);
            if want_sec == 0.0 { continue; }
            // look ahead for "in"
            for j in i+1..words.len().min(i+4) {
                if words[j] != "in" { continue; }
                if j+2 >= words.len() { break; }
                let n: f64 = match words[j+1].replace(',', "").parse() { Ok(v) => v, _ => continue };
                let src_sec = unit_secs(words[j+2]);
                if src_sec == 0.0 || n <= 0.0 { continue; }
                let result = (n * src_sec) / want_sec;
                let ans = if result == result.floor() { format!("{}", result as u64) }
                          else { format!("{:.2}", result) };
                let want_str = words[i].trim_end_matches('s');
                let src_str  = words[j+2].trim_end_matches('s');
                let detail = format!("{} {}s in {} {}s = {}", want_str, want_str, n as u64, src_str, ans);
                return khlm_format(KyLmResult {
                    answer: detail.clone(), engine: "KhLM/Math", tier: 1,
                    ms: start.elapsed().as_millis(),
                }, &ans);
            }
        }
        let _ = TIME_UNITS; // suppress unused warning

        // "X km divided by Y km/h" or "X km / Y km/h" → travel time
        let norm = tq.replace("divided by", "/");
        if (norm.contains("km/h") || norm.contains("km per hour") || norm.contains("kmh"))
            && norm.contains("km")
        {
            fn extract_num_before(s: &str, marker: &str) -> Option<f64> {
                s.find(marker).and_then(|pos| {
                    s[..pos].split_whitespace().rev()
                        .find(|w| w.chars().all(|c| c.is_ascii_digit() || c == '.'))
                        .and_then(|w| w.parse().ok())
                })
            }
            let norm2 = norm.replace("km per hour", "km/h").replace("kmh", "km/h");
            // find distance (number before first "km" that isn't followed by /h)
            // and speed (number before "km/h")
            if let Some(speed) = extract_num_before(&norm2, "km/h") {
                // find the km distance — a number before "km" that doesn't precede /h
                let dist_opt: Option<f64> = norm2.split_whitespace().zip(norm2.split_whitespace().skip(1))
                    .filter(|(_, next)| next.starts_with("km") && !next.starts_with("km/"))
                    .find_map(|(w, _)| w.parse().ok());
                if let Some(dist) = dist_opt {
                    if dist > 0.0 && speed > 0.0 {
                        let hours = dist / speed;
                        let h = hours.floor() as u64;
                        let m = ((hours - hours.floor()) * 60.0).round() as u64;
                        let ans = if m == 0 { format!("{} hours", h) }
                                  else { format!("{} hours {} minutes", h, m) };
                        let detail = format!("{} km ÷ {} km/h = {}", dist as u64, speed as u64, ans);
                        return khlm_format(KyLmResult {
                            answer: detail.clone(), engine: "KhLM/Physics", tier: 1,
                            ms: start.elapsed().as_millis(),
                        }, &ans);
                    }
                }
            }
        }
    }

    // -- TIER 1: Factorial / Square Root / Abs ------------------------------------
    {
        let lq = q.to_lowercase();
        let lq = lq.trim_end_matches('?').trim();

        // Factorial: "10!", "factorial of 10", "factorial(10)"
        let fact_n: Option<u64> = if lq.ends_with('!') {
            lq.trim_end_matches('!').trim().parse().ok()
        } else if lq.starts_with("factorial of ") {
            lq["factorial of ".len()..].split_whitespace().next().and_then(|w| w.parse().ok())
        } else if lq.starts_with("factorial(") && lq.ends_with(')') {
            lq["factorial(".len()..lq.len()-1].trim().parse().ok()
        } else { None };

        if let Some(n) = fact_n {
            if n <= 20 {
                let result: u64 = (1..=n).product();
                let ans = result.to_string();
                let detail = format!("{}! = {}", n, ans);
                return khlm_format(KyLmResult {
                    answer: detail.clone(), engine: "KhLM/Math", tier: 1,
                    ms: start.elapsed().as_millis(),
                }, &ans);
            }
        }

        // Square root: "sqrt(144)", "square root of 144", "√144"
        let sqrt_n: Option<f64> = if lq.starts_with("sqrt(") && lq.ends_with(')') {
            lq["sqrt(".len()..lq.len()-1].trim().parse().ok()
        } else if lq.starts_with("square root of ") {
            lq["square root of ".len()..].split_whitespace().next().and_then(|w| w.parse().ok())
        } else if lq.starts_with('√') {
            lq[1..].trim().split_whitespace().next().and_then(|w| w.parse().ok())
        } else if lq.starts_with("√") {
            lq.trim_start_matches('√').trim().split_whitespace().next().and_then(|w| w.parse().ok())
        } else { None };

        if let Some(n) = sqrt_n {
            if n >= 0.0 {
                let result = n.sqrt();
                let ans = if result == result.floor() {
                    format!("{}", result as u64)
                } else { format!("{:.6}", result) };
                let detail = format!("√{} = {}", n as u64, ans);
                return khlm_format(KyLmResult {
                    answer: detail.clone(), engine: "KhLM/Math", tier: 1,
                    ms: start.elapsed().as_millis(),
                }, &ans);
            }
        }
    }

    // -- TIER 1: Science constants, math constants, geography ------------------------
    {
        let lq = q.to_lowercase();
        let lq = lq.replace("what is", "").replace("what's", "")
                   .replace("the value of", "").replace("tell me", "")
                   .replace('?', "").trim().to_string();

        // Science & math constants
        let constants: &[(&str, &str, &str)] = &[
            // (keyword_match, answer, detail)
            ("speed of light",        "299,792,458 m/s",  "Speed of light (c) = 299,792,458 m/s in vacuum"),
            ("gravitational constant", "6.674×10⁻¹¹ N·m²/kg²", "Newton's gravitational constant G = 6.674×10⁻¹¹ N·m²/kg²"),
            ("planck constant",        "6.626×10⁻³⁴ J·s",  "Planck constant h = 6.626×10⁻³⁴ J·s"),
            ("boltzmann constant",     "1.381×10⁻²³ J/K",  "Boltzmann constant k = 1.381×10⁻²³ J/K"),
            ("avogadro",               "6.022×10²³ mol⁻¹", "Avogadro's number = 6.022×10²³ mol⁻¹"),
            ("electron charge",        "1.602×10⁻¹⁹ C",    "Elementary charge e = 1.602×10⁻¹⁹ C"),
            ("electron mass",          "9.109×10⁻³¹ kg",   "Electron mass = 9.109×10⁻³¹ kg"),
            ("proton mass",            "1.673×10⁻²⁷ kg",   "Proton mass = 1.673×10⁻²⁷ kg"),
            ("boiling point of water", "100°C (212°F) at sea level", "Water boils at 100°C / 212°F at 1 atm"),
            ("freezing point of water","0°C (32°F)",        "Water freezes at 0°C / 32°F at 1 atm"),
            ("absolute zero",          "-273.15°C (-459.67°F)", "Absolute zero = -273.15°C / 0 Kelvin"),
            ("value of pi",            "3.14159265358979", "π = 3.14159265358979..."),
            ("value of e",             "2.71828182845905", "Euler's number e = 2.71828182845905..."),
            ("golden ratio",           "1.61803398874989", "Golden ratio φ = (1+√5)/2 = 1.61803398874989..."),
            ("speed of sound",         "343 m/s in air at 20°C", "Speed of sound = 343 m/s in dry air at 20°C"),
            ("gravity on earth",       "9.81 m/s²",         "Standard gravitational acceleration g = 9.81 m/s²"),
            ("gravity on moon",        "1.62 m/s²",         "Lunar gravity = 1.62 m/s² (≈1/6 of Earth)"),
            ("earth radius",           "6,371 km",          "Mean radius of Earth = 6,371 km"),
            ("earth mass",             "5.972×10²⁴ kg",     "Mass of Earth = 5.972×10²⁴ kg"),
            ("distance sun to earth",  "149.6 million km (1 AU)", "Earth–Sun distance = 149.6 million km = 1 AU"),
            ("distance moon to earth", "384,400 km",        "Earth–Moon distance = 384,400 km (average)"),
            ("how many planets",       "8 planets",         "Solar system has 8 planets: Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune"),
            ("number of planets",      "8 planets",         "Solar system has 8 planets: Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune"),
        ];

        for (kw, ans, detail) in constants {
            if lq.contains(kw) {
                return khlm_format(KyLmResult {
                    answer: detail.to_string(), engine: "KhLM/Science", tier: 1,
                    ms: start.elapsed().as_millis(),
                }, ans);
            }
        }

        // Capital cities — static, never changes
        let capitals: &[(&str, &str)] = &[
            ("capital of india",          "New Delhi"),
            ("capital of usa",            "Washington, D.C."),
            ("capital of united states",  "Washington, D.C."),
            ("capital of uk",             "London"),
            ("capital of united kingdom", "London"),
            ("capital of china",          "Beijing"),
            ("capital of russia",         "Moscow"),
            ("capital of france",         "Paris"),
            ("capital of germany",        "Berlin"),
            ("capital of japan",          "Tokyo"),
            ("capital of australia",      "Canberra"),
            ("capital of canada",         "Ottawa"),
            ("capital of brazil",         "Brasília"),
            ("capital of pakistan",       "Islamabad"),
            ("capital of italy",          "Rome"),
            ("capital of spain",          "Madrid"),
            ("capital of mexico",         "Mexico City"),
            ("capital of south africa",   "Pretoria (executive), Cape Town (legislative), Bloemfontein (judicial)"),
            ("capital of egypt",          "Cairo"),
            ("capital of saudi arabia",   "Riyadh"),
            ("capital of south korea",    "Seoul"),
            ("capital of north korea",    "Pyongyang"),
            ("capital of nigeria",        "Abuja"),
            ("capital of argentina",      "Buenos Aires"),
            ("capital of indonesia",      "Jakarta"),
            ("capital of turkey",         "Ankara"),
            ("capital of israel",         "Jerusalem"),
            ("capital of ukraine",        "Kyiv"),
            ("capital of poland",         "Warsaw"),
            ("capital of sweden",         "Stockholm"),
            ("capital of norway",         "Oslo"),
            ("capital of denmark",        "Copenhagen"),
            ("capital of netherlands",    "Amsterdam"),
            ("capital of belgium",        "Brussels"),
            ("capital of switzerland",    "Bern"),
            ("capital of portugal",       "Lisbon"),
            ("capital of greece",         "Athens"),
            ("capital of iran",           "Tehran"),
        ];

        for (kw, cap) in capitals {
            if lq.contains(kw) || lq.contains(&kw.replace("capital of ", "")) {
                let detail = format!("Capital of {} = {}", &kw["capital of ".len()..], cap);
                return khlm_format(KyLmResult {
                    answer: detail.clone(), engine: "KhLM/Geography", tier: 1,
                    ms: start.elapsed().as_millis(),
                }, cap);
            }
        }
    }

    // -- TIER 1.5: Pure Reasoning short-circuit ---------------------------------
    // Reasoning questions (code, math, analysis) skip 18 web agents entirely.
    // The RLM knows more than any web scrape for these — and finishes 2x faster
    // without waiting for all agent timeouts.
    let question_kind = classify_question(&q);
    let model_for_reasoning: Option<String> = model_path
        .map(|s| s.to_string())
        .or_else(|| {
            if question_kind == QuestionKind::Reasoning {
                crate::inference::pick_default_gguf_for_khlm()
            } else {
                None
            }
        });

    if question_kind == QuestionKind::Reasoning {
        if let Some(ref mp) = model_for_reasoning {
            let use_rlm = is_rlm_model_path(mp);
            let result = if use_rlm {
                crate::inference::killer_think_rlm(mp, &q, 1200)
                    .map(|r| (r.answer_only().to_string(), "KhLM/RLM-Reasoning"))
                    .ok()
            } else {
                crate::inference::killer_llm_as_rlm(mp, &q, 800)
                    .map(|r| (r.answer_only().to_string(), "KhLM/Neural-Reasoning"))
                    .ok()
            };
            if let Some((ans, eng_name)) = result {
                if ans.len() > 20 {
                    let ms = start.elapsed().as_millis();
                    let formatted = format!(
                        "+-- KhLM ---------------------------------------------\n\
                         |  {}  |  Tier 3 RLM  |  {}ms\n\
                         +-----------------------------------------------------\n\n{}",
                        eng_name, ms, ans
                    );
                    if let Ok(mut cache) = khlm_cache().lock() {
                        cache.insert(q, formatted.clone());
                    }
                    return formatted;
                }
            }
            // RLM failed / too short — fall through to web search
        }
    }

    // -- TIER 2 + TIER 3: Parallel race ----------------------------------------
    let (tx, rx) = mpsc::channel::<KyLmResult>();

    // -- WAVE 1 (fires immediately): precision registry searches ------------
    // Only 3 DDG calls at once — avoids rate limiting.
    // These are the most likely to contain the exact name.

    // Is this a person-name query?
    let looks_like_name = q.to_lowercase().contains("who is ")
        || q.split_whitespace()
            .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
            .count() >= 2;

    let base_name = q.to_lowercase()
        .replace("who is ", "").replace('?', "")
        .trim().to_string();

    // Wave 1a: Tofler-MCA DIRECT (Zaubacorp DIN + Tofler profile — richest MCA data)
    // Returns: name, DIN, company, CIN, founding date, industry, status, designation
    {
        let tx = tx.clone(); let n = base_name.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_tofler_direct(&n) {
                if !r.is_empty() {
                    let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Tofler-MCA", tier: 2, ms: 0 });
                }
            }
        });
    }

    // Wave 1b: Zaubacorp DIRECT (independent run for company + CIN data as corroboration)
    {
        let tx = tx.clone(); let n = base_name.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_zaubacorp_direct(&n) {
                if !r.is_empty() {
                    let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Zaubacorp", tier: 2, ms: 0 });
                }
            }
        });
    }

    // Wave 1c: IndiaFilings (DIN registry via DDG site: search)
    {
        let tx = tx.clone(); let n = base_name.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_indiafilings(&n) {
                if !r.is_empty() {
                    let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/IndiaFilings", tier: 2, ms: 0 });
                }
            }
        });
    }

    // Wave 1d: Exact-phrase DDG (precise name match)
    {
        let tx = tx.clone(); let n = base_name.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_exact_phrase(&n) {
                if !r.is_empty() {
                    let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/ExactPhrase", tier: 2, ms: 0 });
                }
            }
        });
    }

    // -- WAVE 2 (fires after 350ms delay): broader web searches --------------
    // By the time Wave 2 fires, Wave 1 has already used its DDG quota.
    {
        let tx2 = tx.clone();
        let q_wave2 = q.clone();
        let bn2 = base_name.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(350));

            // DDG Instant
            {
                let tx = tx2.clone(); let q2 = q_wave2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_ddg(&q2) {
                        if !r.is_empty() {
                            let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Web-DDG", tier: 2, ms: 0 });
                        }
                    }
                });
            }

            // Yahoo web search (works through corporate proxy, broader coverage)
            {
                let tx = tx2.clone(); let q_yh = q_wave2.clone();
                std::thread::spawn(move || {
                    let name = q_yh.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
                    if let Ok(r) = search_yahoo_web(&name) {
                        if !r.is_empty() {
                            let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Yahoo-Web", tier: 2, ms: 0 });
                        }
                    }
                });
            }

            // LinkedIn via Yahoo targeted search (LinkedIn requires auth for API but Yahoo indexes public profiles)
            {
                let tx = tx2.clone(); let q_li = q_wave2.clone();
                std::thread::spawn(move || {
                    let name = q_li.to_lowercase().replace("who is ", "").replace('?', "").trim().to_string();
                    let li_query = format!("linkedin {} india", name);
                    if let Ok(r) = search_yahoo_web(&li_query) {
                        if !r.is_empty() && (r.to_lowercase().contains("linkedin") || r.to_lowercase().contains(&name.to_lowercase())) {
                            let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/LinkedIn-Web", tier: 2, ms: 0 });
                        }
                    }
                });
            }

            // Wikipedia
            {
                let tx = tx2.clone(); let q3 = q_wave2.clone();
                std::thread::spawn(move || {
                    let wq = q3.to_lowercase()
                        .replace("what is the ", "").replace("what is ", "")
                        .replace("who is ", "").replace("who was ", "")
                        .replace('?', "").trim().to_string();
                    if let Ok(r) = search_wikipedia(&wq) {
                        if r.len() > 40 {
                            let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Web-Wiki", tier: 2, ms: 0 });
                        }
                    }
                });
            }

            // DDG Web scraper
            {
                let tx = tx2.clone(); let q4 = q_wave2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_ddg_web(&q4) {
                        if !r.is_empty() {
                            let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Web-Search", tier: 2, ms: 0 });
                        }
                    }
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(350));

            // Wave 3 (fires after 700ms total): global + fuzzy
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_opencorporates(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/OpenCorp", tier: 2, ms: 0 }); }
                    }
                });
            }
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_mca_gov(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/MCA-Gov", tier: 2, ms: 0 }); }
                    }
                });
            }
            {
                let tx = tx2.clone(); let q5 = q_wave2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_bing_web(&q5) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Bing", tier: 2, ms: 0 }); }
                    }
                });
            }
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_companies_house(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/UK-CompHouse", tier: 2, ms: 0 }); }
                    }
                });
            }
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_crunchbase(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Crunchbase", tier: 2, ms: 0 }); }
                    }
                });
            }
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_global_person(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/GlobalReg", tier: 2, ms: 0 }); }
                    }
                });
            }

            // Google HTML — broadest coverage for anyone with web presence
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_google_html(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Google-Web", tier: 2, ms: 0 }); }
                    }
                });
            }

            // GitHub API — tech/developer people (JSON, works everywhere)
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_github_api(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/GitHub", tier: 2, ms: 0 }); }
                    }
                });
            }

            // Google News RSS — people mentioned in Indian/global news
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_google_news_rss(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Google-News", tier: 2, ms: 0 }); }
                    }
                });
            }

            // Economic Times — Indian business professionals and entrepreneurs
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_economic_times(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/EconomicTimes", tier: 2, ms: 0 }); }
                    }
                });
            }

            // LinkedIn public profile — guesses URL slug from name patterns
            {
                let tx = tx2.clone(); let n = bn2.clone();
                std::thread::spawn(move || {
                    if let Ok(r) = search_linkedin_public(&n) {
                        if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/LinkedIn", tier: 2, ms: 0 }); }
                    }
                });
            }

            // Fuzzy variants — only for name queries
            if looks_like_name {
                let variants = fuzzy_name_variants(&bn2);
                for variant in variants.into_iter().take(3) {
                    let tx = tx2.clone();
                    std::thread::spawn(move || {
                        if let Ok(r) = search_indiafilings(&variant) {
                            if !r.is_empty() { let _ = tx.send(KyLmResult { answer: r, engine: "KhLM/Fuzzy-MCA", tier: 2, ms: 0 }); }
                        }
                    });
                }
            }
        });
    }

    // Agent: KhLM Neural (Tier 3) — only if model provided
    if let Some(mp) = model_path {
        let tx  = tx.clone();
        let mp2 = mp.to_string();
        let q5  = q.clone();
        let use_rlm = is_rlm_model_path(mp);
        std::thread::spawn(move || {
            if use_rlm {
                // True RLM model (DeepSeek-R1, QwQ): use killer_think() with <think> prefill
                // for real chain-of-thought reasoning — not just a system-prompt fake
                if let Ok(resp) = crate::inference::killer_think_rlm(&mp2, &q5, 800) {
                    let answer = resp.answer_only().to_string();
                    if answer.len() > 10 {
                        let _ = tx.send(KyLmResult { answer, engine: "KhLM/RLM-Think", tier: 3, ms: 0 });
                    }
                }
            } else {
                // Standard LLM: chain-of-thought system prompt approach
                if let Ok(resp) = crate::inference::killer_llm_as_rlm(&mp2, &q5, 512) {
                    let answer = resp.answer_only().to_string();
                    if answer.len() > 10 {
                        let _ = tx.send(KyLmResult { answer, engine: "KhLM/Neural", tier: 3, ms: 0 });
                    }
                }
            }
        });
    }

    drop(tx);

    // Collect — return first high-quality result
    // For person queries, validate the result actually matches the name asked.
    // Extract significant words from the query (proper nouns = capitalized or last word)
    let query_words: Vec<String> = q.split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphabetic()).to_string())
        .filter(|w| !matches!(w.as_str(), "what" | "who" | "where" | "when" | "is" | "are" | "was" | "were" | "the" | "and" | "for"))
        .collect();

    let timeout = std::time::Duration::from_secs(12);
    let deadline = std::time::Instant::now();
    let mut candidates: Vec<KyLmResult> = Vec::new();

    loop {
        let remaining = timeout.checked_sub(deadline.elapsed())
            .unwrap_or(std::time::Duration::ZERO);
        if remaining.is_zero() { break; }
        match rx.recv_timeout(remaining) {
            Ok(mut hit) => {
                hit.ms = start.elapsed().as_millis();
                let answer_lower = hit.answer.to_lowercase();

                // Score: how many query words appear in the answer?
                let _matched = query_words.iter()
                    .filter(|w| answer_lower.contains(w.as_str()))
                    .count();

                // Always collect ALL agent results — accuracy wins, not first-to-arrive.
                candidates.push(hit);
                // No cap — keep collecting until all agents finish (channel Disconnected) or timeout
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout)      => continue,
        }
    }

    // Pick best candidate using priority tiers:
    //   100% match → 1st priority (perfect, return immediately)
    //    90% match → 2nd priority
    //    80% match → last option
    //   <80%       → no confident match (e.g. Wikipedia "Vootala Sneha Deepthi" = 33% → rejected)
    if !candidates.is_empty() {
        let score_pct = |c: &KyLmResult| -> usize {
            if query_words.is_empty() { return 100; }
            let lower = c.answer.to_lowercase();
            let matched = query_words.iter().filter(|w| lower.contains(w.as_str())).count();
            (matched * 100) / query_words.len()
        };

        candidates.sort_by(|a, b| score_pct(b).cmp(&score_pct(a)));
        let best_pct = score_pct(&candidates[0]);

        let tier_label = if best_pct == 100 { "★★★ 100%" }
                        else if best_pct >= 90 { "★★☆  90%+" }
                        else if best_pct >= 80 { "★☆☆  80%+" }
                        else { "" };

        if !tier_label.is_empty() {
            let primary = candidates.remove(0);
            let primary_ans = primary.answer.clone();

            // Collect secondary: all other agents with ≥ 60% score, deduped by first 60 chars
            let mut seen_prefixes: Vec<String> = Vec::new();
            let mut secondary: Vec<(String, String)> = Vec::new(); // (engine_short, snippet)
            for c in &candidates {
                if score_pct(c) < 60 { continue; }
                // Skip if same source as primary
                if c.engine == primary.engine { continue; }
                // Deduplicate by answer prefix
                let prefix: String = c.answer.chars().take(60).collect();
                if seen_prefixes.iter().any(|p| p == &prefix) { continue; }
                seen_prefixes.push(prefix);
                // Extract engine short name (strip KhLM/ prefix)
                let short = c.engine.trim_start_matches("KhLM/");
                // Compact single-line snippet: collapse newlines, truncate at 200 chars
                let compact = c.answer.replace('\n', "  ").replace("  ", " ");
                let snippet: String = compact.chars().take(200).collect();
                let snippet = if compact.len() > 200 { format!("{}..", snippet.trim()) } else { snippet.trim().to_string() };
                secondary.push((short.to_string(), snippet));
                if secondary.len() >= 5 { break; }
            }

            let formatted = khlm_format_primary(
                &primary_ans, primary.engine, tier_label, primary.ms,
                &secondary,
            );
            if let Ok(mut cache) = khlm_cache().lock() {
                cache.insert(q, formatted.clone());
            }
            return formatted;
        }
    }

    // -- RLM Synthesis fallback ----------------------------------------------
    // If web agents returned results but none scored ≥80% confidence,
    // feed all fragments to the RLM so it can synthesize a coherent answer.
    if let Some(mp) = model_path {
        if !candidates.is_empty() {
            // Build a synthesis prompt with all the raw fragments
            let fragments: String = candidates.iter().take(5)
                .enumerate()
                .map(|(i, c)| {
                    let snippet: String = c.answer.chars().take(300).collect();
                    format!("Source {} [{}]: {}", i + 1, c.engine, snippet)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let synth_q = format!(
                "Based on these web search results, give a concise accurate answer to: {}\n\n{}",
                q, fragments
            );
            let use_rlm = is_rlm_model_path(mp);
            let synth_result = if use_rlm {
                crate::inference::killer_think_rlm(mp, &synth_q, 600)
                    .map(|r| r.answer_only().to_string())
                    .ok()
            } else {
                crate::inference::killer_llm_as_rlm(mp, &synth_q, 400)
                    .map(|r| r.answer_only().to_string())
                    .ok()
            };
            if let Some(synthesized) = synth_result {
                if synthesized.len() > 20 {
                    let ms = start.elapsed().as_millis();
                    let formatted = format!(
                        "+-- KhLM ---------------------------------------------\n\
                         |  KhLM/RLM-Synthesis  |  Tier 3  |  {}ms\n\
                         +-----------------------------------------------------\n\n{}",
                        ms, synthesized
                    );
                    if let Ok(mut cache) = khlm_cache().lock() {
                        cache.insert(q, formatted.clone());
                    }
                    return formatted;
                }
            }
        }
    }

    "KhLM: No result found.".to_string()
}

/// Format primary KhLM result with optional secondary "also found" section.
fn khlm_format_primary(
    primary_ans: &str,
    engine: &'static str,
    tier_label: &str,
    ms: u128,
    secondary: &[(String, String)],  // (short_engine, snippet)
) -> String {
    let mut out = format!(
        "+-- KhLM ---------------------------------------------\n\
         |  {} [{}]  ⚡ {}ms\n\
         +-----------------------------------------------------\n\n",
        engine, tier_label, ms
    );
    // Primary answer — preserve newlines (rich Tofler data has them)
    out.push_str(primary_ans);

    if !secondary.is_empty() {
        out.push_str("\n\n-- Also found ------------------------------------------");
        for (eng, snippet) in secondary {
            out.push_str(&format!("\n  [{}]  {}", eng, snippet));
        }
    }
    out
}

/// Expose the question classifier to Killer code.
/// Returns "math", "factual", or "reasoning".
pub fn khlm_classify_question(question: &str) -> &'static str {
    match classify_question(question) {
        QuestionKind::Reasoning => "reasoning",
        QuestionKind::Factual   => {
            let math_candidate = normalize_math_question(question);
            if is_math_expr(&math_candidate) { "math" } else { "factual" }
        }
    }
}

/// User-composable pipeline: run a named pipeline on a question.
/// pipelines: "web" | "rlm" | "web+rlm" | "rlm+web" | "auto"
pub fn khlm_run_pipeline(model_path: &str, question: &str, pipeline: &str) -> String {
    let q = question.trim();
    match pipeline.trim().to_lowercase().as_str() {
        "rlm" => {
            // Pure RLM — no web, just the model reasoning
            let use_rlm = is_rlm_model_path(model_path);
            if use_rlm {
                crate::inference::killer_think_rlm(model_path, q, 1200)
                    .map(|r| r.answer_only().to_string())
                    .unwrap_or_else(|e| format!("RLM error: {}", e))
            } else {
                crate::inference::killer_llm_as_rlm(model_path, q, 800)
                    .map(|r| r.answer_only().to_string())
                    .unwrap_or_else(|e| format!("LLM error: {}", e))
            }
        }
        "web" => {
            // Pure Ghost-108 web search — no model
            match ghost_108_search(q) {
                Ok(r) if !r.is_empty() => r,
                _ => "No web results found.".to_string(),
            }
        }
        "web+rlm" => {
            // Web first, then RLM synthesizes the results
            let web = ghost_108_search(q).unwrap_or_default();
            rlm_synthesize_answer(model_path, q, &web)
        }
        "rlm+web" => {
            // RLM reasons first, web search fills in facts it doesn't know
            let use_rlm = is_rlm_model_path(model_path);
            let rlm_answer = if use_rlm {
                crate::inference::killer_think_rlm(model_path, q, 600)
                    .map(|r| r.answer_only().to_string()).unwrap_or_default()
            } else {
                crate::inference::killer_llm_as_rlm(model_path, q, 400)
                    .map(|r| r.answer_only().to_string()).unwrap_or_default()
            };
            let web = ghost_108_search(q).unwrap_or_default();
            if rlm_answer.len() > 30 && web.len() > 30 {
                // Merge: RLM reasoning + web facts
                format!("{}\n\n[Web context: {}]", rlm_answer, &web.chars().take(300).collect::<String>())
            } else if rlm_answer.len() > 30 { rlm_answer }
            else { web }
        }
        _ => {
            // "auto" or unknown: use the full smart router
            khlm_route(q, Some(model_path))
        }
    }
}

/// Inner Ghost-108 search used by pipeline functions (returns raw best result).
fn ghost_108_search(question: &str) -> Result<String, String> {
    // Use DuckDuckGo instant answer + Wikipedia combination (same as ghost_ask but no model)
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<String>();
    let q = question.to_string();
    {
        let tx = tx.clone(); let q2 = q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_ddg_web(&q2) { if !r.is_empty() { let _ = tx.send(r); } }
        });
    }
    {
        let tx = tx.clone(); let q2 = q.clone();
        std::thread::spawn(move || {
            if let Ok(r) = search_wikipedia(&q2) { if !r.is_empty() { let _ = tx.send(r); } }
        });
    }
    drop(tx);
    rx.recv_timeout(std::time::Duration::from_secs(8))
        .map_err(|_| "timeout".to_string())
}

/// Run N questions through a model in parallel threads — returns answers in order.
pub fn llm_run_parallel(model_path: &str, questions: &[String], max_tokens: usize) -> Vec<String> {
    use std::sync::{Arc, Mutex};
    let results: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|s| {
        for (i, q) in questions.iter().enumerate() {
            let mp = model_path.to_string();
            let question = q.clone();
            let res = Arc::clone(&results);
            s.spawn(move || {
                let answer = crate::inference::killer_chat_auto(&mp, &question, None, max_tokens)
                    .unwrap_or_else(|e| format!("Error: {}", e));
                if let Ok(mut lock) = res.lock() {
                    lock.push((i, answer));
                }
            });
        }
    });
    let mut pairs = results.lock().map(|l| l.clone()).unwrap_or_default();
    pairs.sort_by_key(|(i, _)| *i);
    pairs.into_iter().map(|(_, a)| a).collect()
}

/// Max UTF-8 **characters** (not bytes) of external context fed into [`rlm_synthesize_answer`]
/// and `khlm_run` web→RLM synthesis. Set `KILLER_KHLM_SYNTH_CONTEXT_CHARS` to override
/// (clamped to 512…120_000; default 12_000).
pub fn khlm_synth_context_char_limit() -> usize {
    std::env::var("KILLER_KHLM_SYNTH_CONTEXT_CHARS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| (512..=120_000).contains(&n))
        .unwrap_or(12_000)
}

/// RLM synthesizes a final answer given the question and context you provide.
/// Use this to build your own retrieval-augmented generation (RAG) pipelines.
pub fn rlm_synthesize_answer(model_path: &str, question: &str, context: &str) -> String {
    let synth_q = if context.is_empty() {
        question.to_string()
    } else {
        let ctx_snippet: String = context.chars().take(khlm_synth_context_char_limit()).collect();
        format!(
            "Using the following context, give a concise accurate answer to: {}\n\nContext:\n{}",
            question, ctx_snippet
        )
    };
    let use_rlm = is_rlm_model_path(model_path);
    if use_rlm {
        crate::inference::killer_think_rlm(model_path, &synth_q, 800)
            .map(|r| r.answer_only().to_string())
            .unwrap_or_else(|e| format!("RLM error: {}", e))
    } else {
        crate::inference::killer_llm_as_rlm(model_path, &synth_q, 600)
            .map(|r| r.answer_only().to_string())
            .unwrap_or_else(|e| format!("LLM error: {}", e))
    }
}

/// Format a KhLM result with the branded header (used for Tier 1 instant results).
fn khlm_format(r: KyLmResult, answer: &str) -> String {
    format!(
        "+-- KhLM ---------------------------------------------\n\
         |  {}  |  Tier {}  |  {}ms\n\
         +-----------------------------------------------------\n\n{}",
        r.engine, r.tier, r.ms, answer
    )
}

/// Convert `Vec<LlmMessage>` to JSON array `[{"role":"...","content":"..."},...]`
fn messages_to_json(messages: &[LlmMessage]) -> String {
    let parts: Vec<String> = messages.iter().map(|m| {
        format!(r#"{{"role":{},"content":{}}}"#, json_string(&m.role), json_string(&m.content))
    }).collect();
    format!("[{}]", parts.join(","))
}

/// Extract HTTP body from a raw HTTP/1.1 response (skip headers).
fn extract_http_body(raw: &str) -> Result<String, String> {
    // Find the blank line separating headers from body
    if let Some(pos) = raw.find("\r\n\r\n") {
        let body = raw[pos + 4..].to_string();
        // Check for chunked transfer encoding — strip chunk sizes
        return if body.starts_with(|c: char| c.is_ascii_hexdigit()) && body.contains("\r\n") {
            Ok(unchunk(&body))
        } else {
            Ok(body)
        };
    }
    if let Some(pos) = raw.find("\n\n") {
        return Ok(raw[pos + 2..].to_string());
    }
    // No headers detected — return as-is (already just the body)
    Ok(raw.to_string())
}

/// Decode HTTP/1.1 chunked transfer encoding.
fn unchunk(body: &str) -> String {
    let mut result = String::new();
    let mut rest = body;
    loop {
        let end_of_size = match rest.find("\r\n") {
            Some(p) => p,
            None    => break,
        };
        let hex_str = rest[..end_of_size].trim();
        let size = usize::from_str_radix(hex_str, 16).unwrap_or(0);
        if size == 0 { break; }
        let data_start = end_of_size + 2;
        if data_start + size > rest.len() { break; }
        result.push_str(&rest[data_start..data_start + size]);
        rest = &rest[data_start + size..].trim_start_matches("\r\n");
    }
    result
}

/// Resolve API key: config field → environment variable.
fn resolve_api_key(config: &LlmConfig, env_var: &str) -> Result<String, String> {
    if let Some(k) = &config.api_key {
        if !k.is_empty() { return Ok(k.clone()); }
    }
    std::env::var(env_var)
        .map_err(|_| format!(
            "No API key found. Pass it to the config constructor or set the {} environment variable.",
            env_var
        ))
}

// --- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_string_escaping() {
        let s = json_string("Hello \"world\"\nline2");
        assert!(s.contains("\\\""));
        assert!(s.contains("\\n"));
    }

    #[test]
    fn test_extract_json_string_simple() {
        let json = r#"{"role":"assistant","content":"Hello!"}"#;
        assert_eq!(extract_json_string(json, "content"), Some("Hello!".to_string()));
        assert_eq!(extract_json_string(json, "role"),    Some("assistant".to_string()));
        assert_eq!(extract_json_string(json, "missing"), None);
    }

    #[test]
    fn test_extract_json_string_escaped() {
        let json = r#"{"content":"She said \"hi\""}"#;
        assert_eq!(extract_json_string(json, "content"), Some(r#"She said "hi""#.to_string()));
    }

    #[test]
    fn test_extract_json_usize() {
        let json = r#"{"prompt_tokens":42,"completion_tokens":7}"#;
        assert_eq!(extract_json_usize(json, "prompt_tokens"),    Some(42));
        assert_eq!(extract_json_usize(json, "completion_tokens"), Some(7));
    }

    #[test]
    fn test_extract_http_body_with_headers() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"hello\":\"world\"}";
        let body = extract_http_body(raw).unwrap();
        assert_eq!(body, "{\"hello\":\"world\"}");
    }

    #[test]
    fn test_messages_to_json() {
        let msgs = vec![
            LlmMessage::system("You are helpful"),
            LlmMessage::user("hi"),
        ];
        let json = messages_to_json(&msgs);
        assert!(json.contains("\"role\":\"system\""));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("You are helpful"));
    }

    #[test]
    fn test_parse_ollama_style_response() {
        let raw = r#"{"model":"llama3","message":{"role":"assistant","content":"Hello!"},"done":true,"prompt_eval_count":5,"eval_count":3}"#;
        let resp = parse_ollama_chat_response(raw, "llama3").unwrap();
        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.prompt_tokens, 5);
        assert_eq!(resp.completion_tokens, 3);
    }

    #[test]
    fn test_parse_openai_style_response() {
        let raw = r#"{"id":"x","object":"chat.completion","model":"gpt-4o-mini","choices":[{"index":0,"message":{"role":"assistant","content":"The answer is 42."}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;
        let resp = parse_openai_response(raw, "gpt-4o-mini").unwrap();
        assert_eq!(resp.content, "The answer is 42.");
        assert_eq!(resp.prompt_tokens, 10);
        assert_eq!(resp.completion_tokens, 5);
    }

    #[test]
    fn test_parse_anthropic_style_response() {
        let raw = r#"{"id":"msg_01","type":"message","role":"assistant","content":[{"type":"text","text":"42 is the answer."}],"model":"claude-3-haiku-20240307","usage":{"input_tokens":12,"output_tokens":8}}"#;
        let resp = parse_anthropic_response(raw, "claude-3-haiku-20240307").unwrap();
        assert_eq!(resp.content, "42 is the answer.");
        assert_eq!(resp.prompt_tokens, 12);
        assert_eq!(resp.completion_tokens, 8);
    }

    #[test]
    fn test_unchunk() {
        let chunked = "7\r\nHello, \r\n6\r\nworld!\r\n0\r\n\r\n";
        assert_eq!(unchunk(chunked), "Hello, world!");
    }

    #[test]
    fn test_config_builders() {
        let cfg = LlmConfig::ollama("llama3");
        assert!(cfg.api_key.is_none());
        assert_eq!(cfg.model, "llama3");

        let cfg2 = LlmConfig::openai("sk-test", "gpt-4o-mini")
            .with_max_tokens(512)
            .with_temperature(0.1);
        assert_eq!(cfg2.max_tokens, 512);
        assert!((cfg2.temperature - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_curl_rejects_non_https() {
        let result = curl_post("http://evil.local/steal", &[], "{}", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTPS"));
    }

    #[test]
    fn eval_simple_math_pow_and_sci() {
        assert!((eval_simple_math("2^3^2").unwrap() - 512.0).abs() < 1e-9);
        assert!((eval_simple_math("2*3^2").unwrap() - 18.0).abs() < 1e-9);
        assert!((eval_simple_math("1e2+1").unwrap() - 101.0).abs() < 1e-9);
        assert!((eval_simple_math("(-2)^3").unwrap() - (-8.0)).abs() < 1e-9);
    }

    #[test]
    fn try_eval_arithmetic_string_normalizes_english() {
        let r = try_eval_arithmetic_string("What is 3 + 4 * 5?").unwrap();
        assert!((r - 23.0).abs() < 1e-9);
        let r2 = try_eval_arithmetic_string("Solve 2 ^ 10").unwrap();
        assert!((r2 - 1024.0).abs() < 1e-9);
    }

    #[test]
    fn try_eval_arithmetic_subst_var_basic() {
        let r = try_eval_arithmetic_subst_var("2*x + 1", "x", 5.0).unwrap();
        assert!((r - 11.0).abs() < 1e-9);
        let r2 = try_eval_arithmetic_subst_var("a^2", "a", 4.0).unwrap();
        assert!((r2 - 16.0).abs() < 1e-9);
    }

    #[test]
    fn bisection_root_sqrt_two() {
        let r = bisection_root("x*x-2", "x", 1.0, 2.0).unwrap();
        assert!((r - 2f64.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn smart_ghost_parse_and_score() {
        let raw = "APPROACH: multiply\nVERIFY_EXPR: 6*7\nNUMERIC_ROOT: NONE\nANSWER: The product is 42.\n";
        let p = parse_smart_ghost_response(raw);
        assert!(p.verify_expr.as_deref() == Some("6*7"));
        let (s, notes) = score_smart_attempt(&p);
        assert!(s >= 80, "score={} notes={:?}", s, notes);
    }
}

// ── Public API for builtin.rs (cross-module access) ─────────────────────
/// Public wrapper for knowledge_base_lookup — used by builtin.rs expert_ask.
pub fn knowledge_base_lookup_pub(q: &str) -> Option<String> {
    knowledge_base_lookup(q)
}

/// Public wrapper for comparison_handler — used by builtin.rs for "X vs Y" questions.
pub fn comparison_handler_pub(q: &str) -> Option<String> {
    comparison_handler(q)
}

// ── JSON-mode output parsing ────────────────────────────────────────────────

/// Attempt to extract a JSON object or array from an LLM response.
/// Handles markdown code fences (```json ... ```) and bare JSON.
pub fn extract_json(response: &str) -> Option<String> {
    let trimmed = response.trim();
    // Strip ```json ... ``` fences
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            let inner = after[..end].trim();
            if inner.starts_with('{') || inner.starts_with('[') {
                return Some(inner.to_string());
            }
        }
    }
    // Bare JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }
    None
}

// ── Multi-turn conversation state ───────────────────────────────────────────

/// Manages a multi-turn conversation with an LLM, preserving message history.
#[derive(Clone, Debug)]
pub struct Conversation {
    pub config:   LlmConfig,
    pub messages: Vec<LlmMessage>,
}

impl Conversation {
    /// Start a new conversation with an optional system prompt.
    pub fn new(config: LlmConfig, system_prompt: Option<&str>) -> Self {
        let mut msgs = Vec::new();
        if let Some(sp) = system_prompt {
            msgs.push(LlmMessage::system(sp));
        }
        Conversation { config, messages: msgs }
    }

    /// Send a user message and get the assistant's reply.  History is preserved.
    pub fn say(&mut self, user_msg: &str) -> Result<String, String> {
        self.messages.push(LlmMessage::user(user_msg));
        let resp = complete(&self.config, &self.messages)?;
        self.messages.push(LlmMessage::assistant(&resp.content));
        Ok(resp.content)
    }

    /// Send a user message expecting JSON back.  Calls `extract_json` on the response.
    pub fn say_json(&mut self, user_msg: &str) -> Result<String, String> {
        let mut json_config = self.config.clone();
        json_config.json_mode = true;
        self.messages.push(LlmMessage::user(user_msg));
        let resp = complete(&json_config, &self.messages)?;
        self.messages.push(LlmMessage::assistant(&resp.content));
        extract_json(&resp.content).ok_or_else(|| "LLM response was not valid JSON".to_string())
    }

    /// Number of messages in the conversation so far.
    pub fn len(&self) -> usize { self.messages.len() }

    /// Clear history (keep system prompt if present).
    pub fn reset(&mut self) {
        let sys: Vec<LlmMessage> = self.messages.iter()
            .filter(|m| m.role == "system")
            .cloned()
            .collect();
        self.messages = sys;
    }
}
