//! Everyday builtins every language ships: character codes, padding, conversions, logarithms,
//! gcd/lcm, statistics, environment/process access, base64/HMAC, sets, dict/array editing.
//!
//! `call` returns `None` for names it does not own, so the main dispatcher can fall through to its
//! own table. All functions are pure Rust with no dependencies.

use crate::error::VmError;
use crate::value::{SetKey, Value};

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn num(v: &Value, who: &str) -> Result<f64, VmError> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Integer(i) => Ok(*i as f64),
        other => err(format!("{}() expects a number, got {}", who, other.type_name())),
    }
}

fn int(v: &Value, who: &str) -> Result<i64, VmError> {
    let n = num(v, who)?;
    if n.fract() != 0.0 || !n.is_finite() || n.abs() > 9.0e15 {
        return err(format!("{}() expects whole numbers, got {}", who, n));
    }
    Ok(n as i64)
}

fn string<'a>(v: &'a Value, who: &str) -> Result<&'a str, VmError> {
    match v {
        Value::Str(s) => Ok(s.as_str()),
        other => err(format!("{}() expects a string, got {}", who, other.type_name())),
    }
}

fn numbers(v: &Value, who: &str) -> Result<Vec<f64>, VmError> {
    match v {
        Value::Array(a) => a.to_vec().iter().map(|x| num(x, who)).collect(),
        other => err(format!("{}() expects an array of numbers, got {}", who, other.type_name())),
    }
}

/// Truthiness shared with the language: null, false, 0, "", [] and {} are false.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
        Value::Integer(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Dict(d) => !d.is_empty(),
        Value::Set(s) => !s.is_empty(),
        Value::Trit(t) => *t > 0,
        _ => true,
    }
}

/// Number of characters (not bytes). ASCII strings take the O(1) path.
pub fn char_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

/// The `i`-th character as a string; `None` when out of range.
pub fn char_at(s: &str, i: usize) -> Option<String> {
    if s.is_ascii() {
        s.as_bytes().get(i).map(|b| (*b as char).to_string())
    } else {
        s.chars().nth(i).map(|c| c.to_string())
    }
}

/// Resolve slice bounds like Python: negatives count from the end, `null` means open, and
/// everything is clamped into `0..=len`.
pub fn slice_bounds(start: &Value, end: &Value, len: usize) -> Result<(usize, usize), VmError> {
    let resolve = |v: &Value, default: i64| -> Result<i64, VmError> {
        match v {
            Value::Null => Ok(default),
            Value::Number(n) if n.is_finite() => Ok(n.trunc() as i64),
            Value::Integer(i) => Ok(*i),
            other => err(format!("slice bounds must be numbers, got {}", other.type_name())),
        }
    };
    let len_i = len as i64;
    let norm = |i: i64| -> usize {
        let i = if i < 0 { i + len_i } else { i };
        i.clamp(0, len_i) as usize
    };
    Ok((norm(resolve(start, 0)?), norm(resolve(end, len_i)?)))
}

fn set_from(items: &[Value]) -> Result<Value, VmError> {
    let mut set = std::collections::BTreeSet::new();
    for item in items {
        match SetKey::from_value(item) {
            Some(k) => {
                set.insert(k);
            }
            None => return err(format!("set() elements must be numbers, strings or booleans, got {}", item.type_name())),
        }
    }
    Ok(Value::Set(Box::new(set)))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0;
    let mut padding = 0;
    for c in text.bytes().filter(|c| !c.is_ascii_whitespace()) {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                padding += 1;
                continue;
            }
            _ => return Err(format!("invalid base64 character '{}'", c as char)),
        };
        if padding > 0 {
            return Err("invalid base64: data after padding".to_string());
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let sha = crate::builtin::sha256_digest;
    let mut k = if key.len() > 64 { sha(key).to_vec() } else { key.to_vec() };
    k.resize(64, 0);
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(data);
    let inner_hash = sha(&inner);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&inner_hash);
    sha(&outer)
}

fn bytes_of(v: &Value, who: &str) -> Result<Vec<u8>, VmError> {
    match v {
        Value::Str(s) => Ok(s.as_bytes().to_vec()),
        Value::Bytes(b) => Ok(b.clone()),
        other => err(format!("{}() expects a string or bytes, got {}", who, other.type_name())),
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn arity<'a>(who: &str, args: &'a [Value], min: usize, max: usize) -> Result<&'a [Value], VmError> {
    if args.len() < min || args.len() > max {
        return if min == max {
            err(format!("{}() expects {} argument(s), got {}", who, min, args.len()))
        } else {
            err(format!("{}() expects {} to {} arguments, got {}", who, min, max, args.len()))
        };
    }
    Ok(args)
}

