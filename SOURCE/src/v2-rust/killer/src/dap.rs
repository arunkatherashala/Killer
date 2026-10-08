//! Debug Adapter Protocol server for Killer (`killer_dap`), built on the VM's debug channel
//! (`vm::set_vm_debug_channel`) and `compile_killer_debug`.
//!
//! Architecture
//! * The adapter (this process) speaks DAP over stdio and never runs user code itself: the
//!   program's own `print` output goes to the real stdout, which would corrupt the protocol.
//! * For `launch` it starts a *debuggee* child process (`killer_dap --debuggee <file> <port>`),
//!   which compiles the file with `compile_killer_debug`, runs the VM on a thread with the debug
//!   channel registered, and talks to the adapter over a loopback TCP socket with one JSON
//!   object per line. The child's stdout/stderr are piped back as DAP `output` events.
//! * At every `__dl(N)` checkpoint the VM reports `(line, variables)` and blocks until the adapter
//!   answers `c` (resume) or `x` (stop). The adapter decides, from the breakpoints and the
//!   stepping mode, whether the client sees a `stopped` event or the checkpoint is passed
//!   silently.
//!
//! Limits: the VM reports only the line and the variables of the innermost scope (plus the
//! globals while inside a function), not the call stack, so `stackTrace` returns the top frame
//! only and `next`/`stepOut` use the enclosing function of each line as an approximation of the
//! stack depth.

use crate::json_lang;
use crate::value::{SharedDict, Value};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc;

// ---------------------------------------------------------------------------------------------
// Instrumentation

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Class,
    Fn,
    Block,
    Data,
}

/// Insert `__dl(N)` checkpoints (N = 1-based original line) before every line that starts a
/// statement inside a function, a block or at the top level. Lines inside data literals, class
/// bodies, declarations and continuation lines are left alone, so the instrumented program is
/// still valid. Each checkpoint copies the indentation of the line it precedes.
pub fn instrument(source: &str) -> String {
    let mut out = String::with_capacity(source.len() * 2);
    let mut stack: Vec<Ctx> = Vec::new();
    let mut prev_code_continues = false;
    for (idx, line) in source.lines().enumerate() {
        let masked = crate::formatter::mask_code(line);
        let t = masked.trim();
        let first: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let starts_closer = t.starts_with(|c| matches!(c, '}' | ')' | ']'));
        let continuation_start = t.starts_with(|c| matches!(c, '.' | '+' | '*' | '/' | '%' | '=' | '<' | '>' | '&' | '|' | '?' | ':' | ','))
            || matches!(first.as_str(), "and" | "or" | "else" | "elif" | "catch" | "finally" | "case" | "default");
        let declaration = matches!(first.as_str(), "fn" | "kfn" | "akfn" | "class" | "import" | "test" | "static")
            || (first == "async" && t.contains("fn "))
            || t.starts_with('@');
        let in_statement_ctx = !matches!(stack.last(), Some(Ctx::Class) | Some(Ctx::Data));
        let wanted = !t.is_empty() && in_statement_ctx && !starts_closer && !continuation_start && !declaration && !prev_code_continues;
        if wanted && !line.trim_start().starts_with('#') {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            out.push_str(&format!("{indent}__dl({})\n", idx + 1));
        }
        out.push_str(line);
        out.push('\n');
        if t.is_empty() {
            continue;
        }
        let stack_before = stack.len();
        // update the bracket context with this line
        let chars: Vec<char> = masked.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '(' | '[' => stack.push(Ctx::Data),
                '{' => {
                    let before: String = chars[..i].iter().collect::<String>();
                    let before = before.trim_end();
                    let prev = before.chars().last();
                    let data = matches!(prev, Some('=') | Some('(') | Some('[') | Some(',') | Some(':'))
                        || before.ends_with("return")
                        || (prev == Some('{') && stack.last() == Some(&Ctx::Data));
                    let kind = if data {
                        Ctx::Data
                    } else if first == "class" {
                        Ctx::Class
                    } else if matches!(first.as_str(), "fn" | "kfn" | "akfn" | "async") {
                        Ctx::Fn
                    } else {
                        Ctx::Block
                    };
                    stack.push(kind);
                }
                '}' | ')' | ']' => {
                    stack.pop();
                }
                _ => {}
            }
        }
        // Plain `x = ...` assignments keep `x` in an anonymous VM slot. Mirror it into a named
        // variable so the debugger can show it: `__v_x = x` (names starting `__` are stored by name).
        if wanted && stack.len() == stack_before && !t.ends_with('{') && !t.ends_with('}') {
            if let Some(name) = assigned_name(t) {
                let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                out.push_str(&format!("{indent}__v_{name} = {name}\n"));
            }
        }
        let last = t.chars().last().unwrap_or(' ');
        prev_code_continues = matches!(last, '+' | '-' | '*' | '/' | '%' | '=' | ',' | '&' | '|' | '.' | '<' | '>' | '?' | ':' | '\\')
            || t.ends_with(" and")
            || t.ends_with(" or");
    }
    out
}

