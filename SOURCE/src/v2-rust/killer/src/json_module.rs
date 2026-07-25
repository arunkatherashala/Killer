// JSON Module for Killer Language
// JSON parsing, serialization, and validation
// Version: 2.1.0

use crate::value::Value;
use std::collections::HashMap;

/// JSON module providing JSON parsing, serialization, and validation
pub struct JsonModule;

#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

#[derive(Debug, Clone)]
pub enum JsonError {
    ParseError(String),
    InvalidUtf8,
    UnexpectedEof,
    InvalidNumber,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            JsonError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            JsonError::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            JsonError::UnexpectedEof => write!(f, "Unexpected end of file"),
            JsonError::InvalidNumber => write!(f, "Invalid number"),
        }
    }
}

pub type JsonResult<T> = Result<T, JsonError>;

impl JsonModule {
    // ==================== Parsing ====================
    
    /// Parse JSON string to JsonValue
    /// parse("{\"name\": \"John\", \"age\": 30}") => Ok(Object(...))
    pub fn parse(json: &str) -> JsonResult<JsonValue> {
        let mut parser = JsonParser::new(json);
        parser.parse_value()
    }
    
    /// Parse JSON string to Killer Value
    pub fn parse_to_value(json: &str) -> JsonResult<Value> {
        let json_val = Self::parse(json)?;
        Ok(Self::json_to_value(&json_val))
    }
    
    /// Convert JSON value to Killer Value
    fn json_to_value(json: &JsonValue) -> Value {
        match json {
            JsonValue::Null => Value::Null,
            JsonValue::Boolean(b) => Value::Bool(*b),
            JsonValue::Number(n) => Value::Number(*n),
            JsonValue::String(s) => Value::Str(s.clone()),
            JsonValue::Array(arr) => {
                Value::Array(arr.iter().map(Self::json_to_value).collect::<crate::value::SharedArray>())
            },
            JsonValue::Object(obj) => {
                let mut dict = HashMap::new();
                for (k, v) in obj {
                    dict.insert(k.clone(), Self::json_to_value(v));
                }
                Value::Dict(Box::new(dict))
            },
        }
    }
    
    // ==================== Serialization ====================
    
    /// Stringify JsonValue to JSON string
    /// stringify(JsonValue::Number(42.0)) => "42"
    pub fn stringify(value: &JsonValue) -> String {
        Self::stringify_internal(value, false)
    }
    
    /// Stringify with pretty printing (indentation)
    /// stringify_pretty(...) => formatted JSON
    pub fn stringify_pretty(value: &JsonValue) -> String {
        Self::stringify_pretty_internal(value, 0)
    }
    
    fn stringify_internal(value: &JsonValue, _pretty: bool) -> String {
        match value {
            JsonValue::Null => "null".to_string(),
            JsonValue::Boolean(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && !n.is_infinite() {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            },
            JsonValue::String(s) => format!("\"{}\"", Self::escape_string(s)),
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter()
                    .map(|v| Self::stringify_internal(v, false))
                    .collect();
                format!("[{}]", items.join(","))
            },
            JsonValue::Object(obj) => {
                let items: Vec<String> = obj.iter()
                    .map(|(k, v)| format!("\"{}\":{}", Self::escape_string(k), Self::stringify_internal(v, false)))
                    .collect();
                format!("{{{}}}", items.join(","))
            },
        }
    }
    
