//! Real HTTP server with Killer handlers.
//!
//! `serve(port, handler[, opts])` binds a TCP listener and runs the accept loop on the VM thread.
//! Each request is turned into a dict `{method, path, query, headers, body, params}` and handed
//! to the Killer function value through `VirtualMachine::call_value_nested` on a worker's own VM;
//! values only cross threads as detached copies. The handler returns a string (200 text), a dict
//! with `status` (and optional `headers`/`body`), or any other dict/array (sent as JSON).
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
//! **Worker pool.** The accept loop runs on the calling VM thread and hands each connection to a
//! bounded queue (`queue`, default 64) served by a fixed pool of worker threads (`workers`, default
//! min(CPUs, 8)). When the queue is full the connection gets `503` at once (from a small rejecter
//! thread, so the accept loop never waits on a client). Every worker owns one VM built from a
//! private detached copy of the program state taken when `serve` starts ([`WorkerSeed`]): handler
//! values, routes and top-level variables are deep-copied per worker, so no `Rc` crosses a thread
//! and a handler's changes to globals stay inside its worker (share state with `mutex_*`, channels or
//! a database). `read_timeout` (seconds, default 10) is the *total* time a client gets to send its
//! request (`408` after it); `request_timeout` (seconds, default 60, 0 = off) is a wall-clock deadline
//! for one handler call, checked on loop back-edges (`503`; a single blocking builtin such as
//! `sleep_ms(1e6)` cannot be interrupted). `max_requests` counts requests that reached routing,
//! across all workers: exactly that many are admitted, any later one gets `503`, and `serve` returns
//! once the admitted ones are answered. `server_stop()` is process-wide (works from a handler or any
//! task); it stops accepting at once, aborts reads and streams within about 200 ms, answers queued
//! connections `503`, and waits up to `drain_timeout` (seconds, default 30) for handlers still running.
//! `serve` returns the number of completed requests.
//!
//! [`WorkerSeed`]: crate::vm::WorkerSeed

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
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

// Safety: the handler values in `PUBLISHED` are private detached copies that are only touched while
// the mutex is held (stored under the lock, copied out under the lock), so no `Rc` is ever used by
// two threads at once.
#[allow(unsafe_code)]
unsafe impl Send for Route {}

thread_local! {
    /// The routes this thread's server uses; handlers are shared with the registering code, so a
    /// handler that updates a captured list or dict keeps its state between requests.
    static ROUTES: std::cell::RefCell<Vec<Route>> = std::cell::RefCell::new(Vec::new());
}

/// Detached copies of every route registered by any thread. A server started in an `async_spawn`
/// task has no routes of its own; it adopts these (a task works on copies of its data) instead of
/// failing with "register routes first" while its client waits.
static PUBLISHED: std::sync::Mutex<Vec<Route>> = std::sync::Mutex::new(Vec::new());

fn published() -> std::sync::MutexGuard<'static, Vec<Route>> {
    PUBLISHED.lock().unwrap_or_else(|p| p.into_inner())
}