/// `x = ...`, `let x = ...`, `x += ...` at the start of a masked statement: the name `x`.
fn assigned_name(t: &str) -> Option<String> {
    let t = t.strip_prefix("let ").map(|r| r.trim_start()).unwrap_or(t);
    let name: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) || name.starts_with("__") || crate::lsp_core::KEYWORDS.contains(&name.as_str()) {
        return None;
    }
    let rest = t[name.len()..].trim_start();
    let plain = rest.starts_with('=') && !rest.starts_with("==") && !rest.starts_with("=>");
    let compound = ["+=", "-=", "*=", "/=", "%="].iter().any(|op| rest.starts_with(op));
    if plain || compound {
        Some(name)
    } else {
        None
    }
}

/// Line ranges (1-based, inclusive) and names of the functions in `source`.
pub fn function_ranges(source: &str) -> Vec<(String, usize, usize)> {
    let mut out: Vec<(String, usize, usize)> = Vec::new();
    let mut depth = 0i64;
    let mut open: Vec<(usize, i64)> = Vec::new(); // (index into out, depth at declaration)
    for (i, line) in source.lines().enumerate() {
        let masked = crate::formatter::mask_code(line);
        let t = masked.trim();
        let t2 = t.strip_prefix("async ").unwrap_or(t);
        for kw in ["fn ", "kfn ", "akfn "] {
            if let Some(rest) = t2.strip_prefix(kw) {
                let name: String = rest.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() {
                    out.push((name, i + 1, i + 1));
                    open.push((out.len() - 1, depth));
                }
            }
        }
        for c in masked.chars() {
            match c {
                '{' | '[' | '(' => depth += 1,
                '}' | ']' | ')' => {
                    depth = (depth - 1).max(0);
                    if let Some(&(idx, d)) = open.last() {
                        if depth == d && c == '}' {
                            out[idx].2 = i + 1;
                            open.pop();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn enclosing(ranges: &[(String, usize, usize)], line: usize) -> Option<usize> {
    // innermost = the one with the latest start containing the line
    ranges
        .iter()
        .enumerate()
        .filter(|(_, (_, s, e))| *s <= line && line <= *e)
        .max_by_key(|(_, (_, s, _))| *s)
        .map(|(i, _)| i)
}

// ---------------------------------------------------------------------------------------------
// Debuggee (child process)

/// Child side: `killer_dap --debuggee <file> <port>`.
pub fn run_debuggee(file: &str, port: u16) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: cannot read {file}: {e}");
            return 1;
        }
    };
    // compile the real program first so syntax errors name the user's lines
    if let Err(e) = crate::compiler::compile_killer_subset(&source) {
        eprintln!("Error: {e}");
        return 1;
    }
    let program = match crate::compiler::compile_killer_debug(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: debug build failed: {e}");
            return 1;
        }
    };
    let stream = match std::net::TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: cannot reach the debug adapter: {e}");
            return 1;
        }
    };
    let (ev_tx, ev_rx) = mpsc::channel::<crate::vm::DbgEvent>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<crate::vm::DbgCmd>();
    let vm_thread = std::thread::spawn(move || {
        crate::vm::set_vm_debug_channel(ev_tx, cmd_rx);
        let mut vm = crate::vm::VirtualMachine::new();
        let result = vm.run(&program);
        crate::vm::clear_vm_debug_channel(); // drops the sender: the forwarder below ends
        result.map_err(|e| e.to_string())
    });
    let mut writer = stream.try_clone().expect("clone stream");
    let mut reader = BufReader::new(stream);
    for (line, vars) in ev_rx {
        let vars_json: Vec<Value> = vars.into_iter().map(|(k, v)| Value::from(vec![Value::Str(k), Value::Str(v)])).collect();
        let msg = Value::Dict(SharedDict::from_pairs(vec![
            ("line".to_string(), Value::Number(line as f64)),
            ("vars".to_string(), Value::from(vars_json)),
        ]));
        if writeln!(writer, "{}", json_lang::stringify(&msg, 0).unwrap_or_default()).is_err() {
            let _ = cmd_tx.send(false);
            break;
        }
        let mut answer = String::new();
        if reader.read_line(&mut answer).unwrap_or(0) == 0 {
            let _ = cmd_tx.send(false);
            break;
        }
        let _ = cmd_tx.send(answer.trim() == "c");
    }
    let code = match vm_thread.join() {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => {
            eprintln!("Error: {e}");
            1
        }
        Err(_) => {
            eprintln!("Error: the program panicked");
            1
        }
    };
    let _ = writeln!(writer, "{{\"exit\":{code}}}");
    code
}

// ---------------------------------------------------------------------------------------------
// Adapter

enum Msg {
    Dap(Value),
    DapClosed,
    Checkpoint(usize, Vec<(String, String)>),
    Exited(i32),
    Output(&'static str, String),
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Continue,
    StepIn,
    Next,
    StepOut,
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
}
fn s(t: &str) -> Value {
    Value::Str(t.to_string())
}
fn num(n: i64) -> Value {
    Value::Number(n as f64)
}
fn get(v: &Value, k: &str) -> Value {
    match v {
        Value::Dict(d) => d.get(k).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::Str(t) => Some(t.clone()),
        _ => None,
    }
}
fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => Some(*n as i64),
        Value::Integer(n) => Some(*n),
        _ => None,
    }
}

