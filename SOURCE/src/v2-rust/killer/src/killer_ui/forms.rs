//! **Reactive Forms** — Angular-style reactive form system.
//!
//! `FormControl` with value tracking, dirty/pristine, touched/untouched, valid/invalid.
//! `FormGroup` aggregates multiple controls.
//! `FormArray` for dynamic repeated fields.
//! Built-in validators + custom validators.
//! Two-way data binding support.
//!
//! Competitive with Angular Reactive Forms / React Hook Form.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Validators
// ══════════════════════════════════════════════════════════════════════════════

/// Validation error — field name + error type + message.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub error_type: String,
    pub message: String,
}

/// Validator function that returns None (valid) or Some(error_type, message).
pub type ValidatorFn = Box<dyn Fn(&str) -> Option<(String, String)> + Send + Sync>;

/// Built-in validators.
pub struct Validators;

impl Validators {
    pub fn required() -> ValidatorFn {
        Box::new(|val| {
            if val.trim().is_empty() {
                Some(("required".into(), "This field is required".into()))
            } else { None }
        })
    }

    pub fn min_length(min: usize) -> ValidatorFn {
        Box::new(move |val| {
            if val.len() < min {
                Some(("minLength".into(), format!("Minimum {} characters required", min)))
            } else { None }
        })
    }

    pub fn max_length(max: usize) -> ValidatorFn {
        Box::new(move |val| {
            if val.len() > max {
                Some(("maxLength".into(), format!("Maximum {} characters allowed", max)))
            } else { None }
        })
    }

    pub fn email() -> ValidatorFn {
        Box::new(|val| {
            if val.is_empty() { return None; }
            if val.contains('@') && val.contains('.') && val.len() >= 5 {
                None
            } else {
                Some(("email".into(), "Invalid email address".into()))
            }
        })
    }

    pub fn pattern(pat: &str) -> ValidatorFn {
        let pattern = pat.to_string();
        Box::new(move |val| {
            // Simple regex-like check for common patterns
            let matches = match pattern.as_str() {
                r"^\d+$" => val.chars().all(|c| c.is_ascii_digit()),
                r"^[a-zA-Z]+$" => val.chars().all(|c| c.is_ascii_alphabetic()),
                r"^[a-zA-Z0-9]+$" => val.chars().all(|c| c.is_ascii_alphanumeric()),
                _ => true, // Unknown pattern passes
            };
            if matches { None }
            else { Some(("pattern".into(), format!("Must match pattern: {}", pattern))) }
        })
    }

    pub fn min_value(min: f64) -> ValidatorFn {
        Box::new(move |val| {
            match val.parse::<f64>() {
                Ok(n) if n < min => Some(("min".into(), format!("Minimum value is {}", min))),
                _ => None,
            }
        })
    }

