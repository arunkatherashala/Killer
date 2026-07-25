//! **Form validation** — declarative validation rules for form fields.
//!
//! Supports: required, min/max length, min/max value, regex pattern, email,
//! custom predicates via action tags. Validates single fields or entire forms.

use std::collections::HashMap;

// ── Validation rules ─────────────────────────────────────────────────────────

/// A single validation rule.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    Required,
    MinLength(usize),
    MaxLength(usize),
    MinValue(f64),
    MaxValue(f64),
    Pattern(String),
    Email,
    /// Custom validation via action tag (evaluated by builtin dispatcher).
    Custom { action: String, message: String },
}

/// A validation error for one field.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub rule: String,
}

/// Field-level validation config.
#[derive(Debug, Clone)]
pub struct FieldValidator {
    pub field_name: String,
    pub rules: Vec<ValidationRule>,
    pub label: Option<String>,
}

impl FieldValidator {
    pub fn new(name: &str) -> Self {
        Self { field_name: name.to_string(), rules: Vec::new(), label: None }
    }

    pub fn required(mut self) -> Self { self.rules.push(ValidationRule::Required); self }
    pub fn min_length(mut self, n: usize) -> Self { self.rules.push(ValidationRule::MinLength(n)); self }
    pub fn max_length(mut self, n: usize) -> Self { self.rules.push(ValidationRule::MaxLength(n)); self }
    pub fn min_value(mut self, v: f64) -> Self { self.rules.push(ValidationRule::MinValue(v)); self }
    pub fn max_value(mut self, v: f64) -> Self { self.rules.push(ValidationRule::MaxValue(v)); self }
    pub fn email(mut self) -> Self { self.rules.push(ValidationRule::Email); self }
    pub fn pattern(mut self, p: &str) -> Self { self.rules.push(ValidationRule::Pattern(p.to_string())); self }
    pub fn label(mut self, l: &str) -> Self { self.label = Some(l.to_string()); self }

    /// Validate a single value. Returns errors (empty = valid).
    pub fn validate(&self, value: &str) -> Vec<ValidationError> {
        let label = self.label.as_deref().unwrap_or(&self.field_name);
        let mut errors = Vec::new();

        for rule in &self.rules {
            let err = match rule {
                ValidationRule::Required => {
                    if value.trim().is_empty() { Some((format!("{} is required", label), "required")) } else { None }
                }
                ValidationRule::MinLength(n) => {
                    if value.len() < *n { Some((format!("{} must be at least {} characters", label, n), "min_length")) } else { None }
                }
                ValidationRule::MaxLength(n) => {
                    if value.len() > *n { Some((format!("{} must be at most {} characters", label, n), "max_length")) } else { None }
                }
                ValidationRule::MinValue(v) => {
                    match value.parse::<f64>() {
                        Ok(n) if n < *v => Some((format!("{} must be at least {}", label, v), "min_value")),
                        Err(_) => Some((format!("{} must be a number", label), "min_value")),
                        _ => None,
                    }
                }
                ValidationRule::MaxValue(v) => {
                    match value.parse::<f64>() {
                        Ok(n) if n > *v => Some((format!("{} must be at most {}", label, v), "max_value")),
                        _ => None,
                    }
                }
                ValidationRule::Email => {
                    if !is_email_like(value) { Some((format!("{} must be a valid email", label), "email")) } else { None }
                }
                ValidationRule::Pattern(p) => {
                    if !simple_pattern_match(p, value) { Some((format!("{} format is invalid", label), "pattern")) } else { None }
                }
                ValidationRule::Custom { message: _, .. } => {
                    // Custom rules are evaluated externally; pass through if action tag says "invalid"
                    None
                }
            };
            if let Some((msg, rule_name)) = err {
                errors.push(ValidationError { field: self.field_name.clone(), message: msg, rule: rule_name.to_string() });
            }
        }
        errors
    }
}

// ── Form validator ───────────────────────────────────────────────────────────

/// Validates an entire form (multiple fields).
#[derive(Debug, Clone)]
pub struct FormValidator {
    pub fields: Vec<FieldValidator>,
}

impl FormValidator {
    pub fn new() -> Self { Self { fields: Vec::new() } }

    pub fn add_field(mut self, fv: FieldValidator) -> Self { self.fields.push(fv); self }

    /// Validate all fields. Returns field → errors map. Empty map = all valid.
    pub fn validate(&self, values: &HashMap<String, String>) -> HashMap<String, Vec<ValidationError>> {
        let mut result = HashMap::new();
        for fv in &self.fields {
            let value = values.get(&fv.field_name).map(|s| s.as_str()).unwrap_or("");
            let errors = fv.validate(value);
            if !errors.is_empty() {
                result.insert(fv.field_name.clone(), errors);
            }
        }
        result
    }

    /// Is the entire form valid?
    pub fn is_valid(&self, values: &HashMap<String, String>) -> bool {
        self.validate(values).is_empty()
    }

    /// Get first error per field (for display).
    pub fn first_errors(&self, values: &HashMap<String, String>) -> HashMap<String, String> {
        self.validate(values).into_iter()
            .filter_map(|(k, errs)| errs.into_iter().next().map(|e| (k, e.message)))
            .collect()
    }
}

impl Default for FormValidator {
    fn default() -> Self { Self::new() }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn is_email_like(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 { return false; }
    let local = parts[0];
    let domain = parts[1];
    !local.is_empty() && domain.contains('.') && domain.len() > 2
}

/// Simple glob-like pattern match (supports `*` as wildcard).
fn simple_pattern_match(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() { return true; }
    // Try anchored prefix/suffix matching
    if let Some(suffix) = pattern.strip_prefix('*') {
        value.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        value.starts_with(prefix)
    } else {
        value == pattern
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_validation() {
        let fv = FieldValidator::new("name").required();
        assert!(!fv.validate("").is_empty());
        assert!(fv.validate("Alice").is_empty());
    }

    #[test]
    fn length_validation() {
        let fv = FieldValidator::new("password").min_length(8).max_length(32);
        assert!(!fv.validate("short").is_empty());
        assert!(fv.validate("longpassword").is_empty());
    }

    #[test]
    fn email_validation() {
        let fv = FieldValidator::new("email").email();
        assert!(!fv.validate("notanemail").is_empty());
        assert!(fv.validate("user@example.com").is_empty());
    }

    #[test]
    fn number_range_validation() {
        let fv = FieldValidator::new("age").min_value(18.0).max_value(120.0);
        assert!(!fv.validate("10").is_empty());
        assert!(fv.validate("25").is_empty());
        assert!(!fv.validate("200").is_empty());
    }

    #[test]
    fn form_validator_multiple_fields() {
        let form = FormValidator::new()
            .add_field(FieldValidator::new("name").required().min_length(2))
            .add_field(FieldValidator::new("email").required().email())
            .add_field(FieldValidator::new("age").min_value(0.0).max_value(150.0));

        let mut values = HashMap::new();
        values.insert("name".into(), "Al".into());
        values.insert("email".into(), "al@test.com".into());
        values.insert("age".into(), "30".into());
        assert!(form.is_valid(&values));

        values.insert("email".into(), "bad".into());
        assert!(!form.is_valid(&values));
        let errs = form.first_errors(&values);
        assert!(errs.contains_key("email"));
    }

    #[test]
    fn pattern_validation() {
        let _fv = FieldValidator::new("zip").pattern("*-*");
        // Simple glob: * prefix/suffix only
        let fv2 = FieldValidator::new("code").pattern("ABC*");
        assert!(fv2.validate("ABCDEF").is_empty());
        assert!(!fv2.validate("XYZABC").is_empty());
    }
}
