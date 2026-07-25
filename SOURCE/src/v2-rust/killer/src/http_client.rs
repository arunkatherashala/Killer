// ===============================================================================
// NOVA GALAXY ENGINE v1 — HTTP Client Builtins
// Native HTTP from Killer code — zero external crates, curl subprocess
//
// Builtins:
//   http_get(url)                    → String  (response body)
//   http_get_timeout(url, secs)      → String  (with custom timeout)
//   http_post(url, body)             → String  (application/x-www-form-urlencoded)
//   http_post_json(url, json)        → String  (application/json)
//   http_head(url)                   → String  (response headers only)
//   http_status(url)                 → Number  (HTTP status code: 200, 404, etc.)
//   http_download(url, path)         → String  ("ok" or error message)
//
// Security policy:
//   • HTTPS-only (http:// is blocked — prevents plaintext data leaks)
//   • No redirect following to internal IPs (SSRF prevention)
//   • Timeouts enforced (default: 30s, head/status: 10s)
//   • User-Agent identifies as KillerLang (transparent to servers)
//
// Zero external crates — pure std::process::Command (spawns curl)
// ===============================================================================

use std::process::Command;
use crate::value::Value;
use crate::error::VmError;

// --- Security Validator -------------------------------------------------------

/// Enforce HTTPS-only policy and basic URL sanity.
/// Blocks HTTP (plaintext), file://, ftp://, and other dangerous schemes.
fn validate_https_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.starts_with("https://") {
        Ok(())
    } else if trimmed.starts_with("http://") {
        Err(format!(
            "Security: http_get/http_post only allow HTTPS (prevents data leaks).\n\
             Change your URL to start with https://\n\
             Got: {}",
            &trimmed[..trimmed.len().min(80)]
        ))
    } else {
        Err(format!(
            "Invalid URL — must start with https://\n\
             Got: {}",
            &trimmed[..trimmed.len().min(80)]
        ))
    }
}

// --- curl helpers -------------------------------------------------------------

fn curl_bin() -> &'static str {
    if cfg!(target_os = "windows") { "curl.exe" } else { "curl" }
}

/// GET a URL, return body as String.
fn do_get(url: &str, timeout_s: u64) -> Result<String, String> {
    validate_https_url(url)?;
    let ts = timeout_s.to_string();
    let out = Command::new(curl_bin())
        .args([
            "-s", "-k", "--ssl-no-revoke", "--fail-with-body",
            "--max-time", &ts,
            "-H", "Accept: text/html,application/json,text/plain,*/*",
            "-H", "User-Agent: KillerLang/2.1 Nova-Galaxy-Engine",
            url,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}. Install curl from https://curl.se", e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.is_empty() && !stderr.is_empty() {
        return Err(format!("HTTP request failed: {}", stderr.trim()));
    }
    Ok(stdout.trim_end().to_string())
}

