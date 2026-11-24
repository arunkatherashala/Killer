//! Hardened curl-backed HTTP client core, shared by every `http_*` builtin.
//!
//! Security properties (all unit-tested):
//! - secrets never reach the process command line: URL, headers and body are sent to curl as a
//!   config file on stdin (`curl -K -`), so API keys do not show up in `ps` or the task manager;
//! - TLS verification is on; `-k` is only added with `insecure: true`;
//! - `https://` is required, except `http://` to localhost / 127.0.0.1 / ::1 (or `allow_http: true`);
//! - SSRF guard: link-local / cloud-metadata targets are always refused, loopback / private ranges unless
//!   `allow_private` or KILLER_ALLOW_PRIVATE_NET=1 (see docs/project/SECURITY.md);
//! - header names/values and URLs containing control characters are rejected (no header injection);
//! - timeouts always apply; retries use exponential backoff on connection errors and 5xx.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::VmError;
use crate::value::{SharedDict, Value};

#[derive(Clone, Debug)]
pub struct ReqSpec {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_s: f64,
    pub retries: u32,
    pub insecure: bool,
    pub allow_http: bool,
    /// HEAD request (curl `--head`)
    pub head: bool,
    /// write the body to this file instead of returning it
    pub output: Option<String>,
    /// allow loopback / private-range targets (default off; env KILLER_ALLOW_PRIVATE_NET=1 also allows)
    pub allow_private: bool,
    /// allow link-local / cloud-metadata targets (Rust-only override; env KILLER_ALLOW_METADATA_NET=1)
    pub allow_metadata: bool,
}

impl ReqSpec {
    pub fn new(method: &str, url: &str) -> Self {
        ReqSpec {
            method: method.to_ascii_uppercase(),
            url: url.trim().to_string(),
            headers: Vec::new(),
            body: None,
            timeout_s: 30.0,
            retries: 0,
            insecure: false,
            allow_http: false,
            head: false,
            output: None,
            allow_private: false,
            allow_metadata: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub raw_head: String,
    pub body: String,
}

fn curl_bin() -> Result<std::path::PathBuf, String> {
    crate::tool_path::curl()
}

/// Host part of a URL (no userinfo, no port, no brackets), lowercase.
pub fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    let auth = &rest[..end];
    let auth = auth.rsplit('@').next()?; // text after the last '@' is what curl connects to
    let host = if let Some(stripped) = auth.strip_prefix('[') {
        stripped.split(']').next()?.to_string()
    } else {
        auth.split(':').next()?.to_string()
    };
    if host.is_empty() { None } else { Some(host.to_ascii_lowercase()) }
}

pub fn is_loopback_host(h: &str) -> bool {
    h == "localhost" || h == "127.0.0.1" || h == "::1"
}

// --- SSRF policy ------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetClass {
    Public,
    Loopback,
    Private,
    /// 169.254.0.0/16, fe80::/10
    LinkLocal,
    /// cloud instance-metadata endpoints (169.254.169.254, fd00:ec2::254, 100.100.100.200)
    Metadata,
}

fn classify_v4(a: std::net::Ipv4Addr) -> NetClass {
    let o = a.octets();
    if o == [169, 254, 169, 254] || o == [100, 100, 100, 200] || o == [192, 0, 0, 192] {
        NetClass::Metadata
    } else if o[0] == 169 && o[1] == 254 {
        NetClass::LinkLocal
    } else if o[0] == 127 || o[0] == 0 {
        NetClass::Loopback
    } else if o[0] == 10
        || (o[0] == 172 && (o[1] & 0xf0) == 16)
        || (o[0] == 192 && o[1] == 168)
        || (o[0] == 100 && (o[1] & 0xc0) == 64)
        || o[0] >= 240
    {
        NetClass::Private
    } else {
        NetClass::Public
    }
}

