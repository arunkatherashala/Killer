//! Killer **native** MCP server (Model Context Protocol) over stdio — **zero external dependencies**
//! (uses in-tree `mcp_minjson` for JSON-RPC bodies).
//!
//! ## Build
//! ```text
//! cargo build --release --bin killer-mcp
//! ```
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
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

use killer_native::compiler::compile_killer_default;
use killer_native::llm::ollama_is_running;
use killer_native::VERSION;
use minjson::{stringify, Json};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdin = BufReader::new(stdin.lock());
    let mut stdout = io::stdout();

    while let Some(raw) = read_mcp_message(&mut stdin)? {
        let v = match minjson::parse(raw.trim()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[killer-mcp] invalid JSON skipped: {e}");
                continue;
            }
        };

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
                    let out = handle_tool_call(v.get("params"));
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

fn read_mcp_message<R: BufRead>(r: &mut R) -> io::Result<Option<String>> {
    let mut first = String::new();
    loop {
        first.clear();
        if r.read_line(&mut first)? == 0 {
            return Ok(None);
        }
        let t = first.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix("Content-Length:") {
            let n: usize = rest.trim().parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Content-Length: {e}"))
            })?;
            let mut _blank = String::new();
            r.read_line(&mut _blank)?;
            let mut buf = vec![0u8; n];
            r.read_exact(&mut buf)?;
            return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
        }
        return Ok(Some(t.to_string()));
    }
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

/// Run `killer-native` on a script; capture combined output for MCP.
fn execute_killer_subprocess(source: Option<&str>, path: Option<&str>) -> Result<String, String> {
    let bin = killer_native_binary();
    if !bin.exists() {
        return Err(format!(
            "killer-native not found at {}. Build the workspace or set KILLER_NATIVE to the executable path.",
            bin.display()
        ));
    }

    let arg_path: PathBuf = match (source, path) {
        (Some(src), None) => {
            let tmp = std::env::temp_dir().join(format!(
                "killer_mcp_{}.killer",
                std::process::id()
            ));
            fs::write(&tmp, src).map_err(|e| format!("temp write: {e}"))?;
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

    let out = Command::new(&bin)
        .arg(&arg_path)
        .output()
        .map_err(|e| format!("spawn killer-native: {e}"))?;

    if source.is_some() {
        let _ = fs::remove_file(&arg_path);
    }

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
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
    if !out.status.success() {
        return Err(format!(
            "exit {:?}: {}",
            out.status.code(),
            text.trim_end()
        ));
    }
    Ok(text)
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
