// ===============================================================================
// NOVA GALAXY ENGINE v1 — HTTP Client Builtins
// Native HTTP from Killer code — zero external crates, curl subprocess
//
// Builtins (every verb also accepts a headers dict and a timeout number after its
// required arguments, in either order):
//   http_get(url [, headers] [, secs])        → String  (response body)
//   http_get_timeout(url, secs)               → String
//   http_post(url, body [, content_type] [, headers] [, secs])
//   http_post_json(url, json_or_value [, headers] [, secs])
//   http_put / http_patch (url, body ...), http_delete(url ...)
//   http_head(url ...)                        → String  (response headers only)
//   http_status(url ...)                      → Number
//   http_download(url, path ...)              → String
//   http_request({method, url, headers, body, timeout, retries, insecure, allow_http})
//                                             → {status, headers, body}
//
// Security policy (see http_req.rs, where the logic and its unit tests live):
//   • HTTPS-only, except http:// to localhost/127.0.0.1/::1 (or `allow_http: true`)
//   • TLS verification ON (only `insecure: true` in http_request adds curl -k)
//   • URL, headers and body go to curl on stdin (-K -), never on the command line
//   • Timeouts enforced (default 30s, head/status 10s); retries with exponential backoff
// ===============================================================================

use crate::error::VmError;
use crate::http_req::{exec, headers_from, ReqSpec};
use crate::value::Value;

/// Pull an optional headers dict and timeout out of the trailing arguments.
fn extras(args: &[Value], from: usize, default_timeout: f64) -> (Vec<(String, String)>, f64) {
    let mut headers = Vec::new();
    let mut timeout = default_timeout;
    for a in args.iter().skip(from) {
        match a {
            Value::Dict(d) => headers = headers_from(d),
            Value::Number(n) => timeout = *n,
            Value::Integer(n) => timeout = *n as f64,
            _ => {}
        }
    }
    (headers, timeout)
}

fn spec(method: &str, url: &str, headers: Vec<(String, String)>, body: Option<String>, timeout: f64) -> ReqSpec {
    let mut s = ReqSpec::new(method, url);
    s.headers = headers;
    s.body = body;
    s.timeout_s = timeout;
    s
}

fn body_text(r: &crate::http_req::Resp) -> String {
    r.body.trim_end().to_string()
}

fn err_str(e: String) -> Value {
    Value::Str(format!("HTTP Error: {}", e))
}

fn url_arg(args: &[Value], usage: &str) -> Result<String, Value> {
    match args.first() {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(Value::Str(format!("Error: {}", usage))),
    }
}

fn ensure_content_type(h: &mut Vec<(String, String)>, ct: &str) {
    if !h.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
        h.push(("Content-Type".to_string(), ct.to_string()));
    }
}

