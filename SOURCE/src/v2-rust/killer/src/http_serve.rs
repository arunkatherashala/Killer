//! Real HTTP server with Killer handlers.
//!
//! `serve(port, handler[, opts])` binds a TCP listener and runs the accept loop on the VM thread.
//! Each request is turned into a dict `{method, path, query, headers, body, params}` and handed
//! to the Killer function value through `VirtualMachine::call_value_nested`, so no Killer value
//! ever crosses a thread. The handler returns a string (200 text), a dict with `status` (and
//! optional `headers`/`body`), or any other dict/array (sent as JSON).
//!
//! Builtins: `serve`, `serve_once`, `route`, `routes_clear`, `server_stop`, `free_port`.
//! `serve` and `serve_once` are VM-level (see `vm_call`, hooked in the CallBuiltin handler);
//! the others are plain builtins reached through `call`.
//!
//! Options (dict, third argument of `serve`): `host` (default "127.0.0.1"; use "0.0.0.0" to
//! expose the server), `max_requests` (stop after N requests), `idle_timeout` (seconds without a
//! connection before stopping). A handler can call `server_stop()` to stop gracefully after the
//! current response has been written. A handler may return `{stream: generator_or_array}` to
//! send Server-Sent Events (see `http_sse`); `stream_timeout` (seconds, default 300) bounds a stream.
//!
//! **One request at a time.** The accept loop and the handlers share the VM thread, so requests are
//! handled strictly one after another. To keep one bad client or handler from wedging the process:
//! `read_timeout` (seconds, default 10) is the *total* time a client gets to send its request, an
//! idle or dribbling connection gets `408` and the next client is served; `request_timeout`
//! (seconds, default 60, 0 = off) is a wall-clock deadline for one handler call, checked on loop
//! back-edges (a handler stuck in `while true` gets a `503` and the server carries on; a single
//! blocking builtin such as `sleep_ms(1e6)` cannot be interrupted); `server_stop()` is process-wide,
//! so it also works from a task on another thread (e.g. a watchdog started with `async_spawn`), and
//! it interrupts reads and streams within about 200 ms.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::bytecode::Program;
use crate::error::VmError;
use crate::value::{SharedDict, Value};
use crate::vm::VirtualMachine;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

struct Route {
    method: String,
    segments: Vec<String>,
    handler: Value,
}

thread_local! {
    static ROUTES: RefCell<Vec<Route>> = RefCell::new(Vec::new());
}

/// Set by `server_stop()`; process-wide so a task on another thread can stop the server too.
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True once `server_stop()` has been called (streams and reads poll this to wind down).
pub fn stop_requested() -> bool {
    STOP.load(std::sync::atomic::Ordering::SeqCst)
}

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

// --- parsing ------------------------------------------------------------------

