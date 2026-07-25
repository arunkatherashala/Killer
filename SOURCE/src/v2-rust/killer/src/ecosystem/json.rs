// Phase 5.2: JSON Serialization/Deserialization
// Core JSON support for data interchange

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

#[derive(Debug)]
pub struct JsonParser {
    /// Parsing configuration
    strict_mode: bool,
}

impl JsonParser {
    pub fn new(strict_mode: bool) -> Self {
        JsonParser { strict_mode }
    }

    /// Parse JSON string to JsonValue
    pub fn parse(&self, json_str: &str) -> Result<JsonValue, String> {
        let trimmed = json_str.trim();

        // Determine JSON type
        if trimmed == "null" {
            Ok(JsonValue::Null)
        } else if trimmed == "true" {
            Ok(JsonValue::Bool(true))
        } else if trimmed == "false" {
            Ok(JsonValue::Bool(false))
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // String
            let string_content = &trimmed[1..trimmed.len() - 1];
            Ok(JsonValue::String(string_content.to_string()))
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Array - simplified parser
            let contents = &trimmed[1..trimmed.len() - 1];
            let mut array = Vec::new();

            if !contents.is_empty() {
                for element in contents.split(',') {
                    array.push(self.parse(element.trim())?);
                }
            }

            Ok(JsonValue::Array(array))
        } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
            // Object - simplified parser
            let contents = &trimmed[1..trimmed.len() - 1];
            let mut object = HashMap::new();

            if !contents.is_empty() {
                for pair in contents.split(',') {
                    if let Some(colon_pos) = pair.find(':') {
                        let key_part = pair[..colon_pos].trim();
                        let value_part = pair[colon_pos + 1..].trim();

                        // Remove quotes from key
                        let key = if key_part.starts_with('"') && key_part.ends_with('"') {
                            key_part[1..key_part.len() - 1].to_string()
                        } else {
                            key_part.to_string()
                        };

                        object.insert(key, self.parse(value_part)?);
                    }
                }
            }

            Ok(JsonValue::Object(object))
        } else {
            // Try parsing as number
            trimmed
                .parse::<f64>()
                .map(JsonValue::Number)
                .map_err(|_| "Invalid JSON".to_string())
        }
    }

    /// Serialize JsonValue to JSON string
    pub fn stringify(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            JsonValue::String(s) => format!("\"{}\"", s),
            JsonValue::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| self.stringify(v)).collect();
                format!("[{}]", elements.join(","))
            }
            JsonValue::Object(obj) => {
                let mut pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, self.stringify(v)))
                    .collect();
                pairs.sort();
                format!("{{{}}}", pairs.join(","))
            }
        }
    }

    /// Get statistics for JSON data
    pub fn get_statistics(&self, value: &JsonValue) -> JsonStats {
        let (elements, max_depth) = self.count_elements(value, 0);

        JsonStats {
            total_elements: elements,
            max_nesting_depth: max_depth,
        }
    }

    fn count_elements(&self, value: &JsonValue, depth: usize) -> (usize, usize) {
        match value {
            JsonValue::Array(arr) => {
                let mut total = arr.len();
                let mut max_depth = depth;
                for elem in arr {
                    let (count, elem_depth) = self.count_elements(elem, depth + 1);
                    total += count;
                    max_depth = max_depth.max(elem_depth);
                }
                (total, max_depth)
            }
            JsonValue::Object(obj) => {
                let mut total = obj.len();
                let mut max_depth = depth;
                for value in obj.values() {
                    let (count, elem_depth) = self.count_elements(value, depth + 1);
                    total += count;
                    max_depth = max_depth.max(elem_depth);
                }
                (total, max_depth)
            }
            _ => (1, depth),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonStats {
    pub total_elements: usize,
    pub max_nesting_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = JsonParser::new(true);
        assert!(parser.strict_mode);
    }

    #[test]
    fn test_parse_null() {
        let parser = JsonParser::new(true);
        let result = parser.parse("null").unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn test_parse_bool() {
        let parser = JsonParser::new(true);
        assert_eq!(parser.parse("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(parser.parse("false").unwrap(), JsonValue::Bool(false));
    }

    #[test]
    fn test_parse_number() {
        let parser = JsonParser::new(true);
        if let JsonValue::Number(n) = parser.parse("42.5").unwrap() {
            assert_eq!(n, 42.5);
        }
    }

    #[test]
    fn test_parse_string() {
        let parser = JsonParser::new(true);
        let result = parser.parse("\"hello\"").unwrap();
        if let JsonValue::String(s) = result {
            assert_eq!(s, "hello");
        }
    }

    #[test]
    fn test_parse_array() {
        let parser = JsonParser::new(true);
        let result = parser.parse("[1,2,3]").unwrap();
        assert!(matches!(result, JsonValue::Array(_)));
    }

    #[test]
    fn test_stringify_null() {
        let parser = JsonParser::new(true);
        assert_eq!(parser.stringify(&JsonValue::Null), "null");
    }

    #[test]
    fn test_stringify_string() {
        let parser = JsonParser::new(true);
        let json = JsonValue::String("test".to_string());
        assert_eq!(parser.stringify(&json), "\"test\"");
    }

    #[test]
    fn test_roundtrip() {
        let parser = JsonParser::new(true);
        let original = "[1,2,3]";
        let parsed = parser.parse(original).unwrap();
        let stringified = parser.stringify(&parsed);
        assert_eq!(stringified, "[1,2,3]");
    }
}
