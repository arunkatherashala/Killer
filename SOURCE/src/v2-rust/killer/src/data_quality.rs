//! Phase 8: Data Quality Variable Module
//! 
//! Provides DataQuality type for tracking and validating data with quality metrics.
//! Only `quality` type variables have these features (not regular variables).
//!
//! Supports two quality frameworks:
//! 1. Six Metrics: Completeness, Accuracy, Consistency, Uniqueness, Timeliness, Validity
//! 2. TRIM Framework: Truthfulness, Representativeness, Integrity, Modernness
//!
//! # Example
//! ```text
//! quality email = "alice@example.com"
//! email.validate_email()
//! if email.quality() >= 0.9:
//!     save_user(email)
//! ```

use std::collections::HashMap;
use crate::value::Value;

/// Quality score level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    Excellent,   // 0.95 - 1.0
    Good,        // 0.85 - 0.95
    Acceptable,  // 0.75 - 0.85
    Fair,        // 0.60 - 0.75
    Poor,        // < 0.60
}

impl QualityLevel {
    pub fn as_str(&self) -> &str {
        match self {
            QualityLevel::Excellent => "Excellent",
            QualityLevel::Good => "Good",
            QualityLevel::Acceptable => "Acceptable",
            QualityLevel::Fair => "Fair",
            QualityLevel::Poor => "Poor",
        }
    }

    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.95 => QualityLevel::Excellent,
            s if s >= 0.85 => QualityLevel::Good,
            s if s >= 0.75 => QualityLevel::Acceptable,
            s if s >= 0.60 => QualityLevel::Fair,
            _ => QualityLevel::Poor,
        }
    }
}

/// Guaranteed property of data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Guarantee {
    Privacy,
    Encryption,
    Durability,
    Consistency,
    Availability,
}

impl Guarantee {
    pub fn as_str(&self) -> &str {
        match self {
            Guarantee::Privacy => "Privacy",
            Guarantee::Encryption => "Encryption",
            Guarantee::Durability => "Durability",
            Guarantee::Consistency => "Consistency",
            Guarantee::Availability => "Availability",
        }
    }
}

/// Data quality status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityStatus {
    Unknown,
    Valid,
    Invalid,
    Warning,
}

impl QualityStatus {
    pub fn as_str(&self) -> &str {
        match self {
            QualityStatus::Unknown => "Unknown",
            QualityStatus::Valid => "Valid",
            QualityStatus::Invalid => "Invalid",
            QualityStatus::Warning => "Warning",
        }
    }
}

/// Core DataQuality tracking struct
/// Only `quality` type variables create this
#[derive(Debug, Clone, PartialEq)]
pub struct DataQuality {
    pub value: Value,
    
    // 6 Quality Metrics (each 0.0 to 1.0)
    pub completeness: f64,   // Is all required data present?
    pub accuracy: f64,       // Is the data correct/valid?
    pub consistency: f64,    // Does it follow all rules?
    pub uniqueness: f64,     // Is it unique (no duplicates)?
    pub timeliness: f64,     // Is the data fresh/current?
    pub validity: f64,       // Correct format/schema?
    
    // Overall metrics
    pub quality_score: f64,  // Average of all 6 metrics (0.0 to 1.0)
    pub level: QualityLevel, // Excellent/Good/Acceptable/Fair/Poor
    pub status: QualityStatus,
    