/// A parsed HTTP request (plain Rust, convertible to a Killer dict).
#[derive(Debug, Clone, PartialEq)]
pub struct Req {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub fn percent_decode(s: &str, plus_is_space: bool) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match h {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parse the head (request line + headers) of a request. Returns the request without body.
pub fn parse_head(head: &str) -> Result<Req, String> {
    let mut lines = head.split("\r\n");
    let first = lines.next().ok_or("empty request")?;
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() != 3 || !parts[2].starts_with("HTTP/") {
        return Err(format!("malformed request line: {}", first));
    }
    let method = parts[0].to_ascii_uppercase();
    if method.is_empty() || !method.bytes().all(|c| c.is_ascii_alphabetic()) {
        return Err("invalid method".to_string());
    }
    let target = parts[1];
    if !target.starts_with('/') {
        return Err("request target must start with /".to_string());
    }
    let (raw_path, raw_query) = match target.find('?') {
        Some(i) => (&target[..i], &target[i + 1..]),
        None => (target, ""),
    };
    let mut query = Vec::new();
    for pair in raw_query.split('&').filter(|p| !p.is_empty()) {
        match pair.find('=') {
            Some(i) => query.push((percent_decode(&pair[..i], true), percent_decode(&pair[i + 1..], true))),
            None => query.push((percent_decode(pair, true), String::new())),
        }
    }
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(i) = line.find(':') {
            headers.push((line[..i].trim().to_ascii_lowercase(), line[i + 1..].trim().to_string()));
        }
    }
    Ok(Req { method, path: percent_decode(raw_path, false), query, headers, body: String::new() })
}

/// One read bounded by an overall `deadline` (not per read, so a client dribbling bytes cannot
/// hold the single-threaded server) and interruptible by `server_stop()`: waits in 200 ms slices.
fn read_some(stream: &mut TcpStream, buf: &mut [u8], deadline: Instant) -> Result<usize, String> {
    loop {
        if stop_requested() {
            return Err("server stopping".to_string());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err("timeout".to_string());
        }
        let _ = stream.set_read_timeout(Some((deadline - now).min(Duration::from_millis(200))));
        match stream.read(buf) {
            Ok(n) => return Ok(n),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => continue,
            Err(e) => return Err(format!("read: {}", e)),
        }
    }
}

fn read_request(stream: &mut TcpStream, deadline: Instant) -> Result<Req, String> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(p) = find_seq(&buf, b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err("headers too large".to_string());
        }
        let n = read_some(stream, &mut chunk, deadline)?;
        if n == 0 {
            return Err("connection closed".to_string());
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut req = parse_head(&head)?;
    let len = req
        .headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len > MAX_BODY_BYTES {
        return Err("body too large".to_string());
    }
    let mut body: Vec<u8> = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = read_some(stream, &mut chunk, deadline)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    req.body = String::from_utf8_lossy(&body).into_owned();
    Ok(req)
}

fn find_seq(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// --- routing ------------------------------------------------------------------

fn split_path(p: &str) -> Vec<String> {
    p.split('/').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
}

/// Match `pattern` segments (with `:name` params and a trailing `*`) against a path.
pub fn match_path(pattern: &[String], path: &[String]) -> Option<Vec<(String, String)>> {
    let mut params = Vec::new();
    for (i, seg) in pattern.iter().enumerate() {
        if seg == "*" && i == pattern.len() - 1 {
            params.push(("*".to_string(), path[i.min(path.len())..].join("/")));
            return Some(params);
        }
        let actual = path.get(i)?;
        if let Some(name) = seg.strip_prefix(':') {
            params.push((name.to_string(), actual.clone()));
        } else if seg != actual {
            return None;
        }
    }
    if pattern.len() == path.len() { Some(params) } else { None }
}

fn find_route(method: &str, path: &str) -> Option<(Value, Vec<(String, String)>)> {
    let segs = split_path(path);
    ROUTES.with(|r| {
        for route in r.borrow().iter() {
            if route.method != "*" && route.method != method {
                continue;
            }
            if let Some(p) = match_path(&route.segments, &segs) {
                return Some((route.handler.clone(), p));
            }
        }
        None
    })
}

// --- responses ----------------------------------------------------------------

pub fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "OK",
    }
}

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn text_resp(status: u16, ctype: &str, body: String) -> Resp {
    Resp { status, headers: vec![("Content-Type".to_string(), ctype.to_string())], body }
}

fn display(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        other => format!("{}", other),
    }
}

fn to_json(v: &Value) -> Result<String, VmError> {
    crate::json_lang::stringify(v, 0).map_err(VmError::runtime_error)
}

fn value_to_resp(v: &Value) -> Result<Resp, VmError> {
    match v {
        Value::Str(s) => Ok(text_resp(200, "text/plain; charset=utf-8", s.clone())),
        Value::Null => Ok(Resp { status: 204, headers: vec![], body: String::new() }),
        Value::Dict(d) => {
            let status = match d.get("status") {
                Some(Value::Number(n)) if (100.0..600.0).contains(&n) => Some(n as u16),
                Some(Value::Integer(n)) if (100..600).contains(&n) => Some(n as u16),
                _ => None,
            };
            let is_response = status.is_some() && d.keys().iter().all(|k| k == "status" || k == "headers" || k == "body");
            if !is_response {
                return Ok(text_resp(200, "application/json", to_json(v)?));
            }
            let mut headers: Vec<(String, String)> = Vec::new();
            if let Some(Value::Dict(h)) = d.get("headers") {
                for (k, val) in h.iter() {
                    headers.push((k, display(&val)));
                }
            }
            let has_ct = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
            let body = match d.get("body") {
                None | Some(Value::Null) => String::new(),
                Some(Value::Str(s)) => {
                    if !has_ct {
                        headers.push(("Content-Type".to_string(), "text/plain; charset=utf-8".to_string()));
                    }
                    s
                }
                Some(other @ (Value::Dict(_) | Value::Array(_))) => {
                    if !has_ct {
                        headers.push(("Content-Type".to_string(), "application/json".to_string()));
                    }
                    to_json(&other)?
                }
                Some(other) => display(&other),
            };
            Ok(Resp { status: status.unwrap_or(200), headers, body })
        }
        Value::Array(_) => Ok(text_resp(200, "application/json", to_json(v)?)),
        other => Ok(text_resp(200, "text/plain; charset=utf-8", display(other))),
    }
}

