/// Function Parameters Module - Support for named and default parameters
/// Week 11 Implementation

use std::collections::HashMap;
use crate::value::Value;

/// A function parameter with optional default value
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Default value (None = required parameter)
    pub default: Option<Value>,
}

impl Parameter {
    /// Create a required parameter (no default)
    pub fn required(name: String) -> Self {
        Parameter {
            name,
            default: None,
        }
    }

    /// Create an optional parameter with default value
    pub fn with_default(name: String, default: Value) -> Self {
        Parameter {
            name,
            default: Some(default),
        }
    }

    /// Check if parameter is required
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }

    /// Get the default value if available
    pub fn get_default(&self) -> Option<Value> {
        self.default.clone()
    }
}

/// Function arguments - can be positional or named
#[derive(Debug, Clone)]
pub enum FunctionArg {
    /// Positional argument: func(5, 10)
    Positional(Value),
    /// Named argument: func(x: 5, y: 10)
    Named {
        name: String,
        value: Value,
    },
}

impl FunctionArg {
    /// Create a positional argument
    pub fn pos(value: Value) -> Self {
        FunctionArg::Positional(value)
    }

    /// Create a named argument
    pub fn named(name: String, value: Value) -> Self {
        FunctionArg::Named { name, value }
    }

    /// Check if this is a named argument
    pub fn is_named(&self) -> bool {
        matches!(self, FunctionArg::Named { .. })
    }

    /// Get the value
    pub fn value(&self) -> &Value {
        match self {
            FunctionArg::Positional(v) => v,
            FunctionArg::Named { value, .. } => value,
        }
    }
}

/// Function call argument matcher
/// Maps function arguments to parameter slots
pub struct ArgumentMatcher {
    pub parameters: Vec<Parameter>,
}

impl ArgumentMatcher {
    /// Create a new argument matcher from parameters
    pub fn new(parameters: Vec<Parameter>) -> Self {
        ArgumentMatcher { parameters }
    }

    /// Match arguments to parameters
    /// Returns: HashMap<parameter_name, value>
    /// Or: Err with detailed error message
    pub fn match_args(&self, args: &[FunctionArg]) -> Result<HashMap<String, Value>, String> {
        let mut matched: HashMap<String, Value> = HashMap::new();
        let mut positional_idx = 0;

        // Separate positional and named arguments
        let mut positional_args = Vec::new();
        let mut named_args = HashMap::new();

        for arg in args {
            match arg {
                FunctionArg::Positional(v) => {
                    positional_args.push(v.clone());
                }
                FunctionArg::Named { name, value } => {
                    if named_args.contains_key(name) {
                        return Err(format!("Duplicate named argument: {}", name));
                    }
                    named_args.insert(name.clone(), value.clone());
                }
            }
        }

        // Match positional arguments first
        for param in self.parameters.iter() {
            if positional_idx < positional_args.len() {
                matched.insert(param.name.clone(), positional_args[positional_idx].clone());
                positional_idx += 1;
            }
        }

        // Fill in named arguments
        for (name, value) in &named_args {
            // Check if parameter exists
            if !self.parameters.iter().any(|p| &p.name == name) {
                return Err(format!("Unknown parameter: {}", name));
            }

            // Check if already matched via positional
            if matched.contains_key(name) {
                return Err(format!("Parameter '{}' specified twice (positional and named)", name));
            }

            matched.insert(name.clone(), value.clone());
        }

        // Fill in defaults for missing parameters
        for param in &self.parameters {
            if !matched.contains_key(&param.name) {
                if let Some(default) = &param.default {
                    matched.insert(param.name.clone(), default.clone());
                } else {
                    return Err(format!("Missing required parameter: {}", param.name));
                }
            }
        }

        Ok(matched)
    }

    /// Get positional arguments in order
    pub fn get_positional_values(&self, args: &[FunctionArg]) -> Result<Vec<Value>, String> {
        let matched = self.match_args(args)?;
        let mut values = Vec::new();
        for param in &self.parameters {
            if let Some(v) = matched.get(&param.name) {
                values.push(v.clone());
            }
        }
        Ok(values)
    }

    /// Validate parameter count and types
    pub fn validate(&self, args: &[FunctionArg]) -> Result<(), String> {
        // Count positional args
        let positional_count = args.iter().filter(|a| !a.is_named()).count();

        // Count required parameters
        let required_count = self.parameters.iter().filter(|p| p.is_required()).count();

        // Check minimum required args
        if positional_count > self.parameters.len() {
            return Err(format!(
                "Too many arguments: expected at most {}, got {}",
                self.parameters.len(),
                positional_count
            ));
        }

        // Named arguments can fill in any missing required params
        let named_count = args.iter().filter(|a| a.is_named()).count();
        if positional_count + named_count < required_count {
            return Err(format!(
                "Not enough arguments: expected at least {}, got {}",
                required_count,
                positional_count + named_count
            ));
        }

        Ok(())
    }

