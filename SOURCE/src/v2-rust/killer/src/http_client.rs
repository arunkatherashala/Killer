// ===============================================================================
// NOVA GALAXY ENGINE v1 — HTTP Client Builtins
// Native HTTP from Killer code — zero external crates, curl subprocess
//
// Builtins (every verb also accepts a headers dict and a timeout number after its
// required arguments, in either order):
//   http_get(url [, headers] [, secs])        → String  (response body)
//   http_get_timeout(url, secs)               → String
//   http_post(url, body [, content_type] [, headers] [, secs])
//   http_post_json(url, json_or_value [, headers] [, secs])
//   http_put / http_patch (url, body ...), http_delete(url ...)
//   http_head(url ...)                        → String  (response headers only)
//   http_status(url ...)                      → Number
//   http_download(url, path ...)              → String
//   http_request({method, url, headers, body, timeout, retries, insecure, allow_http})
//                                             → {status, headers, body}
//
// Security policy (see http_req.rs, where the logic and its unit tests live):
//   • HTTPS-only, except http:// to localhost/127.0.0.1/::1 (or `allow_http: true`)
//   • TLS verification ON (only `insecure: true` in http_request adds curl -k)
//   • URL, headers and body go to curl on stdin (-K -), never on the command line
//   • Timeouts enforced (default 30s, head/status 10s); retries with exponential backoff
// ===============================================================================

use crate::error::VmError;
use crate::http_req::{exec, headers_from, ReqSpec};
use crate::value::Value;

/// Pull an optional headers dict and timeout out of the trailing arguments.
fn extras(args: &[Value], from: usize, default_timeout: f64) -> (Vec<(String, String)>, f64) {
    let mut headers = Vec::new();
    let mut timeout = default_timeout;
    for a in args.iter().skip(from) {
        match a {
            Value::Dict(d) => headers = headers_from(d),
            Value::Number(n) => timeout = *n,
            Value::Integer(n) => timeout = *n as f64,
            _ => {}
        }
    }
    (headers, timeout)
}

fn spec(method: &str, url: &str, headers: Vec<(String, String)>, body: Option<String>, timeout: f64) -> ReqSpec {
    let mut s = ReqSpec::new(method, url);
    s.headers = headers;
    s.body = body;
    s.timeout_s = timeout;
    s
}

fn body_text(r: &crate::http_req::Resp) -> String {
    r.body.trim_end().to_string()
}

fn err_str(e: String) -> Value {
    Value::Str(format!("HTTP Error: {}", e))
}

fn url_arg(args: &[Value], usage: &str) -> Result<String, Value> {
    match args.first() {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(Value::Str(format!("Error: {}", usage))),
    }
}

fn ensure_content_type(h: &mut Vec<(String, String)>, ct: &str) {
    if !h.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
        h.push(("Content-Type".to_string(), ct.to_string()));
    }
}

