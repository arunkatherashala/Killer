//! `isDir(path)` and `isFile(path)`: `fileExists` is true for both, and `listDir` returns bare
//! names, so a program walking a folder had no way to tell a sub-folder from a file.
//!
//! Both need the file-read capability (sandboxed scripts may not probe the file system) and
//! return false, never an error, for a path that does not exist.

use crate::error::VmError;
use crate::value::Value;

pub const NAMES: &[&str] = &["isDir", "isFile"];

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !NAMES.contains(&name) {
        return None;
    }
    Some((|| {
        let path = match args {
            [Value::Str(p)] => p,
            _ => return Err(VmError::runtime_error(format!("{}(path) expects one string", name))),
        };
        crate::security::require_file_read()?;
        let meta = std::fs::metadata(path);
        Ok(Value::Bool(match (name, meta) {
            ("isDir", Ok(m)) => m.is_dir(),
            ("isFile", Ok(m)) => m.is_file(),
            _ => false,
        }))
    })())
}
