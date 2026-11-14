//! Safe lookup of the external helper programs the TLS and HTTP builtins run (`openssl`, `curl`, PowerShell).
//!
//! The program is never started by bare name. It is resolved once to an absolute path:
//!   1. `KILLER_OPENSSL` / `KILLER_CURL`, when set, must be an absolute path to an existing file;
//!   2. otherwise the entries of PATH are searched in order, skipping empty and relative entries and the current
//!      directory (also when it is written as an absolute PATH entry).
//!
//! PATH and the override variables are read from a snapshot taken the first time they are needed, or just before
//! a script changes them with `env_set`, so a script cannot redirect the helper by editing PATH afterwards.
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

struct Snapshot {
    path: Option<OsString>,
    openssl: Option<String>,
    curl: Option<String>,
}

static SNAP: OnceLock<Snapshot> = OnceLock::new();
static CACHE: Mutex<Vec<(String, PathBuf)>> = Mutex::new(Vec::new());

fn snap() -> &'static Snapshot {
    SNAP.get_or_init(|| Snapshot {
        path: std::env::var_os("PATH"),
        openssl: std::env::var("KILLER_OPENSSL").ok(),
        curl: std::env::var("KILLER_CURL").ok(),
    })
}

/// Called by `env_set` before it changes the environment so the original PATH is kept.
pub fn freeze_environment() {
    let _ = snap();
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn candidates(tool: &str) -> Vec<String> {
    if cfg!(windows) {
        // only real executables; .bat/.cmd/.com are not accepted
        vec![format!("{}.exe", tool)]
    } else {
        vec![tool.to_string()]
    }
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

/// Pure resolution step (testable): `path_var` is the PATH value, `override_val` the KILLER_* value, `cwd` the
/// current directory to exclude.
pub fn resolve_in(tool: &str, env_name: &str, override_val: Option<&str>, path_var: Option<&std::ffi::OsStr>, cwd: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(o) = override_val.filter(|o| !o.is_empty()) {
        let p = Path::new(o);
        if !p.is_absolute() {
            return Err(format!("{} must be an absolute path to the `{}` program, got '{}'", env_name, tool, o));
        }
        if !is_file(p) {
            return Err(format!("{} points at '{}', which is not a file", env_name, o));
        }
        return Ok(p.to_path_buf());
    }
    let names = candidates(tool);
    if let Some(pv) = path_var {
        for dir in std::env::split_paths(pv) {
            if dir.as_os_str().is_empty() || !dir.is_absolute() {
                continue;
            }
            if let Some(c) = cwd {
                if same_dir(&dir, c) {
                    continue;
                }
            }
            for n in &names {
                let full = dir.join(n);
                if is_file(&full) {
                    return Ok(full);
                }
            }
        }
    }
    Err(format!(
        "the `{}` program was not found in any absolute PATH directory (the current directory and relative PATH entries are never searched); install it or set {} to its absolute path",
        tool, env_name
    ))
}

fn resolve_cached(tool: &str, env_name: &str, ov: &Option<String>) -> Result<PathBuf, String> {
    if let Ok(c) = CACHE.lock() {
        if let Some((_, p)) = c.iter().find(|(t, _)| t == tool) {
            return Ok(p.clone());
        }
    }
    let s = snap();
    let cwd = std::env::current_dir().ok();
    let p = resolve_in(tool, env_name, ov.as_deref(), s.path.as_deref(), cwd.as_deref())?;
    if let Ok(mut c) = CACHE.lock() {
        c.push((tool.to_string(), p.clone()));
    }
    Ok(p)
}

/// Absolute path of the `openssl` command-line tool.
pub fn openssl() -> Result<PathBuf, String> {
    resolve_cached("openssl", "KILLER_OPENSSL", &snap().openssl)
}

/// Absolute path of the `curl` tool.
pub fn curl() -> Result<PathBuf, String> {
    resolve_cached("curl", "KILLER_CURL", &snap().curl)
}

/// Windows PowerShell: the copy under %SystemRoot% first, then the normal lookup.
#[cfg(windows)]
pub fn powershell() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir")) {
        let p = Path::new(&root).join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        if is_file(&p) {
            return Ok(p);
        }
    }
    resolve_cached("powershell", "KILLER_POWERSHELL", &None)
}
