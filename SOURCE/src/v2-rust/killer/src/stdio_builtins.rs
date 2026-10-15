//! Standard error output: `eprintln(values...)` and `eprint(values...)`.
//!
//! Command-line tools need to report errors on stderr while keeping stdout clean for data.
//! Values are joined with single spaces exactly like `println`; `eprintln` appends a newline and
//! `eprint` does not. `stdout_write(values...)` writes to stdout WITHOUT a newline (`print` and
//! `println` both end the line) and `stdout_flush()` flushes it. The stderr writers flush stdout
//! first so interleaved output keeps its order on a terminal.

use std::io::Write;

use crate::error::VmError;
use crate::value::Value;

pub const NAMES: &[&str] = &["eprintln", "eprint", "stdout_write", "stdout_flush"];

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !NAMES.contains(&name) {
        return None;
    }
    // `print` always ends the line, so a prompt or progress bar needs a writer that does not.
    if name == "stdout_write" {
        let text = args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ");
        let mut out = std::io::stdout();
        let _ = write!(out, "{}", text);
        return Some(Ok(Value::Null));
    }
    if name == "stdout_flush" {
        let _ = std::io::stdout().flush();
        return Some(Ok(Value::Null));
    }
    let text = args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ");
    let _ = std::io::stdout().flush();
    let mut err = std::io::stderr();
    let _ = if name == "eprintln" { writeln!(err, "{}", text) } else { write!(err, "{}", text) };
    let _ = err.flush();
    Some(Ok(Value::Null))
}