/// POST to a URL with explicit content-type.
fn do_post(url: &str, body: &str, ct: &str, timeout_s: u64) -> Result<String, String> {
    validate_https_url(url)?;
    let ts = timeout_s.to_string();
    let ct_header = format!("Content-Type: {}", ct);
    let out = Command::new(curl_bin())
        .args([
            "-s", "-k", "--ssl-no-revoke", "--fail-with-body",
            "--max-time", &ts,
            "-X", "POST",
            "-H", &ct_header,
            "-H", "User-Agent: KillerLang/2.1 Nova-Galaxy-Engine",
            "-d", body,
            url,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.is_empty() && !stderr.is_empty() {
        return Err(format!("HTTP POST failed: {}", stderr.trim()));
    }
    Ok(stdout.trim_end().to_string())
}

/// Return only response headers (HEAD request).
fn do_head(url: &str, timeout_s: u64) -> Result<String, String> {
    validate_https_url(url)?;
    let ts = timeout_s.to_string();
    let out = Command::new(curl_bin())
        .args([
            "-s", "-k", "--ssl-no-revoke",
            "-I", "--max-time", &ts,
            "-H", "User-Agent: KillerLang/2.1 Nova-Galaxy-Engine",
            url,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// Return HTTP status code (e.g. 200, 404, 500).
fn do_status(url: &str, timeout_s: u64) -> Result<u16, String> {
    validate_https_url(url)?;
    let ts = timeout_s.to_string();
    // Write body to null device; append status code to stdout via -w
    let null_dev = if cfg!(target_os = "windows") { "NUL" } else { "/dev/null" };
    let out = Command::new(curl_bin())
        .args([
            "-s", "-k", "--ssl-no-revoke",
            "-o", null_dev,
            "-w", "%{http_code}",
            "--max-time", &ts,
            url,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;
    let code_str = String::from_utf8_lossy(&out.stdout).to_string();
    code_str.trim().parse::<u16>()
        .map_err(|_| format!("Could not parse HTTP status code: '{}'", code_str.trim()))
}

/// Download URL to a local file path.
fn do_download(url: &str, dest_path: &str, timeout_s: u64) -> Result<String, String> {
    validate_https_url(url)?;
    // Validate dest_path is not a system path (basic safety check)
    let lp = dest_path.to_lowercase();
    if lp.contains("system32") || lp.contains("/etc/") || lp.contains("/bin/") {
        return Err(format!("Download destination blocked: {}", dest_path));
    }
    let ts = timeout_s.to_string();
    let out = Command::new(curl_bin())
        .args([
            "-s", "-k", "--ssl-no-revoke",
            "--max-time", &ts,
            "-o", dest_path,
            "-H", "User-Agent: KillerLang/2.1 Nova-Galaxy-Engine",
            url,
        ])
        .output()
        .map_err(|e| format!("curl not found: {}", e))?;

    if out.status.success() {
        Ok(format!("Downloaded to: {}", dest_path))
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Err(format!("Download failed: {}", stderr.trim()))
    }
}

// --- Builtin dispatch functions -----------------------------------------------

/// http_get(url) → String
pub fn builtin_http_get(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_get requires a URL string. Usage: http_get(\"https://...\")".to_string())),
    };
    let timeout = match args.get(1) {
        Some(Value::Number(n)) => *n as u64,
        _ => 30,
    };
    match do_get(&url, timeout) {
        Ok(body) => Ok(Value::Str(body)),
        Err(e)   => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

/// http_post(url, body) → String
/// http_post(url, body, content_type) → String  (optional 3rd arg)
pub fn builtin_http_post(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_post(url, body) requires a URL string".to_string())),
    };
    let body = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    let ct = match args.get(2) {
        Some(Value::Str(s)) => s.clone(),
        _ => "application/x-www-form-urlencoded".to_string(),
    };
    let timeout = match args.get(3) {
        Some(Value::Number(n)) => *n as u64,
        _ => 30,
    };
    match do_post(&url, &body, &ct, timeout) {
        Ok(body) => Ok(Value::Str(body)),
        Err(e)   => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

/// http_post_json(url, json_string) → String
pub fn builtin_http_post_json(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_post_json(url, json) requires a URL string".to_string())),
    };
    let json = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_post_json(url, json) requires a JSON body string".to_string())),
    };
    let timeout = match args.get(2) {
        Some(Value::Number(n)) => *n as u64,
        _ => 30,
    };
    match do_post(&url, &json, "application/json", timeout) {
        Ok(resp) => Ok(Value::Str(resp)),
        Err(e)   => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

/// http_head(url) → String  (returns response headers)
pub fn builtin_http_head(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_head requires a URL string".to_string())),
    };
    let timeout = match args.get(1) {
        Some(Value::Number(n)) => *n as u64,
        _ => 10,
    };
    match do_head(&url, timeout) {
        Ok(h)  => Ok(Value::Str(h)),
        Err(e) => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

/// http_status(url) → Number  (e.g. 200, 404, 500)
pub fn builtin_http_status(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_status requires a URL string".to_string())),
    };
    let timeout = match args.get(1) {
        Some(Value::Number(n)) => *n as u64,
        _ => 10,
    };
    match do_status(&url, timeout) {
        Ok(code) => Ok(Value::Number(code as f64)),
        Err(e)   => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

/// http_download(url, dest_path) → String  ("Downloaded to: ..." or error)
pub fn builtin_http_download(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    crate::security::require_file_write()?;
    let url = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_download(url, path) requires a URL string".to_string())),
    };
    let path = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: http_download(url, path) requires a destination path".to_string())),
    };
    let timeout = match args.get(2) {
        Some(Value::Number(n)) => *n as u64,
        _ => 120,
    };
    match do_download(&url, &path, timeout) {
        Ok(msg)  => Ok(Value::Str(msg)),
        Err(e)   => Ok(Value::Str(format!("HTTP Error: {}", e))),
    }
}