/// http_get(url [, headers] [, secs]) → String
pub fn builtin_http_get(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_get requires a URL string. Usage: http_get(\"https://...\")") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 30.0);
    Ok(match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_post(url, body [, content_type] [, headers] [, secs]) → String
pub fn builtin_http_post(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_post(url, body) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let body = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    let ct = match args.get(2) {
        Some(Value::Str(s)) => s.clone(),
        _ => "application/x-www-form-urlencoded".to_string(),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, &ct);
    Ok(match exec(&spec("POST", &url, h, Some(body), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_post_json(url, json_string_or_value [, headers] [, secs]) → String
pub fn builtin_http_post_json(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_post_json(url, json) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let json = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        Some(v @ (Value::Dict(_) | Value::Array(_))) => crate::json_lang::stringify(v, 0).map_err(VmError::runtime_error)?,
        _ => return Ok(Value::Str("Error: http_post_json(url, json) requires a JSON body string".to_string())),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, "application/json");
    Ok(match exec(&spec("POST", &url, h, Some(json), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_head(url [, headers] [, secs]) → String  (returns response headers)
pub fn builtin_http_head(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_head requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 10.0);
    let mut s = spec("HEAD", &url, h, None, t);
    s.head = true;
    Ok(match exec(&s) {
        Ok(r) => Value::Str(r.raw_head.trim_end().to_string()),
        Err(e) => err_str(e),
    })
}

/// http_status(url [, headers] [, secs]) → Number  (e.g. 200, 404, 500)
pub fn builtin_http_status(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_status requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 10.0);
    Ok(match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Value::Number(r.status as f64),
        Err(e) => err_str(e),
    })
}

/// http_download(url, dest_path [, headers] [, secs]) → String  ("Downloaded to: ..." or error)
pub fn builtin_http_download(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    crate::security::require_file_write()?;
    let url = match url_arg(args, "http_download(url, path) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let path = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_download(url, path) requires a destination path".to_string())),
    };
    let lp = path.to_lowercase();
    if lp.contains("system32") || lp.contains("/etc/") || lp.contains("/bin/") {
        return Ok(err_str(format!("Download destination blocked: {}", path)));
    }
    let (h, t) = extras(args, 2, 120.0);
    let mut s = spec("GET", &url, h, None, t);
    s.output = Some(path.clone());
    Ok(match exec(&s) {
        Ok(_) => Value::Str(format!("Downloaded to: {}", path)),
        Err(e) => err_str(e),
    })
}

pub use crate::http_req::builtin_http_request;

fn with_body(who: &str, method: &str, args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let (url, body) = match args {
        [Value::Str(u), Value::Str(b), ..] => (u.clone(), b.clone()),
        [Value::Str(u), v @ (Value::Dict(_) | Value::Array(_)), ..] => {
            (u.clone(), crate::json_lang::stringify(v, 0).map_err(VmError::runtime_error)?)
        }
        _ => return Err(VmError::runtime_error(format!("{}(url, body) expects a string url and body", who))),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, "application/json");
    Ok(match exec(&spec(method, &url, h, Some(body), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_put(url, body [, headers] [, secs]) → String
pub fn builtin_http_put(args: &[Value]) -> Result<Value, VmError> { with_body("http_put", "PUT", args) }

/// http_patch(url, body [, headers] [, secs]) → String
pub fn builtin_http_patch(args: &[Value]) -> Result<Value, VmError> { with_body("http_patch", "PATCH", args) }

/// http_delete(url [, headers] [, secs]) → String
pub fn builtin_http_delete(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("http_delete(url) expects a URL string".to_string())),
    };
    let (h, t) = extras(args, 1, 30.0);
    Ok(match exec(&spec("DELETE", &url, h, None, t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_get_json(url [, headers] [, secs]) → Dict  (parses JSON response into a Killer dict/array)
pub fn builtin_http_get_json(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("http_get_json(url) expects a URL string".to_string())),
    };
    let (h, t) = extras(args, 1, 30.0);
    match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Ok(crate::json_lang::parse(&r.body).unwrap_or_else(|_| parse_json_to_value(&body_text(&r)))),
        Err(e) => Err(VmError::runtime_error(format!("http_get_json({url}): {e}"))),
    }
}

/// http_with_headers(url, headers_dict [, secs]) → String  (GET with custom headers)
pub fn builtin_http_with_headers(args: &[Value]) -> Result<Value, VmError> {
    if !matches!(args.first(), Some(Value::Str(_))) {
        return Err(VmError::runtime_error("http_with_headers(url, dict) expects a URL".to_string()));
    }
    builtin_http_get(args)
}

// ── Minimal JSON → Value parser ────────────────────────────────────────────

fn parse_json_to_value(s: &str) -> Value {
    let s = s.trim();
    if s.starts_with('{') {
        parse_json_object(s)
    } else if s.starts_with('[') {
        parse_json_array(s)
    } else if s.starts_with('"') && s.ends_with('"') {
        Value::Str(s[1..s.len()-1].replace("\\\"", "\"").replace("\\n", "\n").replace("\\t", "\t"))
    } else if s == "true" {
        Value::Bool(true)
    } else if s == "false" {
        Value::Bool(false)
    } else if s == "null" {
        Value::Null
    } else if let Ok(n) = s.parse::<f64>() {
        Value::Number(n)
    } else {
        Value::Str(s.to_string())
    }
}

fn parse_json_object(s: &str) -> Value {
    let inner = s.trim().trim_start_matches('{').trim_end_matches('}').trim();
    let mut dict = std::collections::HashMap::new();
    if inner.is_empty() { return Value::Dict(crate::value::SharedDict::new(dict)); }
    for pair in split_json_items(inner) {
        if let Some(colon) = pair.find(':') {
            let key_raw = pair[..colon].trim().trim_matches('"');
            let val_raw = pair[colon+1..].trim();
            dict.insert(key_raw.to_string(), parse_json_to_value(val_raw));
        }
    }
    Value::Dict(crate::value::SharedDict::new(dict))
}

fn parse_json_array(s: &str) -> Value {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() { return Value::Array(crate::value::SharedArray::new(vec![])); }
    let items: Vec<Value> = split_json_items(inner).into_iter()
        .map(|item| parse_json_to_value(item.trim()))
        .collect();
    Value::Array(crate::value::SharedArray::new(items))
}

fn split_json_items(s: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_str => { in_str = true; }
            b'"' if in_str && (i == 0 || bytes[i-1] != b'\\') => { in_str = false; }
            b'{' | b'[' if !in_str => depth += 1,
            b'}' | b']' if !in_str => depth -= 1,
            b',' if !in_str && depth == 0 => {
                items.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < s.len() { items.push(&s[start..]); }
    items
}