struct Session<W: Write> {
    out: W,
    seq: i64,
    program: Option<String>,
    source: String,
    ranges: Vec<(String, usize, usize)>,
    breakpoints: HashSet<usize>,
    stop_on_entry: bool,
    mode: Mode,
    origin: Option<usize>,
    skipping: bool,
    first_event: bool,
    child: Option<std::process::Child>,
    to_child: Option<std::net::TcpStream>,
    paused: bool,
    line: usize,
    vars: Vec<(String, String)>,
    terminated: bool,
}

impl<W: Write> Session<W> {
    fn send(&mut self, v: Value) {
        let body = json_lang::stringify(&v, 0).unwrap_or_default();
        let _ = write!(self.out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
        let _ = self.out.flush();
    }
    fn respond(&mut self, req: &Value, ok: bool, body: Value, message: &str) {
        self.seq += 1;
        let mut pairs = vec![
            ("seq", num(self.seq)),
            ("type", s("response")),
            ("request_seq", get(req, "seq")),
            ("success", Value::Bool(ok)),
            ("command", get(req, "command")),
        ];
        if !ok {
            pairs.push(("message", s(message)));
        }
        pairs.push(("body", body));
        self.send(obj(pairs));
    }
    fn event(&mut self, name: &str, body: Value) {
        self.seq += 1;
        let v = obj(vec![("seq", num(self.seq)), ("type", s("event")), ("event", s(name)), ("body", body)]);
        self.send(v);
    }
    fn answer_child(&mut self, resume: bool) {
        if let Some(t) = self.to_child.as_mut() {
            let _ = writeln!(t, "{}", if resume { "c" } else { "x" });
            let _ = t.flush();
        }
    }
    fn finish(&mut self, code: i32) {
        if self.terminated {
            return;
        }
        self.terminated = true;
        self.paused = false;
        self.event("exited", obj(vec![("exitCode", num(code as i64))]));
        self.event("terminated", obj(vec![]));
    }

    fn start(&mut self, tx: mpsc::Sender<Msg>) -> Result<(), String> {
        let program = self.program.clone().ok_or("launch was not called")?;
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = std::process::Command::new(exe)
            .arg("--debuggee")
            .arg(&program)
            .arg(port.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot start the debuggee: {e}"))?;
        for (stream, cat) in [(child.stdout.take().map(|x| Box::new(x) as Box<dyn Read + Send>), "stdout"), (child.stderr.take().map(|x| Box::new(x) as Box<dyn Read + Send>), "stderr")] {
            if let Some(mut r) = stream {
                let tx = tx.clone();
                std::thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    loop {
                        match r.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let _ = tx.send(Msg::Output(cat, String::from_utf8_lossy(&buf[..n]).to_string()));
                            }
                        }
                    }
                });
            }
        }
        self.child = Some(child);
        // accept the debuggee's connection (it connects right after compiling)
        let tx2 = tx.clone();
        let (conn_tx, conn_rx) = mpsc::channel::<std::net::TcpStream>();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let _ = conn_tx.send(stream.try_clone().unwrap());
                let mut reader = BufReader::new(stream);
                loop {
                    let mut l = String::new();
                    if reader.read_line(&mut l).unwrap_or(0) == 0 {
                        let _ = tx2.send(Msg::Exited(-1));
                        break;
                    }
                    if let Ok(v) = json_lang::parse(l.trim()) {
                        if let Some(code) = as_i64(&get(&v, "exit")) {
                            let _ = tx2.send(Msg::Exited(code as i32));
                            break;
                        }
                        let line = as_i64(&get(&v, "line")).unwrap_or(0) as usize;
                        let mut vars = Vec::new();
                        if let Value::Array(a) = get(&v, "vars") {
                            for p in a.to_vec() {
                                if let Value::Array(kv) = p {
                                    let kv = kv.to_vec();
                                    if kv.len() == 2 {
                                        vars.push((as_str(&kv[0]).unwrap_or_default(), as_str(&kv[1]).unwrap_or_default()));
                                    }
                                }
                            }
                        }
                        let _ = tx2.send(Msg::Checkpoint(line, vars));
                    }
                }
            } else {
                let _ = tx2.send(Msg::Exited(1));
            }
        });
        // the child may exit before connecting (compile error): do not block forever
        match conn_rx.recv_timeout(std::time::Duration::from_secs(20)) {
            Ok(stream) => {
                self.to_child = Some(stream);
                Ok(())
            }
            Err(_) => {
                // the debuggee exited without connecting: its output already explains why
                Ok(())
            }
        }
    }

    fn should_stop(&mut self, line: usize) -> Option<&'static str> {
        let here = enclosing(&self.ranges, line);
        let first = self.first_event;
        self.first_event = false;
        if self.breakpoints.contains(&line) {
            self.skipping = false;
            return Some("breakpoint");
        }
        if first && self.stop_on_entry {
            return Some("entry");
        }
        match self.mode {
            Mode::Continue => None,
            Mode::StepIn => Some("step"),
            Mode::Next => {
                // skip events inside functions other than the one we started in
                if here == self.origin || here.is_none() {
                    self.skipping = false;
                    Some("step")
                } else if self.skipping || self.origin.is_some() || here.is_some() {
                    self.skipping = true;
                    None
                } else {
                    Some("step")
                }
            }
            Mode::StepOut => {
                if here != self.origin {
                    Some("step")
                } else {
                    None
                }
            }
        }
    }

    fn handle_checkpoint(&mut self, line: usize, vars: Vec<(String, String)>) {
        match self.should_stop(line) {
            Some(reason) => {
                self.paused = true;
                self.line = line;
                self.vars = vars;
                self.event("stopped", obj(vec![("reason", s(reason)), ("threadId", num(1)), ("allThreadsStopped", Value::Bool(true))]));
            }
            None => self.answer_child(true),
        }
    }

    fn resume(&mut self, mode: Mode) {
        self.mode = mode;
        self.origin = enclosing(&self.ranges, self.line);
        self.skipping = false;
        self.paused = false;
        self.answer_child(true);
    }

    fn handle_request(&mut self, req: &Value, tx: &mpsc::Sender<Msg>) -> bool {
        let cmd = as_str(&get(req, "command")).unwrap_or_default();
        let args = get(req, "arguments");
        match cmd.as_str() {
            "initialize" => {
                self.respond(
                    req,
                    true,
                    obj(vec![
                        ("supportsConfigurationDoneRequest", Value::Bool(true)),
                        ("supportsTerminateRequest", Value::Bool(true)),
                    ]),
                    "",
                );
                self.event("initialized", obj(vec![]));
            }
            "launch" => {
                match as_str(&get(&args, "program")) {
                    Some(p) => match std::fs::read_to_string(&p) {
                        Ok(src) => {
                            self.ranges = function_ranges(&src);
                            self.source = src;
                            self.program = Some(p);
                            self.stop_on_entry = matches!(get(&args, "stopOnEntry"), Value::Bool(true));
                            self.respond(req, true, obj(vec![]), "");
                        }
                        Err(e) => self.respond(req, false, obj(vec![]), &format!("cannot read {p}: {e}")),
                    },
                    None => self.respond(req, false, obj(vec![]), "launch needs a `program` path"),
                }
            }
            "setBreakpoints" => {
                let mut lines: Vec<usize> = Vec::new();
                if let Value::Array(a) = get(&args, "breakpoints") {
                    for b in a.to_vec() {
                        if let Some(l) = as_i64(&get(&b, "line")) {
                            lines.push(l as usize);
                        }
                    }
                } else if let Value::Array(a) = get(&args, "lines") {
                    for l in a.to_vec() {
                        if let Some(l) = as_i64(&l) {
                            lines.push(l as usize);
                        }
                    }
                }
                self.breakpoints = lines.iter().copied().collect();
                let bps: Vec<Value> = lines.iter().map(|l| obj(vec![("verified", Value::Bool(true)), ("line", num(*l as i64))])).collect();
                self.respond(req, true, obj(vec![("breakpoints", Value::from(bps))]), "");
            }
            "setExceptionBreakpoints" => self.respond(req, true, obj(vec![]), ""),
            "configurationDone" => {
                match self.start(tx.clone()) {
                    Ok(()) => self.respond(req, true, obj(vec![]), ""),
                    Err(e) => self.respond(req, false, obj(vec![]), &e),
                }
            }
            "threads" => self.respond(req, true, obj(vec![("threads", Value::from(vec![obj(vec![("id", num(1)), ("name", s("main"))])]))]), ""),
            "stackTrace" => {
                let name = enclosing(&self.ranges, self.line).map(|i| self.ranges[i].0.clone()).unwrap_or_else(|| "<main>".to_string());
                let path = self.program.clone().unwrap_or_default();
                let frame = obj(vec![
                    ("id", num(1)),
                    ("name", s(&name)),
                    ("source", obj(vec![("name", s(std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).unwrap_or(&path))), ("path", s(&path))])),
                    ("line", num(self.line as i64)),
                    ("column", num(1)),
                ]);
                self.respond(req, true, obj(vec![("stackFrames", Value::from(vec![frame])), ("totalFrames", num(1))]), "");
            }
            "scopes" => {
                let scopes = vec![
                    obj(vec![("name", s("Locals")), ("variablesReference", num(1)), ("expensive", Value::Bool(false))]),
                    obj(vec![("name", s("Globals")), ("variablesReference", num(2)), ("expensive", Value::Bool(false))]),
                ];
                self.respond(req, true, obj(vec![("scopes", Value::from(scopes))]), "");
            }
            "variables" => {
                let reference = as_i64(&get(&args, "variablesReference")).unwrap_or(1);
                let func = enclosing(&self.ranges, self.line);
                let in_function = func.is_some();
                // the VM keeps parameters as `arg0`, `arg1`, ... and `args`: show the source names
                let params: Vec<String> = func
                    .and_then(|i| self.source.lines().nth(self.ranges[i].1 - 1))
                    .map(|decl| {
                        let inner = decl.split_once('(').map(|x| x.1).unwrap_or("");
                        let inner = inner.split(')').next().unwrap_or("");
                        inner.split(',').filter_map(|p| p.trim().split(|c: char| !(c.is_alphanumeric() || c == '_')).next().filter(|w| !w.is_empty()).map(|w| w.to_string())).collect()
                    })
                    .unwrap_or_default();
                let mut list: Vec<Value> = Vec::new();
                for (k, v) in &self.vars {
                    if in_function && k == "args" {
                        continue;
                    }
                    // `__v_x` is the named mirror of the slot variable `x` (see `instrument`)
                    let k = &match k.strip_prefix("::") {
                        Some(rest) => match rest.strip_prefix("__v_") {
                            Some(n) => format!("::{n}"),
                            None => k.clone(),
                        },
                        None => match k.strip_prefix("__v_") {
                            Some(n) => n.to_string(),
                            None => k.clone(),
                        },
                    };
                    let k = &match k.strip_prefix("arg").and_then(|n| n.parse::<usize>().ok()) {
                        Some(n) if in_function && params.get(n).is_some() => params[n].clone(),
                        _ => k.clone(),
                    };
                    let is_global_entry = k.starts_with("::");
                    // inside a function the VM reports locals first and globals prefixed with `::`;
                    // at the top level the single scope is the globals
                    let belongs_to_globals = if in_function { is_global_entry } else { true };
                    let wanted = if reference == 2 { belongs_to_globals } else { !belongs_to_globals };
                    if wanted && !k.starts_with("__") {
                        list.push(obj(vec![("name", s(k.trim_start_matches("::"))), ("value", s(v)), ("variablesReference", num(0))]));
                    }
                }
                self.respond(req, true, obj(vec![("variables", Value::from(list))]), "");
            }
            "continue" => {
                self.respond(req, true, obj(vec![("allThreadsContinued", Value::Bool(true))]), "");
                self.resume(Mode::Continue);
            }
            "next" => {
                self.respond(req, true, obj(vec![]), "");
                self.resume(Mode::Next);
            }
            "stepIn" => {
                self.respond(req, true, obj(vec![]), "");
                self.resume(Mode::StepIn);
            }
            "stepOut" => {
                self.respond(req, true, obj(vec![]), "");
                self.resume(Mode::StepOut);
            }
            "terminate" | "disconnect" => {
                self.answer_child(false);
                if let Some(mut c) = self.child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                self.respond(req, true, obj(vec![]), "");
                return false;
            }
            _ => self.respond(req, false, obj(vec![]), &format!("unsupported request: {cmd}")),
        }
        true
    }
}