/// http_get(url [, headers] [, secs]) → String
pub fn builtin_http_get(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_get requires a URL string. Usage: http_get(\"https://...\")") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 30.0);
    Ok(match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_post(url, body [, content_type] [, headers] [, secs]) → String
pub fn builtin_http_post(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_post(url, body) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let body = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    let ct = match args.get(2) {
        Some(Value::Str(s)) => s.clone(),
        _ => "application/x-www-form-urlencoded".to_string(),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, &ct);
    Ok(match exec(&spec("POST", &url, h, Some(body), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_post_json(url, json_string_or_value [, headers] [, secs]) → String
pub fn builtin_http_post_json(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_post_json(url, json) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let json = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        Some(v @ (Value::Dict(_) | Value::Array(_))) => crate::json_lang::stringify(v, 0).map_err(VmError::runtime_error)?,
        _ => return Ok(Value::Str("Error: http_post_json(url, json) requires a JSON body string".to_string())),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, "application/json");
    Ok(match exec(&spec("POST", &url, h, Some(json), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_head(url [, headers] [, secs]) → String  (returns response headers)
pub fn builtin_http_head(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_head requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 10.0);
    let mut s = spec("HEAD", &url, h, None, t);
    s.head = true;
    Ok(match exec(&s) {
        Ok(r) => Value::Str(r.raw_head.trim_end().to_string()),
        Err(e) => err_str(e),
    })
}

/// http_status(url [, headers] [, secs]) → Number  (e.g. 200, 404, 500)
pub fn builtin_http_status(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match url_arg(args, "http_status requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let (h, t) = extras(args, 1, 10.0);
    Ok(match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Value::Number(r.status as f64),
        Err(e) => err_str(e),
    })
}

/// Lower-cased path components with `.`/`..` resolved; accepts both separators and drops a drive/UNC prefix.
fn dest_components(p: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let p = p.trim_start_matches("\\\\?\\");
    for c in p.replace('\\', "/").split('/') {
        let c = c.trim().to_lowercase();
        match c.as_str() {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            _ => out.push(c),
        }
    }
    if out.first().map_or(false, |c| c.len() == 2 && c.ends_with(':')) {
        out.remove(0);
    }
    out
}

fn home_dir_components() -> Option<Vec<String>> {
    std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok().map(|h| dest_components(&h))
}

/// Pure policy check on a normalized destination (components as from `dest_components`).
fn dest_components_denied(c: &[String], home: Option<&[String]>) -> Option<&'static str> {
    const SYSTEM_TOP: &[&str] = &[
        "etc", "bin", "sbin", "usr", "lib", "lib32", "lib64", "boot", "sys", "proc", "dev", "root", "system", "library", "windows", "winnt",
        "program files", "program files (x86)", "programdata", "efi",
    ];
    const RC_FILES: &[&str] = &[
        ".bashrc", ".bash_profile", ".bash_login", ".bash_logout", ".profile", ".zshrc", ".zshenv", ".zprofile", ".zlogin", ".cshrc", ".tcshrc",
        ".kshrc", ".inputrc", "authorized_keys", "known_hosts", ".netrc", ".npmrc", ".pypirc", "microsoft.powershell_profile.ps1",
        "profile.ps1", "microsoft.vscode_profile.ps1", "config.fish", ".gitconfig", "crontab",
    ];
    const SENSITIVE_DIRS: &[&str] = &[".ssh", ".gnupg", ".aws", ".kube", ".docker", "startup", "autostart", "cron.d", "cron.daily", "sudoers.d"];
    if let Some(t) = c.first() {
        if SYSTEM_TOP.contains(&t.as_str()) {
            return Some("system directory");
        }
    }
    if c.iter().any(|x| x.contains("system32") || x.contains("syswow64")) {
        return Some("system directory");
    }
    if c.len() >= 2 && c[0] == "var" && (c[1] == "spool" || c[1] == "run" || c[1] == "lib") {
        return Some("system directory");
    }
    if c.len() >= 2 && c[0] == "private" && (c[1] == "etc" || c[1] == "var") {
        return Some("system directory");
    }
    // the final component is the file; every earlier one is a directory
    if let Some((file, dirs)) = c.split_last() {
        if dirs.iter().any(|d| SENSITIVE_DIRS.contains(&d.as_str())) {
            return Some("startup or credential folder");
        }
        if RC_FILES.contains(&file.as_str()) || file.starts_with("id_rsa") || file.starts_with("id_ecdsa") || file.starts_with("id_ed25519") {
            return Some("shell startup or credential file");
        }
        if let Some(h) = home {
            if c.len() > h.len() && c.starts_with(h) && SENSITIVE_DIRS.contains(&c[h.len()].as_str()) {
                return Some("startup or credential folder");
            }
        }
        if dirs.windows(2).any(|w| w[0] == "start menu" && w[1] == "programs") {
            return Some("startup folder");
        }
    }
    None
}

/// Resolve `path` (`~`, relative parts, `..`, symlinks in the existing part) and refuse destinations
/// under system directories, startup folders, credential folders or shell startup files.
fn download_dest_check(path: &str) -> Result<(), &'static str> {
    let home = home_dir_components();
    let mut expanded = path.to_string();
    if expanded == "~" || expanded.starts_with("~/") || expanded.starts_with("~\\") {
        if let Ok(h) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            expanded = format!("{}{}", h, &expanded[1..]);
        }
    }
    // 1. lexical check of what the script wrote, relative paths resolved against the cwd
    let lexical = if std::path::Path::new(&expanded).is_absolute() || expanded.starts_with('/') || expanded.starts_with('\\') {
        dest_components(&expanded)
    } else {
        let cwd = std::env::current_dir().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        dest_components(&format!("{}/{}", cwd, expanded))
    };
    if let Some(w) = dest_components_denied(&lexical, home.as_deref()) {
        return Err(w);
    }
    // 2. check again after resolving symlinks in the deepest existing ancestor
    let abs = std::path::PathBuf::from(&expanded);
    let abs = if abs.is_absolute() { abs } else { std::env::current_dir().map(|d| d.join(&abs)).unwrap_or(abs) };
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = abs.as_path();
    loop {
        if let Ok(real) = cur.canonicalize() {
            let mut full = real;
            for t in tail.iter().rev() {
                full.push(t);
            }
            let comps = dest_components(&full.to_string_lossy());
            if let Some(w) = dest_components_denied(&comps, home.as_deref()) {
                return Err(w);
            }
            return Ok(());
        }
        match (cur.file_name(), cur.parent()) {
            (Some(n), Some(par)) => {
                tail.push(n.to_os_string());
                cur = par;
            }
            _ => return Ok(()),
        }
    }
}

/// http_download(url, dest_path [, headers] [, secs]) → String  ("Downloaded to: ..." or error)
pub fn builtin_http_download(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    crate::security::require_file_write()?;
    let url = match url_arg(args, "http_download(url, path) requires a URL string") { Ok(u) => u, Err(v) => return Ok(v) };
    let path = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_download(url, path) requires a destination path".to_string())),
    };
    if let Err(why) = download_dest_check(&path) {
        return Ok(err_str(format!("Download destination blocked: {} ({})", path, why)));
    }
    let (h, t) = extras(args, 2, 120.0);
    let mut s = spec("GET", &url, h, None, t);
    s.output = Some(path.clone());
    Ok(match exec(&s) {
        Ok(_) => Value::Str(format!("Downloaded to: {}", path)),
        Err(e) => err_str(e),
    })
}

pub use crate::http_req::builtin_http_request;

fn with_body(who: &str, method: &str, args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let (url, body) = match args {
        [Value::Str(u), Value::Str(b), ..] => (u.clone(), b.clone()),
        [Value::Str(u), v @ (Value::Dict(_) | Value::Array(_)), ..] => {
            (u.clone(), crate::json_lang::stringify(v, 0).map_err(VmError::runtime_error)?)
        }
        _ => return Err(VmError::runtime_error(format!("{}(url, body) expects a string url and body", who))),
    };
    let (mut h, t) = extras(args, 2, 30.0);
    ensure_content_type(&mut h, "application/json");
    Ok(match exec(&spec(method, &url, h, Some(body), t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_put(url, body [, headers] [, secs]) → String
pub fn builtin_http_put(args: &[Value]) -> Result<Value, VmError> { with_body("http_put", "PUT", args) }

/// http_patch(url, body [, headers] [, secs]) → String
pub fn builtin_http_patch(args: &[Value]) -> Result<Value, VmError> { with_body("http_patch", "PATCH", args) }

/// http_delete(url [, headers] [, secs]) → String
pub fn builtin_http_delete(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("http_delete(url) expects a URL string".to_string())),
    };
    let (h, t) = extras(args, 1, 30.0);
    Ok(match exec(&spec("DELETE", &url, h, None, t)) {
        Ok(r) => Value::Str(body_text(&r)),
        Err(e) => err_str(e),
    })
}

/// http_get_json(url [, headers] [, secs]) → Dict  (parses JSON response into a Killer dict/array)
pub fn builtin_http_get_json(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Err(VmError::runtime_error("http_get_json(url) expects a URL string".to_string())),
    };
    let (h, t) = extras(args, 1, 30.0);
    match exec(&spec("GET", &url, h, None, t)) {
        Ok(r) => Ok(crate::json_lang::parse(&r.body).unwrap_or_else(|_| parse_json_to_value(&body_text(&r)))),
        Err(e) => Err(VmError::runtime_error(format!("http_get_json({url}): {e}"))),
    }
}

/// http_with_headers(url, headers_dict [, secs]) → String  (GET with custom headers)
pub fn builtin_http_with_headers(args: &[Value]) -> Result<Value, VmError> {
    if !matches!(args.first(), Some(Value::Str(_))) {
        return Err(VmError::runtime_error("http_with_headers(url, dict) expects a URL".to_string()));
    }
    builtin_http_get(args)
}

// ── Minimal JSON → Value parser ────────────────────────────────────────────

fn parse_json_to_value(s: &str) -> Value {
    let s = s.trim();
    if s.starts_with('{') {
        parse_json_object(s)
    } else if s.starts_with('[') {
        parse_json_array(s)
    } else if s.starts_with('"') && s.ends_with('"') {
        Value::Str(s[1..s.len()-1].replace("\\\"", "\"").replace("\\n", "\n").replace("\\t", "\t"))
    } else if s == "true" {
        Value::Bool(true)
    } else if s == "false" {
        Value::Bool(false)
    } else if s == "null" {
        Value::Null
    } else if let Ok(n) = s.parse::<f64>() {
        Value::Number(n)
    } else {
        Value::Str(s.to_string())
    }
}

fn parse_json_object(s: &str) -> Value {
    let inner = s.trim().trim_start_matches('{').trim_end_matches('}').trim();
    let mut dict = std::collections::HashMap::new();
    if inner.is_empty() { return Value::Dict(crate::value::SharedDict::new(dict)); }
    for pair in split_json_items(inner) {
        if let Some(colon) = pair.find(':') {
            let key_raw = pair[..colon].trim().trim_matches('"');
            let val_raw = pair[colon+1..].trim();
            dict.insert(key_raw.to_string(), parse_json_to_value(val_raw));
        }
    }
    Value::Dict(crate::value::SharedDict::new(dict))
}

fn parse_json_array(s: &str) -> Value {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() { return Value::Array(crate::value::SharedArray::new(vec![])); }
    let items: Vec<Value> = split_json_items(inner).into_iter()
        .map(|item| parse_json_to_value(item.trim()))
        .collect();
    Value::Array(crate::value::SharedArray::new(items))
}

fn split_json_items(s: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_str => { in_str = true; }
            b'"' if in_str && (i == 0 || bytes[i-1] != b'\\') => { in_str = false; }
            b'{' | b'[' if !in_str => depth += 1,
            b'}' | b']' if !in_str => depth -= 1,
            b',' if !in_str && depth == 0 => {
                items.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < s.len() { items.push(&s[start..]); }
    items
}


#[cfg(test)]
mod download_dest_tests {
    use super::*;

    #[test]
    fn dangerous_destinations_are_refused() {
        for p in [
            "~/.ssh/authorized_keys",
            "/usr/lib/x",
            "\\etc\\x",
            "/etc/passwd",
            "/tmp/../etc/cron.d/x",
            "C:\\Windows\\System32\\x.dll",
            "c:/WINDOWS/x.exe",
            "C:\\Users\\bob\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\evil.bat",
            "~/.bashrc",
            "/home/u/.zshrc",
            "foo/../../../../../../../../../../../../../../../../../../../../etc/x",
            "C:\\Users\\bob\\Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1",
            "/Library/x",
        ] {
            assert!(download_dest_check(p).is_err(), "{} should be blocked", p);
        }
    }

    #[test]
    fn ordinary_destinations_are_allowed() {
        let d = std::env::temp_dir().join("killer_dl_ok.bin");
        assert!(download_dest_check(&d.to_string_lossy()).is_ok());
        assert!(download_dest_check("out.bin").is_ok());
        assert!(download_dest_check("data/etcetera/x.txt").is_ok());
    }
}
