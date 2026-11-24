//! Killer **native** MCP server (Model Context Protocol) over stdio — **zero external dependencies**
//! (uses in-tree `mcp_minjson` for JSON-RPC bodies).
//!
//! ## Build
//! ```text
//! cargo build --release --bin killer-mcp
//! ```
//!
//! ## Limits and authentication
//! * a message body is capped at 8 MiB (`Content-Length` beyond that is drained and answered with an error);
//! * JSON nesting deeper than 64 levels is rejected;
//! * `killer_run` has a wall-clock budget (`KILLER_MCP_RUN_TIMEOUT_S`, default 30 s; the child is killed) and its
//!   captured output is capped at 1 MiB;
//! * tool calls run on a worker thread with a large stack and the join result is checked, so a compiler panic is
//!   answered with an error result *only in a build that unwinds* (the dev and test profiles). The release profile
//!   sets `panic = "abort"`, where a panic ends the whole server process and this isolation is inert;
//! * framing: headers are `Name: value` lines in any order (names case-insensitive, unknown ones such as
//!   `Content-Type` ignored); a missing or unparsable `Content-Length` cannot be framed, so the error is
//!   answered and the connection is closed rather than guessing where the next message starts;
//! * optional token: when `KILLER_MCP_TOKEN` is set, every request must carry it as `params._meta.token` (or a
//!   top-level `token`), otherwise it is answered with error -32001. stdio MCP has no transport authentication,
//!   so set it when the server is reachable by anything but the launching client.
//!
//! ## Cursor (`mcp.json`)
//! ```json
//! "killer": {
//!   "command": "C:/path/to/killer-native/target/release/killer-mcp.exe",
//!   "args": []
//! }
//! ```

#[path = "../mcp_minjson.rs"]
mod minjson;

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::Command;

use killer_native::compiler::compile_killer_default;
use killer_native::llm::ollama_is_running;
use killer_native::VERSION;
use minjson::{stringify, Json};

/// Largest accepted message body.
const MAX_BODY: usize = 8 * 1024 * 1024;
/// Largest captured stdout/stderr of a `killer_run` child.
const MAX_CAPTURE: usize = 1024 * 1024;
const DEFAULT_RUN_TIMEOUT_S: u64 = 30;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdin = BufReader::new(stdin.lock());
    let mut stdout = io::stdout();
    let token = std::env::var("KILLER_MCP_TOKEN").ok().filter(|t| !t.is_empty());

    loop {
        let raw = match read_mcp_message(&mut stdin)? {
            Incoming::Eof => break,
            Incoming::Msg(raw) => raw,
            Incoming::TooLarge(n, drain) => {
                // answer first (the client may still be sending), then stream the body away to stay in sync
                write_error(&mut stdout, None, -32600, &format!("request too large ({n} bytes; limit {MAX_BODY})"))?;
                io::copy(&mut stdin.by_ref().take(drain), &mut io::sink())?;
                continue;
            }
            Incoming::BadHeader(e) => {
                // the body that follows cannot be framed, so reading on would take it for a request: report and close
                write_error(&mut stdout, None, -32700, &format!("bad header: {e}; closing the connection"))?;
                break;
            }
        };
        let v = match minjson::parse(raw.trim()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[killer-mcp] invalid JSON skipped: {e}");
                write_error(&mut stdout, None, -32700, &format!("parse error: {e}"))?;
                continue;
            }
        };
        if let Some(tok) = &token {
            if !authorized(&v, tok) {
                if matches!(v.get("id"), Some(id) if !id.is_null()) {
                    write_error(&mut stdout, v.get("id"), -32001, "unauthorized: missing or invalid token (KILLER_MCP_TOKEN)")?;
                }
                continue;
            }
        }

        let method = v.get("method").and_then(|m| m.as_str());
        let has_id = matches!(v.get("id"), Some(id) if !id.is_null());

        match method {
            Some("initialize") => {
                if has_id {
                    let result = Json::obj(&[
                        ("protocolVersion", Json::Str("2024-11-05".into())),
                        ("capabilities", Json::obj(&[("tools", Json::obj(&[]))])),
                        (
                            "serverInfo",
                            Json::obj(&[
                                ("name", Json::Str("killer-native-mcp".into())),
                                ("version", Json::Str(env!("CARGO_PKG_VERSION").into())),
                            ]),
                        ),
                        (
                            "instructions",
                            Json::Str(
                                concat!(
                                    "Killer tools: killer_version, killer_compile, killer_run, killer_ollama_status. ",
                                    "killer_compile checks syntax via compile_killer_default. ",
                                    "killer_run executes a .killer program in a subprocess (captures stdout/stderr) — ",
                                    "use it instead of embedding run in MCP stdio."
                                )
                                .into(),
                            ),
                        ),
                    ]);
                    write_result(&mut stdout, v.get("id"), result)?;
                }
            }
            Some("notifications/initialized") | Some("initialized") => {}
            Some("tools/list") => {
                if has_id {
                    write_result(
                        &mut stdout,
                        v.get("id"),
                        Json::obj(&[("tools", Json::arr(tool_definitions()))]),
                    )?;
                }
            }
            Some("tools/call") => {
                if has_id {
                    let out = guarded_tool_call(v.get("params"));
                    write_result(&mut stdout, v.get("id"), out)?;
                }
            }
            Some("ping") => {
                if has_id {
                    write_result(&mut stdout, v.get("id"), Json::obj(&[]))?;
                }
            }
            Some(m) => {
                if has_id {
                    write_error(
                        &mut stdout,
                        v.get("id"),
                        -32601,
                        &format!("Method not found: {m}"),
                    )?;
                }
            }
            None => {
                if has_id {
                    write_error(&mut stdout, v.get("id"), -32600, "Missing method")?;
                }
            }
        }
    }
    Ok(())
}