/// Classify an address; IPv4-mapped (::ffff:a.b.c.d), IPv4-compatible and NAT64 (64:ff9b::/96) forms
/// are judged by the IPv4 address they carry.
pub fn classify_ip(ip: std::net::IpAddr) -> NetClass {
    match ip {
        std::net::IpAddr::V4(a) => classify_v4(a),
        std::net::IpAddr::V6(a) => {
            let seg = a.segments();
            if let Some(v4) = a.to_ipv4_mapped() {
                return classify_v4(v4);
            }
            if seg[..6] == [0, 0, 0, 0, 0, 0] && !(seg[6] == 0 && seg[7] <= 1) {
                return classify_v4(std::net::Ipv4Addr::new((seg[6] >> 8) as u8, seg[6] as u8, (seg[7] >> 8) as u8, seg[7] as u8));
            }
            if seg[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
                return classify_v4(std::net::Ipv4Addr::new((seg[6] >> 8) as u8, seg[6] as u8, (seg[7] >> 8) as u8, seg[7] as u8));
            }
            if a.is_loopback() || a.is_unspecified() {
                NetClass::Loopback
            } else if seg == [0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254] {
                NetClass::Metadata
            } else if (seg[0] & 0xffc0) == 0xfe80 {
                NetClass::LinkLocal
            } else if (seg[0] & 0xfe00) == 0xfc00 {
                NetClass::Private
            } else {
                NetClass::Public
            }
        }
    }
}

/// inet_aton-style IPv4 ("2130706433", "0x7f.1", "0177.0.0.1", "127.1"): curl and libc accept these, so the
/// guard must judge the address they mean.
fn parse_ipv4_loose(host: &str) -> Option<std::net::Ipv4Addr> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut nums = Vec::new();
    for p in &parts {
        let v = if let Some(h) = p.strip_prefix("0x").or_else(|| p.strip_prefix("0X")) {
            u64::from_str_radix(h, 16).ok()?
        } else if p.len() > 1 && p.starts_with('0') {
            u64::from_str_radix(&p[1..], 8).ok()?
        } else {
            p.parse::<u64>().ok()?
        };
        nums.push(v);
    }
    let (last, head) = nums.split_last()?;
    let mut addr: u64 = 0;
    for (i, h) in head.iter().enumerate() {
        if *h > 255 {
            return None;
        }
        addr |= h << (24 - 8 * i as u64);
    }
    let room = 8 * (4 - head.len() as u32);
    if room < 32 && *last >= (1u64 << room) || *last > u32::MAX as u64 {
        return None;
    }
    Some(std::net::Ipv4Addr::from((addr | last) as u32))
}

fn env_on(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1") | Ok("true") | Ok("yes"))
}

/// Port of the URL (explicit, else the scheme default).
pub fn url_port(url: &str) -> u16 {
    let (scheme, rest) = url.split_once("://").unwrap_or(("http", url));
    let end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    let auth = rest[..end].rsplit('@').next().unwrap_or("");
    let after_host = if auth.starts_with('[') { auth.split_once(']').map(|x| x.1).unwrap_or("") } else { &auth[auth.find(':').unwrap_or(auth.len())..] };
    after_host.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(if scheme.eq_ignore_ascii_case("https") { 443 } else { 80 })
}

fn is_metadata_name(h: &str) -> bool {
    let h = h.trim_end_matches('.');
    h == "metadata.google.internal" || h == "metadata" || h == "instance-data" || h.ends_with(".metadata.google.internal")
}

fn check_class(class: NetClass, host: &str, spec: &ReqSpec) -> Result<(), String> {
    match class {
        NetClass::Public => Ok(()),
        NetClass::Metadata | NetClass::LinkLocal => {
            if spec.allow_metadata || env_on("KILLER_ALLOW_METADATA_NET") {
                Ok(())
            } else {
                Err(format!("Security: {} is a link-local / cloud-metadata address and is blocked (SSRF). Set KILLER_ALLOW_METADATA_NET=1 only if you really mean it.", host))
            }
        }
        NetClass::Loopback | NetClass::Private => {
            if spec.allow_private || env_on("KILLER_ALLOW_PRIVATE_NET") {
                Ok(())
            } else {
                Err(format!("Security: {} is a loopback / private-network address and is blocked (SSRF). Pass allow_private: true or set KILLER_ALLOW_PRIVATE_NET=1 to allow it.", host))
            }
        }
    }
}

