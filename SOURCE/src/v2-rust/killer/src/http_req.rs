//! Hardened curl-backed HTTP client core, shared by every `http_*` builtin.
//!
//! Security properties (all unit-tested):
//! - secrets never reach the process command line: URL, headers and body are sent to curl as a
//!   config file on stdin (`curl -K -`), so API keys do not show up in `ps` or the task manager;
//! - TLS verification is on; `-k` is only added with `insecure: true`;
//! - `https://` is required, except `http://` to localhost / 127.0.0.1 / ::1 (or `allow_http: true`);
//! - header names/values and URLs containing control characters are rejected (no header injection);
//! - timeouts always apply; retries use exponential backoff on connection errors and 5xx.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::VmError;
use crate::value::{SharedDict, Value};

#[derive(Clone, Debug)]
pub struct ReqSpec {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_s: f64,
    pub retries: u32,
    pub insecure: bool,
    pub allow_http: bool,
    /// HEAD request (curl `--head`)
    pub head: bool,
    /// write the body to this file instead of returning it
    pub output: Option<String>,
}

impl ReqSpec {
    pub fn new(method: &str, url: &str) -> Self {
        ReqSpec {
            method: method.to_ascii_uppercase(),
            url: url.trim().to_string(),
            headers: Vec::new(),
            body: None,
            timeout_s: 30.0,
            retries: 0,
            insecure: false,
            allow_http: false,
            head: false,
            output: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub raw_head: String,
    pub body: String,
}

fn curl_bin() -> &'static str {
    if cfg!(target_os = "windows") { "curl.exe" } else { "curl" }
}

/// Host part of a URL (no userinfo, no port, no brackets), lowercase.
pub fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    let auth = &rest[..end];
    let auth = auth.rsplit('@').next()?; // text after the last '@' is what curl connects to
    let host = if let Some(stripped) = auth.strip_prefix('[') {
        stripped.split(']').next()?.to_string()
    } else {
        auth.split(':').next()?.to_string()
    };
    if host.is_empty() { None } else { Some(host.to_ascii_lowercase()) }
}

pub fn is_loopback_host(h: &str) -> bool {
    h == "localhost" || h == "127.0.0.1" || h == "::1"
}

/// Validate the URL against the scheme policy.
pub fn validate_url(url: &str, allow_http: bool) -> Result<(), String> {
    let u = url.trim();
    if u.chars().any(|c| c.is_control() || c == ' ') {
        return Err("URL contains spaces or control characters".to_string());
    }
    let shown = &u[..u.char_indices().nth(80).map(|(i, _)| i).unwrap_or(u.len())];
    if u.starts_with("https://") {
        return Ok(());
    }
    if u.starts_with("http://") {
        let host = url_host(u).unwrap_or_default();
        if allow_http || is_loopback_host(&host) {
            return Ok(());
        }
        return Err(format!(
            "Security: plain http:// is only allowed for localhost/127.0.0.1/::1 (pass allow_http: true to override).\nGot: {}",
            shown
        ));
    }
    Err(format!("Invalid URL: must start with https:// (or http:// for localhost)\nGot: {}", shown))
}

fn check_header(name: &str, value: &str) -> Result<(), String> {
    let bad = |s: &str| s.chars().any(|c| c == '\r' || c == '\n' || c == '\0');
    if name.is_empty() || bad(name) || bad(value) || name.contains(':') || name.contains(' ') {
        return Err(format!("invalid header '{}'", name));
    }
    Ok(())
}

/// Quote a string for a curl config file (`-K`): backslash escapes `\\ \" \n \r \t \v`.
pub fn cfg_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0b}' => out.push_str("\\v"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Command-line arguments for curl. Contains no URL, header or body: only fixed flags and
/// numeric limits, so nothing sensitive can leak through the process list.
pub fn build_argv(spec: &ReqSpec) -> Vec<String> {
    let mut a: Vec<String> = vec!["-s".into(), "-S".into(), "--ssl-no-revoke".into()];
    if spec.output.is_none() {
        a.push("-i".into());
    }
    a.push("--max-time".into());
    a.push(format!("{}", spec.timeout_s.max(0.1)));
    if spec.insecure {
        a.push("-k".into());
    }
    a.push("-K".into());
    a.push("-".into());
    a
}

/// The curl config text (written to curl's stdin): URL, method, headers and body.
pub fn build_config(spec: &ReqSpec) -> Result<String, String> {
    validate_url(&spec.url, spec.allow_http)?;
    let mut c = String::new();
    c.push_str(&format!("url = {}\n", cfg_quote(&spec.url)));
    if spec.head {
        c.push_str("head\n");
    } else if spec.method != "GET" || spec.body.is_some() {
        c.push_str(&format!("request = {}\n", cfg_quote(&spec.method)));
    }
    let mut has_ua = false;
    for (k, v) in &spec.headers {
        check_header(k, v)?;
        if k.eq_ignore_ascii_case("user-agent") {
            has_ua = true;
        }
        c.push_str(&format!("header = {}\n", cfg_quote(&format!("{}: {}", k, v))));
    }
    if !has_ua {
        c.push_str("header = \"User-Agent: KillerLang/2.1\"\n");
    }
    if let Some(b) = &spec.body {
        c.push_str(&format!("data-raw = {}\n", cfg_quote(b)));
    }
    if let Some(o) = &spec.output {
        c.push_str(&format!("output = {}\n", cfg_quote(o)));
    }
    Ok(c)
}