fn read_dap<R: BufRead>(r: &mut R) -> Option<String> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let t = line.trim();
        if t.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some(v) = t.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().ok();
        }
    }
    let mut buf = vec![0u8; len?];
    r.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// Run the adapter on `input`/`output`. Returns the exit code.
pub fn serve<R: BufRead + Send + 'static, W: Write>(mut input: R, output: W) -> i32 {
    let (tx, rx) = mpsc::channel::<Msg>();
    let tx_in = tx.clone();
    std::thread::spawn(move || {
        while let Some(body) = read_dap(&mut input) {
            if let Ok(v) = json_lang::parse(&body) {
                if tx_in.send(Msg::Dap(v)).is_err() {
                    return;
                }
            }
        }
        let _ = tx_in.send(Msg::DapClosed);
    });
    let mut sess = Session {
        out: output,
        seq: 0,
        program: None,
        source: String::new(),
        ranges: Vec::new(),
        breakpoints: HashSet::new(),
        stop_on_entry: false,
        mode: Mode::Continue,
        origin: None,
        skipping: false,
        first_event: true,
        child: None,
        to_child: None,
        paused: false,
        line: 0,
        vars: Vec::new(),
        terminated: false,
    };
    let _ = HashMap::<u8, u8>::new();
    for msg in rx {
        match msg {
            Msg::Dap(req) => {
                if !sess.handle_request(&req, &tx) {
                    return 0;
                }
            }
            Msg::DapClosed => {
                if let Some(mut c) = sess.child.take() {
                    let _ = c.kill();
                }
                return 0;
            }
            Msg::Checkpoint(line, vars) => sess.handle_checkpoint(line, vars),
            Msg::Output(cat, text) => sess.event("output", obj(vec![("category", s(cat)), ("output", s(&text))])),
            Msg::Exited(code) => {
                if let Some(mut c) = sess.child.take() {
                    let _ = c.wait();
                }
                sess.finish(code);
            }
        }
    }
    0
}