/// On a thread that registered no routes, take over the published ones.
fn adopt_published_routes() {
    ROUTES.with(|r| {
        let mut local = r.borrow_mut();
        if local.is_empty() {
            for route in published().iter() {
                local.push(Route { method: route.method.clone(), segments: route.segments.clone(), handler: route.handler.detached() });
            }
        }
    });
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
/// hold a worker) and interruptible by `server_stop()` or by the server closing (`closing`): waits
/// in 200 ms slices.
fn read_some(stream: &mut TcpStream, buf: &mut [u8], deadline: Instant, closing: &AtomicBool) -> Result<usize, String> {
    loop {
        if stop_requested() || closing.load(Ordering::SeqCst) {
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

fn read_request(stream: &mut TcpStream, deadline: Instant, closing: &AtomicBool) -> Result<Req, String> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(p) = find_seq(&buf, b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err("headers too large".to_string());
        }
        let n = read_some(stream, &mut chunk, deadline, closing)?;
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
        let n = read_some(stream, &mut chunk, deadline, closing)?;
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

fn find_route(routes: &[Route], method: &str, path: &str) -> Option<(Value, Vec<(String, String)>)> {
    let segs = split_path(path);
    for route in routes {
        if route.method != "*" && route.method != method {
            continue;
        }
        if let Some(p) = match_path(&route.segments, &segs) {
            return Some((route.handler.clone(), p));
        }
    }
    None
}

/// A private copy of this thread's routes (handlers detached) for one worker.
fn routes_for_worker() -> Vec<Route> {
    ROUTES.with(|r| {
        r.borrow()
            .iter()
            .map(|route| Route { method: route.method.clone(), segments: route.segments.clone(), handler: route.handler.detached() })
            .collect()
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

/// A handler failure: the real error goes to the log (stderr, level error) with the request line;
/// the client only sees a generic JSON body unless `opts.debug` is true.
fn internal_error(req: &Req, e: &VmError, debug: bool) -> Resp {
    let fields = SharedDict::from_pairs(vec![
        ("method".to_string(), Value::Str(req.method.clone())),
        ("path".to_string(), Value::Str(req.path.clone())),
        ("error".to_string(), Value::Str(format!("{}", e))),
    ]);
    let _ = crate::log_builtins::call("log_error", &[Value::Str("handler error".to_string()), Value::Dict(fields)]);
    let body = if debug {
        let d = SharedDict::from_pairs(vec![
            ("error".to_string(), Value::Str("internal server error".to_string())),
            ("message".to_string(), Value::Str(format!("{}", e))),
        ]);
        crate::json_lang::stringify(&Value::Dict(d), 0).unwrap_or_else(|_| "{\"error\":\"internal server error\"}".to_string())
    } else {
        "{\"error\":\"internal server error\"}".to_string()
    };
    text_resp(500, "application/json", body)
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

/// Most workers a server may start (each one is an OS thread with its own VM).
const MAX_WORKERS: usize = 256;
/// Largest accepted `queue` option.
const MAX_QUEUE: usize = 65536;
/// Connections waiting for a polite `503` from the rejecter thread; beyond this they are closed.
const REJECT_BACKLOG: usize = 64;

/// State shared by the accept loop and the workers of one `serve` call.
struct Pool {
    /// Set once the server stops accepting (stop, `max_requests` reached, idle, error): reads in
    /// progress abort and queued connections are answered `503`.
    closing: AtomicBool,
    /// Requests that reached routing (at most `max_requests` are let through).
    admitted: AtomicU64,
    /// Requests answered; the value `serve` returns.
    completed: AtomicU64,
    /// Connections queued or being handled by a worker.
    pending: AtomicUsize,
    /// Milliseconds since `born` at which the last request was answered (for `idle_timeout`).
    last_done_ms: AtomicU64,
    born: Instant,
    max_requests: Option<u64>,
}

/// Per-request limits taken from the options.
#[derive(Clone, Copy)]
struct Limits {
    read_limit: Duration,
    request_ms: u64,
    debug: bool,
    stream_limit: Duration,
}

/// Decrements the pending count when a worker is done with a connection (also on unwind).
struct PendingGuard(Arc<Pool>);
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::SeqCst);
    }
}

fn unavailable(why: &str) -> Resp {
    let mut r = text_resp(503, "text/plain; charset=utf-8", format!("Service Unavailable: {}", why));
    r.headers.push(("Retry-After".to_string(), "1".to_string()));
    r
}

/// Answer a connection we will not serve. The request is read first (briefly) and the socket is
/// half-closed and drained afterwards: closing a socket with unread data sends a reset, and the
/// client would see "connection reset" instead of the reply.
fn refuse(mut stream: TcpStream, r: &Resp) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let until = Instant::now() + Duration::from_millis(200);
    while find_seq(&buf, b"\r\n\r\n").is_none() && buf.len() < MAX_HEADER_BYTES && Instant::now() < until {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    write_resp(&mut stream, false, r);
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let until = Instant::now() + Duration::from_millis(200);
    while Instant::now() < until {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => continue,
            _ => break,
        }
    }
}

/// One connection on a worker: read the request, admit it against `max_requests`, call the
/// handler on this worker's VM and write the reply.
fn handle_conn(vm: &mut VirtualMachine, program: &Program, routes: &[Route], handler: &Option<Value>, lim: Limits, pool: &Pool, mut stream: TcpStream) {
    if stop_requested() || pool.closing.load(Ordering::SeqCst) {
        refuse(stream, &unavailable("the server is stopping"));
        return;
    }
    let req = match read_request(&mut stream, Instant::now() + lim.read_limit, &pool.closing) {
        Ok(r) => r,
        Err(e) => {
            if e == "timeout" {
                write_resp(&mut stream, false, &text_resp(408, "text/plain; charset=utf-8", "Request Timeout".to_string()));
            } else if e != "connection closed" && e != "server stopping" {
                write_resp(&mut stream, false, &text_resp(400, "text/plain; charset=utf-8", format!("Bad Request: {}", e)));
            }
            return;
        }
    };
    let n = pool.admitted.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(m) = pool.max_requests {
        if n > m {
            write_resp(&mut stream, req.method == "HEAD", &unavailable("the server is finishing its last requests"));
            return;
        }
        if n == m {
            // the last admitted request: stop accepting, abort other reads, answer the queue 503
            pool.closing.store(true, Ordering::SeqCst);
        }
    }
    let (target, params) = match find_route(routes, &req.method, &req.path) {
        Some((h, p)) => (Some(h), p),
        None => (handler.clone(), Vec::new()),
    };
    let resp = match target {
        None => Some(text_resp(404, "text/plain; charset=utf-8", format!("Not Found: {} {}", req.method, req.path))),
        Some(f) => match vm.call_value_nested_with_deadline(&f, vec![req_to_value(&req, &params)], program, lim.request_ms) {
            Ok(v) => match crate::http_sse::stream_reply(&v) {
                Some(reply) => {
                    crate::http_sse::write_stream(&mut stream, vm, program, &reply, lim.stream_limit);
                    None
                }
                None => Some(value_to_resp(&v).unwrap_or_else(|e| internal_error(&req, &e, lim.debug))),
            },
            Err(VmError::SecurityError { ref message, .. }) if message.contains("time limit") => Some(text_resp(
                503,
                "text/plain; charset=utf-8",
                format!("Service Unavailable: the handler exceeded the {} s request deadline", lim.request_ms as f64 / 1000.0),
            )),
            Err(e) => Some(internal_error(&req, &e, lim.debug)),
        },
    };
    if let Some(resp) = resp {
        write_resp(&mut stream, req.method == "HEAD", &resp);
    }
    pool.completed.fetch_add(1, Ordering::SeqCst);
    pool.last_done_ms.store(pool.born.elapsed().as_millis() as u64, Ordering::SeqCst);
}

/// A worker thread: take connections from the shared queue until the accept loop drops its end.
fn worker_loop(vm: &mut VirtualMachine, program: &Program, routes: &[Route], handler: &Option<Value>, lim: Limits, pool: &Arc<Pool>, rx: &Mutex<Receiver<TcpStream>>) {
    loop {
        let next = rx.lock().unwrap_or_else(|p| p.into_inner()).recv();
        let Ok(stream) = next else { break };
        let _done = PendingGuard(Arc::clone(pool));
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
        handle_conn(vm, program, routes, handler, lim, pool, stream);
    }
}

/// A whole-number option in `lo..=hi`, or the default when absent.
fn opt_count(opts: Option<&SharedDict>, key: &str, default: usize, lo: usize, hi: usize, who: &str) -> Result<usize, VmError> {
    match opts.and_then(|o| o.get(key)) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Integer(i)) if i >= lo as i64 && i <= hi as i64 => Ok(i as usize),
        Some(Value::Number(n)) if n.fract() == 0.0 && n >= lo as f64 && n <= hi as f64 => Ok(n as usize),
        Some(_) => err(format!("{}(): {} must be a whole number from {} to {}", who, key, lo, hi)),
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
    let idle = opt_num(opts.as_ref(), "idle_timeout").map(|s| Duration::from_secs_f64(s.max(0.0)));
    let default_workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
    let workers = if once { 1 } else { opt_count(opts.as_ref(), "workers", default_workers, 1, MAX_WORKERS, who)? };
    let queue = opt_count(opts.as_ref(), "queue", 64, 0, MAX_QUEUE, who)?;
    // how long server_stop() waits for handlers still running before serve returns anyway
    let drain = Duration::from_secs_f64(opt_num(opts.as_ref(), "drain_timeout").unwrap_or(30.0).clamp(0.0, 86400.0));
    let lim = Limits {
        // overall time a client gets to send its request (head and body); 0 or less is not allowed
        read_limit: Duration::from_secs_f64(opt_num(opts.as_ref(), "read_timeout").unwrap_or(10.0).clamp(0.05, 3600.0)),
        // wall-clock deadline for one handler call (checked on loop back-edges); 0 turns it off
        request_ms: (opt_num(opts.as_ref(), "request_timeout").unwrap_or(60.0).clamp(0.0, 86400.0) * 1000.0) as u64,
        debug: matches!(opts.as_ref().and_then(|o| o.get("debug")), Some(Value::Bool(true))),
        stream_limit: Duration::from_secs_f64(opt_num(opts.as_ref(), "stream_timeout").unwrap_or(300.0).clamp(0.1, 86400.0)),
    };
    adopt_published_routes();
    if handler.is_none() && ROUTES.with(|r| r.borrow().is_empty()) {
        return err(format!("{}(): pass a handler function or register routes with route() first", who));
    }
    // `tls: {cert, key}` serves HTTPS: a TLS front (openssl) listens on host:port and forwards plain text to a private
    // loopback listener, which the loop below serves as usual.
    let tls_files = match opts.as_ref().and_then(|o| o.get("tls")) {
        None | Some(Value::Null) => None,
        Some(Value::Dict(t)) => match (t.get("cert"), t.get("key")) {
            (Some(Value::Str(c)), Some(Value::Str(k))) => Some((c, k)),
            _ => return err(format!("{}(): tls must be a dict {{\"cert\": file, \"key\": file}}", who)),
        },
        Some(_) => return err(format!("{}(): tls must be a dict {{\"cert\": file, \"key\": file}}", who)),
    };
    let tls_stop = Arc::new(AtomicBool::new(false));
    let tls_active = Arc::new(AtomicUsize::new(0));
    // On the way out: stop accepting, then give TLS connections that are still delivering their reply time to finish.
    // Returning at once (for example right after the last of `max_requests`) would end the process while the helper
    // still holds the end of the reply, and the client would see a reset instead of the response.
    struct StopOnDrop(Arc<AtomicBool>, Arc<AtomicUsize>);
    impl Drop for StopOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
            let end = Instant::now() + Duration::from_secs(15);
            while self.1.load(Ordering::SeqCst) > 0 && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    let _tls_guard = StopOnDrop(tls_stop.clone(), tls_active.clone());
    let listener = if let Some((cert, key)) = tls_files {
        crate::net_tls_server::check_credentials(&cert, &key, who)?;
        let front = TcpListener::bind((host.as_str(), port)).map_err(|e| VmError::runtime_error(format!("{}: cannot bind {}:{}: {}", who, host, port, e)))?;
        let inner = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))?;
        let backend = inner.local_addr().map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))?;
        crate::net_tls_server::spawn_front(front, backend, cert, key, tls_stop.clone(), tls_active.clone());
        inner
    } else {
        TcpListener::bind((host.as_str(), port)).map_err(|e| VmError::runtime_error(format!("{}: cannot bind {}:{}: {}", who, host, port, e)))?
    };
    listener.set_nonblocking(true).map_err(|e| VmError::runtime_error(format!("{}: {}", who, e)))?;
    STOP.store(false, Ordering::SeqCst);

    let pool = Arc::new(Pool {
        closing: AtomicBool::new(false),
        admitted: AtomicU64::new(0),
        completed: AtomicU64::new(0),
        pending: AtomicUsize::new(0),
        last_done_ms: AtomicU64::new(0),
        born: Instant::now(),
        max_requests,
    });
    // Closing the pool on every way out (including `?` below) makes workers refuse what is left in the queue.
    struct CloseOnDrop(Arc<Pool>);
    impl Drop for CloseOnDrop {
        fn drop(&mut self) {
            self.0.closing.store(true, Ordering::SeqCst);
        }
    }
    let _close = CloseOnDrop(Arc::clone(&pool));
    let (tx, rx): (SyncSender<TcpStream>, Receiver<TcpStream>) = sync_channel(queue);
    let rx = Arc::new(Mutex::new(rx));
    for i in 0..workers {
        // every worker gets its own detached copy of the globals, the routes and the handler
        let seed = vm.worker_seed(program);
        let routes = routes_for_worker();
        let handler = handler.as_ref().map(Value::detached);
        let (rx, pool) = (Arc::clone(&rx), Arc::clone(&pool));
        seed.spawn(format!("killer-http-{}", i), move |wvm, prog| worker_loop(wvm, prog, &routes, &handler, lim, &pool, &rx))
            .map_err(|e| VmError::runtime_error(format!("{}: cannot start worker thread: {}", who, e)))?;
    }
    // queue full: a single rejecter thread answers 503 so the accept loop never waits on a client
    let (reject_tx, reject_rx) = sync_channel::<TcpStream>(REJECT_BACKLOG);
    let _ = std::thread::Builder::new().name("killer-http-reject".to_string()).spawn(move || {
        let busy = unavailable("the server is busy (request queue full)");
        for s in reject_rx {
            refuse(s, &busy);
        }
    });

    let mut last_accept = Instant::now();
    loop {
        if stop_requested() || pool.closing.load(Ordering::SeqCst) {
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                last_accept = Instant::now();
                let _ = stream.set_nonblocking(false);
                pool.pending.fetch_add(1, Ordering::SeqCst);
                match tx.try_send(stream) {
                    Ok(()) => {}
                    Err(TrySendError::Full(s)) | Err(TrySendError::Disconnected(s)) => {
                        pool.pending.fetch_sub(1, Ordering::SeqCst);
                        // the rejecter is backed up too: just close the connection
                        let _ = reject_tx.try_send(s);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(d) = idle {
                    let last_done = pool.born + Duration::from_millis(pool.last_done_ms.load(Ordering::SeqCst));
                    if pool.pending.load(Ordering::SeqCst) == 0 && last_accept.max(last_done).elapsed() >= d {
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => return err(format!("{}: accept failed: {}", who, e)),
        }
    }
    // stop accepting: new connections are refused by the OS, queued ones get 503 from the workers
    pool.closing.store(true, Ordering::SeqCst);
    drop(listener);
    drop(tx);
    drop(reject_tx);
    // Drain. After max_requests the admitted requests are always answered (each is bounded by its own
    // request/stream timeout); after server_stop() handlers still running get `drain_timeout`.
    let mut deadline = if stop_requested() { Some(Instant::now() + drain) } else { None };
    while pool.pending.load(Ordering::SeqCst) > 0 {
        if deadline.is_none() && stop_requested() {
            deadline = Some(Instant::now() + drain);
        }
        if deadline.map_or(false, |d| Instant::now() >= d) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(Value::Number(pool.completed.load(Ordering::SeqCst) as f64))
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
                    let (method, segments) = (m.to_ascii_uppercase(), split_path(p));
                    published().push(Route { method: method.clone(), segments: segments.clone(), handler: f.detached() });
                    ROUTES.with(|r| r.borrow_mut().push(Route { method, segments, handler: f.clone() }));
                    Ok(Value::Null)
                }
                _ => err("route(method, path, fn) expects two strings and a function"),
            }
        })()),
        "routes_clear" => Some({
            ROUTES.with(|r| r.borrow_mut().clear());
            published().clear();
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