fn write_resp(stream: &mut TcpStream, head_only: bool, r: &Resp) {
    let mut out = format!("HTTP/1.1 {} {}\r\n", r.status, status_text(r.status));
    for (k, v) in &r.headers {
        // never let a handler inject extra header lines
        let k: String = k.chars().filter(|c| *c != '\r' && *c != '\n' && *c != ':').collect();
        let v: String = v.chars().filter(|c| *c != '\r' && *c != '\n').collect();
        out.push_str(&format!("{}: {}\r\n", k, v));
    }
    out.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", r.body.len()));
    let mut bytes = out.into_bytes();
    if !head_only {
        bytes.extend_from_slice(r.body.as_bytes());
    }
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
}

// --- request -> Killer value ---------------------------------------------------

fn pairs_to_dict(p: &[(String, String)]) -> Value {
    Value::Dict(SharedDict::from_pairs(p.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect()))
}

fn req_to_value(r: &Req, params: &[(String, String)]) -> Value {
    Value::Dict(SharedDict::from_pairs(vec![
        ("method".to_string(), Value::Str(r.method.clone())),
        ("path".to_string(), Value::Str(r.path.clone())),
        ("query".to_string(), pairs_to_dict(&r.query)),
        ("headers".to_string(), pairs_to_dict(&r.headers)),
        ("body".to_string(), Value::Str(r.body.clone())),
        ("params".to_string(), pairs_to_dict(params)),
    ]))
}

// --- the accept loop ----------------------------------------------------------

fn opt_num(opts: Option<&SharedDict>, key: &str) -> Option<f64> {
    match opts?.get(key)? {
        Value::Number(n) => Some(n),
        Value::Integer(i) => Some(i as f64),
        _ => None,
    }
}

fn serve_loop(vm: &mut VirtualMachine, args: &[Value], program: &Program, once: bool) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let who = if once { "serve_once" } else { "serve" };
    let port = match args.first() {
        Some(Value::Number(n)) if *n >= 0.0 && *n <= 65535.0 => *n as u16,
        Some(Value::Integer(n)) if (0..=65535).contains(n) => *n as u16,
        _ => return err(format!("{}(port, handler[, opts]) expects a port number", who)),
    };
    let handler = match args.get(1) {
        None | Some(Value::Null) => None,
        Some(f @ Value::Function { .. }) => Some(f.clone()),
        Some(other) => return err(format!("{}() handler must be a function, got {}", who, other.type_name())),
    };
    let opts = match args.get(2) {
        Some(Value::Dict(d)) => Some(d.clone()),
        None | Some(Value::Null) => None,
        Some(_) => return err(format!("{}() options must be a dict", who)),
    };
    let host = match opts.as_ref().and_then(|o| o.get("host")) {
        Some(Value::Str(h)) => h,
        _ => "127.0.0.1".to_string(),
    };
    let max_requests = if once { Some(1) } else { opt_num(opts.as_ref(), "max_requests").map(|n| n.max(0.0) as u64) };
    let idle = opt_num(opts.as_ref(), "idle_timeout").map(Duration::from_secs_f64);
    // overall time a client gets to send its request (head and body); 0 or less is not allowed
    let read_limit = Duration::from_secs_f64(opt_num(opts.as_ref(), "read_timeout").unwrap_or(10.0).clamp(0.05, 3600.0));
    // wall-clock deadline for one handler call (checked on loop back-edges); 0 turns it off
    let request_ms = (opt_num(opts.as_ref(), "request_timeout").unwrap_or(60.0).clamp(0.0, 86400.0) * 1000.0) as u64;
    let stream_limit = Duration::from_secs_f64(opt_num(opts.as_ref(), "stream_timeout").unwrap_or(300.0).clamp(0.1, 86400.0));
    if handler.is_none() && ROUTES.with(|r| r.borrow().is_empty()) {
        return err(format!("{}(): pass a handler function or register routes with route() first", who));
    }
    let listener = TcpListener::bind((host.as_str(), port)).map_err(|e| VmError::runtime_error(format!("{}: cannot bind {}:{}: {}", who, host, port, e)))?;
    listener.set_nonblocking(true).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))?;
    STOP.store(false, std::sync::atomic::Ordering::SeqCst);

    let mut served: u64 = 0;
    let mut last_activity = Instant::now();
    loop {
        if stop_requested() || max_requests.map_or(false, |m| served >= m) {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                last_activity = Instant::now();
                let req = match read_request(&mut stream, Instant::now() + read_limit) {
                    Ok(r) => r,
                    Err(e) => {
                        if e == "timeout" {
                            write_resp(&mut stream, false, &text_resp(408, "text/plain; charset=utf-8", "Request Timeout".to_string()));
                        } else if e != "connection closed" && e != "server stopping" {
                            write_resp(&mut stream, false, &text_resp(400, "text/plain; charset=utf-8", format!("Bad Request: {}", e)));
                        }
                        continue;
                    }
                };
                served += 1;
                let (target, params) = match find_route(&req.method, &req.path) {
                    Some((h, p)) => (Some(h), p),
                    None => (handler.clone(), Vec::new()),
                };
                let resp = match target {
                    None => text_resp(404, "text/plain; charset=utf-8", format!("Not Found: {} {}", req.method, req.path)),
                    Some(f) => match vm.call_value_nested_with_deadline(&f, vec![req_to_value(&req, &params)], program, request_ms) {
                        Ok(v) if crate::http_sse::stream_reply(&v).is_some() => {
                            let reply = crate::http_sse::stream_reply(&v).unwrap();
                            crate::http_sse::write_stream(&mut stream, vm, program, &reply, stream_limit);
                            continue;
                        }
                        Ok(v) => value_to_resp(&v).unwrap_or_else(|e| text_resp(500, "text/plain; charset=utf-8", format!("Internal Server Error: {}", e))),
                        Err(VmError::SecurityError { ref message, .. }) if message.contains("time limit") => text_resp(503, "text/plain; charset=utf-8", format!("Service Unavailable: the handler exceeded the {} s request deadline", request_ms as f64 / 1000.0)),
                        Err(e) => text_resp(500, "text/plain; charset=utf-8", format!("Internal Server Error: {}", e)),
                    },
                };
                write_resp(&mut stream, req.method == "HEAD", &resp);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(d) = idle {
                    if last_activity.elapsed() >= d {
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => return err(format!("{}: accept failed: {}", who, e)),
        }
    }
    Ok(Value::Number(served as f64))
}

/// VM-level entry: handles `serve` and `serve_once` (they need to call back into bytecode).
pub fn vm_call(vm: &mut VirtualMachine, name: &str, args: &[Value], program: &Program) -> Option<Result<Value, VmError>> {
    match name {
        "serve" => Some(serve_loop(vm, args, program, false)),
        "serve_once" => Some(serve_loop(vm, args, program, true)),
        _ => None,
    }
}

/// Plain builtins: `route`, `routes_clear`, `server_stop`, `free_port`.
pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    match name {
        "route" => Some((|| {
            match args {
                [Value::Str(m), Value::Str(p), f @ Value::Function { .. }] => {
                    if !p.starts_with('/') {
                        return err("route(): path must start with /");
                    }
                    ROUTES.with(|r| {
                        r.borrow_mut().push(Route { method: m.to_ascii_uppercase(), segments: split_path(p), handler: f.clone() })
                    });
                    Ok(Value::Null)
                }
                _ => err("route(method, path, fn) expects two strings and a function"),
            }
        })()),
        "routes_clear" => Some({
            ROUTES.with(|r| r.borrow_mut().clear());
            Ok(Value::Null)
        }),
        "server_stop" => Some({
            STOP.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(Value::Null)
        }),
        "free_port" => Some(
            TcpListener::bind(("127.0.0.1", 0))
                .and_then(|l| l.local_addr())
                .map(|a| Value::Number(a.port() as f64))
                .map_err(|e| VmError::runtime_error(format!("free_port: {}", e))),
        ),
        _ => None,
    }
}

