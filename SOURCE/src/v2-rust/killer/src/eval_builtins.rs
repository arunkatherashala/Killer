//! `eval(code[, vars])` and `exec(code[, vars])`: compile and run Killer source given as text.
//!
//! The code runs in its own fresh virtual machine (it cannot see or change the caller's variables); `vars` is a dict
//! of values handed in as variables (numbers, strings, booleans, null, lists and dicts of those; JSON-like data).
//! It runs under the caller's capabilities, so it can do nothing a script could not already do: with files or
//! network denied by the sandbox, `eval`-ed code is denied too (the fresh VM is given the caller's capability set,
//! not the default trusted one). Nesting is limited to 8 levels. Every evaluation also runs under an instruction
//! budget (50 million steps) and a wall-clock limit (30 s, shared by nested evaluations), so a loop in the text cannot hang
//! the caller.
//!
//! WARNING: `eval` and `exec` run text as code with all the caller's powers. Never pass them text that came from
//! an untrusted source (network, files, users, an LLM) unless the caller itself runs in a restricted sandbox.
//!
//! `eval("1 + 2")` returns the value of one expression. `exec("x = 1\nfn f() { return x }")` runs statements and
//! returns a dict of the names it defined (names starting with an underscore are left out).

use crate::error::VmError;
use crate::value::{SharedDict, Value};
use std::cell::Cell;
use std::collections::HashMap;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

const MAX_DEPTH: u32 = 8;
const MAX_STEPS: u64 = 50_000_000;
const MAX_WALL_MS: u64 = 30_000;

thread_local! {
    /// Deadline of the outermost running evaluation on this thread; nested ones may not outlive it.
    static DEADLINE: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    match name {
        "eval" | "exec" => Some(run(name, args)),
        _ => None,
    }
}

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Killer expression that rebuilds `v` (data only: no functions or objects).
fn literal(v: &Value, who: &str) -> Result<String, VmError> {
    Ok(match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) if n.is_finite() => format!("{:?}", n),
        Value::Integer(i) => i.to_string(),
        Value::Str(s) => quote(s),
        Value::Array(a) => {
            let items: Result<Vec<String>, VmError> = a.to_vec().iter().map(|x| literal(x, who)).collect();
            format!("[{}]", items?.join(", "))
        }
        Value::Dict(d) => {
            let mut keys: Vec<String> = d.keys();
            keys.sort();
            let mut parts = Vec::new();
            for k in keys {
                let item = d.get(&k).unwrap_or(Value::Null);
                parts.push(format!("{}: {}", quote(&k), literal(&item, who)?));
            }
            format!("{{{}}}", parts.join(", "))
        }
        other => return err(format!("{}(): a variable of type {} cannot be passed in (use numbers, strings, lists and dicts)", who, other.type_name())),
    })
}