/// SSRF guard. Checks IP literals, and resolves host names, refusing when any resolved address is blocked.
/// Returns curl `resolve` pins for resolved names so curl connects to exactly the addresses checked (no DNS
/// rebinding between check and connect). curl is run without -L, so redirects are never followed.
pub fn check_target(spec: &ReqSpec) -> Result<Vec<String>, String> {
    let deny = |why: &str| Err(format!("Security: request refused by the SSRF guard: {}.", why));
    // The authority is judged exactly as curl will read it: no control characters, whitespace or backslashes
    // (curl and the URL parsers disagree about what they mean), so nothing can hide a different host.
    let rest = spec.url.split_once("://").map(|x| x.1).unwrap_or("");
    let end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.chars().any(|c| c.is_control() || c.is_whitespace() || c == '\\') {
        return deny("the host part contains control characters, whitespace or a backslash");
    }
    let host = match url_host(&spec.url) {
        Some(h) => h,
        None => return deny("the URL has no host"),
    };
    let bracketed = authority.rsplit('@').next().map_or(false, |a| a.starts_with('['));
    if bracketed {
        // IPv6 literal, optionally with a zone id ("fe80::1%25eth0"): the zone never changes the class.
        let (addr, zone) = match host.split_once('%') {
            Some((a, z)) => (a, Some(z)),
            None => (host.as_str(), None),
        };
        if let Some(z) = zone {
            if z.is_empty() || !z.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
                return deny("invalid IPv6 zone id");
            }
        }
        return match addr.parse::<std::net::Ipv6Addr>() {
            Ok(ip) => {
                check_class(classify_ip(std::net::IpAddr::V6(ip)), &host, spec)?;
                Ok(Vec::new())
            }
            Err(_) => deny("the bracketed host is not a valid IPv6 address"),
        };
    }
    if host.contains('%') {
        return deny("percent-encoded host names are not accepted");
    }
    let host = host.trim_end_matches('.').to_string();
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return deny("the host is not a plain IP address or DNS name");
    }
    if is_metadata_name(&host) {
        check_class(NetClass::Metadata, &host, spec)?;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        check_class(classify_ip(ip), &host, spec)?;
        return Ok(Vec::new());
    }
    if let Some(ip) = parse_ipv4_loose(&host) {
        check_class(classify_v4(ip), &format!("{} ({})", host, ip), spec)?;
        return Ok(Vec::new());
    }
    use std::net::ToSocketAddrs;
    let port = url_port(&spec.url);
    // Fail closed: a name that cannot be resolved here is not handed to curl (curl may read it differently).
    let addrs: Vec<std::net::IpAddr> = match (host.as_str(), port).to_socket_addrs() {
        Ok(it) => it.map(|a| a.ip()).collect(),
        Err(_) => return deny(&format!("cannot resolve host '{}' to check it", host)),
    };
    if addrs.is_empty() {
        return deny(&format!("host '{}' resolves to no address", host));
    }
    for ip in &addrs {
        check_class(classify_ip(*ip), &format!("{} (resolves to {})", host, ip), spec)?;
    }
    let list: Vec<String> = addrs.iter().map(|ip| if ip.is_ipv6() { format!("[{}]", ip) } else { ip.to_string() }).collect();
    Ok(vec![format!("{}:{}:{}", host, port, list.join(","))])
}

/// Validate the URL against the scheme policy.
pub fn validate_url(url: &str, allow_http: bool) -> Result<(), String> {
    let u = url.trim();
    if u.chars().any(|c| c.is_control() || c == ' ') {
        return Err("URL contains spaces or control characters".to_string());
    }
    let shown = &u[..u.char_indices().nth(80).map(|(i, _)| i).unwrap_or(u.len())];
    if u.starts_with("https://") {
        return Ok(());
    }
    if u.starts_with("http://") {
        let host = url_host(u).unwrap_or_default();
        if allow_http || is_loopback_host(&host) {
            return Ok(());
        }
        return Err(format!(
            "Security: HTTPS is required; plain http:// is only allowed for localhost/127.0.0.1/::1 (pass allow_http: true to override).\nGot: {}",
            shown
        ));
    }
    Err(format!("Invalid URL: must start with https:// (or http:// for localhost)\nGot: {}", shown))
}

fn check_header(name: &str, value: &str) -> Result<(), String> {
    let bad = |s: &str| s.chars().any(|c| c == '\r' || c == '\n' || c == '\0');
    if name.is_empty() || bad(name) || bad(value) || name.contains(':') || name.contains(' ') {
        return Err(format!("invalid header '{}'", name));
    }
    Ok(())
}