pub const NAMES: &[&str] = &["serve", "serve_once", "route", "routes_clear", "server_stop", "free_port"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_head_with_query_and_headers() {
        let r = parse_head("GET /a%20b/c?x=1&y=hello+world&z HTTP/1.1\r\nHost: h\r\nX-Key: V").unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/a b/c");
        assert_eq!(r.query, vec![("x".into(), "1".into()), ("y".into(), "hello world".into()), ("z".into(), "".into())]);
        assert_eq!(r.headers[1], ("x-key".to_string(), "V".to_string()));
    }

    #[test]
    fn rejects_bad_request_lines() {
        assert!(parse_head("garbage").is_err());
        assert!(parse_head("GET nopath HTTP/1.1").is_err());
    }

    #[test]
    fn route_matching_with_params_and_wildcard() {
        let pat = split_path("/users/:id/posts");
        assert_eq!(match_path(&pat, &split_path("/users/42/posts")), Some(vec![("id".into(), "42".into())]));
        assert_eq!(match_path(&pat, &split_path("/users/42")), None);
        assert_eq!(match_path(&pat, &split_path("/users/42/posts/9")), None);
        let wild = split_path("/static/*");
        assert_eq!(match_path(&wild, &split_path("/static/a/b.css")), Some(vec![("*".into(), "a/b.css".into())]));
        assert_eq!(match_path(&split_path("/"), &split_path("/")), Some(vec![]));
    }

    #[test]
    fn percent_decode_edges() {
        assert_eq!(percent_decode("100%", false), "100%");
        assert_eq!(percent_decode("%zz", false), "%zz");
        assert_eq!(percent_decode("a%2Fb", false), "a/b");
        assert_eq!(percent_decode("a+b", false), "a+b");
    }
}