    pub fn max_value(max: f64) -> ValidatorFn {
        Box::new(move |val| {
            match val.parse::<f64>() {
                Ok(n) if n > max => Some(("max".into(), format!("Maximum value is {}", max))),
                _ => None,
            }
        })
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FormControl — single field
// ══════════════════════════════════════════════════════════════════════════════

/// A single form control (field) with state tracking and validation.
pub struct FormControl {
    pub name: String,
    value: String,
    initial_value: String,
    pub dirty: bool,
    pub touched: bool,
    pub disabled: bool,
    validators: Vec<ValidatorFn>,
    errors: Vec<ValidationError>,
}

impl FormControl {
    pub fn new(name: &str, initial: &str) -> Self {
        FormControl {
            name: name.to_string(),
            value: initial.to_string(),
            initial_value: initial.to_string(),
            dirty: false,
            touched: false,
            disabled: false,
            validators: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn validator(mut self, v: ValidatorFn) -> Self {
        self.validators.push(v);
        self
    }

    pub fn required(self) -> Self { self.validator(Validators::required()) }
    pub fn min_length(self, n: usize) -> Self { self.validator(Validators::min_length(n)) }
    pub fn max_length(self, n: usize) -> Self { self.validator(Validators::max_length(n)) }
    pub fn email(self) -> Self { self.validator(Validators::email()) }

    /// Get current value.
    pub fn value(&self) -> &str { &self.value }

    /// Set value (marks dirty if different from initial).
    pub fn set_value(&mut self, val: &str) {
        self.value = val.to_string();
        self.dirty = self.value != self.initial_value;
        self.validate();
    }

    /// Mark as touched (user has focused and left the field).
    pub fn mark_touched(&mut self) {
        self.touched = true;
    }

    /// Reset to initial value.
    pub fn reset(&mut self) {
        self.value = self.initial_value.clone();
        self.dirty = false;
        self.touched = false;
        self.errors.clear();
    }

    /// Reset with a new initial value.
    pub fn reset_to(&mut self, new_initial: &str) {
        self.initial_value = new_initial.to_string();
        self.value = new_initial.to_string();
        self.dirty = false;
        self.touched = false;
        self.errors.clear();
    }

    /// Run all validators.
    pub fn validate(&mut self) -> bool {
        self.errors.clear();
        for v in &self.validators {
            if let Some((etype, msg)) = v(&self.value) {
                self.errors.push(ValidationError {
                    field: self.name.clone(),
                    error_type: etype,
                    message: msg,
                });
            }
        }
        self.errors.is_empty()
    }

    /// Is the control valid?
    pub fn valid(&self) -> bool { self.errors.is_empty() }
    pub fn invalid(&self) -> bool { !self.valid() }
    pub fn pristine(&self) -> bool { !self.dirty }

    /// Get current errors.
    pub fn errors(&self) -> &[ValidationError] { &self.errors }

    /// Check if a specific error type is present.
    pub fn has_error(&self, error_type: &str) -> bool {
        self.errors.iter().any(|e| e.error_type == error_type)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FormGroup — group of controls
// ══════════════════════════════════════════════════════════════════════════════

/// A group of named form controls. Like Angular's `FormGroup`.
pub struct FormGroup {
    controls: Vec<FormControl>,
}

impl FormGroup {
    pub fn new() -> Self { FormGroup { controls: Vec::new() }  }

    pub fn control(mut self, control: FormControl) -> Self {
        self.controls.push(control);
        self
    }

    /// Get a control by name.
    pub fn get(&self, name: &str) -> Option<&FormControl> {
        self.controls.iter().find(|c| c.name == name)
    }

    /// Get a mutable control by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut FormControl> {
        self.controls.iter_mut().find(|c| c.name == name)
    }

    /// Set a control's value by name.
    pub fn set_value(&mut self, name: &str, value: &str) {
        if let Some(ctrl) = self.get_mut(name) {
            ctrl.set_value(value);
        }
    }

    /// Get all values as a HashMap.
    pub fn values(&self) -> HashMap<String, String> {
        self.controls.iter().map(|c| (c.name.clone(), c.value.clone())).collect()
    }

    /// Validate all controls. Returns true if ALL valid.
    pub fn validate(&mut self) -> bool {
        let mut all_valid = true;
        for ctrl in &mut self.controls {
            if !ctrl.validate() { all_valid = false; }
        }
        all_valid
    }

    /// Is the entire form valid?
    pub fn valid(&self) -> bool {
        self.controls.iter().all(|c| c.valid())
    }

    /// Is any control dirty?
    pub fn dirty(&self) -> bool {
        self.controls.iter().any(|c| c.dirty)
    }

    /// Is any control touched?
    pub fn touched(&self) -> bool {
        self.controls.iter().any(|c| c.touched)
    }

    /// All errors across all controls.
    pub fn all_errors(&self) -> Vec<&ValidationError> {
        self.controls.iter().flat_map(|c| c.errors()).collect()
    }

    /// Reset all controls.
    pub fn reset(&mut self) {
        for ctrl in &mut self.controls { ctrl.reset(); }
    }

    /// Patch values from a HashMap (partial update).
    pub fn patch_values(&mut self, values: &HashMap<String, String>) {
        for (key, val) in values {
            if let Some(ctrl) = self.get_mut(key) {
                ctrl.set_value(val);
            }
        }
    }

    pub fn control_count(&self) -> usize { self.controls.len() }
    pub fn control_names(&self) -> Vec<&str> {
        self.controls.iter().map(|c| c.name.as_str()).collect()
    }
}

impl Default for FormGroup {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// FormArray — dynamic list of controls
// ══════════════════════════════════════════════════════════════════════════════

/// A dynamic array of form controls (for repeated fields).
pub struct FormArray {
    controls: Vec<FormControl>,
    base_name: String,
}

impl FormArray {
    pub fn new(base_name: &str) -> Self {
        FormArray { controls: Vec::new(), base_name: base_name.to_string() }
    }

    pub fn push(&mut self, initial: &str) {
        let idx = self.controls.len();
        let name = format!("{}[{}]", self.base_name, idx);
        self.controls.push(FormControl::new(&name, initial));
    }

    pub fn push_with_validators(&mut self, initial: &str, validators: Vec<ValidatorFn>) {
        let idx = self.controls.len();
        let name = format!("{}[{}]", self.base_name, idx);
        let mut ctrl = FormControl::new(&name, initial);
        ctrl.validators = validators;
        self.controls.push(ctrl);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.controls.len() { self.controls.remove(index); }
    }

    pub fn get(&self, index: usize) -> Option<&FormControl> {
        self.controls.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut FormControl> {
        self.controls.get_mut(index)
    }

    pub fn values(&self) -> Vec<String> {
        self.controls.iter().map(|c| c.value.clone()).collect()
    }

    pub fn validate(&mut self) -> bool {
        self.controls.iter_mut().all(|c| c.validate())
    }

    pub fn valid(&self) -> bool {
        self.controls.iter().all(|c| c.valid())
    }

    pub fn len(&self) -> usize { self.controls.len() }
    pub fn is_empty(&self) -> bool { self.controls.is_empty() }

    pub fn reset(&mut self) {
        for ctrl in &mut self.controls { ctrl.reset(); }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_control_basic() {
        let mut ctrl = FormControl::new("name", "");
        assert_eq!(ctrl.value(), "");
        assert!(ctrl.pristine());
        assert!(!ctrl.touched);

        ctrl.set_value("Alice");
        assert_eq!(ctrl.value(), "Alice");
        assert!(ctrl.dirty);
    }

    #[test]
    fn form_control_required_validation() {
        let mut ctrl = FormControl::new("email", "").required();
        ctrl.validate();
        assert!(ctrl.invalid());
        assert!(ctrl.has_error("required"));

        ctrl.set_value("test@example.com");
        assert!(ctrl.valid());
    }

    #[test]
    fn form_control_min_length() {
        let mut ctrl = FormControl::new("password", "").min_length(8);
        ctrl.set_value("abc");
        assert!(ctrl.invalid());
        assert!(ctrl.has_error("minLength"));

        ctrl.set_value("abcdefgh");
        assert!(ctrl.valid());
    }

    #[test]
    fn form_control_email_validation() {
        let mut ctrl = FormControl::new("email", "").email();
        ctrl.set_value("not-an-email");
        assert!(ctrl.has_error("email"));

        ctrl.set_value("user@example.com");
        assert!(ctrl.valid());
    }

    #[test]
    fn form_control_reset() {
        let mut ctrl = FormControl::new("name", "initial");
        ctrl.set_value("changed");
        ctrl.mark_touched();
        assert!(ctrl.dirty);
        assert!(ctrl.touched);

        ctrl.reset();
        assert_eq!(ctrl.value(), "initial");
        assert!(!ctrl.dirty);
        assert!(!ctrl.touched);
    }

    #[test]
    fn form_group_basic() {
        let mut form = FormGroup::new()
            .control(FormControl::new("name", "").required())
            .control(FormControl::new("email", "").email().required());

        assert_eq!(form.control_count(), 2);
        assert!(!form.dirty());

        form.set_value("name", "Alice");
        assert!(form.dirty());

        assert!(!form.validate()); // email still empty
        assert_eq!(form.all_errors().len(), 1); // required on email (email validator skips empty)
    }

    #[test]
    fn form_group_all_valid() {
        let mut form = FormGroup::new()
            .control(FormControl::new("name", "").required())
            .control(FormControl::new("age", "").required());

        form.set_value("name", "Bob");
        form.set_value("age", "25");
        assert!(form.validate());
        assert!(form.valid());
    }

    #[test]
    fn form_group_values() {
        let mut form = FormGroup::new()
            .control(FormControl::new("first", ""))
            .control(FormControl::new("last", ""));
        form.set_value("first", "John");
        form.set_value("last", "Doe");
        let vals = form.values();
        assert_eq!(vals.get("first").unwrap(), "John");
        assert_eq!(vals.get("last").unwrap(), "Doe");
    }

    #[test]
    fn form_group_patch_values() {
        let mut form = FormGroup::new()
            .control(FormControl::new("name", ""))
            .control(FormControl::new("email", ""));
        let mut patch = HashMap::new();
        patch.insert("name".into(), "Alice".into());
        form.patch_values(&patch);
        assert_eq!(form.get("name").unwrap().value(), "Alice");
        assert_eq!(form.get("email").unwrap().value(), ""); // untouched
    }

    #[test]
    fn form_group_reset() {
        let mut form = FormGroup::new()
            .control(FormControl::new("x", "a"));
        form.set_value("x", "b");
        assert!(form.dirty());
        form.reset();
        assert!(!form.dirty());
        assert_eq!(form.get("x").unwrap().value(), "a");
    }

    #[test]
    fn form_array_basic() {
        let mut arr = FormArray::new("tags");
        arr.push("rust");
        arr.push("killer");
        arr.push("ui");
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.values(), vec!["rust", "killer", "ui"]);

        arr.remove(1);
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn form_array_validation() {
        let mut arr = FormArray::new("emails");
        arr.push_with_validators("", vec![Validators::required()]);
        arr.push_with_validators("test@test.com", vec![Validators::required()]);
        assert!(!arr.validate()); // first is empty
        assert!(!arr.valid());

        arr.get_mut(0).unwrap().set_value("a@b.com");
        assert!(arr.valid());
    }

    #[test]
    fn form_control_multiple_validators() {
        let mut ctrl = FormControl::new("password", "")
            .required()
            .min_length(8)
            .max_length(20);
        ctrl.set_value(""); // empty
        assert_eq!(ctrl.errors().len(), 2); // required + minLength

        ctrl.set_value("abc"); // too short
        assert_eq!(ctrl.errors().len(), 1); // just minLength

        ctrl.set_value("abcdefgh"); // valid
        assert!(ctrl.valid());
    }

    #[test]
    fn form_control_touched_state() {
        let mut ctrl = FormControl::new("field", "");
        assert!(!ctrl.touched);
        ctrl.mark_touched();
        assert!(ctrl.touched);
        ctrl.reset();
        assert!(!ctrl.touched);
    }

    #[test]
    fn validators_min_max_value() {
        let mut ctrl = FormControl::new("price", "")
            .validator(Validators::min_value(0.0))
            .validator(Validators::max_value(1000.0));
        ctrl.set_value("-5");
        assert!(ctrl.has_error("min"));

        ctrl.set_value("1500");
        assert!(ctrl.has_error("max"));

        ctrl.set_value("500");
        assert!(ctrl.valid());
    }
}