enum Incoming {
    Eof,
    Msg(String),
    /// (claimed size, bytes still to discard)
    TooLarge(usize, u64),
    BadHeader(String),
}

fn authorized(v: &Json, token: &str) -> bool {
    let given = v
        .get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("token"))
        .or_else(|| v.get("token"))
        .and_then(|t| t.as_str());
    match given {
        Some(g) => {
            let (a, b) = (g.as_bytes(), token.as_bytes());
            let mut diff = a.len() ^ b.len();
            for i in 0..a.len().max(b.len()) {
                diff |= (*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0)) as usize;
            }
            diff == 0
        }
        None => false,
    }
}

/// Read one line (bytes, lossy UTF-8) of at most `MAX_BODY` bytes. `Ok(None)` at end of input; the flag is true
/// when the line was longer than the limit (the rest of it has been discarded).
fn read_line_capped<R: BufRead>(r: &mut R) -> io::Result<Option<(String, bool)>> {
    let mut buf = Vec::new();
    let n = r.by_ref().take(MAX_BODY as u64 + 1).read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    let too_long = buf.len() > MAX_BODY && buf.last() != Some(&b'\n');
    if too_long {
        loop {
            let mut sink = Vec::new();
            let m = r.by_ref().take(64 * 1024).read_until(b'\n', &mut sink)?;
            if m == 0 || sink.last() == Some(&b'\n') {
                break;
            }
        }
    }
    Ok(Some((String::from_utf8_lossy(&buf).into_owned(), too_long)))
}

