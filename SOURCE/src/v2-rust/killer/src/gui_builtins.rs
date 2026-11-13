//! `gui_open(target)`: open a web address (`http://` / `https://`) or an existing file in the default browser.
//! Together with `serve(...)` this is Killer's window system: a script serves a page and the browser is the window
//! (see `gui_show` in `std/gui.killer`). It needs the process-spawn capability. The target is passed to the
//! operating system opener as one argument, never through a shell.

use crate::error::VmError;
use crate::value::Value;
use std::process::{Command, Stdio};

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if name != "gui_open" {
        return crate::gui_native::call(name, args); // native widgets
    }
    Some(open(args))
}

fn open(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_process_spawn()?;
    let target = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("gui_open() expects an http(s) address or a file path".to_string())),
    };
    let is_url = target.starts_with("http://") || target.starts_with("https://");
    if !is_url && !std::path::Path::new(&target).exists() {
        return Err(VmError::runtime_error(format!("gui_open(): \"{}\" is not an http(s) address or an existing file", target)));
    }
    if target.chars().any(|c| c.is_control()) {
        return Err(VmError::runtime_error("gui_open(): the target contains control characters".to_string()));
    }
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler").arg(&target);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(&target);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(&target);
        c
    };
    match cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        Ok(_) => Ok(Value::Bool(true)),
        Err(e) => Err(VmError::runtime_error(format!("gui_open(): cannot start the system opener: {}", e))),
    }
}