    /// Get parameter list as string for documentation
    pub fn signature(&self) -> String {
        let param_strs: Vec<String> = self.parameters.iter().map(|p| {
            if let Some(default) = &p.default {
                format!("{} = {}", p.name, default)
            } else {
                p.name.clone()
            }
        }).collect();
        format!("({})", param_strs.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_creation() {
        let req = Parameter::required("x".to_string());
        assert!(req.is_required());
        assert_eq!(req.name, "x");

        let opt = Parameter::with_default("y".to_string(), Value::Number(10.0));
        assert!(!opt.is_required());
    }

    #[test]
    fn test_positional_args_only() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),
            FunctionArg::pos(Value::Number(10.0)),
        ];

        let matched = matcher.match_args(&args).unwrap();
        assert_eq!(matched.get("x"), Some(&Value::Number(5.0)));
        assert_eq!(matched.get("y"), Some(&Value::Number(10.0)));
    }

    #[test]
    fn test_named_args_only() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::named("y".to_string(), Value::Number(10.0)),
            FunctionArg::named("x".to_string(), Value::Number(5.0)),
        ];

        let matched = matcher.match_args(&args).unwrap();
        assert_eq!(matched.get("x"), Some(&Value::Number(5.0)));
        assert_eq!(matched.get("y"), Some(&Value::Number(10.0)));
    }

    #[test]
    fn test_mixed_positional_and_named() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
            Parameter::required("z".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),  // x
            FunctionArg::named("z".to_string(), Value::Number(15.0)),  // z
            FunctionArg::named("y".to_string(), Value::Number(10.0)),  // y
        ];

        let matched = matcher.match_args(&args).unwrap();
        assert_eq!(matched.get("x"), Some(&Value::Number(5.0)));
        assert_eq!(matched.get("y"), Some(&Value::Number(10.0)));
        assert_eq!(matched.get("z"), Some(&Value::Number(15.0)));
    }

    #[test]
    fn test_default_parameters() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::with_default("y".to_string(), Value::Number(20.0)),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),  // x only
        ];

        let matched = matcher.match_args(&args).unwrap();
        assert_eq!(matched.get("x"), Some(&Value::Number(5.0)));
        assert_eq!(matched.get("y"), Some(&Value::Number(20.0)));  // Default used
    }

    #[test]
    fn test_missing_required_parameter() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),  // Only x
        ];

        assert!(matcher.match_args(&args).is_err());
    }

    #[test]
    fn test_too_many_arguments() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),
            FunctionArg::pos(Value::Number(10.0)),
            FunctionArg::pos(Value::Number(15.0)),  // Too many
        ];

        assert!(matcher.validate(&args).is_err());
    }

    #[test]
    fn test_unknown_named_parameter() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::named("z".to_string(), Value::Number(5.0)),  // z doesn't exist
        ];

        assert!(matcher.match_args(&args).is_err());
    }

    #[test]
    fn test_duplicate_parameter() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::required("y".to_string()),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Number(5.0)),
            FunctionArg::named("x".to_string(), Value::Number(10.0)),  // x specified twice
        ];

        assert!(matcher.match_args(&args).is_err());
    }

    #[test]
    fn test_signature_generation() {
        let params = vec![
            Parameter::required("x".to_string()),
            Parameter::with_default("y".to_string(), Value::Number(20.0)),
        ];
        let matcher = ArgumentMatcher::new(params);
        
        let sig = matcher.signature();
        assert!(sig.contains("x"));
        assert!(sig.contains("y"));
        assert!(sig.contains("20"));
    }

    #[test]
    fn test_complex_defaults() {
        let params = vec![
            Parameter::required("name".to_string()),
            Parameter::with_default("age".to_string(), Value::Number(18.0)),
            Parameter::with_default("city".to_string(), Value::Str("Unknown".to_string())),
        ];
        let matcher = ArgumentMatcher::new(params);

        let args = vec![
            FunctionArg::pos(Value::Str("Alice".to_string())),
        ];

        let matched = matcher.match_args(&args).unwrap();
        assert_eq!(matched.get("name"), Some(&Value::Str("Alice".to_string())));
        assert_eq!(matched.get("age"), Some(&Value::Number(18.0)));
        assert_eq!(matched.get("city"), Some(&Value::Str("Unknown".to_string())));
    }
}
