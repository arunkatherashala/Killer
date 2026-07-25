// Type Utilities Module for Killer Language
// Type checking, conversion, and validation
// Version: 2.1.0

use crate::value::Value;
use std::collections::HashMap;

/// Type utilities for runtime type checking and conversion
pub struct TypeModule;

impl TypeModule {
    // ==================== Type Checking ====================
    
    /// Get type name of a value
    /// typeof(Value::Number(42.0)) => "number"
    pub fn typeof_value(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Dict(_) => "object",
            Value::Function { .. } => "function",
            Value::Object(_) => "object",
            Value::Class(_) => "class",
            Value::Generator(_) => "generator",
            Value::QualityWrapped(_) => "quality",
        }
    }
    
    /// Check if value is null
    pub fn is_null(value: &Value) -> bool {
        matches!(value, Value::Null)
    }
    
    /// Check if value is boolean
    pub fn is_bool(value: &Value) -> bool {
        matches!(value, Value::Bool(_))
    }
    
    /// Check if value is number
    pub fn is_number(value: &Value) -> bool {
        matches!(value, Value::Number(_))
    }
    
    /// Check if value is string
    pub fn is_string(value: &Value) -> bool {
        matches!(value, Value::Str(_))
    }
    
    /// Check if value is array
    pub fn is_array(value: &Value) -> bool {
        matches!(value, Value::Array(_))
    }
    
    /// Check if value is object/dict
    pub fn is_object(value: &Value) -> bool {
        matches!(value, Value::Dict(_) | Value::Object(_))
    }
    
    /// Check if value is function
    pub fn is_function(value: &Value) -> bool {
        matches!(value, Value::Function { .. })
    }
    
    /// Check if value is integer
    /// is_integer(Value::Number(42.0)) => true
    /// is_integer(Value::Number(42.5)) => false
    pub fn is_integer(value: &Value) -> bool {
        if let Value::Number(n) = value {
            n.fract() == 0.0 && n.is_finite()
        } else {
            false
        }
    }
    
    /// Check if value is finite number
    pub fn is_finite(value: &Value) -> bool {
        if let Value::Number(n) = value {
            n.is_finite()
        } else {
            false
        }
    }
    
    /// Check if value is infinite
    pub fn is_infinite(value: &Value) -> bool {
        if let Value::Number(n) = value {
            n.is_infinite()
        } else {
            false
        }
    }
    
    /// Check if value is NaN
    pub fn is_nan(value: &Value) -> bool {
        if let Value::Number(n) = value {
            n.is_nan()
        } else {
            false
        }
    }
    
    /// Check if value is empty (for arrays, dicts, strings)
    pub fn is_empty(value: &Value) -> bool {
        match value {
            Value::Array(arr) => arr.is_empty(),
            Value::Dict(dict) => dict.is_empty(),
            Value::Str(s) => s.is_empty(),
            Value::Null => true,
            _ => false,
        }
    }
    
    /// Check if value is truthy (loose truthiness check)
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Dict(dict) => !dict.is_empty(),
            _ => true,
        }
    }
    
    // ==================== Type Conversion ====================
    
    /// Convert value to boolean (strict)
    /// to_bool(Value::Number(1.0)) => Some(true)
    pub fn to_bool(value: &Value) -> Option<bool> {
        match value {
            Value::Bool(b) => Some(*b),
            Value::Number(n) if *n == 0.0 => Some(false),
            Value::Number(n) if *n == 1.0 => Some(true),
            Value::Str(s) if s == "true" => Some(true),
            Value::Str(s) if s == "false" => Some(false),
            _ => None,
        }
    }
    
    /// Convert value to number (strict)
    /// to_number(Value::Number(42.0)) => Some(42.0)
    pub fn to_number(value: &Value) -> Option<f64> {
        match value {
            Value::Number(n) => Some(*n),
            Value::Bool(true) => Some(1.0),
            Value::Bool(false) => Some(0.0),
            Value::Str(s) => s.parse::<f64>().ok(),
            Value::Null => Some(0.0),
            _ => None,
        }
    }
    
    /// Convert value to string
    pub fn to_string(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            },
            Value::Str(s) => s.clone(),
            Value::Array(_) => "[Array]".to_string(),
            Value::Dict(_) => "[Object]".to_string(),
            Value::Function { .. } => "[Function]".to_string(),
            Value::Object(_) => "[Object]".to_string(),
            Value::Class(_) => "[Class]".to_string(),
            Value::Generator(_) => "[Generator]".to_string(),
            Value::QualityWrapped(quality) => format!("[Quality score={:.2}]", quality.get_trim_score()),
        }
    }
    
    /// Convert string to specific type
    /// parse_as(s, "number") => Some(Value::Number(...))
    pub fn parse_as(s: &str, type_name: &str) -> Option<Value> {
        match type_name {
            "number" => s.parse::<f64>().ok().map(Value::Number),
            "boolean" => {
                match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Some(Value::Bool(true)),
                    "false" | "0" | "no" => Some(Value::Bool(false)),
                    _ => None,
                }
            },
            "string" => Some(Value::Str(s.to_string())),
            "null" if s == "null" => Some(Value::Null),
            _ => None,
        }
    }
    
    // ==================== Collection Operations ====================
    
    /// Get length of a collection
    /// length(Value::Array(...)) => Some(5)
    pub fn length(value: &Value) -> Option<usize> {
        match value {
            Value::Array(arr) => Some(arr.len()),
            Value::Dict(dict) => Some(dict.len()),
            Value::Str(s) => Some(s.len()),
            _ => None,
        }
    }
    
    /// Check if collection contains a key/value
    /// has(dict, "key") => true/false
    pub fn has(value: &Value, key: &str) -> bool {
        match value {
            Value::Dict(dict) => dict.contains_key(key),
            Value::Array(_) => {
                key.parse::<usize>()
                    .ok()
                    .and_then(|idx| Self::length(value).map(|len| idx < len))
                    .unwrap_or(false)
            },
            Value::Str(s) => s.contains(key),
            _ => false,
        }
    }
    
    /// Get keys from dictionary
    /// keys(dict) => ["a", "b", "c"]
    pub fn keys(value: &Value) -> Vec<String> {
        match value {
            Value::Dict(dict) => dict.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }
    
    /// Get values from dictionary
    /// values(dict) => [Value::Number(...), Value::Str(...), ...]
    pub fn values(value: &Value) -> Vec<Value> {
        match value {
            Value::Dict(dict) => dict.values().cloned().collect(),
            _ => Vec::new(),
        }
    }
    
    // ==================== Type Comparison ====================
    
    /// Check if two values are of the same type
    /// same_type(Value::Number(1.0), Value::Number(2.0)) => true
    /// same_type(Value::Number(1.0), Value::Str("1")) => false
    pub fn same_type(a: &Value, b: &Value) -> bool {
        Self::typeof_value(a) == Self::typeof_value(b)
    }
    
    /// Deep equality comparison (type-sensitive)
    /// equals(Value::Number(1.0), Value::Number(1.0)) => true
    /// equals(Value::Number(1.0), Value::Bool(true)) => false
    pub fn equals(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Number(x), Value::Number(y)) => {
                if x.is_nan() && y.is_nan() {
                    false
                } else {
                    (x - y).abs() < f64::EPSILON
                }
            },
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Array(x), Value::Array(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| Self::equals(a, b))
            },
            (Value::Dict(x), Value::Dict(y)) => {
                x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).map(|v2| Self::equals(v, v2)).unwrap_or(false))
            },
            _ => false,
        }
    }
    
    /// Type-coercing equality (loose equality)
    /// loose_equals(Value::Number(1.0), Value::Str("1")) => true
    pub fn loose_equals(a: &Value, b: &Value) -> bool {
        // Try strict equality first
        if Self::equals(a, b) {
            return true;
        }
        
        // Try numeric comparison
        if let (Some(n1), Some(n2)) = (Self::to_number(a), Self::to_number(b)) {
            return (n1 - n2).abs() < f64::EPSILON;
        }
        
        // Try string comparison
        let s1 = Self::to_string(a);
        let s2 = Self::to_string(b);
        s1 == s2
    }
    
    // ==================== Type Casting ====================
    
    /// Cast value to target type
    /// cast(Value::Str("42"), "number") => Some(Value::Number(42.0))
    pub fn cast(value: &Value, target_type: &str) -> Option<Value> {
        match target_type {
            "null" => Some(Value::Null),
            "boolean" => Self::to_bool(value).map(Value::Bool),
            "number" => Self::to_number(value).map(Value::Number),
            "string" => Some(Value::Str(Self::to_string(value))),
            _ => None,
        }
    }
    
    /// Try casting with default fallback
    /// cast_or(Value::Str("invalid"), "number", 0.0) => Value::Number(0.0)
    pub fn cast_or(value: &Value, target_type: &str, default: &Value) -> Value {
        Self::cast(value, target_type).unwrap_or_else(|| default.clone())
    }
    
    // ==================== Type Inspection ====================
    
    /// Get detailed type information
    /// inspect(value) => "number (42.0), finite, positive, integer"
    pub fn inspect(value: &Value) -> String {
        let base = Self::typeof_value(value);
        
        match value {
            Value::Number(n) => {
                let mut info = format!("number ({})", n);
                if n.is_nan() {
                    info.push_str(", NaN");
                } else if n.is_infinite() {
                    if *n > 0.0 {
                        info.push_str(", +Infinity");
                    } else {
                        info.push_str(", -Infinity");
                    }
                } else {
                    info.push_str(", finite");
                }
                
                if n.fract() == 0.0 && n.is_finite() {
                    info.push_str(", integer");
                } else {
                    info.push_str(", float");
                }
                
                if *n > 0.0 {
                    info.push_str(", positive");
                } else if *n < 0.0 {
                    info.push_str(", negative");
                } else {
                    info.push_str(", zero");
                }
                
                info
            },
            Value::Str(s) => format!("string (length: {})", s.len()),
            Value::Array(arr) => format!("array (length: {})", arr.len()),
            Value::Dict(dict) => format!("object (keys: {})", dict.len()),
            _ => base.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_typeof() {
        assert_eq!(TypeModule::typeof_value(&Value::Null), "null");
        assert_eq!(TypeModule::typeof_value(&Value::Bool(true)), "boolean");
        assert_eq!(TypeModule::typeof_value(&Value::Number(42.0)), "number");
        assert_eq!(TypeModule::typeof_value(&Value::Str("hello".to_string())), "string");
        assert_eq!(TypeModule::typeof_value(&Value::from(Vec::<Value>::new())), "array");
    }
    
    #[test]
    fn test_type_checking() {
        let num = Value::Number(42.0);
        assert!(TypeModule::is_number(&num));
        assert!(!TypeModule::is_string(&num));
        assert!(!TypeModule::is_bool(&num));
        assert!(TypeModule::is_integer(&num));
    }
    
    #[test]
    fn test_to_number() {
        assert_eq!(TypeModule::to_number(&Value::Number(42.0)), Some(42.0));
        assert_eq!(TypeModule::to_number(&Value::Bool(true)), Some(1.0));
        assert_eq!(TypeModule::to_number(&Value::Str("42".to_string())), Some(42.0));
        assert_eq!(TypeModule::to_number(&Value::Str("invalid".to_string())), None);
    }
    
    #[test]
    fn test_equals() {
        assert!(TypeModule::equals(&Value::Number(42.0), &Value::Number(42.0)));
        assert!(!TypeModule::equals(&Value::Number(42.0), &Value::Str("42".to_string())));
        assert!(TypeModule::equals(&Value::Str("hello".to_string()), &Value::Str("hello".to_string())));
    }
    
    #[test]
    fn test_loose_equals() {
        assert!(TypeModule::loose_equals(&Value::Number(1.0), &Value::Bool(true)));
        assert!(TypeModule::loose_equals(&Value::Number(42.0), &Value::Str("42".to_string())));
        assert!(!TypeModule::loose_equals(&Value::Number(42.0), &Value::Str("invalid".to_string())));
    }
    
    #[test]
    fn test_cast() {
        assert_eq!(TypeModule::cast(&Value::Str("42".to_string()), "number"), Some(Value::Number(42.0)));
        assert_eq!(TypeModule::cast(&Value::Number(1.0), "boolean"), Some(Value::Bool(true)));
        assert_eq!(TypeModule::cast(&Value::Null, "null"), Some(Value::Null));
    }
    
    #[test]
    fn test_inspect() {
        let info = TypeModule::inspect(&Value::Number(42.0));
        assert!(info.contains("number"));
        assert!(info.contains("integer"));
        assert!(info.contains("positive"));
    }
}