    // Guarantees and audit
    pub guarantees: Vec<Guarantee>,
    pub audit_log: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DataQuality {
    /// Create new quality variable
    pub fn new(value: Value) -> Self {
        let mut dq = DataQuality {
            value,
            completeness: 1.0,    // Has value = complete
            accuracy: 0.0,        // Unknown = 0
            consistency: 1.0,     // Assume ok
            uniqueness: 0.0,      // Unknown = 0
            timeliness: 1.0,      // Fresh = ok
            validity: 0.0,        // Unknown = 0
            quality_score: 0.0,
            level: QualityLevel::Poor,
            status: QualityStatus::Unknown,
            guarantees: Vec::new(),
            audit_log: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        dq.update_quality_score();
        dq
    }

    /// Recalculate quality score and level
    fn update_quality_score(&mut self) {
        let sum = self.completeness + self.accuracy + self.consistency 
                + self.uniqueness + self.timeliness + self.validity;
        self.quality_score = sum / 6.0;
        self.level = QualityLevel::from_score(self.quality_score);
    }

    // ============ VALIDATION METHODS ============

    /// Validate email format
    pub fn validate_email(&mut self) {
        let email_str = self.value.to_string();
        
        if email_str.contains('@') && email_str.contains('.') {
            // Simple email validation
            let parts: Vec<&str> = email_str.split('@').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                self.accuracy = 1.0;
                self.validity = 1.0;
                self.status = QualityStatus::Valid;
                self.update_quality_score();
                return;
            }
        }
        
        self.accuracy = 0.0;
        self.validity = 0.0;
        self.status = QualityStatus::Invalid;
        self.errors.push("Invalid email format".to_string());
        self.update_quality_score();
    }

    /// Validate phone format (basic US format check)
    pub fn validate_phone(&mut self) {
        let phone_str = self.value.to_string();
        let digits_only: String = phone_str.chars().filter(|c| c.is_numeric()).collect();
        
        if digits_only.len() >= 10 {
            self.accuracy = 1.0;
            self.validity = 1.0;
            self.status = QualityStatus::Valid;
            self.update_quality_score();
            return;
        }
        
        self.accuracy = 0.0;
        self.validity = 0.0;
        self.status = QualityStatus::Invalid;
        self.errors.push("Invalid phone format".to_string());
        self.update_quality_score();
    }

    /// Validate value is positive (for numbers)
    pub fn validate_positive(&mut self) {
        match &self.value {
            Value::Number(n) => {
                if *n > 0.0 {
                    self.accuracy = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.accuracy = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Value must be positive".to_string());
                }
            }
            _ => {
                self.errors.push("Cannot validate non-numeric value as positive".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate value is in range [min, max]
    pub fn validate_range(&mut self, min: f64, max: f64) {
        match &self.value {
            Value::Number(n) => {
                if *n >= min && *n <= max {
                    self.accuracy = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.accuracy = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Value out of range [{}, {}]", min, max));
                }
            }
            _ => {
                self.errors.push("Cannot validate non-numeric value".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate string length in range [min, max]
    pub fn validate_length(&mut self, min: usize, max: usize) {
        let str_val = self.value.to_string();
        let len = str_val.len();
        
        if len >= min && len <= max {
            self.accuracy = 1.0;
            self.validity = 1.0;
            self.status = QualityStatus::Valid;
        } else {
            self.accuracy = 0.0;
            self.validity = 0.0;
            self.status = QualityStatus::Invalid;
            self.errors.push(format!("String length out of range [{}, {}]", min, max));
        }
        self.update_quality_score();
    }

    /// Validate value is not null/empty
    pub fn validate_not_null(&mut self) {
        let str_val = self.value.to_string();
        
        if !str_val.is_empty() && str_val != "null" && str_val != "nil" {
            self.completeness = 1.0;
            self.accuracy = 1.0;
            self.status = QualityStatus::Valid;
        } else {
            self.completeness = 0.0;
            self.accuracy = 0.0;
            self.status = QualityStatus::Invalid;
            self.errors.push("Value is null or empty".to_string());
        }
        self.update_quality_score();
    }

    /// Validate numeric format
    pub fn validate_numeric(&mut self) {
        match &self.value {
            Value::Number(_) => {
                self.validity = 1.0;
                self.accuracy = 1.0;
                self.status = QualityStatus::Valid;
            }
            _ => {
                let str_val = self.value.to_string();
                if str_val.parse::<f64>().is_ok() {
                    self.validity = 1.0;
                    self.accuracy = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.validity = 0.0;
                    self.accuracy = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Value is not numeric".to_string());
                }
            }
        }
        self.update_quality_score();
    }

    // ============ PHASE 8.2: ARRAY VALIDATORS ============

    /// Validate array element count is within range [min, max]
    pub fn validate_array_length(&mut self, min: usize, max: usize) {
        match &self.value {
            Value::Array(arr) => {
                let len = arr.len();
                if len >= min && len <= max {
                    self.completeness = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Array length {} out of range [{}, {}]", len, min, max));
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate all array elements are unique (no duplicates)
    pub fn validate_array_unique(&mut self) {
        match &self.value {
            Value::Array(arr) => {
                let mut seen = std::collections::HashSet::new();
                let mut has_duplicates = false;
                
                for item in arr {
                    let str_item = item.to_string();
                    if seen.contains(&str_item) {
                        has_duplicates = true;
                        break;
                    }
                    seen.insert(str_item);
                }
                
                if !has_duplicates {
                    self.uniqueness = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.uniqueness = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Array contains duplicate values".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate all array elements are positive numbers
    pub fn validate_array_all_positive(&mut self) {
        match &self.value {
            Value::Array(arr) => {
                let all_positive = arr.iter().all(|item| {
                    match item {
                        Value::Number(n) => n > 0.0,
                        _ => false,
                    }
                });
                
                if all_positive && !arr.is_empty() {
                    self.accuracy = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.accuracy = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Not all array elements are positive numbers".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate all array elements are numeric
    pub fn validate_array_all_numeric(&mut self) {
        match &self.value {
            Value::Array(arr) => {
                let all_numeric = arr.iter().all(|item| matches!(item, Value::Number(_)));
                
                if all_numeric && !arr.is_empty() {
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Not all array elements are numeric".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate all array elements are in range [min, max]
    pub fn validate_array_items_in_range(&mut self, min: f64, max: f64) {
        match &self.value {
            Value::Array(arr) => {
                let all_in_range = arr.iter().all(|item| {
                    match item {
                        Value::Number(n) => n >= min && n <= max,
                        _ => false,
                    }
                });
                
                if all_in_range && !arr.is_empty() {
                    self.accuracy = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.accuracy = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Not all array items in range [{}, {}]", min, max));
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate array contains no null values
    pub fn validate_array_no_nulls(&mut self) {
        match &self.value {
            Value::Array(arr) => {
                let has_nulls = arr.iter().any(|item| {
                    matches!(item, Value::Null) || item.to_string() == "null" || item.to_string().is_empty()
                });
                
                if !has_nulls && !arr.is_empty() {
                    self.completeness = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Array contains null or empty values".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not an array".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    // ============ PHASE 8.2: DICTIONARY/COLLECTION VALIDATORS ============

    /// Validate dictionary contains all required keys
    pub fn validate_dict_required_keys(&mut self, required_keys: Vec<String>) {
        match &self.value {
            Value::Dict(dict) => {
                let has_all_keys = required_keys.iter().all(|key| dict.contains_key(key));
                
                if has_all_keys {
                    self.completeness = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    let missing: Vec<&String> = required_keys.iter()
                        .filter(|k| !dict.contains_key(*k))
                        .collect();
                    self.errors.push(format!("Missing required keys: {:?}", missing));
                }
            }
            _ => {
                self.errors.push("Value is not a dictionary".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate dictionary has no empty string values
    pub fn validate_dict_no_empty_values(&mut self) {
        match &self.value {
            Value::Dict(dict) => {
                let has_empty = dict.iter().any(|(_, v)| {
                    let str_val = v.to_string();
                    str_val.is_empty() || str_val == "null" || str_val == "nil"
                });
                
                if !has_empty {
                    self.completeness = 1.0;
                    self.accuracy = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.accuracy = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Dictionary contains empty values".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not a dictionary".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate dictionary size does not exceed maximum
    pub fn validate_dict_max_size(&mut self, max_size: usize) {
        match &self.value {
            Value::Dict(dict) => {
                let size = dict.len();
                
                if size <= max_size {
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Dictionary size {} exceeds maximum {}", size, max_size));
                }
            }
            _ => {
                self.errors.push("Value is not a dictionary".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    // ============ PHASE 8.3: OBJECT VALIDATORS ============

    /// Validate object has all required fields
    pub fn validate_object_required_fields(&mut self, required_fields: Vec<String>) {
        match &self.value {
            Value::Object(obj) => {
                let has_all_fields = required_fields.iter().all(|field| {
                    obj.fields.contains_key(field)
                });
                
                if has_all_fields {
                    self.completeness = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    let missing: Vec<&String> = required_fields.iter()
                        .filter(|f| !obj.fields.contains_key(*f))
                        .collect();
                    self.errors.push(format!("Missing required object fields: {:?}", missing));
                }
            }
            _ => {
                self.errors.push("Value is not an object".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate all object fields are not null/empty
    pub fn validate_object_all_fields_not_null(&mut self) {
        match &self.value {
            Value::Object(obj) => {
                let has_nulls = obj.fields.values().any(|v| {
                    matches!(v, Value::Null) || v.to_string() == "null" || v.to_string().is_empty()
                });
                
                if !has_nulls && !obj.fields.is_empty() {
                    self.completeness = 1.0;
                    self.accuracy = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.accuracy = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push("Object contains null or empty fields".to_string());
                }
            }
            _ => {
                self.errors.push("Value is not an object".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate object field count does not exceed maximum
    pub fn validate_object_max_fields(&mut self, max_fields: usize) {
        match &self.value {
            Value::Object(obj) => {
                let field_count = obj.fields.len();
                
                if field_count <= max_fields {
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Object has {} fields, maximum {}", field_count, max_fields));
                }
            }
            _ => {
                self.errors.push("Value is not an object".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate object has minimum number of fields
    pub fn validate_object_min_fields(&mut self, min_fields: usize) {
        match &self.value {
            Value::Object(obj) => {
                let field_count = obj.fields.len();
                
                if field_count >= min_fields {
                    self.completeness = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.completeness = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Object has {} fields, minimum {}", field_count, min_fields));
                }
            }
            _ => {
                self.errors.push("Value is not an object".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    /// Validate object class name matches expected
    pub fn validate_object_class(&mut self, expected_class: &str) {
        match &self.value {
            Value::Object(obj) => {
                if obj.class_name == expected_class {
                    self.consistency = 1.0;
                    self.validity = 1.0;
                    self.status = QualityStatus::Valid;
                } else {
                    self.consistency = 0.0;
                    self.validity = 0.0;
                    self.status = QualityStatus::Invalid;
                    self.errors.push(format!("Object class is '{}', expected '{}'", obj.class_name, expected_class));
                }
            }
            _ => {
                self.errors.push("Value is not an object".to_string());
                self.status = QualityStatus::Invalid;
            }
        }
        self.update_quality_score();
    }

    // ============ INFORMATION METHODS ============

    /// Get quality score (0.0 to 1.0)
    pub fn quality(&self) -> f64 {
        self.quality_score
    }

    /// Get quality level as string
    pub fn get_level_str(&self) -> &str {
        self.level.as_str()
    }

    /// Check if data is valid
    pub fn is_valid(&self) -> bool {
        self.status == QualityStatus::Valid
    }

    /// Get status as string
    pub fn get_status_str(&self) -> &str {
        self.status.as_str()
    }

    /// Get error messages
    pub fn get_errors(&self) -> Vec<String> {
        self.errors.clone()
    }

    /// Get warning messages
    pub fn get_warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }

    /// Get list of guarantees
    pub fn get_guarantees(&self) -> Vec<String> {
        self.guarantees
            .iter()
            .map(|g| g.as_str().to_string())
            .collect()
    }

    /// Get audit trail
    pub fn get_audit_trail(&self) -> Vec<String> {
        self.audit_log.clone()
    }

    /// Get all metrics as a map
    pub fn get_all_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        metrics.insert("completeness".to_string(), self.completeness);
        metrics.insert("accuracy".to_string(), self.accuracy);
        metrics.insert("consistency".to_string(), self.consistency);
        metrics.insert("uniqueness".to_string(), self.uniqueness);
        metrics.insert("timeliness".to_string(), self.timeliness);
        metrics.insert("validity".to_string(), self.validity);
        metrics.insert("quality_score".to_string(), self.quality_score);
        metrics
    }

    /// Get TRIM metrics (Truthfulness, Representativeness, Integrity, Modernness)
    /// TRIM is an alternative quality framework
    pub fn get_trim_metrics(&self) -> HashMap<String, f64> {
        let mut trim = HashMap::new();
        
        // Truthfulness ≈ Accuracy + Validity
        trim.insert(
            "truthfulness".to_string(),
            (self.accuracy + self.validity) / 2.0,
        );
        
        // Representativeness ≈ Completeness + Consistency
        trim.insert(
            "representativeness".to_string(),
            (self.completeness + self.consistency) / 2.0,
        );
        
        // Integrity ≈ Consistency + Uniqueness
        trim.insert(
            "integrity".to_string(),
            (self.consistency + self.uniqueness) / 2.0,
        );
        
        // Modernness ≈ Timeliness
        trim.insert(
            "modernness".to_string(),
            self.timeliness,
        );
        
        // Overall TRIM score (average of 4)
        let trim_score = (trim["truthfulness"] + trim["representativeness"] 
                         + trim["integrity"] + trim["modernness"]) / 4.0;
        trim.insert("trim_score".to_string(), trim_score);
        
        trim
    }

    /// Get TRIM score as single value (0.0 to 1.0)
    pub fn get_trim_score(&self) -> f64 {
        let trim = self.get_trim_metrics();
        trim["trim_score"]
    }

    // ============ METADATA METHODS ============

    /// Add guarantee (doesn't change quality score)
    pub fn add_guarantee(&mut self, guarantee: Guarantee) {
        if !self.guarantees.contains(&guarantee) {
            self.guarantees.push(guarantee);
        }
    }

    /// Add audit log entry
    pub fn audit(&mut self, message: &str) {
        self.audit_log.push(message.to_string());
    }

    /// Add error message
    pub fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }

    /// Add warning message
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }

    /// Get raw value
    pub fn get_value(&self) -> Value {
        self.value.clone()
    }

    /// Convert to string representation
    pub fn to_string_detailed(&self) -> String {
        format!(
            "DataQuality {{ value: {}, score: {:.2}, level: {}, status: {} }}",
            self.value,
            self.quality_score,
            self.level.as_str(),
            self.status.as_str()
        )
    }
}

impl std::fmt::Display for DataQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_quality_variable() {
        let val = Value::Str("test".to_string());
        let dq = DataQuality::new(val);
        // Check that initial quality is low (not all metrics known)
        assert!(dq.quality() > 0.0 && dq.quality() < 1.0);
        assert_eq!(dq.status, QualityStatus::Unknown);
    }

    #[test]
    fn test_validate_email_valid() {
        let val = Value::Str("alice@example.com".to_string());
        let mut dq = DataQuality::new(val);
        dq.validate_email();
        
        assert!(dq.is_valid());
        assert_eq!(dq.status, QualityStatus::Valid);
        assert!(dq.quality() >= 0.83);
    }

    #[test]
    fn test_validate_email_invalid() {
        let val = Value::Str("invalid-email".to_string());
        let mut dq = DataQuality::new(val);
        dq.validate_email();
        
        assert!(!dq.is_valid());
        assert_eq!(dq.status, QualityStatus::Invalid);
        assert!(!dq.errors.is_empty());
    }

    #[test]
    fn test_validate_phone_valid() {
        let val = Value::Str("555-123-4567".to_string());
        let mut dq = DataQuality::new(val);
        dq.validate_phone();
        
        assert!(dq.is_valid());
        assert_eq!(dq.status, QualityStatus::Valid);
    }

    #[test]
    fn test_validate_phone_invalid() {
        let val = Value::Str("123".to_string());
        let mut dq = DataQuality::new(val);
        dq.validate_phone();
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_positive() {
        let val = Value::Number(100.0);
        let mut dq = DataQuality::new(val);
        dq.validate_positive();
        
        assert!(dq.is_valid());
        assert_eq!(dq.accuracy, 1.0);
    }

    #[test]
    fn test_validate_positive_negative() {
        let val = Value::Number(-50.0);
        let mut dq = DataQuality::new(val);
        dq.validate_positive();
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_range() {
        let val = Value::Number(50.0);
        let mut dq = DataQuality::new(val);
        dq.validate_range(0.0, 100.0);
        
        assert!(dq.is_valid());
    }

    #[test]
    fn test_validate_range_out() {
        let val = Value::Number(150.0);
        let mut dq = DataQuality::new(val);
        dq.validate_range(0.0, 100.0);
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_quality_level_excellent() {
        let val = Value::Str("test".to_string());
        let mut dq = DataQuality::new(val);
        
        // Set all metrics to 1.0
        dq.completeness = 1.0;
        dq.accuracy = 1.0;
        dq.consistency = 1.0;
        dq.uniqueness = 1.0;
        dq.timeliness = 1.0;
        dq.validity = 1.0;
        dq.update_quality_score();
        
        assert_eq!(dq.quality(), 1.0);
        assert_eq!(dq.level, QualityLevel::Excellent);
    }

    #[test]
    fn test_guarantee() {
        let val = Value::Str("sensitive".to_string());
        let mut dq = DataQuality::new(val);
        
        dq.add_guarantee(Guarantee::Privacy);
        dq.add_guarantee(Guarantee::Encryption);
        
        assert_eq!(dq.guarantees.len(), 2);
    }

    #[test]
    fn test_audit_trail() {
        let val = Value::Str("data".to_string());
        let mut dq = DataQuality::new(val);
        
        dq.audit("Created");
        dq.audit("Validated");
        dq.audit("Saved");
        
        assert_eq!(dq.audit_log.len(), 3);
        assert_eq!(dq.audit_log[0], "Created");
    }

    #[test]
    fn test_quality_calculation() {
        let val = Value::Str("test@test.com".to_string());
        let mut dq = DataQuality::new(val);
        
        // Validate email
        dq.validate_email();
        
        // Score should be > 0.8 (good)
        assert!(dq.quality() >= 0.80);
    }

    #[test]
    fn test_trim_metrics() {
        let val = Value::Str("test@test.com".to_string());
        let mut dq = DataQuality::new(val);
        
        // Validate email
        dq.validate_email();
        
        // Get TRIM metrics
        let trim = dq.get_trim_metrics();
        
        // Should have all 4 TRIM + overall score
        assert!(trim.contains_key("truthfulness"));
        assert!(trim.contains_key("representativeness"));
        assert!(trim.contains_key("integrity"));
        assert!(trim.contains_key("modernness"));
        assert!(trim.contains_key("trim_score"));
        
        // TRIM score should be valid (0.0 to 1.0)
        let trim_score = dq.get_trim_score();
        assert!(trim_score >= 0.0 && trim_score <= 1.0);
    }

    #[test]
    fn test_trim_vs_six_metrics() {
        let val = Value::Str("alice@example.com".to_string());
        let mut dq = DataQuality::new(val);
        dq.validate_email();
        
        let six_metric_score = dq.quality();
        let trim_score = dq.get_trim_score();
        
        // Both should give similar but different scores
        // TRIM is simplified version of 6 metrics
        assert!(six_metric_score > 0.0);
        assert!(trim_score > 0.0);
        // They should be close but not identical
        assert!((six_metric_score - trim_score).abs() < 0.5);
    }

    // ============ PHASE 8.2: ARRAY VALIDATOR TESTS ============

    #[test]
    fn test_validate_array_length_valid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_length(1, 5);
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_array_length_invalid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_length(5, 10);
        
        assert!(!dq.is_valid());
        assert!(!dq.errors.is_empty());
    }

    #[test]
    fn test_validate_array_unique_valid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_unique();
        
        assert!(dq.is_valid());
        assert_eq!(dq.uniqueness, 1.0);
    }

    #[test]
    fn test_validate_array_unique_duplicates() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(1.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_unique();
        
        assert!(!dq.is_valid());
        assert_eq!(dq.uniqueness, 0.0);
    }

    #[test]
    fn test_validate_array_all_positive_valid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.5),
            Value::Number(100.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_all_positive();
        
        assert!(dq.is_valid());
        assert_eq!(dq.accuracy, 1.0);
    }

    #[test]
    fn test_validate_array_all_positive_negative() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(-2.0),
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_all_positive();
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_array_all_numeric_valid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_all_numeric();
        
        assert!(dq.is_valid());
        assert_eq!(dq.validity, 1.0);
    }

    #[test]
    fn test_validate_array_all_numeric_mixed() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Str("two".to_string()),
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_all_numeric();
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_array_items_in_range_valid() {
        let arr = Value::from(vec![
            Value::Number(25.0),
            Value::Number(50.0),
            Value::Number(75.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_items_in_range(0.0, 100.0);
        
        assert!(dq.is_valid());
        assert_eq!(dq.accuracy, 1.0);
    }

    #[test]
    fn test_validate_array_items_in_range_out() {
        let arr = Value::from(vec![
            Value::Number(25.0),
            Value::Number(150.0),
            Value::Number(75.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_items_in_range(0.0, 100.0);
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_array_no_nulls_valid() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Str("test".to_string()),
            Value::Bool(true),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_no_nulls();
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_array_no_nulls_with_null() {
        let arr = Value::from(vec![
            Value::Number(1.0),
            Value::Null,
            Value::Number(3.0),
        ]);
        let mut dq = DataQuality::new(arr);
        dq.validate_array_no_nulls();
        
        assert!(!dq.is_valid());
        assert_eq!(dq.completeness, 0.0);
    }

    // ============ PHASE 8.2: DICTIONARY VALIDATOR TESTS ============

    #[test]
    fn test_validate_dict_required_keys_valid() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        dict.insert("email".to_string(), Value::Str("alice@test.com".to_string()));
        dict.insert("age".to_string(), Value::Number(30.0));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_required_keys(vec!["name".to_string(), "email".to_string()]);
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_dict_required_keys_missing() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_required_keys(vec!["name".to_string(), "email".to_string()]);
        
        assert!(!dq.is_valid());
        assert_eq!(dq.completeness, 0.0);
    }

    #[test]
    fn test_validate_dict_no_empty_values_valid() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        dict.insert("email".to_string(), Value::Str("alice@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_no_empty_values();
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_dict_no_empty_values_empty() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        dict.insert("email".to_string(), Value::Str("".to_string()));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_no_empty_values();
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_dict_max_size_valid() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        dict.insert("email".to_string(), Value::Str("alice@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_max_size(5);
        
        assert!(dq.is_valid());
        assert_eq!(dq.validity, 1.0);
    }

    #[test]
    fn test_validate_dict_max_size_exceeded() {
        let mut dict = HashMap::new();
        dict.insert("name".to_string(), Value::Str("Alice".to_string()));
        dict.insert("email".to_string(), Value::Str("alice@test.com".to_string()));
        dict.insert("age".to_string(), Value::Number(30.0));
        
        let mut dq = DataQuality::new(Value::Dict(Box::new(dict)));
        dq.validate_dict_max_size(2);
        
        assert!(!dq.is_valid());
    }

    // ============ PHASE 8.3: OBJECT VALIDATOR TESTS ============

    #[test]
    fn test_validate_object_required_fields_valid() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(123.0));
        obj.fields.insert("name".to_string(), Value::Str("Alice".to_string()));
        obj.fields.insert("email".to_string(), Value::Str("alice@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_required_fields(vec!["id".to_string(), "name".to_string()]);
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_object_required_fields_missing() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(456.0));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_required_fields(vec!["id".to_string(), "email".to_string()]);
        
        assert!(!dq.is_valid());
        assert_eq!(dq.completeness, 0.0);
    }

    #[test]
    fn test_validate_object_all_fields_not_null_valid() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("name".to_string(), Value::Str("Bob".to_string()));
        obj.fields.insert("email".to_string(), Value::Str("bob@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_all_fields_not_null();
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_object_all_fields_not_null_with_null() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("name".to_string(), Value::Str("Alice".to_string()));
        obj.fields.insert("email".to_string(), Value::Null);
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_all_fields_not_null();
        
        assert!(!dq.is_valid());
        assert_eq!(dq.completeness, 0.0);
    }

    #[test]
    fn test_validate_object_max_fields_valid() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(789.0));
        obj.fields.insert("name".to_string(), Value::Str("Charlie".to_string()));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_max_fields(5);
        
        assert!(dq.is_valid());
        assert_eq!(dq.validity, 1.0);
    }

    #[test]
    fn test_validate_object_max_fields_exceeded() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(101.0));
        obj.fields.insert("name".to_string(), Value::Str("David".to_string()));
        obj.fields.insert("email".to_string(), Value::Str("david@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_max_fields(2);
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_object_min_fields_valid() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(202.0));
        obj.fields.insert("name".to_string(), Value::Str("Eve".to_string()));
        obj.fields.insert("email".to_string(), Value::Str("eve@test.com".to_string()));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_min_fields(2);
        
        assert!(dq.is_valid());
        assert_eq!(dq.completeness, 1.0);
    }

    #[test]
    fn test_validate_object_min_fields_insufficient() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(303.0));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_min_fields(3);
        
        assert!(!dq.is_valid());
    }

    #[test]
    fn test_validate_object_class_valid() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "User".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(404.0));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_class("User");
        
        assert!(dq.is_valid());
        assert_eq!(dq.consistency, 1.0);
    }

    #[test]
    fn test_validate_object_class_mismatch() {
        use crate::value::ObjectInstance;
        
        let mut obj = ObjectInstance {
            class_name: "Product".to_string(),
            fields: HashMap::new(),
        };
        obj.fields.insert("id".to_string(), Value::Number(505.0));
        
        let mut dq = DataQuality::new(Value::Object(Box::new(obj)));
        dq.validate_object_class("User");
        
        assert!(!dq.is_valid());
        assert_eq!(dq.consistency, 0.0);
    }
}
