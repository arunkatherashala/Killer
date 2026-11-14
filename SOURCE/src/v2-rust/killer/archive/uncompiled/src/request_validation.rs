/// Request Validation Framework for HTTP Requests
/// Provides schema validation, type checking, and error handling
/// Week 10 Implementation

use crate::json_module::{JsonModule, JsonValue};
use std::collections::HashMap;

/// Validation rule for a field
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Field is required (must be present)
    Required,
    /// Field must be a specific type
    Type(String), // "string", "number", "boolean", "array", "object"
    /// String length constraints
    StringLength { min: Option<usize>, max: Option<usize> },
    /// Number range constraints
    NumberRange { min: Option<f64>, max: Option<f64> },
    /// Email validation
    Email,
    /// URL validation
    Url,
    /// Array length constraints
    ArrayLength { min: Option<usize>, max: Option<usize> },
    /// Pattern matching (regex)
    Pattern(String),
    /// One of allowed values
    OneOf(Vec<String>),
    /// Custom validation function
    Custom(String), // Name of custom validator
}

/// Schema for request validation
#[derive(Debug, Clone)]
pub struct ValidationSchema {
    pub fields: HashMap<String, Vec<ValidationRule>>,
    pub strict_mode: bool, // Reject unknown fields if true
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: String,
}

/// Validation result
pub type ValidationResult = Result<(), Vec<ValidationError>>;