/// Entry point of the `killer_dap` binary.
pub fn main_with_args(args: &[String]) -> i32 {
    if args.first().map(|a| a.as_str()) == Some("--debuggee") {
        let (Some(file), Some(port)) = (args.get(1), args.get(2).and_then(|p| p.parse::<u16>().ok())) else {
            eprintln!("usage: killer_dap --debuggee <file> <port>");
            return 2;
        };
        return run_debuggee(file, port);
    }
    let stdin = std::io::stdin();
    serve(BufReader::new(stdin), std::io::stdout())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrument_only_statement_lines() {
        let src = "x = 1\nfn f(a) {\n  y = {\n    \"k\": 1\n  }\n  if a {\n    return 1\n  } else {\n    return [\n      2\n    ]\n  }\n}\nprint(f(x))\n";
        let out = instrument(src);
        let marks: Vec<usize> = out.lines().filter_map(|l| l.trim().strip_prefix("__dl(")).filter_map(|r| r.trim_end_matches(')').parse().ok()).collect();
        // statements: x = 1 (1), y = { (3), if (6), return 1 (7), return [ (9), print (14)
        assert_eq!(marks, vec![1, 3, 6, 7, 9, 14]);
    }

    #[test]
    fn function_ranges_and_enclosing() {
        let src = "x = 1\nfn f() {\n  return 1\n}\nfn g() {\n  return 2\n}\n";
        let r = function_ranges(src);
        assert_eq!(r, vec![("f".to_string(), 2, 4), ("g".to_string(), 5, 7)]);
        assert_eq!(enclosing(&r, 3), Some(0));
        assert_eq!(enclosing(&r, 1), None);
    }
}
