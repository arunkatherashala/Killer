//! JSON for scripts: a strict RFC 8259 parser and a recursive serializer.
//!
//! `json_parse(text)` -> numbers, strings, booleans, null, arrays, dicts (errors carry the
//! line/column of the problem). `json_stringify(value[, indent])` -> compact text, or pretty
//! text when `indent` > 0. Dict keys are written in insertion order (object fields sorted by name).

use crate::error::VmError;
use crate::value::{SharedDict, Value};
use std::collections::HashMap;

const MAX_DEPTH: usize = 512;

struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn fail<T>(&self, msg: &str) -> Result<T, String> {
        let (mut line, mut col) = (1, 1);
        for &b in &self.bytes[..self.pos.min(self.bytes.len())] {
            if b == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        Err(format!("invalid JSON at line {}, column {}: {}", line, col, msg))
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return self.fail("nesting is too deep");
        }
        self.skip_ws();
        match self.bytes.get(self.pos) {
            None => self.fail("unexpected end of input"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            Some(_) => self.fail("unexpected character"),
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.src[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(value)
        } else {
            self.fail("unexpected token")
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.bytes[self.pos] == b'-' {
            self.pos += 1;
        }
        let digits = |p: &mut Parser| {
            let s = p.pos;
            while p.pos < p.bytes.len() && p.bytes[p.pos].is_ascii_digit() {
                p.pos += 1;
            }
            p.pos - s
        };
        match self.bytes.get(self.pos) {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                digits(self);
            }
            _ => return self.fail("expected a digit"),
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if digits(self) == 0 {
                return self.fail("expected digits after the decimal point");
            }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if digits(self) == 0 {
                return self.fail("expected digits in the exponent");
            }
        }
        self.src[start..self.pos]
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| "invalid JSON number".to_string())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return self.fail("truncated \\u escape");
        }
        let h = &self.src[self.pos..self.pos + 4];
        match u32::from_str_radix(h, 16) {
            Ok(v) if h.bytes().all(|b| b.is_ascii_hexdigit()) => {
                self.pos += 4;
                Ok(v)
            }
            _ => self.fail("invalid \\u escape"),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let Some(&b) = self.bytes.get(self.pos) else {
                return self.fail("unterminated string");
            };
            match b {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(&e) = self.bytes.get(self.pos) else {
                        return self.fail("unterminated escape");
                    };
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                if self.src[self.pos..].starts_with("\\u") {
                                    self.pos += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return self.fail("invalid surrogate pair");
                                    }
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                } else {
                                    return self.fail("unpaired surrogate");
                                }
                            } else {
                                hi
                            };
                            match char::from_u32(code) {
                                Some(c) => out.push(c),
                                None => return self.fail("invalid unicode escape"),
                            }
                        }
                        _ => return self.fail("invalid escape"),
                    }
                }
                0..=0x1f => return self.fail("control character in string"),
                _ => {
                    let ch = self.src[self.pos..].chars().next().unwrap();
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Value::from(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::from(items));
                }
                _ => return self.fail("expected ',' or ']'"),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.pos += 1;
        let mut map: Vec<(String, Value)> = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Value::Dict(SharedDict::from_pairs(map)));
        }
        loop {
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b'"') {
                return self.fail("expected a string key");
            }
            let key = self.string()?;
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return self.fail("expected ':'");
            }
            self.pos += 1;
            let v = self.value(depth + 1)?;
            map.push((key, v));
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Dict(SharedDict::from_pairs(map)));
                }
                _ => return self.fail("expected ',' or '}'"),
            }
        }
    }
}

/// Parse JSON text into a Killer value.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { src: text, bytes: text.as_bytes(), pos: 0 };
    let v = p.value(0)?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return p.fail("unexpected data after the JSON value");
    }
    Ok(v)
}