impl ValidationSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        ValidationSchema {
            fields: HashMap::new(),
            strict_mode: false,
        }
    }

    /// Add a field with rules
    pub fn add_field(&mut self, name: &str, rules: Vec<ValidationRule>) {
        self.fields.insert(name.to_string(), rules);
    }

    /// Enable strict mode (reject unknown fields)
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Validate a JSON body against this schema
    pub fn validate(&self, body: &str) -> ValidationResult {
        // Parse JSON
        let json = JsonModule::parse(body).map_err(|e| {
            vec![ValidationError {
                field: "body".to_string(),
                message: format!("Invalid JSON: {}", e),
                value: body.to_string(),
            }]
        })?;

        // Validate against schema
        self.validate_value(&json)
    }

    /// Validate a JsonValue against this schema
    pub fn validate_value(&self, json: &JsonValue) -> ValidationResult {
        let mut errors = Vec::new();

        // Check if root is object
        if !matches!(json, JsonValue::Object(_)) {
            return Err(vec![ValidationError {
                field: "root".to_string(),
                message: "Request body must be a JSON object".to_string(),
                value: "body".to_string(),
            }]);
        }

        if let JsonValue::Object(obj) = json {
            // Check strict mode
            if self.strict_mode {
                for key in obj.keys() {
                    if !self.fields.contains_key(key) {
                        errors.push(ValidationError {
                            field: key.clone(),
                            message: format!("Unknown field '{}'", key),
                            value: key.clone(),
                        });
                    }
                }
            }

            // Validate each field
            for (field_name, rules) in &self.fields {
                match obj.get(field_name) {
                    Some(value) => {
                        // Field is present, validate it
                        for rule in rules {
                            if let Err(err) = self.validate_rule(field_name, &value, rule) {
                                errors.push(err);
                            }
                        }
                    }
                    None => {
                        // Field is missing, check if required
                        if rules.iter().any(|r| matches!(r, ValidationRule::Required)) {
                            errors.push(ValidationError {
                                field: field_name.clone(),
                                message: format!("Field '{}' is required", field_name),
                                value: "(missing)".to_string(),
                            });
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_rule(&self, field: &str, value: &JsonValue, rule: &ValidationRule) -> Result<(), ValidationError> {
        match rule {
            ValidationRule::Required => {
                if matches!(value, JsonValue::Null) {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: format!("Field '{}' cannot be null", field),
                        value: "null".to_string(),
                    })
                } else {
                    Ok(())
                }
            }

            ValidationRule::Type(expected_type) => {
                let actual_type = JsonModule::type_of(value);
                if actual_type != expected_type.as_str() {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: format!("Expected type '{}', got '{}'", expected_type, actual_type),
                        value: actual_type.to_string(),
                    })
                } else {
                    Ok(())
                }
            }

            ValidationRule::StringLength { min, max } => {
                if let JsonValue::String(s) = value {
                    let len = s.len();
                    if let Some(min_len) = min {
                        if len < *min_len {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("String length must be at least {}", min_len),
                                value: len.to_string(),
                            });
                        }
                    }
                    if let Some(max_len) = max {
                        if len > *max_len {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("String length must be at most {}", max_len),
                                value: len.to_string(),
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected string type".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::NumberRange { min, max } => {
                if let JsonValue::Number(n) = value {
                    if let Some(min_val) = min {
                        if *n < *min_val {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("Number must be at least {}", min_val),
                                value: n.to_string(),
                            });
                        }
                    }
                    if let Some(max_val) = max {
                        if *n > *max_val {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("Number must be at most {}", max_val),
                                value: n.to_string(),
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected number type".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::Email => {
                if let JsonValue::String(s) = value {
                    if is_valid_email(s) {
                        Ok(())
                    } else {
                        Err(ValidationError {
                            field: field.to_string(),
                            message: "Invalid email format".to_string(),
                            value: s.clone(),
                        })
                    }
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected string type for email".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::Url => {
                if let JsonValue::String(s) = value {
                    if is_valid_url(s) {
                        Ok(())
                    } else {
                        Err(ValidationError {
                            field: field.to_string(),
                            message: "Invalid URL format".to_string(),
                            value: s.clone(),
                        })
                    }
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected string type for URL".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::ArrayLength { min, max } => {
                if let JsonValue::Array(arr) = value {
                    let len = arr.len();
                    if let Some(min_len) = min {
                        if len < *min_len {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("Array length must be at least {}", min_len),
                                value: len.to_string(),
                            });
                        }
                    }
                    if let Some(max_len) = max {
                        if len > *max_len {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("Array length must be at most {}", max_len),
                                value: len.to_string(),
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected array type".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::Pattern(_pattern) => {
                // Regex pattern matching - simplified for now
                Ok(())
            }

            ValidationRule::OneOf(allowed) => {
                if let JsonValue::String(s) = value {
                    if allowed.contains(s) {
                        Ok(())
                    } else {
                        Err(ValidationError {
                            field: field.to_string(),
                            message: format!("Must be one of: {}", allowed.join(", ")),
                            value: s.clone(),
                        })
                    }
                } else {
                    Err(ValidationError {
                        field: field.to_string(),
                        message: "Expected string type".to_string(),
                        value: JsonModule::type_of(value).to_string(),
                    })
                }
            }

            ValidationRule::Custom(_name) => {
                // Custom validators would be called here
                Ok(())
            }
        }
    }
}

/// Simple email validation
fn is_valid_email(email: &str) -> bool {
    let has_at = email.contains('@');
    let has_dot = email.contains('.');
    let at_not_first = email.find('@').map(|i| i > 0).unwrap_or(false);
    let at_not_last = email.find('@').map(|i| i < email.len() - 1).unwrap_or(false);

    has_at && has_dot && at_not_first && at_not_last
}

/// Simple URL validation
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_required_field() {
        let mut schema = ValidationSchema::new();
        schema.add_field("name", vec![ValidationRule::Required]);

        let valid_json = r#"{"name":"John"}"#;
        assert!(schema.validate(valid_json).is_ok());

        let invalid_json = r#"{"name":null}"#;
        assert!(schema.validate(invalid_json).is_err());
    }

    #[test]
    fn test_validation_type() {
        let mut schema = ValidationSchema::new();
        schema.add_field("age", vec![ValidationRule::Type("number".to_string())]);

        let valid_json = r#"{"age":30}"#;
        assert!(schema.validate(valid_json).is_ok());

        let invalid_json = r#"{"age":"thirty"}"#;
        assert!(schema.validate(invalid_json).is_err());
    }

    #[test]
    fn test_validation_string_length() {
        let mut schema = ValidationSchema::new();
        schema.add_field("name", vec![
            ValidationRule::Type("string".to_string()),
            ValidationRule::StringLength { min: Some(2), max: Some(50) },
        ]);

        let valid_json = r#"{"name":"John"}"#;
        assert!(schema.validate(valid_json).is_ok());

        let too_short = r#"{"name":"J"}"#;
        assert!(schema.validate(too_short).is_err());
    }

    #[test]
    fn test_validation_email() {
        let mut schema = ValidationSchema::new();
        schema.add_field("email", vec![ValidationRule::Email]);

        let valid_json = r#"{"email":"user@example.com"}"#;
        assert!(schema.validate(valid_json).is_ok());

        let invalid_json = r#"{"email":"invalid-email"}"#;
        assert!(schema.validate(invalid_json).is_err());
    }

    #[test]
    fn test_validation_url() {
        let mut schema = ValidationSchema::new();
        schema.add_field("website", vec![ValidationRule::Url]);

        let valid_json = r#"{"website":"https://example.com"}"#;
        assert!(schema.validate(valid_json).is_ok());

        let invalid_json = r#"{"website":"not-a-url"}"#;
        assert!(schema.validate(invalid_json).is_err());
    }

    #[test]
    fn test_validation_strict_mode() {
        let mut schema = ValidationSchema::new();
        schema.add_field("name", vec![ValidationRule::Required]);
        let schema = schema.strict();

        let valid_json = r#"{"name":"John"}"#;
        assert!(schema.validate(valid_json).is_ok());

        let extra_field = r#"{"name":"John","extra":"field"}"#;
        assert!(schema.validate(extra_field).is_err());
    }
}