/// Dispatch a builtin implemented in this module. `None` means "not mine".
pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        // ---- characters and strings ----
        "ord" => (|| {
            let a = arity("ord", args, 1, 1)?;
            let s = string(&a[0], "ord")?;
            match s.chars().next() {
                Some(c) => Ok(Value::Number(c as u32 as f64)),
                None => err("ord() of an empty string"),
            }
        })(),
        "chr" => (|| {
            let a = arity("chr", args, 1, 1)?;
            let n = int(&a[0], "chr")?;
            match u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c) => Ok(Value::Str(c.to_string())),
                None => err(format!("chr() of {} is not a valid character", n)),
            }
        })(),
        "pad_left" | "pad_right" => (|| {
            let a = arity(name, args, 2, 3)?;
            let s = string(&a[0], name)?;
            let width = int(&a[1], name)?.max(0) as usize;
            let fill = match a.get(2) {
                Some(Value::Str(f)) if f.chars().count() == 1 => f.chars().next().unwrap(),
                None => ' ',
                _ => return err(format!("{}() fill must be a single character", name)),
            };
            let len = char_len(s);
            let pad: String = std::iter::repeat(fill).take(width.saturating_sub(len)).collect();
            Ok(Value::Str(if name == "pad_left" { format!("{}{}", pad, s) } else { format!("{}{}", s, pad) }))
        })(),
        "repeat" => (|| {
            let a = arity("repeat", args, 2, 2)?;
            let s = string(&a[0], "repeat")?;
            let n = int(&a[1], "repeat")?;
            if n < 0 || (n as u128) * (s.len() as u128) > 1 << 28 {
                return err("repeat() count out of range");
            }
            Ok(Value::Str(s.repeat(n as usize)))
        })(),
        "strip" | "lstrip" | "rstrip" => (|| {
            let a = arity(name, args, 1, 1)?;
            let s = string(&a[0], name)?;
            Ok(Value::Str(match name {
                "strip" => s.trim(),
                "lstrip" => s.trim_start(),
                _ => s.trim_end(),
            }.to_string()))
        })(),
        "startsWith" | "endsWith" => (|| {
            let a = arity(name, args, 2, 2)?;
            let (s, p) = (string(&a[0], name)?, string(&a[1], name)?);
            Ok(Value::Bool(if name == "startsWith" { s.starts_with(p) } else { s.ends_with(p) }))
        })(),
        "base64_encode" => (|| {
            let a = arity("base64_encode", args, 1, 1)?;
            Ok(Value::Str(base64_encode(&bytes_of(&a[0], "base64_encode")?)))
        })(),
        "base64_decode" => (|| {
            let a = arity("base64_decode", args, 1, 1)?;
            let bytes = base64_decode(string(&a[0], "base64_decode")?).map_err(VmError::runtime_error)?;
            Ok(match String::from_utf8(bytes) {
                Ok(s) => Value::Str(s),
                Err(e) => Value::Bytes(e.into_bytes()),
            })
        })(),
        "hmac_sha256" => (|| {
            let a = arity("hmac_sha256", args, 2, 2)?;
            let mac = hmac_sha256(&bytes_of(&a[0], "hmac_sha256")?, &bytes_of(&a[1], "hmac_sha256")?);
            Ok(Value::Str(mac.iter().map(|b| format!("{:02x}", b)).collect()))
        })(),

        // ---- conversions ----
        "float" => (|| {
            let a = arity("float", args, 1, 1)?;
            match &a[0] {
                Value::Number(n) => Ok(Value::Number(*n)),
                Value::Integer(i) => Ok(Value::Number(*i as f64)),
                Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
                Value::Str(s) => match s.trim().parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n)),
                    Err(_) => err(format!("float(): cannot convert \"{}\" to a number", s)),
                },
                other => err(format!("float() cannot convert {}", other.type_name())),
            }
        })(),
        "bool" => (|| {
            let a = arity("bool", args, 1, 1)?;
            Ok(Value::Bool(truthy(&a[0])))
        })(),

        // ---- math ----
        "log" => (|| {
            let a = arity("log", args, 1, 2)?;
            let x = num(&a[0], "log")?;
            Ok(Value::Number(match a.get(1) {
                Some(b) => x.ln() / num(b, "log")?.ln(),
                None => x.ln(),
            }))
        })(),
        "log2" | "log10" | "exp" | "cbrt" | "trunc" | "sign" => (|| {
            let a = arity(name, args, 1, 1)?;
            let x = num(&a[0], name)?;
            Ok(Value::Number(match name {
                "log2" => x.log2(),
                "log10" => x.log10(),
                "exp" => x.exp(),
                "cbrt" => x.cbrt(),
                "trunc" => x.trunc(),
                _ => {
                    if x > 0.0 {
                        1.0
                    } else if x < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                }
            }))
        })(),
        "atan2" | "hypot" => (|| {
            let a = arity(name, args, 2, 2)?;
            let (y, x) = (num(&a[0], name)?, num(&a[1], name)?);
            Ok(Value::Number(if name == "atan2" { y.atan2(x) } else { y.hypot(x) }))
        })(),
        "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "degrees" | "radians" => (|| {
            let a = arity(name, args, 1, 1)?;
            let x = num(&a[0], name)?;
            Ok(Value::Number(match name {
                "asin" => x.asin(),
                "acos" => x.acos(),
                "atan" => x.atan(),
                "sinh" => x.sinh(),
                "cosh" => x.cosh(),
                "tanh" => x.tanh(),
                "degrees" => x.to_degrees(),
                _ => x.to_radians(),
            }))
        })(),
        "gcd" | "lcm" => (|| {
            let a = arity(name, args, 2, 2)?;
            let (x, y) = (int(&a[0], name)?, int(&a[1], name)?);
            if name == "gcd" {
                Ok(Value::Number(gcd(x, y) as f64))
            } else if x == 0 || y == 0 {
                Ok(Value::Number(0.0))
            } else {
                match (x / gcd(x, y)).checked_mul(y) {
                    Some(l) => Ok(Value::Number(l.abs() as f64)),
                    None => err("lcm() overflow"),
                }
            }
        })(),

        // ---- statistics ----
        "mean" | "median" | "variance" | "stdev" | "pvariance" | "pstdev" => (|| {
            let a = arity(name, args, 1, 1)?;
            let xs = numbers(&a[0], name)?;
            if xs.is_empty() {
                return err(format!("{}() of an empty array", name));
            }
            let n = xs.len() as f64;
            let mean = xs.iter().sum::<f64>() / n;
            let ss = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
            let sample = |label: &str| -> Result<f64, VmError> {
                if xs.len() < 2 {
                    err(format!("{}() needs at least two values", label))
                } else {
                    Ok(ss / (n - 1.0))
                }
            };
            Ok(Value::Number(match name {
                "mean" => mean,
                "median" => {
                    let mut s = xs.clone();
                    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let m = s.len() / 2;
                    if s.len() % 2 == 1 { s[m] } else { (s[m - 1] + s[m]) / 2.0 }
                }
                "variance" => sample("variance")?,
                "stdev" => sample("stdev")?.sqrt(),
                "pvariance" => ss / n,
                _ => (ss / n).sqrt(),
            }))
        })(),

        // ---- environment and process ----
        "env" => (|| {
            let a = arity("env", args, 1, 2)?;
            let key = string(&a[0], "env")?;
            match std::env::var(key) {
                Ok(v) => Ok(Value::Str(v)),
                Err(_) => Ok(a.get(1).cloned().unwrap_or(Value::Null)),
            }
        })(),
        "args" => (|| {
            arity("args", args, 0, 0)?;
            // arguments after the script path (flags such as --run are not the program's own)
            let all: Vec<String> = std::env::args().collect();
            let script_at = all.iter().position(|a| a.ends_with(".killer") || a.ends_with(".kl"));
            let rest: Vec<Value> = match script_at {
                Some(i) => all[i + 1..].iter().filter(|a| a.as_str() != "--run").map(|a| Value::Str(a.clone())).collect(),
                None => Vec::new(),
            };
            Ok(Value::from(rest))
        })(),
        "exit" => (|| {
            let a = arity("exit", args, 0, 1)?;
            let code = match a.first() {
                Some(v) => int(v, "exit")? as i32,
                None => 0,
            };
            use std::io::Write;
            let _ = std::io::stdout().flush();
            std::process::exit(code)
        })(),
        "sleep" => (|| {
            let a = arity("sleep", args, 1, 1)?;
            let seconds = num(&a[0], "sleep")?;
            if !(0.0..=3600.0).contains(&seconds) {
                return err("sleep() seconds must be between 0 and 3600");
            }
            std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
            Ok(Value::Null)
        })(),
        "assert" => (|| {
            let a = arity("assert", args, 1, 2)?;
            if truthy(&a[0]) {
                Ok(Value::Null)
            } else {
                match a.get(1) {
                    Some(m) => err(format!("assertion failed: {}", m)),
                    None => err("assertion failed"),
                }
            }
        })(),

        // ---- collections ----
        "set" => (|| {
            let a = arity("set", args, 0, 1)?;
            match a.first() {
                None => set_from(&[]),
                Some(Value::Array(arr)) => set_from(&arr.to_vec()),
                Some(Value::Str(s)) => set_from(&s.chars().map(|c| Value::Str(c.to_string())).collect::<Vec<_>>()),
                Some(other) => err(format!("set() expects an array or string, got {}", other.type_name())),
            }
        })(),
        "delete" => (|| {
            let a = arity("delete", args, 2, 2)?;
            match (&a[0], &a[1]) {
                (Value::Dict(d), Value::Str(k)) => Ok(d.remove(k).unwrap_or(Value::Null)),
                (Value::Array(arr), idx) => {
                    let i = int(idx, "delete")?;
                    let len = arr.len() as i64;
                    let i = if i < 0 { i + len } else { i };
                    if i < 0 || i >= len {
                        return err(format!("delete() index {} out of range", i));
                    }
                    Ok(arr.drain_range(i as usize, i as usize + 1).into_iter().next().unwrap_or(Value::Null))
                }
                _ => err("delete() expects (dict, key) or (array, index)"),
            }
        })(),
        "insert" => (|| {
            let a = arity("insert", args, 3, 3)?;
            match &a[0] {
                Value::Array(arr) => {
                    let i = int(&a[1], "insert")?;
                    let len = arr.len() as i64;
                    let i = if i < 0 { (i + len).max(0) } else { i.min(len) };
                    arr.insert(i as usize, a[2].clone());
                    Ok(Value::Array(arr.clone()))
                }
                _ => err("insert() expects (array, index, value)"),
            }
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64) -> Value {
        Value::Number(x)
    }
    fn s(x: &str) -> Value {
        Value::Str(x.to_string())
    }
    fn run(name: &str, args: &[Value]) -> Value {
        call(name, args).expect("known builtin").expect("ok")
    }

    #[test]
    fn character_functions() {
        assert_eq!(run("ord", &[s("a")]), n(97.0));
        assert_eq!(run("ord", &[s("é")]), n(233.0));
        assert_eq!(run("chr", &[n(98.0)]), s("b"));
        assert_eq!(run("chr", &[n(8364.0)]), s("€"));
        assert!(call("chr", &[n(-1.0)]).unwrap().is_err());
        assert!(call("ord", &[s("")]).unwrap().is_err());
    }

    #[test]
    fn padding_counts_characters() {
        assert_eq!(run("pad_left", &[s("7"), n(3.0), s("0")]), s("007"));
        assert_eq!(run("pad_right", &[s("ab"), n(4.0)]), s("ab  "));
        assert_eq!(run("pad_left", &[s("héllo"), n(7.0), s("*")]), s("**héllo"));
        assert_eq!(run("pad_left", &[s("toolong"), n(3.0)]), s("toolong"));
    }

    #[test]
    fn conversions() {
        assert_eq!(run("float", &[s("2.5")]), n(2.5));
        assert_eq!(run("float", &[Value::Bool(true)]), n(1.0));
        assert!(call("float", &[s("x")]).unwrap().is_err());
        assert_eq!(run("bool", &[n(0.0)]), Value::Bool(false));
        assert_eq!(run("bool", &[s("")]), Value::Bool(false));
        assert_eq!(run("bool", &[s("0")]), Value::Bool(true));
        assert_eq!(run("bool", &[Value::Null]), Value::Bool(false));
    }

    #[test]
    fn math_functions() {
        assert_eq!(run("gcd", &[n(12.0), n(18.0)]), n(6.0));
        assert_eq!(run("gcd", &[n(-12.0), n(18.0)]), n(6.0));
        assert_eq!(run("gcd", &[n(0.0), n(5.0)]), n(5.0));
        assert_eq!(run("lcm", &[n(4.0), n(6.0)]), n(12.0));
        assert_eq!(run("lcm", &[n(0.0), n(6.0)]), n(0.0));
        assert!(call("gcd", &[n(1.5), n(2.0)]).unwrap().is_err());
        assert_eq!(run("log", &[n(1.0)]), n(0.0));
        assert_eq!(run("log", &[n(8.0), n(2.0)]), n(3.0));
        assert_eq!(run("log10", &[n(1000.0)]), n(3.0));
        assert_eq!(run("exp", &[n(0.0)]), n(1.0));
        assert_eq!(run("sign", &[n(-4.0)]), n(-1.0));
    }

    #[test]
    fn statistics_match_known_values() {
        let xs = Value::from(vec![n(2.0), n(4.0), n(4.0), n(4.0), n(5.0), n(5.0), n(7.0), n(9.0)]);
        assert_eq!(run("mean", &[xs.clone()]), n(5.0));
        assert_eq!(run("median", &[xs.clone()]), n(4.5));
        assert_eq!(run("pstdev", &[xs.clone()]), n(2.0));
        assert_eq!(run("pvariance", &[xs.clone()]), n(4.0));
        match run("stdev", &[xs]) {
            Value::Number(v) => assert!((v - 2.138089935299395).abs() < 1e-12),
            other => panic!("{:?}", other),
        }
        assert!(call("mean", &[Value::from(Vec::new())]).unwrap().is_err());
        assert!(call("stdev", &[Value::from(vec![n(1.0)])]).unwrap().is_err());
    }

    #[test]
    fn base64_round_trips_and_matches_rfc4648() {
        for (plain, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(run("base64_encode", &[s(plain)]), s(enc));
            assert_eq!(run("base64_decode", &[s(enc)]), s(plain));
        }
        assert!(call("base64_decode", &[s("@@@@")]).unwrap().is_err());
    }

    #[test]
    fn hmac_sha256_matches_rfc4231() {
        // RFC 4231 test case 2
        let mac = run("hmac_sha256", &[s("Jefe"), s("what do ya want for nothing?")]);
        assert_eq!(mac, s("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"));
    }

    #[test]
    fn slice_bounds_follow_python_rules() {
        let b = |a: Value, e: Value, len| slice_bounds(&a, &e, len).unwrap();
        assert_eq!(b(n(1.0), n(3.0), 5), (1, 3));
        assert_eq!(b(Value::Null, n(2.0), 5), (0, 2));
        assert_eq!(b(n(2.0), Value::Null, 5), (2, 5));
        assert_eq!(b(n(-2.0), Value::Null, 5), (3, 5));
        assert_eq!(b(n(0.0), n(-1.0), 5), (0, 4));
        assert_eq!(b(n(-99.0), n(99.0), 5), (0, 5));
        assert_eq!(b(n(4.0), n(2.0), 5), (4, 2)); // empty: start > end is handled by the caller
    }

    #[test]
    fn sets_and_collection_editing() {
        let st = run("set", &[Value::from(vec![n(1.0), n(2.0), n(2.0), n(3.0)])]);
        match st {
            Value::Set(inner) => assert_eq!(inner.len(), 3),
            other => panic!("{:?}", other),
        }
        let arr = Value::from(vec![n(10.0), n(20.0), n(30.0)]);
        assert_eq!(run("delete", &[arr.clone(), n(-1.0)]), n(30.0));
        run("insert", &[arr.clone(), n(0.0), n(5.0)]);
        assert_eq!(format!("{}", arr), "[5, 10, 20]");
        assert!(call("delete", &[arr, n(9.0)]).unwrap().is_err());
    }

    #[test]
    fn char_helpers_handle_non_ascii() {
        assert_eq!(char_len("héllo"), 5);
        assert_eq!(char_len("hello"), 5);
        assert_eq!(char_at("héllo", 1), Some("é".to_string()));
        assert_eq!(char_at("hello", 9), None);
    }

    #[test]
    fn unknown_names_fall_through() {
        assert!(call("definitely_not_mine", &[]).is_none());
    }
}