/// Quote a string for a curl config file (`-K`): backslash escapes `\\ \" \n \r \t \v`.
pub fn cfg_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0b}' => out.push_str("\\v"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The warning printed (to stderr, once per request) when `insecure: true` turns certificate checks off. It names
/// the host only: the URL may carry a key in its query string.
pub fn insecure_warning(spec: &ReqSpec) -> Option<String> {
    if !spec.insecure {
        return None;
    }
    let host = url_host(&spec.url).unwrap_or_else(|| "(unknown host)".to_string());
    Some(format!(
        "warning: TLS certificate verification is DISABLED for {} (insecure: true); the connection can be intercepted or altered",
        host
    ))
}

/// Print [`insecure_warning`] to stderr if it applies. Called once per request by every path that runs curl.
pub fn warn_if_insecure(spec: &ReqSpec) {
    if let Some(w) = insecure_warning(spec) {
        eprintln!("{}", w);
    }
}

/// Command-line arguments for curl. Contains no URL, header or body: only fixed flags and
/// numeric limits, so nothing sensitive can leak through the process list.
pub fn build_argv(spec: &ReqSpec) -> Vec<String> {
    let mut a: Vec<String> = vec!["-s".into(), "-S".into(), "--ssl-no-revoke".into()];
    if spec.output.is_none() {
        a.push("-i".into());
    }
    a.push("--max-time".into());
    a.push(format!("{}", spec.timeout_s.max(0.1)));
    if spec.insecure {
        a.push("-k".into());
    }
    a.push("-K".into());
    a.push("-".into());
    a
}

/// The curl config text (written to curl's stdin): URL, method, headers and body.
pub fn build_config(spec: &ReqSpec) -> Result<String, String> {
    validate_url(&spec.url, spec.allow_http)?;
    let pins = check_target(spec)?;
    let mut c = String::new();
    for pin in &pins {
        c.push_str(&format!("resolve = {}\n", cfg_quote(pin)));
    }
    c.push_str(&format!("url = {}\n", cfg_quote(&spec.url)));
    if spec.head {
        c.push_str("head\n");
    } else if spec.method != "GET" || spec.body.is_some() {
        c.push_str(&format!("request = {}\n", cfg_quote(&spec.method)));
    }
    let mut has_ua = false;
    for (k, v) in &spec.headers {
        check_header(k, v)?;
        if k.eq_ignore_ascii_case("user-agent") {
            has_ua = true;
        }
        c.push_str(&format!("header = {}\n", cfg_quote(&format!("{}: {}", k, v))));
    }
    if !has_ua {
        c.push_str("header = \"User-Agent: KillerLang/2.1\"\n");
    }
    if let Some(b) = &spec.body {
        c.push_str(&format!("data-raw = {}\n", cfg_quote(b)));
    }
    if let Some(o) = &spec.output {
        c.push_str(&format!("output = {}\n", cfg_quote(o)));
    }
    Ok(c)
}

/// Delay before retry number `attempt` (0-based): 200 ms, 400 ms, 800 ms ... capped at 5 s.
pub fn backoff_ms(attempt: u32) -> u64 {
    (200u64 << attempt.min(10)).min(5000)
}

/// Split curl's `-i` output into (status, header lines, raw head, body), skipping 1xx blocks.
pub fn parse_response(raw: &[u8]) -> Result<Resp, String> {
    let mut rest = raw;
    loop {
        let sep = rest.windows(4).position(|w| w == b"\r\n\r\n");
        let (head_bytes, after) = match sep {
            Some(p) => (&rest[..p], &rest[p + 4..]),
            None => match rest.windows(2).position(|w| w == b"\n\n") {
                Some(p) => (&rest[..p], &rest[p + 2..]),
                None => return Err("malformed HTTP response".to_string()),
            },
        };
        let head = String::from_utf8_lossy(head_bytes).into_owned();
        let mut lines = head.lines();
        let status_line = lines.next().unwrap_or("");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("bad status line: {}", status_line))?;
        if (100..200).contains(&status) || (status == 200 && status_line.contains("Connection established")) {
            rest = after;
            continue;
        }
        let headers = lines
            .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())))
            .collect();
        return Ok(Resp { status, headers, raw_head: head, body: String::from_utf8_lossy(after).into_owned() });
    }
}

