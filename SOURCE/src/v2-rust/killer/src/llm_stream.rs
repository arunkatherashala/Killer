//! Streaming LLM output: `llm_stream(opts, prompt)` and `llm_stream_cb(opts, prompt, fn)`.
//!
//! `llm_stream` returns a lazy generator of text chunks (deltas): the request is sent when the
//! first chunk is asked for and curl is read incrementally, so `for chunk in llm_stream(..)`
//! prints text as the model produces it and `break` (or `gen_close(g)`) stops consuming; the curl
//! child is killed when the generator is closed, finished, or the VM is dropped.
//! `llm_stream_cb(opts, prompt, fn)` calls `fn(chunk)` per delta (return `false` to stop) and
//! returns the whole text.
//!
//! Options dict: `provider` ("openai" default, "openai_compatible"/"compat", "groq", "anthropic",
//! "ollama", "mock"), `model`, `base_url`, `key` / `key_env` (see `embed_builtins::resolve_key`;
//! provider default env var OPENAI_API_KEY / GROQ_API_KEY / ANTHROPIC_API_KEY), `system`,
//! `max_tokens`, `temperature`, `timeout` (seconds for the whole stream, default 120),
//! `insecure`, `allow_http`. `prompt` is a string or an array of `{role, content}` dicts.
//!
//! Wire formats: OpenAI-compatible SSE (`data: {choices:[{delta:{content}}]}` ... `[DONE]`),
//! Anthropic SSE (`event: content_block_delta` with `delta.text`, `message_stop`), Ollama NDJSON
//! (`{message:{content}, done}`). Secrets travel only on curl's stdin (`http_req`), never argv.
//!
//! `provider: "mock"` needs no network: chunks come from `chunks` (array), else the words of
//! `reply`, else "Mock reply to: <prompt>"; `delay_ms` sleeps before each chunk, `fail_after: n`
//! raises an error after n chunks.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};

use crate::error::VmError;
use crate::generator::NativeSource;
use crate::http_req::{build_argv, build_config, ReqSpec};
use crate::bytecode::Program;
use crate::sse::{LineParser, SseParser};
use crate::value::{SharedArray, SharedDict, Value};
use crate::vm::VirtualMachine;

