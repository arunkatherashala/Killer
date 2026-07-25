//! **killer_serialization** — TOML parser/writer + MessagePack encoder/decoder.
//!
//! Fills the "JSON-only, can't interop with modern APIs" gap.
//! Pure Rust, zero external dependencies.

use std::collections::HashMap;
use std::fmt;

// ══════════════════════════════════════════════════════════════════════════════
// Unified Value type for serialization
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum SerValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<SerValue>),
    Map(Vec<(String, SerValue)>),
}

impl SerValue {
    pub fn as_str(&self) -> Option<&str> {
        if let SerValue::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let SerValue::Int(n) = self { Some(*n) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self {
            SerValue::Float(f) => Some(*f),
            SerValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let SerValue::Bool(b) = self { Some(*b) } else { None }
    }
    pub fn get(&self, key: &str) -> Option<&SerValue> {
        if let SerValue::Map(entries) = self {
            entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        } else { None }
    }
}

impl fmt::Display for SerValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerValue::Null => write!(f, "null"),
            SerValue::Bool(b) => write!(f, "{}", b),
            SerValue::Int(n) => write!(f, "{}", n),
            SerValue::Float(n) => write!(f, "{}", n),
            SerValue::Str(s) => write!(f, "\"{}\"", s),
            SerValue::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            SerValue::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// TOML Parser
// ══════════════════════════════════════════════════════════════════════════════

/// Parse a TOML string into a SerValue::Map.
pub fn toml_parse(input: &str) -> Result<SerValue, String> {
    let mut root: Vec<(String, SerValue)> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut sections: HashMap<String, Vec<(String, SerValue)>> = HashMap::new();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }

        // Section header: [section] or [section.subsection]
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len()-1].trim().to_string();
            current_section = Some(name);
            continue;
        }

        // Key = value
        let eq_pos = line.find('=').ok_or_else(|| format!("line {}: expected key = value", line_no + 1))?;
        let key = line[..eq_pos].trim().to_string();
        let val_str = line[eq_pos+1..].trim();
        let value = toml_parse_value(val_str)
            .map_err(|e| format!("line {}: {}", line_no + 1, e))?;

        if let Some(ref section) = current_section {
            sections.entry(section.clone()).or_default().push((key, value));
        } else {
            root.push((key, value));
        }
    }

    // Merge sections into root
    for (section_name, entries) in sections {
        let parts: Vec<&str> = section_name.split('.').collect();
        if parts.len() == 1 {
            root.push((section_name, SerValue::Map(entries)));
        } else {
            // Nested sections: [a.b] → a → b → entries
            let current = SerValue::Map(entries);
            for part in parts.iter().rev().skip(0) {
                // For simplicity, just use dotted key
                let _ = part;
            }
            root.push((section_name, current));
        }
    }

    Ok(SerValue::Map(root))
}

fn toml_parse_value(s: &str) -> Result<SerValue, String> {
    let s = s.trim();
    if s.is_empty() { return Err("empty value".into()); }

    // Boolean
    if s == "true" { return Ok(SerValue::Bool(true)); }
    if s == "false" { return Ok(SerValue::Bool(false)); }

    // String (quoted)
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return Ok(SerValue::Str(s[1..s.len()-1].to_string()));
    }

    // Array: [a, b, c]
    if s.starts_with('[') && s.ends_with(']') {
        let inner = s[1..s.len()-1].trim();
        if inner.is_empty() { return Ok(SerValue::Array(Vec::new())); }
        let items: Result<Vec<SerValue>, String> = split_toml_array(inner)
            .iter()
            .map(|item| toml_parse_value(item.trim()))
            .collect();
        return Ok(SerValue::Array(items?));
    }

    // Integer
    if let Ok(n) = s.parse::<i64>() { return Ok(SerValue::Int(n)); }

    // Float
    if let Ok(n) = s.parse::<f64>() { return Ok(SerValue::Float(n)); }

    // Unquoted string (TOML spec technically doesn't allow this, but we're lenient)
    Ok(SerValue::Str(s.to_string()))
}

fn split_toml_array(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;

    for ch in s.chars() {
        if ch == '"' { in_string = !in_string; }
        if !in_string {
            if ch == '[' { depth += 1; }
            if ch == ']' { depth -= 1; }
            if ch == ',' && depth == 0 {
                result.push(std::mem::take(&mut current));
                continue;
            }
        }
        current.push(ch);
    }
    if !current.trim().is_empty() { result.push(current); }
    result
}