fn one_attempt(spec: &ReqSpec) -> Result<Resp, (String, bool)> {
    let cfg = build_config(spec).map_err(|e| (e, false))?;
    let curl = curl_bin().map_err(|e| (format!("curl not available: {}", e), false))?;
    let mut child = Command::new(curl)
        .args(build_argv(spec))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (format!("curl not found: {}. Install curl from https://curl.se", e), false))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(cfg.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| (format!("curl failed: {}", e), true))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // exit 6/7/28/35/52/55/56 are connection-level failures worth retrying
        let retry = matches!(out.status.code(), Some(6) | Some(7) | Some(28) | Some(35) | Some(52) | Some(55) | Some(56));
        return Err((format!("HTTP request failed: {}", msg), retry));
    }
    if spec.output.is_some() {
        return Ok(Resp { status: 200, headers: vec![], raw_head: String::new(), body: String::new() });
    }
    parse_response(&out.stdout).map_err(|e| (e, false))
}

/// Execute a request with retries. Retries happen on connection errors and 5xx responses.
pub fn exec(spec: &ReqSpec) -> Result<Resp, String> {
    validate_url(&spec.url, spec.allow_http)?;
    warn_if_insecure(spec);
    let mut attempt = 0u32;
    loop {
        let result = one_attempt(spec);
        let again = match &result {
            Ok(r) => r.status >= 500,
            Err((_, retryable)) => *retryable,
        };
        if again && attempt < spec.retries {
            std::thread::sleep(Duration::from_millis(backoff_ms(attempt)));
            attempt += 1;
            continue;
        }
        return result.map_err(|(e, _)| e);
    }
}

// --- Value helpers --------------------------------------------------------------

pub fn headers_from(d: &SharedDict) -> Vec<(String, String)> {
    d.iter()
        .map(|(k, v)| (k, match v { Value::Str(s) => s, other => format!("{}", other) }))
        .collect()
}