fn escape_into(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn newline(out: &mut String, indent: usize, level: usize) {
    if indent > 0 {
        out.push('\n');
        out.extend(std::iter::repeat(' ').take(indent * level));
    }
}

fn write(out: &mut String, v: &Value, indent: usize, level: usize, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("json_stringify(): value is nested too deeply (or contains itself)".to_string());
    }
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_finite() {
                out.push_str(&n.to_string());
            } else {
                out.push_str("null");
            }
        }
        Value::Integer(i) => out.push_str(&i.to_string()),
        Value::Str(s) => escape_into(out, s),
        Value::Array(a) => {
            let items = a.to_vec();
            if items.is_empty() {
                out.push_str("[]");
                return Ok(());
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, indent, level + 1);
                write(out, item, indent, level + 1, depth + 1)?;
            }
            newline(out, indent, level);
            out.push(']');
        }
        Value::Dict(d) => {
            // insertion order, like Python's json.dumps
            let entries: Vec<(String, Value)> = d.iter().collect();
            write_entries(out, &entries, indent, level, depth)?;
        }
        Value::Object(o) => {
            let mut entries: Vec<(String, Value)> = o.fields().into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            write_entries(out, &entries, indent, level, depth)?;
        }
        Value::Set(s) => {
            let items: Vec<Value> = s.iter().map(|k| k.to_value()).collect();
            write(out, &Value::from(items), indent, level, depth + 1)?;
        }
        // values with no JSON form
        _ => out.push_str("null"),
    }
    Ok(())
}

fn write_entries(out: &mut String, entries: &[(String, Value)], indent: usize, level: usize, depth: usize) -> Result<(), String> {
    if entries.is_empty() {
        out.push_str("{}");
        return Ok(());
    }
    out.push('{');
    for (i, (k, val)) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        newline(out, indent, level + 1);
        escape_into(out, k);
        out.push(':');
        if indent > 0 {
            out.push(' ');
        }
        write(out, val, indent, level + 1, depth + 1)?;
    }
    newline(out, indent, level);
    out.push('}');
    Ok(())
}

/// Serialize `v`; `indent` of 0 gives compact output.
pub fn stringify(v: &Value, indent: usize) -> Result<String, String> {
    let mut out = String::new();
    write(&mut out, v, indent, 0, 0)?;
    Ok(out)
}

pub fn builtin_parse(args: &[Value]) -> Result<Value, VmError> {
    match args {
        [Value::Str(text)] => parse(text).map_err(VmError::runtime_error),
        _ => Err(VmError::runtime_error("json_parse() expects 1 string argument".to_string())),
    }
}

pub fn builtin_stringify(args: &[Value]) -> Result<Value, VmError> {
    let indent = match args.get(1) {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) if *n >= 0.0 && *n <= 16.0 => *n as usize,
        Some(_) => return Err(VmError::runtime_error("json_stringify() indent must be a number from 0 to 16".to_string())),
    };
    match args.first() {
        Some(v) if args.len() <= 2 => stringify(v, indent).map(Value::Str).map_err(VmError::runtime_error),
        _ => Err(VmError::runtime_error("json_stringify() expects (value[, indent])".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(text: &str) -> String {
        stringify(&parse(text).unwrap(), 0).unwrap()
    }

    #[test]
    fn parses_nested_structures() {
        assert_eq!(round(r#"{"a": 1, "b": [1, 2, {"c": null}], "d": "x"}"#), r#"{"a":1,"b":[1,2,{"c":null}],"d":"x"}"#);
        assert_eq!(round("[]"), "[]");
        assert_eq!(round("{}"), "{}");
        assert_eq!(round(" [ true , false ] "), "[true,false]");
    }

    #[test]
    fn numbers_follow_the_grammar() {
        assert_eq!(round("[0, -1, 2.5, 1e3, -1.5E-2]"), "[0,-1,2.5,1000,-0.015]");
        for bad in ["01", "1.", ".5", "+1", "1e", "--1", "NaN"] {
            assert!(parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn strings_unescape_and_escape() {
        assert_eq!(round(r#""a\nb\t\"q\" é 😀""#), "\"a\\nb\\t\\\"q\\\" é 😀\"");
        assert!(parse(r#""\ud83d""#).is_err());
        assert!(parse("\"a\nb\"").is_err());
    }

    #[test]
    fn rejects_malformed_input_with_a_position() {
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "[1] x", "{'a': 1}", "tru"] {
            assert!(parse(bad).is_err(), "{bad:?} should be rejected");
        }
        let msg = parse("{\n  \"a\": ?\n}").unwrap_err();
        assert!(msg.contains("line 2"), "{msg}");
    }

    #[test]
    fn pretty_printing() {
        let v = parse(r#"{"b":[1,2],"a":{}}"#).unwrap();
        assert_eq!(stringify(&v, 2).unwrap(), "{\n  \"b\": [\n    1,\n    2\n  ],\n  \"a\": {}\n}");
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let deep = "[".repeat(2000) + &"]".repeat(2000);
        assert!(parse(&deep).is_err());
    }
}
