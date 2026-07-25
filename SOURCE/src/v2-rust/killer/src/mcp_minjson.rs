//! Minimal JSON parse/stringify for the `killer-mcp` binary (zero external crates).

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(HashMap<String, Json>),
}

impl Json {
    pub fn obj(pairs: &[(&str, Json)]) -> Self {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            m.insert((*k).to_string(), v.clone());
        }
        Json::Obj(m)
    }

    pub fn arr(items: Vec<Json>) -> Self {
        Json::Arr(items)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
}

pub fn parse(input: &str) -> Result<Json, String> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        i: 0,
    };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.i != p.bytes.len() {
        return Err("trailing characters after JSON".into());
    }
    Ok(v)
}

pub fn stringify(v: &Json) -> String {
    let mut s = String::new();
    stringify_into(v, &mut s);
    s
}

fn stringify_into(v: &Json, out: &mut String) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(n) => {
            if n.is_finite() {
                let _ = write!(out, "{n}");
            } else {
                out.push_str("null");
            }
        }
        Json::Str(st) => {
            out.push('"');
            escape_str(st, out);
            out.push('"');
        }
        Json::Arr(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                stringify_into(it, out);
            }
            out.push(']');
        }
        Json::Obj(m) => {
            out.push('{');
            let mut first = true;
            // Sort keys for stable output (optional); HashMap order is random — fine for MCP.
            let mut keys: Vec<_> = m.keys().collect();
            keys.sort();
            for k in keys {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                escape_str(k, out);
                out.push_str("\":");
                stringify_into(m.get(k.as_str()).unwrap(), out);
            }
            out.push('}');
        }
    }
}

fn escape_str(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self
            .bytes
            .get(self.i)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.i).copied()?;
        self.i += 1;
        Some(b)
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        self.skip_ws();
        match self.bump() {
            Some(b) if b == expected => Ok(()),
            _ => Err(format!("expected byte {}", expected as char)),
        }
    }

    fn expect_literal(&mut self, lit: &[u8]) -> Result<(), String> {
        if self.bytes.len() < self.i + lit.len() {
            return Err("unexpected eof".into());
        }
        if &self.bytes[self.i..self.i + lit.len()] != lit {
            return Err("literal mismatch".into());
        }
        self.i += lit.len();
        Ok(())
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::Str(self.parse_string()?)),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(Json::Null)
            }
            _ => Err("invalid JSON value".into()),
        }
    }

    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect_byte(b'{')?;
        self.skip_ws();
        let mut m = HashMap::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(m));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.expect_byte(b':')?;
            let val = self.parse_value()?;
            m.insert(key, val);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err("object parse error".into()),
            }
        }
        Ok(Json::Obj(m))
    }

    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect_byte(b'[')?;
        self.skip_ws();
        let mut arr = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(arr));
        }
        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => break,
                _ => return Err("array parse error".into()),
            }
        }
        Ok(Json::Arr(arr))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        self.parse_string_inner()
    }

    fn parse_string_inner(&mut self) -> Result<String, String> {
        let mut raw: Vec<u8> = Vec::new();
        loop {
            let c = self.bump().ok_or_else(|| "unterminated string".to_string())?;
            match c {
                b'"' => break,
                b'\\' => {
                    let esc = self.bump().ok_or_else(|| "unterminated escape".to_string())?;
                    match esc {
                        b'"' => raw.push(b'"'),
                        b'\\' => raw.push(b'\\'),
                        b'/' => raw.push(b'/'),
                        b'b' => raw.push(0x08),
                        b'f' => raw.push(0x0c),
                        b'n' => raw.push(b'\n'),
                        b'r' => raw.push(b'\r'),
                        b't' => raw.push(b'\t'),
                        b'u' => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                let h = self.bump().ok_or_else(|| "bad \\u".to_string())?;
                                if !h.is_ascii_hexdigit() {
                                    return Err("bad \\u".into());
                                }
                                hex.push(h as char);
                            }
                            let cp = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u".to_string())?;
                            let ch = char::from_u32(cp).ok_or_else(|| "bad codepoint".to_string())?;
                            let mut buf = [0u8; 4];
                            let se = ch.encode_utf8(&mut buf);
                            raw.extend_from_slice(se.as_bytes());
                        }
                        _ => return Err("bad escape".into()),
                    }
                }
                b => raw.push(b),
            }
        }
        String::from_utf8(raw).map_err(|_| "invalid utf-8 in JSON string".into())
    }

    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let digit_start = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        if self.i == digit_start {
            return Err("bad number: expected digit".into());
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let slice = std::str::from_utf8(&self.bytes[start..self.i]).map_err(|_| "number utf8")?;
        let n: f64 = slice.parse().map_err(|_| "bad number".to_string())?;
        Ok(Json::Number(n))
    }
}

