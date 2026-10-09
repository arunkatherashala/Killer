//! Structured logging and `.env` loading.
//!
//! `log_debug/log_info/log_warn/log_error(msg[, fields])` write one line to stderr. The minimum
//! level comes from `KILLER_LOG` (debug, info, warn, error, off; default info) and can be changed
//! at run time with `log_set_level(name)`. With `KILLER_LOG_FORMAT=json` every record is a JSON
//! line `{"ts":..,"level":..,"msg":..,<fields>}`; otherwise `ts LEVEL msg key=value ...`.
//!
//! `load_env(path)` reads a `.env` file (KEY=VALUE, `export KEY=VALUE`, `#` comments, single or
//! double quotes) into the process environment WITHOUT overriding variables that already exist,
//! and returns the names it set (never the values).

use std::io::Write;
use std::sync::atomic::{AtomicI8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::VmError;
use crate::value::{SharedArray, SharedDict, Value};

const DEBUG: i8 = 0;
const INFO: i8 = 1;
const WARN: i8 = 2;
const ERROR: i8 = 3;
const OFF: i8 = 4;

/// -1 = not overridden, follow KILLER_LOG
static OVERRIDE: AtomicI8 = AtomicI8::new(-1);

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

pub fn parse_level(s: &str) -> Option<i8> {
    match s.trim().to_ascii_lowercase().as_str() {
        "debug" | "trace" => Some(DEBUG),
        "info" => Some(INFO),
        "warn" | "warning" => Some(WARN),
        "error" => Some(ERROR),
        "off" | "none" | "quiet" => Some(OFF),
        _ => None,
    }
}

fn min_level() -> i8 {
    let o = OVERRIDE.load(Ordering::Relaxed);
    if o >= 0 {
        return o;
    }
    std::env::var("KILLER_LOG").ok().and_then(|v| parse_level(&v)).unwrap_or(INFO)
}

fn level_name(l: i8) -> &'static str {
    ["debug", "info", "warn", "error"][l.clamp(0, 3) as usize]
}

/// RFC 3339 UTC timestamp with milliseconds, from the system clock (no chrono).
pub fn timestamp(now: SystemTime) -> String {
    let d = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        d.subsec_millis()
    )
}

fn display(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        other => format!("{}", other),
    }
}

/// Quote a field value in text mode when it has spaces, quotes or `=`.
fn text_value(s: &str) -> String {
    if s.is_empty() || s.chars().any(|c| c.is_whitespace() || c == '"' || c == '=') {
        format!("{:?}", s)
    } else {
        s.to_string()
    }
}

/// Build one log line (without the trailing newline).
pub fn format_line(ts: &str, level: i8, msg: &str, fields: &[(String, Value)], json: bool) -> String {
    if json {
        let mut pairs: Vec<(String, Value)> = vec![
            ("ts".to_string(), Value::Str(ts.to_string())),
            ("level".to_string(), Value::Str(level_name(level).to_string())),
            ("msg".to_string(), Value::Str(msg.to_string())),
        ];
        for (k, v) in fields {
            let key = if matches!(k.as_str(), "ts" | "level" | "msg") { format!("{}_", k) } else { k.clone() };
            pairs.push((key, v.clone()));
        }
        crate::json_lang::stringify(&Value::Dict(SharedDict::from_pairs(pairs)), 0)
            .unwrap_or_else(|_| format!("{{\"ts\":\"{}\",\"level\":\"{}\",\"msg\":\"(unserializable)\"}}", ts, level_name(level)))
    } else {
        // keep the log one line per record even if the message has newlines
        let mut line = format!("{} {:<5} {}", ts, level_name(level).to_ascii_uppercase(), msg.replace('\n', "\\n"));
        for (k, v) in fields {
            line.push(' ');
            line.push_str(k);
            line.push('=');
            line.push_str(&text_value(&display(v).replace('\n', "\\n")));
        }
        line
    }
}

fn log(level: i8, args: &[Value], who: &str) -> Result<Value, VmError> {
    let (msg, fields) = match args {
        [m] => (display(m), Vec::new()),
        [m, Value::Dict(d)] => (display(m), d.iter().collect()),
        [m, Value::Null] => (display(m), Vec::new()),
        _ => return err(format!("{}(msg[, fields]) expects a message and an optional dict", who)),
    };
    if level < min_level() {
        return Ok(Value::Null);
    }
    let json = std::env::var("KILLER_LOG_FORMAT").map(|v| v.eq_ignore_ascii_case("json")).unwrap_or(false);
    let line = format_line(&timestamp(SystemTime::now()), level, &msg, &fields, json);
    let _ = writeln!(std::io::stderr().lock(), "{}", line);
    Ok(Value::Null)
}

// --- .env ---------------------------------------------------------------------------