    fn stringify_pretty_internal(value: &JsonValue, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let next_indent = "  ".repeat(depth + 1);
        
        match value {
            JsonValue::Null => "null".to_string(),
            JsonValue::Boolean(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && !n.is_infinite() {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            },
            JsonValue::String(s) => format!("\"{}\"", Self::escape_string(s)),
            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    let items: Vec<String> = arr.iter()
                        .map(|v| format!("{}{}", next_indent, Self::stringify_pretty_internal(v, depth + 1)))
                        .collect();
                    format!("[\n{}\n{}]", items.join(",\n"), indent)
                }
            },
            JsonValue::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    let items: Vec<String> = obj.iter()
                        .map(|(k, v)| format!("{}\"{}\": {}", next_indent, Self::escape_string(k), Self::stringify_pretty_internal(v, depth + 1)))
                        .collect();
                    format!("{{\n{}\n{}}}", items.join(",\n"), indent)
                }
            },
        }
    }
    
    fn escape_string(s: &str) -> String {
        let mut result = String::new();
        for ch in s.chars() {
            match ch {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(ch),
            }
        }
        result
    }
    
    #[allow(dead_code)]
    fn unescape_string(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next) = chars.peek() {
                    chars.next();
                    match next {
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        '/' => result.push('/'),
                        'b' => result.push('\x08'),
                        'f' => result.push('\x0c'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        _ => {
                            result.push('\\');
                            result.push(next);
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    }
    
    // ==================== Validation ====================
    
    /// Check if string is valid JSON
    /// is_valid("{\"name\": \"John\"}") => true
    pub fn is_valid(json: &str) -> bool {
        Self::parse(json).is_ok()
    }
    
    /// Get JSON type of a value (as string)
    /// type_of(JsonValue::Null) => "null"
    pub fn type_of(value: &JsonValue) -> &'static str {
        match value {
            JsonValue::Null => "null",
            JsonValue::Boolean(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }
    
    // ==================== Access & Manipulation ====================
    
    /// Get value from object by key
    /// get(obj, "name") => Some(JsonValue::String(...))
    pub fn get(obj: &JsonValue, key: &str) -> Option<JsonValue> {
        if let JsonValue::Object(map) = obj {
            map.get(key).cloned()
        } else {
            None
        }
    }
    
    /// Get array element by index
    /// get_at(arr, 0) => Some(JsonValue)
    pub fn get_at(arr: &JsonValue, index: usize) -> Option<JsonValue> {
        if let JsonValue::Array(vec) = arr {
            vec.get(index).cloned()
        } else {
            None
        }
    }
    
    /// Get nested value using path (e.g., "person.name")
    /// get_path(obj, "person.name") => Some(JsonValue::String(...))
    pub fn get_path(obj: &JsonValue, path: &str) -> Option<JsonValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = obj.clone();
        
        for part in parts {
            if let JsonValue::Object(map) = current {
                current = map.get(part)?.clone();
            } else {
                return None;
            }
        }
        
        Some(current)
    }
    
    /// Get array length
    /// length(arr) => Some(3)
    pub fn length(value: &JsonValue) -> Option<usize> {
        match value {
            JsonValue::Array(arr) => Some(arr.len()),
            JsonValue::Object(obj) => Some(obj.len()),
            JsonValue::String(s) => Some(s.len()),
            _ => None,
        }
    }
    
    /// Check if object has key
    /// has_key(obj, "name") => true
    pub fn has_key(obj: &JsonValue, key: &str) -> bool {
        if let JsonValue::Object(map) = obj {
            map.contains_key(key)
        } else {
            false
        }
    }
    
    /// Get all keys from object
    /// keys(obj) => ["name", "age", ...]
    pub fn keys(obj: &JsonValue) -> Vec<String> {
        if let JsonValue::Object(map) = obj {
            map.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

// ==================== JSON Parser ====================

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(input: &str) -> Self {
        JsonParser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }
    
    fn parse_value(&mut self) -> JsonResult<JsonValue> {
        self.skip_whitespace();
        
        if self.pos >= self.chars.len() {
            return Err(JsonError::UnexpectedEof);
        }
        
        match self.chars[self.pos] {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => self.parse_string().map(JsonValue::String),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' | '0'..='9' => self.parse_number(),
            _ => Err(JsonError::ParseError("Unexpected character".to_string())),
        }
    }
    
    fn parse_null(&mut self) -> JsonResult<JsonValue> {
        if self.consume_literal("null") {
            Ok(JsonValue::Null)
        } else {
            Err(JsonError::ParseError("Expected 'null'".to_string()))
        }
    }
    
    fn parse_bool(&mut self) -> JsonResult<JsonValue> {
        if self.consume_literal("true") {
            Ok(JsonValue::Boolean(true))
        } else if self.consume_literal("false") {
            Ok(JsonValue::Boolean(false))
        } else {
            Err(JsonError::ParseError("Expected boolean".to_string()))
        }
    }
    
    fn parse_string(&mut self) -> JsonResult<String> {
        if self.chars[self.pos] != '"' {
            return Err(JsonError::ParseError("Expected '\"'".to_string()));
        }
        
        self.pos += 1;
        let mut result = String::new();
        
        while self.pos < self.chars.len() && self.chars[self.pos] != '"' {
            if self.chars[self.pos] == '\\' {
                self.pos += 1;
                if self.pos >= self.chars.len() {
                    return Err(JsonError::UnexpectedEof);
                }
                match self.chars[self.pos] {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    _ => result.push(self.chars[self.pos]),
                }
            } else {
                result.push(self.chars[self.pos]);
            }
            self.pos += 1;
        }
        
        if self.pos >= self.chars.len() {
            return Err(JsonError::UnexpectedEof);
        }
        
        self.pos += 1;
        Ok(result)
    }
    
    fn parse_number(&mut self) -> JsonResult<JsonValue> {
        let start = self.pos;
        
        if self.pos < self.chars.len() && self.chars[self.pos] == '-' {
            self.pos += 1;
        }
        
        while self.pos < self.chars.len() && self.chars[self.pos].is_numeric() {
            self.pos += 1;
        }
        
        if self.pos < self.chars.len() && self.chars[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_numeric() {
                self.pos += 1;
            }
        }
        
        if self.pos < self.chars.len() && (self.chars[self.pos] == 'e' || self.chars[self.pos] == 'E') {
            self.pos += 1;
            if self.pos < self.chars.len() && (self.chars[self.pos] == '+' || self.chars[self.pos] == '-') {
                self.pos += 1;
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_numeric() {
                self.pos += 1;
            }
        }
        
        let num_str: String = self.chars[start..self.pos].iter().collect();
        num_str.parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| JsonError::InvalidNumber)
    }
    
    fn parse_array(&mut self) -> JsonResult<JsonValue> {
        self.pos += 1;
        self.skip_whitespace();
        
        let mut arr = Vec::new();
        
        if self.pos < self.chars.len() && self.chars[self.pos] == ']' {
            self.pos += 1;
            return Ok(JsonValue::Array(arr));
        }
        
        loop {
            arr.push(self.parse_value()?);
            self.skip_whitespace();
            
            if self.pos >= self.chars.len() {
                return Err(JsonError::UnexpectedEof);
            }
            
            match self.chars[self.pos] {
                ',' => {
                    self.pos += 1;
                    self.skip_whitespace();
                },
                ']' => {
                    self.pos += 1;
                    break;
                },
                _ => return Err(JsonError::ParseError("Expected ',' or ']'".to_string())),
            }
        }
        
        Ok(JsonValue::Array(arr))
    }
    
    fn parse_object(&mut self) -> JsonResult<JsonValue> {
        self.pos += 1;
        self.skip_whitespace();
        
        let mut obj = HashMap::new();
        
        if self.pos < self.chars.len() && self.chars[self.pos] == '}' {
            self.pos += 1;
            return Ok(JsonValue::Object(obj));
        }
        
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            
            if self.pos >= self.chars.len() || self.chars[self.pos] != ':' {
                return Err(JsonError::ParseError("Expected ':'".to_string()));
            }
            
            self.pos += 1;
            let value = self.parse_value()?;
            obj.insert(key, value);
            
            self.skip_whitespace();
            
            if self.pos >= self.chars.len() {
                return Err(JsonError::UnexpectedEof);
            }
            
            match self.chars[self.pos] {
                ',' => {
                    self.pos += 1;
                    self.skip_whitespace();
                },
                '}' => {
                    self.pos += 1;
                    break;
                },
                _ => return Err(JsonError::ParseError("Expected ',' or '}'".to_string())),
            }
        }
        
        Ok(JsonValue::Object(obj))
    }
    
    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
    
    fn consume_literal(&mut self, literal: &str) -> bool {
        let remaining: String = self.chars[self.pos..].iter().collect();
        if remaining.starts_with(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_null() {
        assert!(matches!(JsonModule::parse("null"), Ok(JsonValue::Null)));
    }
    
    #[test]
    fn test_parse_bool() {
        assert!(matches!(JsonModule::parse("true"), Ok(JsonValue::Boolean(true))));
        assert!(matches!(JsonModule::parse("false"), Ok(JsonValue::Boolean(false))));
    }
    
    #[test]
    fn test_parse_number() {
        if let Ok(JsonValue::Number(n)) = JsonModule::parse("42") {
            assert_eq!(n, 42.0);
        } else {
            panic!("Failed to parse number");
        }
    }
    
    #[test]
    fn test_parse_string() {
        if let Ok(JsonValue::String(s)) = JsonModule::parse("\"hello\"") {
            assert_eq!(s, "hello");
        } else {
            panic!("Failed to parse string");
        }
    }
    
    #[test]
    fn test_parse_array() {
        assert!(JsonModule::parse("[1, 2, 3]").is_ok());
    }
    
    #[test]
    fn test_parse_object() {
        assert!(JsonModule::parse("{\"name\": \"John\"}").is_ok());
    }
    
    #[test]
    fn test_stringify() {
        let json = JsonValue::Number(42.0);
        assert_eq!(JsonModule::stringify(&json), "42");
    }
    
    #[test]
    fn test_is_valid() {
        assert!(JsonModule::is_valid("{\"name\": \"John\"}"));
        assert!(!JsonModule::is_valid("{invalid}"));
    }
    
    #[test]
    fn test_keys() {
        let json = JsonModule::parse("{\"a\": 1, \"b\": 2}").unwrap();
        let keys = JsonModule::keys(&json);
        assert_eq!(keys.len(), 2);
    }
}