fn run(who: &str, args: &[Value]) -> Result<Value, VmError> {
    let code = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        Some(other) => return err(format!("{}() expects the code as a string, got {}", who, other.type_name())),
        None => return err(format!("{}() expects the code as a string", who)),
    };
    let mut prelude = String::new();
    let mut injected: Vec<String> = Vec::new();
    match args.get(1) {
        None | Some(Value::Null) => {}
        Some(Value::Dict(d)) => {
            for k in d.keys() {
                if !crate::controlflow::is_ident(&k) {
                    return err(format!("{}(): \"{}\" is not a valid variable name", who, k));
                }
                let v = d.get(&k).unwrap_or(Value::Null);
                prelude.push_str(&format!("{} = {}\n", k, literal(&v, who)?));
                injected.push(k);
            }
        }
        Some(other) => return err(format!("{}() variables must be a dict, got {}", who, other.type_name())),
    }
    let nested = prelude.matches('\n').count();
    let source = if who == "eval" {
        format!("{}__eval_result = ({})\n", prelude, code.trim())
    } else {
        format!("{}{}\n{}", prelude, code, collect_names(&code, &injected))
    };
    let depth = DEPTH.with(|d| d.get());
    if depth >= MAX_DEPTH {
        return err(format!("{}(): nested more than {} levels deep", who, MAX_DEPTH));
    }
    DEPTH.with(|d| d.set(depth + 1));
    let now = std::time::Instant::now();
    let outer_deadline = DEADLINE.with(|d| d.get());
    let deadline = match outer_deadline {
        Some(d) if d < now + std::time::Duration::from_millis(MAX_WALL_MS) => d,
        _ => now + std::time::Duration::from_millis(MAX_WALL_MS),
    };
    DEADLINE.with(|d| d.set(Some(deadline)));
    let wall_ms = (deadline.saturating_duration_since(now).as_millis() as u64).max(1);
    let result = (|| -> Result<Value, VmError> {
        let program = crate::compiler::compile_killer_default(&source).map_err(|e| shift(e, nested))?;
        let mut vm = crate::vm::VirtualMachine::new();
        // a fresh VM defaults to the trusted capability set: hand it the caller's instead, so eval cannot escalate
        vm.set_capabilities(crate::security::current_capabilities());
        vm.set_execution_budget(Some(crate::security::ExecutionBudget { max_instruction_steps: MAX_STEPS, max_wall_ms: wall_ms, max_heap_bytes: 0 }));
        vm.run(&program).map_err(|e| shift(e, nested))?;
        let globals = vm.get_globals();
        if who == "eval" {
            return Ok(globals.get("__eval_result").cloned().unwrap_or(Value::Null));
        }
        let mut out: HashMap<String, Value> = HashMap::new();
        // functions and classes are globals; plain variables were gathered into __exec_ns by `collect_names`
        for (k, v) in &globals {
            if !k.starts_with('_') && !injected.contains(k) && !matches!(k.as_str(), "E" | "INF" | "PI" | "TAU") {
                out.insert(k.clone(), v.clone());
            }
        }
        if let Some(Value::Dict(d)) = globals.get("__exec_ns") {
            for k in d.keys() {
                if let Some(v) = d.get(&k) {
                    out.insert(k, v);
                }
            }
        }
        Ok(Value::Dict(SharedDict::new(out)))
    })();
    DEPTH.with(|d| d.set(depth));
    DEADLINE.with(|d| d.set(outer_deadline));
    result.map_err(|e| VmError::runtime_error(format!("{}(): {}", who, e)))
}

/// A trailing statement `__exec_ns = {"a": a, ...}` listing the plain variables the code assigns at its top level
/// (a line that starts with `name =` in column 0), since ordinary top-level variables are not globals.
fn collect_names(code: &str, injected: &[String]) -> String {
    let mut names: Vec<String> = Vec::new();
    for line in code.lines() {
        let t = line.trim_end();
        let end = t.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(t.len());
        let (name, rest) = t.split_at(end);
        let rest = rest.trim_start();
        let assigns = rest.starts_with('=') && !rest.starts_with("==") && !rest.starts_with("=>");
        if !name.is_empty()
            && assigns
            && !name.starts_with('_')
            && !name.chars().next().unwrap().is_ascii_digit()
            && !injected.iter().any(|i| i == name)
            && !names.iter().any(|n| n == name)
        {
            names.push(name.to_string());
        }
    }
    if names.is_empty() {
        return String::new();
    }
    let items: Vec<String> = names.iter().map(|n| format!("\"{}\": {}", n, n)).collect();
    format!("__exec_ns = {{{}}}\n", items.join(", "))
}

/// Error text mentions lines of the generated source; the code the caller wrote starts after the injected lines.
fn shift(e: VmError, injected_lines: usize) -> VmError {
    if injected_lines == 0 {
        return e;
    }
    VmError::runtime_error(format!("{} (the first {} line(s) are the passed-in variables)", e, injected_lines))
}