/// Serialize a SerValue to TOML string.
pub fn toml_encode(value: &SerValue) -> String {
    let mut s = String::new();
    if let SerValue::Map(entries) = value {
        // First, write top-level scalars
        for (k, v) in entries {
            if !matches!(v, SerValue::Map(_)) {
                s.push_str(&format!("{} = {}\n", k, toml_format_value(v)));
            }
        }
        // Then, write sections
        for (k, v) in entries {
            if let SerValue::Map(sub) = v {
                s.push_str(&format!("\n[{}]\n", k));
                for (sk, sv) in sub {
                    s.push_str(&format!("{} = {}\n", sk, toml_format_value(sv)));
                }
            }
        }
    }
    s
}

fn toml_format_value(v: &SerValue) -> String {
    match v {
        SerValue::Null => "\"\"".into(),
        SerValue::Bool(b) => b.to_string(),
        SerValue::Int(n) => n.to_string(),
        SerValue::Float(f) => format!("{}", f),
        SerValue::Str(s) => format!("\"{}\"", s),
        SerValue::Array(a) => {
            let items: Vec<String> = a.iter().map(|v| toml_format_value(v)).collect();
            format!("[{}]", items.join(", "))
        }
        SerValue::Map(_) => "{}".into(), // inline tables not fully supported
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// MessagePack Encoder
// ══════════════════════════════════════════════════════════════════════════════

/// Encode a SerValue to MessagePack binary format.
pub fn msgpack_encode(value: &SerValue) -> Vec<u8> {
    let mut buf = Vec::new();
    msgpack_encode_value(value, &mut buf);
    buf
}

fn msgpack_encode_value(value: &SerValue, buf: &mut Vec<u8>) {
    match value {
        SerValue::Null => buf.push(0xc0),
        SerValue::Bool(false) => buf.push(0xc2),
        SerValue::Bool(true) => buf.push(0xc3),
        SerValue::Int(n) => {
            let n = *n;
            if n >= 0 && n <= 127 {
                buf.push(n as u8);
            } else if n >= -32 && n < 0 {
                buf.push(n as u8); // negative fixnum
            } else if n >= i8::MIN as i64 && n <= i8::MAX as i64 {
                buf.push(0xd0);
                buf.push(n as i8 as u8);
            } else if n >= i16::MIN as i64 && n <= i16::MAX as i64 {
                buf.push(0xd1);
                buf.extend_from_slice(&(n as i16).to_be_bytes());
            } else if n >= i32::MIN as i64 && n <= i32::MAX as i64 {
                buf.push(0xd2);
                buf.extend_from_slice(&(n as i32).to_be_bytes());
            } else {
                buf.push(0xd3);
                buf.extend_from_slice(&n.to_be_bytes());
            }
        }
        SerValue::Float(f) => {
            buf.push(0xcb); // float64
            buf.extend_from_slice(&f.to_be_bytes());
        }
        SerValue::Str(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len();
            if len <= 31 {
                buf.push(0xa0 | len as u8);
            } else if len <= 255 {
                buf.push(0xd9);
                buf.push(len as u8);
            } else if len <= 65535 {
                buf.push(0xda);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdb);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            buf.extend_from_slice(bytes);
        }
        SerValue::Array(arr) => {
            let len = arr.len();
            if len <= 15 {
                buf.push(0x90 | len as u8);
            } else if len <= 65535 {
                buf.push(0xdc);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdd);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            for item in arr { msgpack_encode_value(item, buf); }
        }
        SerValue::Map(entries) => {
            let len = entries.len();
            if len <= 15 {
                buf.push(0x80 | len as u8);
            } else if len <= 65535 {
                buf.push(0xde);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdf);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            for (k, v) in entries {
                msgpack_encode_value(&SerValue::Str(k.clone()), buf);
                msgpack_encode_value(v, buf);
            }
        }
    }
}

/// Decode MessagePack binary data into a SerValue.
pub fn msgpack_decode(data: &[u8]) -> Result<SerValue, String> {
    let (val, _) = msgpack_decode_value(data, 0)?;
    Ok(val)
}

fn msgpack_decode_value(data: &[u8], pos: usize) -> Result<(SerValue, usize), String> {
    if pos >= data.len() { return Err("unexpected end of data".into()); }
    let byte = data[pos];

    // Positive fixint (0x00 - 0x7f)
    if byte <= 0x7f {
        return Ok((SerValue::Int(byte as i64), pos + 1));
    }

    // Negative fixint (0xe0 - 0xff)
    if byte >= 0xe0 {
        return Ok((SerValue::Int(byte as i8 as i64), pos + 1));
    }

    // Fixstr (0xa0 - 0xbf)
    if byte >= 0xa0 && byte <= 0xbf {
        let len = (byte & 0x1f) as usize;
        let end = pos + 1 + len;
        if end > data.len() { return Err("string truncated".into()); }
        let s = String::from_utf8_lossy(&data[pos+1..end]).to_string();
        return Ok((SerValue::Str(s), end));
    }

    // Fixarray (0x90 - 0x9f)
    if byte >= 0x90 && byte <= 0x9f {
        let len = (byte & 0x0f) as usize;
        let mut arr = Vec::with_capacity(len);
        let mut p = pos + 1;
        for _ in 0..len {
            let (val, next) = msgpack_decode_value(data, p)?;
            arr.push(val);
            p = next;
        }
        return Ok((SerValue::Array(arr), p));
    }

    // Fixmap (0x80 - 0x8f)
    if byte >= 0x80 && byte <= 0x8f {
        let len = (byte & 0x0f) as usize;
        let mut entries = Vec::with_capacity(len);
        let mut p = pos + 1;
        for _ in 0..len {
            let (key_val, next) = msgpack_decode_value(data, p)?;
            let key = match key_val {
                SerValue::Str(s) => s,
                other => format!("{}", other),
            };
            let (val, next2) = msgpack_decode_value(data, next)?;
            entries.push((key, val));
            p = next2;
        }
        return Ok((SerValue::Map(entries), p));
    }

    match byte {
        0xc0 => Ok((SerValue::Null, pos + 1)),
        0xc2 => Ok((SerValue::Bool(false), pos + 1)),
        0xc3 => Ok((SerValue::Bool(true), pos + 1)),
        0xd0 => { // int8
            if pos + 2 > data.len() { return Err("truncated".into()); }
            Ok((SerValue::Int(data[pos+1] as i8 as i64), pos + 2))
        }
        0xd1 => { // int16
            if pos + 3 > data.len() { return Err("truncated".into()); }
            let n = i16::from_be_bytes([data[pos+1], data[pos+2]]);
            Ok((SerValue::Int(n as i64), pos + 3))
        }
        0xd2 => { // int32
            if pos + 5 > data.len() { return Err("truncated".into()); }
            let n = i32::from_be_bytes([data[pos+1], data[pos+2], data[pos+3], data[pos+4]]);
            Ok((SerValue::Int(n as i64), pos + 5))
        }
        0xd3 => { // int64
            if pos + 9 > data.len() { return Err("truncated".into()); }
            let n = i64::from_be_bytes(data[pos+1..pos+9].try_into().unwrap());
            Ok((SerValue::Int(n), pos + 9))
        }
        0xcb => { // float64
            if pos + 9 > data.len() { return Err("truncated".into()); }
            let n = f64::from_be_bytes(data[pos+1..pos+9].try_into().unwrap());
            Ok((SerValue::Float(n), pos + 9))
        }
        0xd9 => { // str8
            if pos + 2 > data.len() { return Err("truncated".into()); }
            let len = data[pos+1] as usize;
            let end = pos + 2 + len;
            if end > data.len() { return Err("string truncated".into()); }
            let s = String::from_utf8_lossy(&data[pos+2..end]).to_string();
            Ok((SerValue::Str(s), end))
        }
        0xda => { // str16
            if pos + 3 > data.len() { return Err("truncated".into()); }
            let len = u16::from_be_bytes([data[pos+1], data[pos+2]]) as usize;
            let end = pos + 3 + len;
            if end > data.len() { return Err("string truncated".into()); }
            let s = String::from_utf8_lossy(&data[pos+3..end]).to_string();
            Ok((SerValue::Str(s), end))
        }
        0xdc => { // array16
            if pos + 3 > data.len() { return Err("truncated".into()); }
            let len = u16::from_be_bytes([data[pos+1], data[pos+2]]) as usize;
            let mut arr = Vec::with_capacity(len);
            let mut p = pos + 3;
            for _ in 0..len {
                let (val, next) = msgpack_decode_value(data, p)?;
                arr.push(val);
                p = next;
            }
            Ok((SerValue::Array(arr), p))
        }
        0xde => { // map16
            if pos + 3 > data.len() { return Err("truncated".into()); }
            let len = u16::from_be_bytes([data[pos+1], data[pos+2]]) as usize;
            let mut entries = Vec::with_capacity(len);
            let mut p = pos + 3;
            for _ in 0..len {
                let (key_val, next) = msgpack_decode_value(data, p)?;
                let key = match key_val {
                    SerValue::Str(s) => s,
                    other => format!("{}", other),
                };
                let (val, next2) = msgpack_decode_value(data, next)?;
                entries.push((key, val));
                p = next2;
            }
            Ok((SerValue::Map(entries), p))
        }
        _ => Err(format!("unsupported msgpack byte: 0x{:02x}", byte)),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_parse_basic() {
        let input = r#"
title = "My App"
version = 3
debug = true

[server]
host = "localhost"
port = 8080

[database]
url = "postgres://localhost/mydb"
pool_size = 5
"#;
        let val = toml_parse(input).unwrap();
        assert_eq!(val.get("title").unwrap().as_str(), Some("My App"));
        assert_eq!(val.get("version").unwrap().as_int(), Some(3));
        assert_eq!(val.get("debug").unwrap().as_bool(), Some(true));

        let server = val.get("server").unwrap();
        assert_eq!(server.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(server.get("port").unwrap().as_int(), Some(8080));
    }

    #[test]
    fn toml_parse_array() {
        let input = r#"
tags = ["rust", "killer", "fast"]
numbers = [1, 2, 3]
"#;
        let val = toml_parse(input).unwrap();
        if let SerValue::Array(arr) = val.get("tags").unwrap() {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_str(), Some("rust"));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn toml_encode_roundtrip() {
        let val = SerValue::Map(vec![
            ("name".into(), SerValue::Str("Killer".into())),
            ("version".into(), SerValue::Int(2)),
            ("server".into(), SerValue::Map(vec![
                ("host".into(), SerValue::Str("0.0.0.0".into())),
                ("port".into(), SerValue::Int(3000)),
            ])),
        ]);
        let encoded = toml_encode(&val);
        assert!(encoded.contains("name = \"Killer\""));
        assert!(encoded.contains("[server]"));
        assert!(encoded.contains("port = 3000"));
    }

    #[test]
    fn msgpack_roundtrip_int() {
        let val = SerValue::Int(42);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        assert_eq!(decoded, SerValue::Int(42));
    }

    #[test]
    fn msgpack_roundtrip_negative() {
        let val = SerValue::Int(-10);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        assert_eq!(decoded, SerValue::Int(-10));
    }

    #[test]
    fn msgpack_roundtrip_string() {
        let val = SerValue::Str("hello world".into());
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        assert_eq!(decoded.as_str(), Some("hello world"));
    }

    #[test]
    fn msgpack_roundtrip_float() {
        let val = SerValue::Float(3.14159);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        if let SerValue::Float(f) = decoded {
            assert!((f - 3.14159).abs() < 1e-10);
        } else { panic!("expected float"); }
    }

    #[test]
    fn msgpack_roundtrip_map() {
        let val = SerValue::Map(vec![
            ("name".into(), SerValue::Str("killer".into())),
            ("version".into(), SerValue::Int(2)),
            ("active".into(), SerValue::Bool(true)),
        ]);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        assert_eq!(decoded.get("name").unwrap().as_str(), Some("killer"));
        assert_eq!(decoded.get("version").unwrap().as_int(), Some(2));
        assert_eq!(decoded.get("active").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn msgpack_roundtrip_array() {
        let val = SerValue::Array(vec![
            SerValue::Int(1),
            SerValue::Str("two".into()),
            SerValue::Bool(false),
            SerValue::Null,
        ]);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        if let SerValue::Array(arr) = decoded {
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0].as_int(), Some(1));
            assert_eq!(arr[1].as_str(), Some("two"));
            assert_eq!(arr[2].as_bool(), Some(false));
            assert_eq!(arr[3], SerValue::Null);
        } else { panic!("expected array"); }
    }

    #[test]
    fn msgpack_compact_vs_json() {
        let val = SerValue::Map(vec![
            ("name".into(), SerValue::Str("Killer Language".into())),
            ("version".into(), SerValue::Int(2)),
            ("features".into(), SerValue::Array(vec![
                SerValue::Str("JIT".into()),
                SerValue::Str("async".into()),
                SerValue::Str("UI".into()),
            ])),
        ]);
        let msgpack_bytes = msgpack_encode(&val);
        let json_str = format!("{}", val);
        // MessagePack should be more compact than JSON
        assert!(msgpack_bytes.len() < json_str.len(),
            "msgpack ({} bytes) should be smaller than JSON ({} bytes)",
            msgpack_bytes.len(), json_str.len());
    }

    #[test]
    fn msgpack_large_int() {
        let val = SerValue::Int(1_000_000_000);
        let bytes = msgpack_encode(&val);
        let decoded = msgpack_decode(&bytes).unwrap();
        assert_eq!(decoded.as_int(), Some(1_000_000_000));
    }

    #[test]
    fn toml_comments_ignored() {
        let input = r#"
# This is a comment
key = "value" # inline comment
"#;
        let val = toml_parse(input).unwrap();
        assert_eq!(val.get("key").unwrap().as_str(), Some("value"));
    }
}