fn read_mcp_message<R: BufRead>(r: &mut R) -> io::Result<Incoming> {
    loop {
        let (first, too_long) = match read_line_capped(r)? {
            None => return Ok(Incoming::Eof),
            Some(x) => x,
        };
        if too_long {
            return Ok(Incoming::TooLarge(MAX_BODY + 1, 0));
        }
        let t = first.trim();
        if t.is_empty() {
            continue;
        }
        if header_line(t).is_none() {
            // a bare JSON line (no framing)
            return Ok(Incoming::Msg(t.to_string()));
        }
        // a header block: `Name: value` lines (any order, names case-insensitive, unknown ones ignored) up to a blank line
        let mut len: Option<Result<usize, String>> = None;
        let mut line = t.to_string();
        let mut count = 0;
        loop {
            if let Some((name, value)) = header_line(&line) {
                if name.eq_ignore_ascii_case("content-length") {
                    let parsed = value.parse::<usize>().map_err(|e| format!("Content-Length: {e}"));
                    len = match (len, parsed) {
                        (Some(Ok(a)), Ok(b)) if a != b => Some(Err("conflicting Content-Length headers".to_string())),
                        (Some(Err(e)), _) => Some(Err(e)),
                        (_, p) => Some(p),
                    };
                }
            }
            count += 1;
            let (next, too_long) = match read_line_capped(r)? {
                None => return Ok(Incoming::Eof),
                Some(x) => x,
            };
            if too_long {
                return Ok(Incoming::TooLarge(MAX_BODY + 1, 0));
            }
            line = next.trim().to_string();
            if line.is_empty() {
                break;
            }
            if count >= 64 {
                return Ok(Incoming::BadHeader("too many header lines".into()));
            }
        }
        let n = match len {
            Some(Ok(n)) => n,
            Some(Err(e)) => return Ok(Incoming::BadHeader(e)),
            None => return Ok(Incoming::BadHeader("missing Content-Length".into())),
        };
        if n > MAX_BODY {
            // never allocate it; the caller discards the body after replying
            return Ok(Incoming::TooLarge(n, n as u64));
        }
        let mut buf = vec![0u8; n];
        r.read_exact(&mut buf)?;
        return Ok(Incoming::Msg(String::from_utf8_lossy(&buf).into_owned()));
    }
}

/// `Name: value` when `t` is an HTTP/LSP-style header line (the name is a token: JSON text never is one).
fn header_line(t: &str) -> Option<(&str, &str)> {
    let (name, value) = t.split_once(':')?;
    let token = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    token.then(|| (name, value.trim()))
}

fn write_framed<W: Write>(w: &mut W, body: &str) -> io::Result<()> {
    let b = body.as_bytes();
    write!(w, "Content-Length: {}\r\n\r\n", b.len())?;
    w.write_all(b)?;
    w.flush()?;
    Ok(())
}

fn write_result<W: Write>(w: &mut W, id: Option<&Json>, result: Json) -> io::Result<()> {
    let id_val = id.cloned().unwrap_or(Json::Null);
    let body = Json::Obj(HashMap::from([
        ("jsonrpc".into(), Json::Str("2.0".into())),
        ("id".into(), id_val),
        ("result".into(), result),
    ]));
    write_framed(w, &stringify(&body))
}

fn write_error<W: Write>(w: &mut W, id: Option<&Json>, code: i32, message: &str) -> io::Result<()> {
    let id_val = id.cloned().unwrap_or(Json::Null);
    let err = Json::obj(&[
        ("code", Json::Number(code as f64)),
        ("message", Json::Str(message.into())),
    ]);
    let body = Json::Obj(HashMap::from([
        ("jsonrpc".into(), Json::Str("2.0".into())),
        ("id".into(), id_val),
        ("error".into(), err),
    ]));
    write_framed(w, &stringify(&body))
}