pub fn resp_to_value(r: &Resp) -> Value {
    let h = SharedDict::from_pairs(r.headers.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect());
    Value::Dict(SharedDict::from_pairs(vec![
        ("status".to_string(), Value::Number(r.status as f64)),
        ("headers".to_string(), Value::Dict(h)),
        ("body".to_string(), Value::Str(r.body.clone())),
    ]))
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

/// `http_request({method, url, headers, body, timeout, retries, insecure, allow_http, allow_private})`
pub fn builtin_http_request(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    let d = match args {
        [Value::Dict(d)] => d,
        _ => return Err(VmError::runtime_error("http_request(opts) expects one dict {method, url, headers, body, timeout, retries, insecure}".to_string())),
    };
    let url = match d.get("url") {
        Some(Value::Str(u)) => u,
        _ => return Err(VmError::runtime_error("http_request: opts.url must be a string".to_string())),
    };
    let method = match d.get("method") {
        Some(Value::Str(m)) => m,
        None | Some(Value::Null) => "GET".to_string(),
        _ => return Err(VmError::runtime_error("http_request: opts.method must be a string".to_string())),
    };
    if !method.bytes().all(|c| c.is_ascii_alphabetic()) || method.is_empty() {
        return Err(VmError::runtime_error("http_request: invalid method".to_string()));
    }
    let mut spec = ReqSpec::new(&method, &url);
    if let Some(Value::Dict(h)) = d.get("headers") {
        spec.headers = headers_from(&h);
    }
    spec.body = match d.get("body") {
        None | Some(Value::Null) => None,
        Some(Value::Str(s)) => Some(s),
        Some(v @ (Value::Dict(_) | Value::Array(_))) => {
            if !spec.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                spec.headers.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            Some(crate::json_lang::stringify(&v, 0).map_err(VmError::runtime_error)?)
        }
        Some(other) => Some(format!("{}", other)),
    };
    if let Some(t) = d.get("timeout").as_ref().and_then(num) {
        spec.timeout_s = t;
    }
    if let Some(r) = d.get("retries").as_ref().and_then(num) {
        spec.retries = r.clamp(0.0, 10.0) as u32;
    }
    spec.insecure = matches!(d.get("insecure"), Some(Value::Bool(true)));
    spec.allow_http = matches!(d.get("allow_http"), Some(Value::Bool(true)));
    spec.allow_private = matches!(d.get("allow_private"), Some(Value::Bool(true)));
    match exec(&spec) {
        Ok(r) => Ok(resp_to_value(&r)),
        Err(e) => Err(VmError::runtime_error(format!("http_request: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret_spec() -> ReqSpec {
        let mut s = ReqSpec::new("POST", "https://api.example.com/v1/x?token=URLSECRET");
        s.headers.push(("Authorization".into(), "Bearer SK-TOPSECRET".into()));
        s.headers.push(("x-api-key".into(), "KEYSECRET".into()));
        s.body = Some("{\"prompt\":\"BODYSECRET\"}".into());
        s
    }

    #[test]
    fn no_secret_ever_appears_in_argv() {
        let argv = build_argv(&secret_spec()).join(" ");
        for secret in ["SK-TOPSECRET", "KEYSECRET", "BODYSECRET", "URLSECRET", "Authorization", "api.example.com"] {
            assert!(!argv.contains(secret), "{} leaked into argv: {}", secret, argv);
        }
        assert!(argv.contains("-K -"));
    }

    #[test]
    fn secrets_go_to_the_config_on_stdin() {
        let cfg = build_config(&secret_spec()).unwrap();
        assert!(cfg.contains("Bearer SK-TOPSECRET"));
        assert!(cfg.contains("request = \"POST\""));
        assert!(cfg.contains("data-raw = \"{\\\"prompt\\\":\\\"BODYSECRET\\\"}\""));
    }

    #[test]
    fn tls_verification_is_on_unless_insecure() {
        let mut s = secret_spec();
        assert!(!build_argv(&s).iter().any(|a| a == "-k"));
        s.insecure = true;
        assert!(build_argv(&s).iter().any(|a| a == "-k"));
    }

    #[test]
    fn scheme_policy() {
        assert!(validate_url("https://example.com", false).is_ok());
        assert!(validate_url("http://localhost:8080/x", false).is_ok());
        assert!(validate_url("http://127.0.0.1:1/", false).is_ok());
        assert!(validate_url("http://[::1]:80/", false).is_ok());
        assert!(validate_url("http://example.com", false).is_err());
        assert!(validate_url("http://example.com", true).is_ok());
        assert!(validate_url("http://localhost@evil.com/", false).is_err());
        assert!(validate_url("http://127.0.0.1.evil.com/", false).is_err());
        assert!(validate_url("ftp://x", true).is_err());
        assert!(validate_url("file:///etc/passwd", true).is_err());
        assert!(validate_url("https://a.com/\r\nX: y", false).is_err());
    }

    #[test]
    fn header_injection_is_rejected() {
        let mut s = ReqSpec::new("GET", "https://a.com");
        s.headers.push(("X-A".into(), "v\r\nEvil: 1".into()));
        assert!(build_config(&s).is_err());
        let mut s = ReqSpec::new("GET", "https://a.com");
        s.headers.push(("Bad Name".into(), "v".into()));
        assert!(build_config(&s).is_err());
    }

    #[test]
    fn config_quoting_escapes_specials() {
        assert_eq!(cfg_quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(backoff_ms(0), 200);
        assert_eq!(backoff_ms(1), 400);
        assert_eq!(backoff_ms(2), 800);
        assert_eq!(backoff_ms(10), 5000);
        assert_eq!(backoff_ms(99), 5000);
    }

    #[test]
    fn parses_responses_skipping_100_continue() {
        let raw = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nX-A: b\r\n\r\nhello";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 201);
        assert_eq!(r.body, "hello");
        assert_eq!(r.headers[0], ("content-type".to_string(), "text/plain".to_string()));
    }

    #[test]
    fn insecure_warning_names_the_host_only_and_only_when_insecure() {
        let mut s = ReqSpec::new("GET", "https://user:pw@Example.com:8443/p?key=SECRET");
        assert!(insecure_warning(&s).is_none());
        s.insecure = true;
        let w = insecure_warning(&s).unwrap();
        assert!(w.contains("example.com") && w.contains("DISABLED") && w.contains("insecure: true"), "{w}");
        assert!(!w.contains("SECRET") && !w.contains("pw"), "{w}");
    }

    #[test]
    fn host_extraction() {
        assert_eq!(url_host("https://User:pw@Example.com:8443/p?q=1").as_deref(), Some("example.com"));
        assert_eq!(url_host("http://[::1]:9/").as_deref(), Some("::1"));
    }

    fn blocked(url: &str, private: bool, meta: bool) -> bool {
        let mut s = ReqSpec::new("GET", url);
        s.allow_http = true;
        s.allow_private = private;
        s.allow_metadata = meta;
        build_config(&s).is_err()
    }

    #[test]
    fn ssrf_classification() {
        use std::net::IpAddr;
        let c = |s: &str| classify_ip(s.parse::<IpAddr>().unwrap());
        assert_eq!(c("127.0.0.1"), NetClass::Loopback);
        assert_eq!(c("10.1.2.3"), NetClass::Private);
        assert_eq!(c("172.16.0.1"), NetClass::Private);
        assert_eq!(c("172.32.0.1"), NetClass::Public);
        assert_eq!(c("192.168.1.1"), NetClass::Private);
        assert_eq!(c("169.254.1.1"), NetClass::LinkLocal);
        assert_eq!(c("169.254.169.254"), NetClass::Metadata);
        assert_eq!(c("::1"), NetClass::Loopback);
        assert_eq!(c("fe80::1"), NetClass::LinkLocal);
        assert_eq!(c("fd12::1"), NetClass::Private);
        assert_eq!(c("fd00:ec2::254"), NetClass::Metadata);
        assert_eq!(c("::ffff:127.0.0.1"), NetClass::Loopback);
        assert_eq!(c("::ffff:169.254.169.254"), NetClass::Metadata);
        assert_eq!(c("8.8.8.8"), NetClass::Public);
        assert_eq!(c("2606:4700::1111"), NetClass::Public);
    }

    #[test]
    fn ssrf_blocks_loopback_private_and_metadata_by_default() {
        for u in ["http://127.0.0.1:9/", "http://[::1]:9/", "http://10.0.0.5/", "http://192.168.0.1/", "http://[fc00::1]/",
                  "http://[::ffff:7f00:1]/", "http://[::ffff:127.0.0.1]/", "http://localhost:9/", "http://0.0.0.0/",
                  "http://2130706433/", "http://0x7f.1/", "http://0177.0.0.1/", "http://127.1/", "http://0xa9fea9fe/", "http://169.254.169.254/latest/meta-data/", "http://[fe80::1]/",
                  "http://[fd00:ec2::254]/", "http://metadata.google.internal/computeMetadata/v1/"] {
            assert!(blocked(u, false, false), "{} should be blocked", u);
        }
    }

    #[test]
    fn ssrf_allow_private_does_not_unlock_metadata() {
        assert!(!blocked("http://127.0.0.1:9/", true, false));
        assert!(!blocked("http://[::1]:9/", true, false));
        assert!(!blocked("http://localhost:9/", true, false));
        for u in ["http://169.254.169.254/", "http://[fd00:ec2::254]/", "http://[::ffff:169.254.169.254]/", "http://169.254.1.1/",
                  "http://[fe80::1]/", "http://metadata.google.internal/"] {
            assert!(blocked(u, true, false), "{} must stay blocked with allow_private", u);
            assert!(!blocked(u, true, true) || u.contains("metadata.google"), "{} should unlock with allow_metadata", u);
        }
    }

    #[test]
    fn ssrf_public_ip_literals_pass_and_error_mentions_option() {
        assert!(!blocked("https://8.8.8.8/", false, false));
        let mut s = ReqSpec::new("GET", "http://127.0.0.1:9/");
        s.allow_http = true;
        let e = build_config(&s).unwrap_err();
        assert!(e.contains("allow_private") && e.contains("KILLER_ALLOW_PRIVATE_NET"), "{}", e);
    }

    #[test]
    fn url_port_forms() {
        assert_eq!(url_port("https://a.b/x"), 443);
        assert_eq!(url_port("http://a.b/x"), 80);
        assert_eq!(url_port("http://u:p@a.b:8081/x"), 8081);
        assert_eq!(url_port("http://[::1]:7000/"), 7000);
        assert_eq!(url_port("http://[::1]/"), 80);
    }
}