pub const NAMES: &[&str] = &["llm_stream", "llm_stream_cb", "gen_close"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wire {
    OpenAi,
    Anthropic,
    Ollama,
}

/// What one provider event means.
#[derive(Debug, PartialEq)]
pub enum Delta {
    Text(String),
    Done,
    Skip,
    Err(String),
}

fn dict_get(v: &Value, k: &str) -> Option<Value> {
    match v {
        Value::Dict(d) => d.get(k),
        _ => None,
    }
}

fn error_message(v: &Value) -> Option<String> {
    let e = dict_get(v, "error")?;
    Some(match &e {
        Value::Str(s) => s.clone(),
        other => match dict_get(other, "message") {
            Some(Value::Str(m)) => m,
            _ => "unknown provider error".to_string(),
        },
    })
}

fn text_of(v: Option<Value>) -> Option<String> {
    match v {
        Some(Value::Str(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// One OpenAI-compatible SSE `data:` payload.
pub fn openai_delta(data: &str) -> Delta {
    let d = data.trim();
    if d == "[DONE]" {
        return Delta::Done;
    }
    let Ok(v) = crate::json_lang::parse(d) else { return Delta::Skip };
    if let Some(m) = error_message(&v) {
        return Delta::Err(format!("provider error: {}", m));
    }
    let first = match dict_get(&v, "choices") {
        Some(Value::Array(a)) => a.get(0),
        _ => None,
    };
    match first.and_then(|c| dict_get(&c, "delta")).and_then(|d| text_of(dict_get(&d, "content"))) {
        Some(t) => Delta::Text(t),
        None => Delta::Skip,
    }
}

/// One Anthropic SSE event (`event:` name may be absent: the JSON `type` is used then).
pub fn anthropic_delta(event: Option<&str>, data: &str) -> Delta {
    let Ok(v) = crate::json_lang::parse(data.trim()) else { return Delta::Skip };
    let ty = match event {
        Some(e) => e.to_string(),
        None => match dict_get(&v, "type") {
            Some(Value::Str(t)) => t,
            _ => String::new(),
        },
    };
    match ty.as_str() {
        "content_block_delta" => match dict_get(&v, "delta").and_then(|d| text_of(dict_get(&d, "text"))) {
            Some(t) => Delta::Text(t),
            None => Delta::Skip,
        },
        "message_stop" => Delta::Done,
        "error" => Delta::Err(format!("provider error: {}", error_message(&v).unwrap_or_else(|| "unknown".to_string()))),
        _ => Delta::Skip,
    }
}

/// One Ollama NDJSON line (`/api/chat`: `message.content`; `/api/generate`: `response`).
pub fn ollama_delta(line: &str) -> Delta {
    let Ok(v) = crate::json_lang::parse(line.trim()) else {
        return Delta::Err(format!("unexpected response: {}", line.chars().take(200).collect::<String>()));
    };
    if let Some(Value::Str(e)) = dict_get(&v, "error") {
        return Delta::Err(format!("provider error: {}", e));
    }
    let t = dict_get(&v, "message")
        .and_then(|m| text_of(dict_get(&m, "content")))
        .or_else(|| text_of(dict_get(&v, "response")));
    if let Some(t) = t {
        return Delta::Text(t);
    }
    if matches!(dict_get(&v, "done"), Some(Value::Bool(true))) { Delta::Done } else { Delta::Skip }
}

// --- request building -----------------------------------------------------------

fn opt_str(d: &SharedDict, k: &str) -> Option<String> {
    match d.get(k) {
        Some(Value::Str(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

fn opt_num(d: &SharedDict, k: &str) -> Option<f64> {
    match d.get(k) {
        Some(Value::Number(n)) => Some(n),
        Some(Value::Integer(i)) => Some(i as f64),
        _ => None,
    }
}

fn dict(pairs: Vec<(&str, Value)>) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
}

fn msg(role: &str, content: &str) -> Value {
    dict(vec![("role", Value::Str(role.to_string())), ("content", Value::Str(content.to_string()))])
}

/// `(role, content)` pairs from a prompt string or an array of `{role, content}` dicts.
pub fn messages_of(prompt: &Value) -> Result<Vec<(String, String)>, String> {
    match prompt {
        Value::Str(s) => Ok(vec![("user".to_string(), s.clone())]),
        Value::Array(a) => {
            let mut out = Vec::new();
            for m in a.to_vec() {
                match (dict_get(&m, "role"), dict_get(&m, "content")) {
                    (Some(Value::Str(r)), Some(Value::Str(c))) => out.push((r, c)),
                    _ => return Err("each message must be a dict with string role and content".to_string()),
                }
            }
            if out.is_empty() {
                return Err("prompt has no messages".to_string());
            }
            Ok(out)
        }
        _ => Err("prompt must be a string or an array of {role, content} messages".to_string()),
    }
}

/// A fully described streaming request (no I/O yet).
pub struct Plan {
    pub wire: Wire,
    pub spec: ReqSpec,
}

pub fn build_plan(opts: &SharedDict, messages: &[(String, String)]) -> Result<Plan, String> {
    let provider = opt_str(opts, "provider").unwrap_or_else(|| "openai".to_string()).to_lowercase();
    let max_tokens = opt_num(opts, "max_tokens").unwrap_or(1024.0).max(1.0);
    let temperature = opt_num(opts, "temperature");
    let system = opt_str(opts, "system");
    let (wire, default_base, default_env) = match provider.as_str() {
        "openai" => (Wire::OpenAi, Some("https://api.openai.com/v1"), Some("OPENAI_API_KEY")),
        "groq" => (Wire::OpenAi, Some("https://api.groq.com/openai/v1"), Some("GROQ_API_KEY")),
        "openai_compatible" | "compat" => (Wire::OpenAi, None, None),
        "anthropic" => (Wire::Anthropic, Some("https://api.anthropic.com/v1"), Some("ANTHROPIC_API_KEY")),
        "ollama" => (Wire::Ollama, Some("http://127.0.0.1:11434"), None),
        other => return Err(format!("llm_stream: provider {} is not supported (openai, openai_compatible, groq, anthropic, ollama, mock)", other)),
    };
    let base = match (opt_str(opts, "base_url"), default_base) {
        (Some(b), _) => b.trim_end_matches('/').to_string(),
        (None, Some(b)) => b.to_string(),
        (None, None) => return Err("llm_stream: provider openai_compatible needs a base_url".to_string()),
    };
    let key = crate::embed_builtins::resolve_key(opts, default_env)?;
    let key = if key.is_none() { crate::embed_builtins::resolve_key(&alias_key(opts), None)? } else { key };
    let model = opt_str(opts, "model").unwrap_or_else(|| match wire {
        Wire::OpenAi => "gpt-4o-mini".to_string(),
        Wire::Anthropic => "claude-3-5-haiku-latest".to_string(),
        Wire::Ollama => "llama3".to_string(),
    });
    let needs_key = matches!(provider.as_str(), "openai" | "groq" | "anthropic") && opt_str(opts, "base_url").is_none();
    if needs_key && key.is_none() {
        return Err(format!("llm_stream: no API key: pass key, key_env, or set {}", default_env.unwrap_or("the provider key variable")));
    }

    let mut all: Vec<Value> = Vec::new();
    let mut sys = system;
    for (r, c) in messages {
        if wire == Wire::Anthropic && r == "system" {
            sys = Some(match sys {
                Some(s) => format!("{}\n{}", s, c),
                None => c.clone(),
            });
        } else {
            all.push(msg(r, c));
        }
    }
    let (url, body) = match wire {
        Wire::OpenAi => {
            if let Some(s) = &sys {
                all.insert(0, msg("system", s));
            }
            let mut p = vec![
                ("model", Value::Str(model)),
                ("messages", Value::Array(SharedArray::new(all))),
                ("max_tokens", Value::Number(max_tokens)),
                ("stream", Value::Bool(true)),
            ];
            if let Some(t) = temperature {
                p.push(("temperature", Value::Number(t)));
            }
            (format!("{}/chat/completions", base), dict(p))
        }
        Wire::Anthropic => {
            let mut p = vec![
                ("model", Value::Str(model)),
                ("messages", Value::Array(SharedArray::new(all))),
                ("max_tokens", Value::Number(max_tokens)),
                ("stream", Value::Bool(true)),
            ];
            if let Some(s) = sys {
                p.push(("system", Value::Str(s)));
            }
            if let Some(t) = temperature {
                p.push(("temperature", Value::Number(t)));
            }
            (format!("{}/messages", base), dict(p))
        }
        Wire::Ollama => {
            if let Some(s) = &sys {
                all.insert(0, msg("system", s));
            }
            let mut o = vec![("num_predict", Value::Number(max_tokens))];
            if let Some(t) = temperature {
                o.push(("temperature", Value::Number(t)));
            }
            (
                format!("{}/api/chat", base),
                dict(vec![
                    ("model", Value::Str(model)),
                    ("messages", Value::Array(SharedArray::new(all))),
                    ("stream", Value::Bool(true)),
                    ("options", dict(o)),
                ]),
            )
        }
    };
    let mut spec = ReqSpec::new("POST", &url);
    spec.headers.push(("Content-Type".to_string(), "application/json".to_string()));
    match wire {
        Wire::Anthropic => {
            if let Some(k) = &key {
                spec.headers.push(("x-api-key".to_string(), k.clone()));
            }
            spec.headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
        }
        _ => {
            if let Some(k) = &key {
                spec.headers.push(("Authorization".to_string(), format!("Bearer {}", k)));
            }
        }
    }
    spec.headers.push(("Accept".to_string(), if wire == Wire::Ollama { "application/x-ndjson" } else { "text/event-stream" }.to_string()));
    spec.body = Some(crate::json_lang::stringify(&body, 0)?);
    spec.timeout_s = opt_num(opts, "timeout").unwrap_or(120.0);
    spec.insecure = matches!(opts.get("insecure"), Some(Value::Bool(true)));
    spec.allow_http = matches!(opts.get("allow_http"), Some(Value::Bool(true)));
    Ok(Plan { wire, spec })
}

/// `api_key` is accepted as an alias of `key`.
fn alias_key(opts: &SharedDict) -> SharedDict {
    let mut pairs = Vec::new();
    if let Some(k) = opt_str(opts, "api_key") {
        pairs.push(("key".to_string(), Value::Str(k)));
    }
    SharedDict::from_pairs(pairs)
}

/// curl arguments for a stream: the standard hardened set minus `-i`, plus `-N` (no buffering).
pub fn stream_argv(spec: &ReqSpec) -> Vec<String> {
    let mut a: Vec<String> = build_argv(spec).into_iter().filter(|x| x != "-i").collect();
    a.insert(0, "-N".to_string());
    a
}

// --- the source -----------------------------------------------------------------

enum Inner {
    Curl { child: Child, stdout: ChildStdout },
    Mock { chunks: VecDeque<String>, delay_ms: u64, fail_after: Option<usize>, emitted: usize },
    Closed,
}

/// A lazily read stream of text deltas.
pub struct StreamSource {
    inner: Inner,
    wire: Wire,
    sse: SseParser,
    lines: LineParser,
    queue: VecDeque<Result<String, String>>,
    done: bool,
    saw_event: bool,
    raw_head: Vec<u8>,
}

impl StreamSource {
    fn mock(chunks: Vec<String>, delay_ms: u64, fail_after: Option<usize>) -> Self {
        StreamSource {
            inner: Inner::Mock { chunks: chunks.into(), delay_ms, fail_after, emitted: 0 },
            wire: Wire::OpenAi,
            sse: SseParser::new(),
            lines: LineParser::new(),
            queue: VecDeque::new(),
            done: false,
            saw_event: true,
            raw_head: Vec::new(),
        }
    }

    pub fn start(plan: Plan) -> Result<Self, String> {
        let cfg = build_config(&plan.spec)?;
        let mut child = Command::new(if cfg!(target_os = "windows") { "curl.exe" } else { "curl" })
            .args(stream_argv(&plan.spec))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl not found: {}. Install curl from https://curl.se", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(cfg.as_bytes());
        }
        let stdout = child.stdout.take().ok_or("curl gave no stdout")?;
        Ok(StreamSource {
            inner: Inner::Curl { child, stdout },
            wire: plan.wire,
            sse: SseParser::new(),
            lines: LineParser::new(),
            queue: VecDeque::new(),
            done: false,
            saw_event: false,
            raw_head: Vec::new(),
        })
    }

    fn push_delta(&mut self, d: Delta) {
        // Once the stream is finished (`[DONE]`, `done: true`, an error) nothing after it counts, even
        // when the provider's later events arrived in the same network read.
        if self.done {
            return;
        }
        match d {
            Delta::Text(t) => self.queue.push_back(Ok(t)),
            Delta::Done => self.done = true,
            Delta::Skip => {}
            Delta::Err(e) => {
                self.queue.push_back(Err(e));
                self.done = true;
            }
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        match self.wire {
            Wire::Ollama => {
                for l in self.lines.feed(bytes) {
                    self.saw_event = true;
                    self.push_delta(ollama_delta(&l));
                }
            }
            w => {
                for e in self.sse.feed(bytes) {
                    self.saw_event = true;
                    let d = if w == Wire::OpenAi { openai_delta(&e.data) } else { anthropic_delta(e.event.as_deref(), &e.data) };
                    self.push_delta(d);
                }
                if !self.saw_event && self.raw_head.len() < 600 {
                    self.raw_head.extend_from_slice(bytes);
                }
            }
        }
    }

    fn at_eof(&mut self) {
        match self.wire {
            Wire::Ollama => {
                for l in self.lines.finish() {
                    self.saw_event = true;
                    self.push_delta(ollama_delta(&l));
                }
            }
            w => {
                for e in self.sse.finish() {
                    self.saw_event = true;
                    let d = if w == Wire::OpenAi { openai_delta(&e.data) } else { anthropic_delta(e.event.as_deref(), &e.data) };
                    self.push_delta(d);
                }
            }
        }
        self.done = true;
        let (ok, stderr_text) = match &mut self.inner {
            Inner::Curl { child, .. } => {
                let ok = matches!(child.wait(), Ok(s) if s.success());
                let mut t = String::new();
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_string(&mut t);
                }
                (ok, t)
            }
            _ => return,
        };
        if !ok && !self.done_with_error() {
            let m = stderr_text.trim();
            self.queue.push_back(Err(format!("HTTP request failed: {}", if m.is_empty() { "curl exited with an error" } else { m })));
        } else if !self.saw_event && !self.raw_head.is_empty() && !self.done_with_error() {
            let raw = String::from_utf8_lossy(&self.raw_head).trim().to_string();
            let msg = crate::json_lang::parse(&raw).ok().and_then(|v| error_message(&v)).unwrap_or(raw);
            self.queue.push_back(Err(format!("unexpected response: {}", msg.chars().take(300).collect::<String>())));
        }
    }

    fn done_with_error(&self) -> bool {
        self.queue.iter().any(|r| r.is_err())
    }

    /// The next text chunk, `Ok(None)` at the end of the stream.
    pub fn next_chunk(&mut self) -> Result<Option<String>, String> {
        loop {
            if let Some(r) = self.queue.pop_front() {
                if r.is_err() {
                    self.close();
                }
                return r.map(Some);
            }
            if self.done {
                self.close();
                return Ok(None);
            }
            match &mut self.inner {
                Inner::Closed => return Ok(None),
                Inner::Mock { chunks, delay_ms, fail_after, emitted } => {
                    if *fail_after == Some(*emitted) {
                        self.inner = Inner::Closed;
                        return Err("mock stream failed".to_string());
                    }
                    if *delay_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
                    }
                    match chunks.pop_front() {
                        Some(c) => {
                            *emitted += 1;
                            return Ok(Some(c));
                        }
                        None => {
                            self.inner = Inner::Closed;
                            return Ok(None);
                        }
                    }
                }
                Inner::Curl { stdout, .. } => {
                    let mut buf = [0u8; 4096];
                    let n = match stdout.read(&mut buf) {
                        Ok(n) => n,
                        Err(e) => return Err(format!("read error: {}", e)),
                    };
                    if n == 0 {
                        self.at_eof();
                    } else {
                        self.feed(&buf[..n]);
                    }
                }
            }
        }
    }

    /// Stop the stream now (kills curl).
    pub fn close(&mut self) {
        if let Inner::Curl { child, .. } = &mut self.inner {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.inner = Inner::Closed;
    }
}

impl Drop for StreamSource {
    fn drop(&mut self) {
        self.close();
    }
}

impl NativeSource for StreamSource {
    fn next_value(&mut self) -> Result<Option<Value>, String> {
        self.next_chunk().map(|o| o.map(Value::Str))
    }
}

/// Split text into word tokens that keep their leading space: "a b c" -> ["a", " b", " c"].
pub fn mock_words(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, w) in text.split_whitespace().enumerate() {
        out.push(if i == 0 { w.to_string() } else { format!(" {}", w) });
    }
    out
}

fn open(opts_v: &Value, prompt: &Value, who: &str) -> Result<StreamSource, VmError> {
    let opts = match opts_v {
        Value::Dict(d) => d.clone(),
        Value::Str(p) => SharedDict::from_pairs(vec![("provider".to_string(), Value::Str(p.clone()))]),
        _ => return Err(VmError::runtime_error(format!("{}(opts, prompt) expects an options dict", who))),
    };
    let messages = messages_of(prompt).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))?;
    let provider = opt_str(&opts, "provider").unwrap_or_else(|| "openai".to_string()).to_lowercase();
    if provider == "mock" {
        let chunks: Vec<String> = match opts.get("chunks") {
            Some(Value::Array(a)) => a.to_vec().iter().map(|c| match c {
                Value::Str(s) => s.clone(),
                other => format!("{}", other),
            }).collect(),
            _ => match opt_str(&opts, "reply") {
                Some(r) => mock_words(&r),
                None => mock_words(&format!("Mock reply to: {}", messages.last().map(|m| m.1.as_str()).unwrap_or(""))),
            },
        };
        let delay = opt_num(&opts, "delay_ms").unwrap_or(0.0).max(0.0) as u64;
        let fail = opt_num(&opts, "fail_after").map(|n| n.max(0.0) as usize);
        return Ok(StreamSource::mock(chunks, delay, fail));
    }
    crate::security::require_network()?;
    let plan = build_plan(&opts, &messages).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e.trim_start_matches("llm_stream: "))))?;
    StreamSource::start(plan).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))
}