/// Delay before retry number `attempt` (0-based): 200 ms, 400 ms, 800 ms ... capped at 5 s.
pub fn backoff_ms(attempt: u32) -> u64 {
    (200u64 << attempt.min(10)).min(5000)
}

/// Split curl's `-i` output into (status, header lines, raw head, body), skipping 1xx blocks.
pub fn parse_response(raw: &[u8]) -> Result<Resp, String> {
    let mut rest = raw;
    loop {
        let sep = rest.windows(4).position(|w| w == b"\r\n\r\n");
        let (head_bytes, after) = match sep {
            Some(p) => (&rest[..p], &rest[p + 4..]),
            None => match rest.windows(2).position(|w| w == b"\n\n") {
                Some(p) => (&rest[..p], &rest[p + 2..]),
                None => return Err("malformed HTTP response".to_string()),
            },
        };
        let head = String::from_utf8_lossy(head_bytes).into_owned();
        let mut lines = head.lines();
        let status_line = lines.next().unwrap_or("");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("bad status line: {}", status_line))?;
        if (100..200).contains(&status) || (status == 200 && status_line.contains("Connection established")) {
            rest = after;
            continue;
        }
        let headers = lines
            .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())))
            .collect();
        return Ok(Resp { status, headers, raw_head: head, body: String::from_utf8_lossy(after).into_owned() });
    }
}

fn one_attempt(spec: &ReqSpec) -> Result<Resp, (String, bool)> {
    let cfg = build_config(spec).map_err(|e| (e, false))?;
    let mut child = Command::new(curl_bin())
        .args(build_argv(spec))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (format!("curl not found: {}. Install curl from https://curl.se", e), false))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(cfg.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| (format!("curl failed: {}", e), true))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // exit 6/7/28/35/52/55/56 are connection-level failures worth retrying
        let retry = matches!(out.status.code(), Some(6) | Some(7) | Some(28) | Some(35) | Some(52) | Some(55) | Some(56));
        return Err((format!("HTTP request failed: {}", msg), retry));
    }
    if spec.output.is_some() {
        return Ok(Resp { status: 200, headers: vec![], raw_head: String::new(), body: String::new() });
    }
    parse_response(&out.stdout).map_err(|e| (e, false))
}

/// Execute a request with retries. Retries happen on connection errors and 5xx responses.
pub fn exec(spec: &ReqSpec) -> Result<Resp, String> {
    validate_url(&spec.url, spec.allow_http)?;
    let mut attempt = 0u32;
    loop {
        let result = one_attempt(spec);
        let again = match &result {
            Ok(r) => r.status >= 500,
            Err((_, retryable)) => *retryable,
        };
        if again && attempt < spec.retries {
            std::thread::sleep(Duration::from_millis(backoff_ms(attempt)));
            attempt += 1;
            continue;
        }
        return result.map_err(|(e, _)| e);
    }
}

// --- Value helpers --------------------------------------------------------------

pub fn headers_from(d: &SharedDict) -> Vec<(String, String)> {
    d.iter()
        .map(|(k, v)| (k, match v { Value::Str(s) => s, other => format!("{}", other) }))
        .collect()
}