fn tool_definitions() -> Vec<Json> {
    vec![
        Json::obj(&[
            (
                "name",
                Json::Str("killer_version".into()),
            ),
            (
                "description",
                Json::Str("Killer native runtime version string (library API).".into()),
            ),
            (
                "inputSchema",
                Json::obj(&[
                    ("type", Json::Str("object".into())),
                    ("properties", Json::obj(&[])),
                    ("required", Json::arr(vec![])),
                ]),
            ),
        ]),
        Json::obj(&[
            ("name", Json::Str("killer_compile".into())),
            (
                "description",
                Json::Str(
                    "Parse and compile Killer source via the default line-oriented pipeline (compile_killer_default). Returns instruction count on success or a compiler error message."
                        .into(),
                ),
            ),
            (
                "inputSchema",
                Json::obj(&[
                    ("type", Json::Str("object".into())),
                    (
                        "properties",
                        Json::obj(&[(
                            "source",
                            Json::obj(&[
                                ("type", Json::Str("string".into())),
                                (
                                    "description",
                                    Json::Str("Full Killer source text (.killer).".into()),
                                ),
                            ]),
                        )]),
                    ),
                    (
                        "required",
                        Json::arr(vec![Json::Str("source".into())]),
                    ),
                ]),
            ),
        ]),
        Json::obj(&[
            ("name", Json::Str("killer_ollama_status".into())),
            (
                "description",
                Json::Str(
                    "Whether Ollama appears to be listening on localhost (Killer llm:: Ollama integration)."
                        .into(),
                ),
            ),
            (
                "inputSchema",
                Json::obj(&[
                    ("type", Json::Str("object".into())),
                    ("properties", Json::obj(&[])),
                    ("required", Json::arr(vec![])),
                ]),
            ),
        ]),
        Json::obj(&[
            ("name", Json::Str("killer_run".into())),
            (
                "description",
                Json::Str(
                    "Run Killer source in a child killer-native process; return captured stdout and stderr. Provide exactly one of: source (full .killer text) or path (filesystem path to a .killer file). Binary: same directory as killer-mcp or KILLER_NATIVE env."
                        .into(),
                ),
            ),
            (
                "inputSchema",
                Json::obj(&[
                    ("type", Json::Str("object".into())),
                    (
                        "properties",
                        Json::obj(&[
                            (
                                "source",
                                Json::obj(&[
                                    ("type", Json::Str("string".into())),
                                    (
                                        "description",
                                        Json::Str("Full Killer program text to write to a temp .killer and run.".into()),
                                    ),
                                ]),
                            ),
                            (
                                "path",
                                Json::obj(&[
                                    ("type", Json::Str("string".into())),
                                    (
                                        "description",
                                        Json::Str("Path to an existing .killer file (absolute or cwd-relative).".into()),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                    ("required", Json::arr(vec![])),
                ]),
            ),
        ]),
    ]
}

fn killer_native_binary() -> PathBuf {
    if let Ok(p) = std::env::var("KILLER_NATIVE") {
        return PathBuf::from(p);
    }
    let name = if cfg!(target_os = "windows") {
        "killer-native.exe"
    } else {
        "killer-native"
    };
    std::env::current_exe()
        .ok()
        .and_then(|me| me.parent().map(|dir| dir.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// A temp script that is deleted when dropped.
struct TempScript(PathBuf);

impl Drop for TempScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

const MIN_RUN_TIMEOUT_S: f64 = 0.1;
const MAX_RUN_TIMEOUT_S: f64 = 3600.0;

/// The `killer_run` wall-clock budget in seconds from `KILLER_MCP_RUN_TIMEOUT_S`: a finite positive number clamped to
/// 0.1..=3600; anything else (unset, empty, `inf`, `NaN`, zero, negative, junk) is the default. Never panics in
/// `Duration::from_secs_f64`.
fn run_budget_secs(raw: Option<&str>) -> f64 {
    raw.and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
        .map_or(DEFAULT_RUN_TIMEOUT_S as f64, |s| s.clamp(MIN_RUN_TIMEOUT_S, MAX_RUN_TIMEOUT_S))
}

/// Run `killer-native` on a script; capture combined output for MCP.
fn execute_killer_subprocess(source: Option<&str>, path: Option<&str>) -> Result<String, String> {
    let bin = killer_native_binary();
    if !bin.exists() {
        return Err(format!(
            "killer-native not found at {}. Build the workspace or set KILLER_NATIVE to the executable path.",
            bin.display()
        ));
    }

    let mut temp_guard: Option<TempScript> = None;
    let arg_path: PathBuf = match (source, path) {
        (Some(src), None) => {
            let tmp = std::env::temp_dir().join(format!(
                "killer_mcp_{}.killer",
                std::process::id()
            ));
            // the guard removes the file on every exit path, including an early return or a panic
            let guard = TempScript(tmp.clone());
            fs::write(&tmp, src).map_err(|e| format!("temp write: {e}"))?;
            temp_guard = Some(guard);
            tmp
        }
        (None, Some(p)) => {
            let pb = PathBuf::from(p);
            if !pb.is_file() {
                return Err(format!("not a file: {}", pb.display()));
            }
            let ext = pb.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "killer" && ext != "kl" {
                return Err(format!(
                    "expected .killer (or .kl) extension, got: {}",
                    pb.display()
                ));
            }
            pb
        }
        _ => {
            return Err("provide exactly one of: source, path".into());
        }
    };

    let budget = run_budget_secs(std::env::var("KILLER_MCP_RUN_TIMEOUT_S").ok().as_deref());
    let run = run_with_budget(&bin, &arg_path, std::time::Duration::from_secs_f64(budget));
    drop(temp_guard);
    let (status, stdout, stderr, timed_out) = run?;
    if timed_out {
        return Err(format!("killer_run exceeded its {budget} s budget and was killed (KILLER_MCP_RUN_TIMEOUT_S)"));
    }
    let mut text = String::new();
    if !stdout.is_empty() {
        text.push_str("--- stdout ---\n");
        text.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str("--- stderr ---\n");
        text.push_str(&stderr);
    }
    if text.is_empty() {
        text.push_str("(no output)");
    }
    if !status.success() {
        return Err(format!(
            "exit {:?}: {}",
            status.code(),
            text.trim_end()
        ));
    }
    Ok(text)
}

/// Read a pipe to the end but keep only the first `MAX_CAPTURE` bytes (the rest is drained so the child never blocks).
fn capture<R: Read + Send + 'static>(mut r: R) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let room = MAX_CAPTURE.saturating_sub(kept.len());
                    kept.extend_from_slice(&buf[..n.min(room)]);
                }
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

/// Spawn the child, enforce the wall-clock budget (kill on expiry), return (status, stdout, stderr, timed_out).
fn run_with_budget(
    bin: &std::path::Path,
    script: &std::path::Path,
    budget: std::time::Duration,
) -> Result<(std::process::ExitStatus, String, String, bool), String> {
    let mut child = Command::new(bin)
        .arg(script)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn killer-native: {e}"))?;
    let out_h = child.stdout.take().map(capture);
    let err_h = child.stderr.take().map(capture);
    let deadline = std::time::Instant::now() + budget;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {}
            Err(e) => return Err(format!("wait killer-native: {e}")),
        }
        if std::time::Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().map_err(|e| format!("wait killer-native: {e}"))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let stdout = out_h.map(|h| h.join().unwrap_or_default()).unwrap_or_default();
    let stderr = err_h.map(|h| h.join().unwrap_or_default()).unwrap_or_default();
    Ok((status, stdout, stderr, timed_out))
}

/// Run a tool call on a worker thread with a big stack and turn a panic into an error result: a compiler bug or very
/// deep source must not take the server down. This only works when panics unwind; under `panic = "abort"` (the
/// release profile) the process ends at the panic and `join` never returns `Err`.
fn guarded_tool_call(params: Option<&Json>) -> Json {
    let owned = params.cloned();
    let worker = std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(move || handle_tool_call(owned.as_ref()));
    match worker {
        Ok(h) => match h.join() {
            Ok(j) => j,
            Err(_) => tool_text("internal error: the tool panicked (request rejected, server still running)".into(), true),
        },
        Err(e) => tool_text(format!("internal error: cannot start worker: {e}"), true),
    }
}

fn tool_text(text: String, is_error: bool) -> Json {
    Json::obj(&[
        (
            "content",
            Json::arr(vec![Json::obj(&[
                ("type", Json::Str("text".into())),
                ("text", Json::Str(text)),
            ])]),
        ),
        ("isError", Json::Bool(is_error)),
    ])
}

fn handle_tool_call(params: Option<&Json>) -> Json {
    let Some(params) = params else {
        return tool_text("missing params".into(), true);
    };
    let name = match params.get("name").and_then(|n| n.as_str()) {
        Some(n) => n,
        None => return tool_text("tools/call missing name".into(), true),
    };
    let args = params.get("arguments").cloned().unwrap_or_else(|| Json::obj(&[]));

    match name {
        "killer_version" => tool_text(
            format!(
                "killer-native library VERSION: {VERSION}\nmcp binary: {}",
                env!("CARGO_PKG_VERSION")
            ),
            false,
        ),
        "killer_compile" => {
            let Some(source) = args.get("source").and_then(|s| s.as_str()) else {
                return tool_text("arguments.source (string) required".into(), true);
            };
            match compile_killer_default(source) {
                Ok(prog) => {
                    let n = prog.instructions.len();
                    let funcs = prog.function_names.len();
                    tool_text(
                        format!("Compile OK.\ninstructions: {n}\nfunctions (named): {funcs}"),
                        false,
                    )
                }
                Err(e) => tool_text(format!("Compile error: {e}"), true),
            }
        }
        "killer_ollama_status" => {
            let ok = ollama_is_running();
            tool_text(format!("ollama_reachable: {ok}"), false)
        }
        "killer_run" => {
            let source = args.get("source").and_then(|s| s.as_str());
            let path = args.get("path").and_then(|s| s.as_str());
            match execute_killer_subprocess(source, path) {
                Ok(s) => tool_text(s, false),
                Err(e) => tool_text(e, true),
            }
        }
        other => tool_text(format!("unknown tool: {other}"), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_budget_is_always_finite_and_in_range() {
        assert_eq!(run_budget_secs(None), 30.0);
        assert_eq!(run_budget_secs(Some("2")), 2.0);
        assert_eq!(run_budget_secs(Some(" 1.5 ")), 1.5);
        assert_eq!(run_budget_secs(Some("1e300")), MAX_RUN_TIMEOUT_S);
        assert_eq!(run_budget_secs(Some("100000")), MAX_RUN_TIMEOUT_S);
        assert_eq!(run_budget_secs(Some("0.0001")), MIN_RUN_TIMEOUT_S);
        for junk in ["inf", "-inf", "NaN", "0", "-3", "", "abc"] {
            assert_eq!(run_budget_secs(Some(junk)), 30.0, "{junk}");
        }
    }

    fn read(bytes: &[u8]) -> Vec<String> {
        let mut r = io::BufReader::new(bytes);
        let mut out = Vec::new();
        loop {
            match read_mcp_message(&mut r).unwrap() {
                Incoming::Msg(m) => out.push(m),
                Incoming::Eof => return out,
                Incoming::BadHeader(e) => {
                    out.push(format!("BAD {e}"));
                    return out;
                }
                Incoming::TooLarge(n, _) => {
                    out.push(format!("BIG {n}"));
                    return out;
                }
            }
        }
    }

    #[test]
    fn headers_are_parsed_case_insensitively_in_any_order() {
        assert_eq!(read(b"content-length: 2\r\n\r\nab"), ["ab"]);
        assert_eq!(read(b"Content-Type: x\r\nCONTENT-LENGTH:3\r\n\r\nabc"), ["abc"]);
        assert_eq!(read(b"Content-Length: 1\r\nX: y\r\n\r\na\n{\"k\":1}\n"), ["a", "{\"k\":1}"]);
        assert!(read(b"Content-Length: 1\r\nContent-Length: 2\r\n\r\nab")[0].starts_with("BAD"));
        assert!(read(b"Content-Length: -1\r\n\r\n")[0].starts_with("BAD"));
        assert!(read(b"X: 1\r\n\r\n")[0].starts_with("BAD missing"));
    }
}