/// Parse `.env` text into (name, value) pairs in file order.
pub fn parse_env(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").map(|s| s.trim_start()).unwrap_or(line);
        let Some(eq) = line.find('=') else { continue };
        let key = line[..eq].trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || key.chars().next().unwrap().is_ascii_digit() {
            continue;
        }
        let rest = line[eq + 1..].trim_start();
        let value = if let Some(q) = rest.strip_prefix('"') {
            let mut v = String::new();
            let mut chars = q.chars();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => v.push('\n'),
                        Some('t') => v.push('\t'),
                        Some('r') => v.push('\r'),
                        Some(o) => v.push(o),
                        None => v.push('\\'),
                    },
                    c => v.push(c),
                }
            }
            v
        } else if let Some(q) = rest.strip_prefix('\'') {
            q.split('\'').next().unwrap_or("").to_string()
        } else {
            // unquoted: an inline comment starts at " #"
            let end = rest.find(" #").or_else(|| rest.find("\t#")).unwrap_or(rest.len());
            rest[..end].trim_end().to_string()
        };
        out.push((key.to_string(), value));
    }
    out
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        "log_debug" => log(DEBUG, args, name),
        "log_info" => log(INFO, args, name),
        "log_warn" => log(WARN, args, name),
        "log_error" => log(ERROR, args, name),
        "log_set_level" => (|| match args {
            [Value::Str(s)] => match parse_level(s) {
                Some(l) => {
                    OVERRIDE.store(l, Ordering::Relaxed);
                    Ok(Value::Null)
                }
                None => err("log_set_level(): level must be debug, info, warn, error or off"),
            },
            _ => err("log_set_level(level) expects a string"),
        })(),
        "load_env" => (|| match args {
            [Value::Str(path)] => {
                crate::security::require_file_read()?;
                let text = std::fs::read_to_string(path).map_err(|e| VmError::runtime_error(format!("load_env: cannot read {}: {}", path, e)))?;
                let mut set = Vec::new();
                for (k, v) in parse_env(&text) {
                    if std::env::var_os(&k).is_none() && !v.contains('\0') {
                        std::env::set_var(&k, &v);
                        set.push(Value::Str(k));
                    }
                }
                Ok(Value::Array(SharedArray::new(set)))
            }
            _ => err("load_env(path) expects a path string"),
        })(),
        _ => return None,
    })
}

pub const NAMES: &[&str] = &["log_debug", "log_info", "log_warn", "log_error", "log_set_level", "load_env"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_parse() {
        assert_eq!(parse_level("DEBUG"), Some(DEBUG));
        assert_eq!(parse_level(" warning "), Some(WARN));
        assert_eq!(parse_level("off"), Some(OFF));
        assert_eq!(parse_level("loud"), None);
    }

    #[test]
    fn timestamps_are_correct_civil_dates() {
        assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(timestamp(UNIX_EPOCH + std::time::Duration::from_millis(1_709_210_096_789)), "2024-02-29T12:34:56.789Z");
        assert_eq!(timestamp(UNIX_EPOCH + std::time::Duration::from_secs(4_102_444_800)), "2100-01-01T00:00:00.000Z");
    }

    #[test]
    fn text_and_json_formats() {
        let fields = vec![("user".to_string(), Value::Str("a b".to_string())), ("n".to_string(), Value::Number(3.0))];
        let t = format_line("T", INFO, "hello", &fields, false);
        assert_eq!(t, "T INFO  hello user=\"a b\" n=3");
        let j = format_line("T", WARN, "hi \"x\"", &fields, true);
        assert_eq!(j, "{\"ts\":\"T\",\"level\":\"warn\",\"msg\":\"hi \\\"x\\\"\",\"user\":\"a b\",\"n\":3}");
        // reserved keys cannot overwrite the envelope
        let j = format_line("T", INFO, "m", &[("level".to_string(), Value::Str("x".to_string()))], true);
        assert!(j.contains("\"level\":\"info\"") && j.contains("\"level_\":\"x\""));
        // multi-line messages stay on one line
        assert!(!format_line("T", INFO, "a\nb", &[], false).contains('\n'));
    }

    #[test]
    fn env_file_parsing() {
        let text = "# comment\nA=1\nexport B = two words # trailing\nC=\"x\\ny\"\nD='single # kept'\n\n1BAD=no\nE=\nnoequals\n";
        let p = parse_env(text);
        let get = |k: &str| p.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!(get("A").as_deref(), Some("1"));
        assert_eq!(get("B").as_deref(), Some("two words"));
        assert_eq!(get("C").as_deref(), Some("x\ny"));
        assert_eq!(get("D").as_deref(), Some("single # kept"));
        assert_eq!(get("E").as_deref(), Some(""));
        assert!(get("1BAD").is_none());
        assert_eq!(p.len(), 5);
    }
}
