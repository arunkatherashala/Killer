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
    /// Every `KILLER_*` variable at snapshot time (polyglot runtime overrides and settings).
    killer_vars: Vec<(String, String)>,
    /// The whole environment at snapshot time (the allowlisted variables a polyglot child gets).
    env: Vec<(OsString, OsString)>,
}

static SNAP: OnceLock<Snapshot> = OnceLock::new();
static CACHE: Mutex<Vec<(String, PathBuf)>> = Mutex::new(Vec::new());

fn snap() -> &'static Snapshot {
    SNAP.get_or_init(|| Snapshot {
        path: std::env::var_os("PATH"),
        openssl: std::env::var("KILLER_OPENSSL").ok(),
        curl: std::env::var("KILLER_CURL").ok(),
        killer_vars: std::env::vars_os()
            .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
            .filter(|(k, _)| k.to_ascii_uppercase().starts_with("KILLER_"))
            .collect(),
        env: std::env::vars_os().collect(),
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

/// A `KILLER_*` variable as it was when the environment snapshot was taken (a script's `env_set`
/// cannot change it afterwards).
pub fn var(name: &str) -> Option<String> {
    snap().killer_vars.iter().find(|(k, _)| if cfg!(windows) { k.eq_ignore_ascii_case(name) } else { k == name }).map(|(_, v)| v.clone())
}

/// Any environment variable as it was when the snapshot was taken (`env_set` cannot change it
/// afterwards). Names are case-insensitive on Windows.
pub fn var_os(name: &str) -> Option<OsString> {
    snap()
        .env
        .iter()
        .find(|(k, _)| if cfg!(windows) { k.to_string_lossy().eq_ignore_ascii_case(name) } else { k.as_os_str() == std::ffi::OsStr::new(name) })
        .map(|(_, v)| v.clone())
}

/// A file, or (Windows) an app execution alias, which `metadata` cannot follow but which can run.
fn is_runnable_file(p: &Path) -> bool {
    match std::fs::metadata(p) {
        Ok(m) => m.is_file(),
        Err(_) => std::fs::symlink_metadata(p).map(|m| !m.is_dir()).unwrap_or(false),
    }
}

/// Validate an override variable: an absolute path to an existing file.
pub fn check_override(env_name: &str, value: &str) -> Result<PathBuf, String> {
    let p = Path::new(value);
    if !p.is_absolute() {
        return Err(format!("{} must be an absolute path, got '{}'", env_name, value));
    }
    if !is_runnable_file(p) {
        return Err(format!("{} points at '{}', which is not a file", env_name, value));
    }
    Ok(p.to_path_buf())
}

/// Pure lookup step of [`find_on_path`] (testable): the first `tool` (Windows: `.exe`, `.cmd`,
/// `.bat`) in an absolute PATH directory that is not the current directory.
pub fn find_in(tool: &str, path_var: Option<&std::ffi::OsStr>, cwd: Option<&Path>) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) {
        if Path::new(tool).extension().is_some() {
            vec![tool.to_string()]
        } else {
            ["exe", "cmd", "bat"].iter().map(|e| format!("{}.{}", tool, e)).collect()
        }
    } else {
        vec![tool.to_string()]
    };
    for dir in std::env::split_paths(path_var?) {
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
            if is_runnable_file(&full) {
                return Some(full);
            }
        }
    }
    None
}

/// Absolute path of a polyglot runtime program found through the PATH snapshot (never the current
/// directory, never a relative PATH entry).
pub fn find_on_path(tool: &str) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok();
    find_in(tool, snap().path.as_deref(), cwd.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_and_current_directory_entries_are_skipped() {
        let base = std::env::temp_dir().join(format!("killer_tp_{}_{}", std::process::id(), line!()));
        let planted = base.join("cwd");
        let real = base.join("real");
        std::fs::create_dir_all(&planted).unwrap();
        std::fs::create_dir_all(&real).unwrap();
        let name = if cfg!(windows) { "fakeexe_zz.exe" } else { "fakeexe_zz" };
        std::fs::write(planted.join(name), b"x").unwrap();
        std::fs::write(real.join(name), b"x").unwrap();
        let pv = std::env::join_paths([std::path::PathBuf::from("."), planted.clone(), real.clone()]).unwrap();
        let got = find_in("fakeexe_zz", Some(&pv), Some(&planted)).unwrap();
        assert_eq!(got, real.join(name));
        assert!(check_override("X", "relative/python").unwrap_err().contains("absolute"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