/// VM-level entry for `llm_stream`, `llm_stream_cb` and `gen_close`.
pub fn vm_call(vm: &mut VirtualMachine, name: &str, args: &[Value], program: &Program) -> Option<Result<Value, VmError>> {
    match name {
        "llm_stream" => Some((|| match args {
            [o, p] => {
                let src = open(o, p, "llm_stream")?;
                Ok(vm.register_native_gen(Box::new(src)))
            }
            _ => Err(VmError::runtime_error("llm_stream(opts, prompt) expects 2 arguments".to_string())),
        })()),
        "llm_stream_cb" => Some((|| match args {
            [o, p, f @ Value::Function { .. }] => {
                let mut src = open(o, p, "llm_stream_cb")?;
                let mut all = String::new();
                loop {
                    match src.next_chunk() {
                        Ok(Some(c)) => {
                            all.push_str(&c);
                            let r = vm.call_value_nested(f, vec![Value::Str(c)], program)?;
                            if matches!(r, Value::Bool(false)) {
                                src.close();
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => return Err(VmError::runtime_error(format!("llm_stream_cb: {}", e))),
                    }
                }
                Ok(Value::Str(all))
            }
            _ => Err(VmError::runtime_error("llm_stream_cb(opts, prompt, fn) expects options, a prompt and a function".to_string())),
        })()),
        "gen_close" => Some(match args {
            [Value::Generator(id)] => {
                vm.close_generator(id);
                Ok(Value::Null)
            }
            _ => Err(VmError::runtime_error("gen_close(generator) expects a generator".to_string())),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed_source(wire: Wire) -> StreamSource {
        StreamSource {
            inner: Inner::Closed,
            wire,
            sse: SseParser::new(),
            lines: LineParser::new(),
            queue: VecDeque::new(),
            done: false,
            saw_event: false,
            raw_head: Vec::new(),
        }
    }

    fn drain(s: &mut StreamSource) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(Some(c)) = s.next_chunk() {
            out.push(c);
        }
        out
    }

    #[test]
    fn nothing_after_done_is_delivered_even_in_one_read() {
        let mut s = closed_source(Wire::OpenAi);
        let one_read = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n\
                        data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n\
                        data: [DONE]\n\n\
                        data: {\"choices\":[{\"delta\":{\"content\":\"AFTER DONE\"}}]}\n\n";
        s.feed(one_read.as_bytes());
        assert_eq!(drain(&mut s), vec!["Hel".to_string(), "lo".to_string()]);
    }

    #[test]
    fn same_result_when_the_bytes_arrive_one_at_a_time() {
        let mut s = closed_source(Wire::OpenAi);
        let text = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: [DONE]\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"late\"}}]}\n\n";
        for b in text.as_bytes() {
            s.feed(std::slice::from_ref(b));
        }
        assert_eq!(drain(&mut s), vec!["a".to_string()]);
    }

    #[test]
    fn ollama_stops_at_done_true_within_one_read() {
        let mut s = closed_source(Wire::Ollama);
        let one_read = "{\"message\":{\"content\":\"Yo\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true}\n{\"message\":{\"content\":\"late\"},\"done\":false}\n";
        s.feed(one_read.as_bytes());
        assert_eq!(drain(&mut s), vec!["Yo".to_string()]);
    }

    #[test]
    fn an_error_event_ends_the_stream_in_one_read() {
        let mut s = closed_source(Wire::OpenAi);
        let one_read = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: {\"error\":{\"message\":\"boom\"}}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"late\"}}]}\n\n";
        s.feed(one_read.as_bytes());
        assert_eq!(s.next_chunk(), Ok(Some("ok".to_string())));
        assert_eq!(s.next_chunk(), Err("provider error: boom".to_string()));
        assert_eq!(s.next_chunk(), Ok(None));
    }

    #[test]
    fn openai_deltas() {
        assert_eq!(openai_delta(r#"{"choices":[{"delta":{"content":"Hel"}}]}"#), Delta::Text("Hel".into()));
        assert_eq!(openai_delta(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#), Delta::Skip);
        assert_eq!(openai_delta(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#), Delta::Skip);
        assert_eq!(openai_delta(" [DONE]"), Delta::Done);
        assert_eq!(openai_delta("not json"), Delta::Skip);
        assert_eq!(openai_delta(r#"{"error":{"message":"bad key"}}"#), Delta::Err("provider error: bad key".into()));
    }

    #[test]
    fn anthropic_deltas() {
        let d = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;
        assert_eq!(anthropic_delta(Some("content_block_delta"), d), Delta::Text("Hi".into()));
        assert_eq!(anthropic_delta(None, d), Delta::Text("Hi".into()));
        assert_eq!(anthropic_delta(Some("ping"), r#"{"type":"ping"}"#), Delta::Skip);
        assert_eq!(anthropic_delta(Some("message_start"), r#"{"type":"message_start"}"#), Delta::Skip);
        assert_eq!(anthropic_delta(Some("message_stop"), r#"{"type":"message_stop"}"#), Delta::Done);
        assert_eq!(
            anthropic_delta(Some("error"), r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#),
            Delta::Err("provider error: Overloaded".into())
        );
    }

    #[test]
    fn ollama_deltas() {
        assert_eq!(ollama_delta(r#"{"message":{"role":"assistant","content":"Yo"},"done":false}"#), Delta::Text("Yo".into()));
        assert_eq!(ollama_delta(r#"{"response":"x","done":false}"#), Delta::Text("x".into()));
        assert_eq!(ollama_delta(r#"{"message":{"content":""},"done":true}"#), Delta::Done);
        assert_eq!(ollama_delta(r#"{"error":"model not found"}"#), Delta::Err("provider error: model not found".into()));
        assert!(matches!(ollama_delta("<html>"), Delta::Err(_)));
    }

    fn opts(pairs: Vec<(&str, Value)>) -> SharedDict {
        SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn plan_for_each_wire_streams_and_keeps_secrets_off_argv() {
        let msgs = vec![("system".to_string(), "be brief".to_string()), ("user".to_string(), "SECRETPROMPT".to_string())];
        for (prov, path) in [("openai", "/chat/completions"), ("anthropic", "/messages"), ("ollama", "/api/chat")] {
            let o = opts(vec![("provider", Value::Str(prov.into())), ("key", Value::Str("SK-SECRET".into())), ("model", Value::Str("m".into()))]);
            let p = build_plan(&o, &msgs).unwrap();
            assert!(p.spec.url.ends_with(path), "{}", p.spec.url);
            let body = p.spec.body.clone().unwrap();
            assert!(body.contains("\"stream\":true"), "{}", body);
            let argv = stream_argv(&p.spec).join(" ");
            assert!(argv.contains("-N") && !argv.contains(" -i"), "{}", argv);
            for s in ["SK-SECRET", "SECRETPROMPT", p.spec.url.as_str()] {
                assert!(!argv.contains(s), "{} leaked: {}", s, argv);
            }
        }
        let a = build_plan(&opts(vec![("provider", Value::Str("anthropic".into())), ("key", Value::Str("k".into()))]), &msgs).unwrap();
        assert!(a.spec.body.unwrap().contains("\"system\":\"be brief\""));
        assert!(a.spec.headers.iter().any(|(k, _)| k == "x-api-key"));
    }

    #[test]
    fn plan_requires_a_key_or_base_url_and_validates_provider() {
        let m = vec![("user".to_string(), "x".to_string())];
        std::env::remove_var("GROQ_API_KEY");
        assert!(build_plan(&opts(vec![("provider", Value::Str("groq".into()))]), &m).is_err());
        assert!(build_plan(&opts(vec![("provider", Value::Str("nope".into()))]), &m).is_err());
        assert!(build_plan(&opts(vec![("provider", Value::Str("compat".into()))]), &m).is_err());
        assert!(build_plan(&opts(vec![("provider", Value::Str("compat".into())), ("base_url", Value::Str("http://127.0.0.1:1/v1".into()))]), &m).is_ok());
    }

    #[test]
    fn mock_source_streams_words_and_can_fail() {
        let mut s = StreamSource::mock(mock_words("a bc d"), 0, None);
        let mut got = Vec::new();
        while let Some(c) = s.next_chunk().unwrap() {
            got.push(c);
        }
        assert_eq!(got, vec!["a", " bc", " d"]);
        let mut f = StreamSource::mock(mock_words("a b c"), 0, Some(1));
        assert_eq!(f.next_chunk().unwrap().as_deref(), Some("a"));
        assert!(f.next_chunk().is_err());
    }

    #[test]
    fn messages_from_string_and_array() {
        assert_eq!(messages_of(&Value::Str("hi".into())).unwrap(), vec![("user".to_string(), "hi".to_string())]);
        assert!(messages_of(&Value::Number(1.0)).is_err());
        assert!(messages_of(&Value::Array(SharedArray::new(vec![]))).is_err());
    }
}