pub fn resp_to_value(r: &Resp) -> Value {
    let h = SharedDict::from_pairs(r.headers.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect());
    Value::Dict(SharedDict::from_pairs(vec![
        ("status".to_string(), Value::Number(r.status as f64)),
        ("headers".to_string(), Value::Dict(h)),
        ("body".to_string(), Value::Str(r.body.clone())),
    ]))
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

/// `http_request({method, url, headers, body, timeout, retries, insecure, allow_http})`
pub fn builtin_http_request(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let d = match args {
        [Value::Dict(d)] => d,
        _ => return Err(VmError::runtime_error("http_request(opts) expects one dict {method, url, headers, body, timeout, retries, insecure}".to_string())),
    };
    let url = match d.get("url") {
        Some(Value::Str(u)) => u,
        _ => return Err(VmError::runtime_error("http_request: opts.url must be a string".to_string())),
    };
    let method = match d.get("method") {
        Some(Value::Str(m)) => m,
        None | Some(Value::Null) => "GET".to_string(),
        _ => return Err(VmError::runtime_error("http_request: opts.method must be a string".to_string())),
    };
    if !method.bytes().all(|c| c.is_ascii_alphabetic()) || method.is_empty() {
        return Err(VmError::runtime_error("http_request: invalid method".to_string()));
    }
    let mut spec = ReqSpec::new(&method, &url);
    if let Some(Value::Dict(h)) = d.get("headers") {
        spec.headers = headers_from(&h);
    }
    spec.body = match d.get("body") {
        None | Some(Value::Null) => None,
        Some(Value::Str(s)) => Some(s),
        Some(v @ (Value::Dict(_) | Value::Array(_))) => {
            if !spec.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                spec.headers.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            Some(crate::json_lang::stringify(&v, 0).map_err(VmError::runtime_error)?)
        }
        Some(other) => Some(format!("{}", other)),
    };
    if let Some(t) = d.get("timeout").as_ref().and_then(num) {
        spec.timeout_s = t;
    }
    if let Some(r) = d.get("retries").as_ref().and_then(num) {
        spec.retries = r.clamp(0.0, 10.0) as u32;
    }
    spec.insecure = matches!(d.get("insecure"), Some(Value::Bool(true)));
    spec.allow_http = matches!(d.get("allow_http"), Some(Value::Bool(true)));
    match exec(&spec) {
        Ok(r) => Ok(resp_to_value(&r)),
        Err(e) => Err(VmError::runtime_error(format!("http_request: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret_spec() -> ReqSpec {
        let mut s = ReqSpec::new("POST", "https://api.example.com/v1/x?token=URLSECRET");
        s.headers.push(("Authorization".into(), "Bearer SK-TOPSECRET".into()));
        s.headers.push(("x-api-key".into(), "KEYSECRET".into()));
        s.body = Some("{\"prompt\":\"BODYSECRET\"}".into());
        s
    }

    #[test]
    fn no_secret_ever_appears_in_argv() {
        let argv = build_argv(&secret_spec()).join(" ");
        for secret in ["SK-TOPSECRET", "KEYSECRET", "BODYSECRET", "URLSECRET", "Authorization", "api.example.com"] {
            assert!(!argv.contains(secret), "{} leaked into argv: {}", secret, argv);
        }
        assert!(argv.contains("-K -"));
    }

    #[test]
    fn secrets_go_to_the_config_on_stdin() {
        let cfg = build_config(&secret_spec()).unwrap();
        assert!(cfg.contains("Bearer SK-TOPSECRET"));
        assert!(cfg.contains("request = \"POST\""));
        assert!(cfg.contains("data-raw = \"{\\\"prompt\\\":\\\"BODYSECRET\\\"}\""));
    }

    #[test]
    fn tls_verification_is_on_unless_insecure() {
        let mut s = secret_spec();
        assert!(!build_argv(&s).iter().any(|a| a == "-k"));
        s.insecure = true;
        assert!(build_argv(&s).iter().any(|a| a == "-k"));
    }

    #[test]
    fn scheme_policy() {
        assert!(validate_url("https://example.com", false).is_ok());
        assert!(validate_url("http://localhost:8080/x", false).is_ok());
        assert!(validate_url("http://127.0.0.1:1/", false).is_ok());
        assert!(validate_url("http://[::1]:80/", false).is_ok());
        assert!(validate_url("http://example.com", false).is_err());
        assert!(validate_url("http://example.com", true).is_ok());
        assert!(validate_url("http://localhost@evil.com/", false).is_err());
        assert!(validate_url("http://127.0.0.1.evil.com/", false).is_err());
        assert!(validate_url("ftp://x", true).is_err());
        assert!(validate_url("file:///etc/passwd", true).is_err());
        assert!(validate_url("https://a.com/\r\nX: y", false).is_err());
    }

    #[test]
    fn header_injection_is_rejected() {
        let mut s = ReqSpec::new("GET", "https://a.com");
        s.headers.push(("X-A".into(), "v\r\nEvil: 1".into()));
        assert!(build_config(&s).is_err());
        let mut s = ReqSpec::new("GET", "https://a.com");
        s.headers.push(("Bad Name".into(), "v".into()));
        assert!(build_config(&s).is_err());
    }

    #[test]
    fn config_quoting_escapes_specials() {
        assert_eq!(cfg_quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(backoff_ms(0), 200);
        assert_eq!(backoff_ms(1), 400);
        assert_eq!(backoff_ms(2), 800);
        assert_eq!(backoff_ms(10), 5000);
        assert_eq!(backoff_ms(99), 5000);
    }

    #[test]
    fn parses_responses_skipping_100_continue() {
        let raw = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nX-A: b\r\n\r\nhello";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 201);
        assert_eq!(r.body, "hello");
        assert_eq!(r.headers[0], ("content-type".to_string(), "text/plain".to_string()));
    }

    #[test]
    fn host_extraction() {
        assert_eq!(url_host("https://User:pw@Example.com:8443/p?q=1").as_deref(), Some("example.com"));
        assert_eq!(url_host("http://[::1]:9/").as_deref(), Some("::1"));
    }
}
