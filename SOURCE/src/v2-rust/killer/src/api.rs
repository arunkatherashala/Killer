//! Public API Contract for Killer VM
//! 
//! This module defines the stable public API, version compatibility,
//! and backward compatibility guarantees for users of the Killer runtime.
//! 
//! # Stability Levels
//! - **Stable**: Will not change within a major version (guaranteed backward compat)
//! - **Unstable**: May change without notice (experimental features)
//! - **Deprecated**: Scheduled for removal in next major version

use crate::version::{VersionInfo, DeprecationInfo, register_deprecation};
use std::collections::HashMap;

/// API stability marker for functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStability {
    /// Stable API - guaranteed backward compatible
    Stable,
    /// Unstable API - may change without notice
    Unstable,
    /// Deprecated API - scheduled for removal
    Deprecated,
}

/// API function metadata
#[derive(Debug, Clone)]
pub struct ApiFunction {
    /// Function name
    pub name: String,
    /// Stability level
    pub stability: ApiStability,
    /// Version introduced
    pub introduced: String,
    /// Deprecation info (if deprecated)
    pub deprecation: Option<DeprecationInfo>,
    /// Description
    pub description: String,
}

impl ApiFunction {
    /// Create new stable API function
    pub fn stable(name: String, introduced: String, description: String) -> Self {
        ApiFunction {
            name,
            stability: ApiStability::Stable,
            introduced,
            deprecation: None,
            description,
        }
    }

    /// Create new unstable API function
    pub fn unstable(name: String, introduced: String, description: String) -> Self {
        ApiFunction {
            name,
            stability: ApiStability::Unstable,
            introduced,
            deprecation: None,
            description,
        }
    }

    /// Create deprecated API function
    pub fn deprecated(name: String, introduced: String, description: String) -> Self {
        ApiFunction {
            name,
            stability: ApiStability::Deprecated,
            introduced,
            deprecation: None,
            description,
        }
    }

    /// Set deprecation info
    pub fn with_deprecation(mut self, info: DeprecationInfo) -> Self {
        self.deprecation = Some(info);
        self
    }
}

/// Public API contract registry
pub struct ApiContract {
    functions: HashMap<String, ApiFunction>,
    version: VersionInfo,
}

impl ApiContract {
    /// Create new API contract for current version
    pub fn new(version: VersionInfo) -> Self {
        ApiContract {
            functions: HashMap::new(),
            version,
        }
    }

    /// Register a public API function
    pub fn register(&mut self, func: ApiFunction) {
        self.functions.insert(func.name.clone(), func.clone());
        
        // Auto-register deprecation if deprecated
        if let Some(deprecation) = func.deprecation.clone() {
            register_deprecation(deprecation);
        }
    }

    /// Get API function info
    pub fn get(&self, name: &str) -> Option<&ApiFunction> {
        self.functions.get(name)
    }

    /// Check if API is stable
    pub fn is_stable(&self, name: &str) -> bool {
        self.get(name)
            .map(|f| f.stability == ApiStability::Stable)
            .unwrap_or(false)
    }

    /// Check if API exists
    pub fn exists(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get all stable APIs
    pub fn get_stable_apis(&self) -> Vec<&ApiFunction> {
        self.functions
            .values()
            .filter(|f| f.stability == ApiStability::Stable)
            .collect()
    }

    /// Get all unstable APIs
    pub fn get_unstable_apis(&self) -> Vec<&ApiFunction> {
        self.functions
            .values()
            .filter(|f| f.stability == ApiStability::Unstable)
            .collect()
    }

    /// Get all deprecated APIs
    pub fn get_deprecated_apis(&self) -> Vec<&ApiFunction> {
        self.functions
            .values()
            .filter(|f| f.stability == ApiStability::Deprecated)
            .collect()
    }

    /// Get count of functions by stability
    pub fn count_by_stability(&self) -> (usize, usize, usize) {
        let stable = self.get_stable_apis().len();
        let unstable = self.get_unstable_apis().len();
        let deprecated = self.get_deprecated_apis().len();
        (stable, unstable, deprecated)
    }

    /// Generate API compatibility report
    pub fn compatibility_report(&self) -> String {
        let (stable, unstable, deprecated) = self.count_by_stability();
        let total = stable + unstable + deprecated;
        
        format!(
            "=== Killer API Contract v{} ===\n\
             Total Functions: {}\n\
             Stable: {} ({:.1}%)\n\
             Unstable: {} ({:.1}%)\n\
             Deprecated: {} ({:.1}%)",
            self.version.to_string(),
            total,
            stable,
            if total > 0 { (stable as f64 / total as f64) * 100.0 } else { 0.0 },
            unstable,
            if total > 0 { (unstable as f64 / total as f64) * 100.0 } else { 0.0 },
            deprecated,
            if total > 0 { (deprecated as f64 / total as f64) * 100.0 } else { 0.0 }
        )
    }
}

/// Backward compatibility layer
pub struct BackwardCompatibility {
    /// Mapping of old API to new API (for aliases)
    pub aliases: HashMap<String, String>,
    /// Deprecated since version -> replacement version
    pub migration_paths: HashMap<String, String>,
}

impl BackwardCompatibility {
    /// Create new compatibility layer
    pub fn new() -> Self {
        BackwardCompatibility {
            aliases: HashMap::new(),
            migration_paths: HashMap::new(),
        }
    }

    /// Add API alias (old name -> new name)
    pub fn add_alias(&mut self, old_name: String, new_name: String) {
        self.aliases.insert(old_name, new_name);
    }

    /// Get canonical name for API (resolves aliases)
    pub fn resolve_alias(&self, name: &str) -> String {
        self.aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Add migration path
    pub fn add_migration_path(&mut self, old_version: String, new_version: String) {
        self.migration_paths.insert(old_version, new_version);
    }

    /// Get migration path from old version
    pub fn get_migration_path(&self, from_version: &str) -> Option<&String> {
        self.migration_paths.get(from_version)
    }

    /// Check if all aliases are resolvable in contract
    pub fn validate_against_contract(&self, contract: &ApiContract) -> Vec<String> {
        let mut errors = Vec::new();
        
        for (old_name, new_name) in &self.aliases {
            if !contract.exists(new_name) {
                errors.push(format!(
                    "Alias '{}' -> '{}' references non-existent API",
                    old_name, new_name
                ));
            }
        }
        
        errors
    }
}

impl Default for BackwardCompatibility {
    fn default() -> Self {
        Self::new()
    }
}

/// Core Killer VM public API functions (STABLE in v2.1.0)
pub fn create_default_api_contract() -> ApiContract {
    let mut contract = ApiContract::new(crate::version::get_version());

    // ===== Core VM Functions (STABLE) =====
    contract.register(ApiFunction::stable(
        "vm::new".to_string(),
        "2.0.0".to_string(),
        "Create new virtual machine instance".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "vm::execute".to_string(),
        "2.0.0".to_string(),
        "Execute bytecode in virtual machine".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "vm::reset".to_string(),
        "2.0.0".to_string(),
        "Clear all state from virtual machine".to_string(),
    ));

    // ===== Compiler Functions (STABLE) =====
    contract.register(ApiFunction::stable(
        "compiler::compile".to_string(),
        "2.0.0".to_string(),
        "Compile source code to bytecode".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "compiler::optimize".to_string(),
        "2.0.0".to_string(),
        "Optimize compiled bytecode".to_string(),
    ));

    // ===== Parser Functions (STABLE) =====
    contract.register(ApiFunction::stable(
        "parser::parse".to_string(),
        "2.0.0".to_string(),
        "Parse source code to AST".to_string(),
    ));

    // ===== REPL Functions (STABLE - NEW in v2.1.0) =====
    contract.register(ApiFunction::stable(
        "repl::start".to_string(),
        "2.1.0".to_string(),
        "Start interactive REPL session".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "repl::eval_line".to_string(),
        "2.1.0".to_string(),
        "Evaluate single line in REPL".to_string(),
    ));

    // ===== Debugger Functions (STABLE - NEW in v2.1.0) =====
    contract.register(ApiFunction::stable(
        "debugger::start".to_string(),
        "2.1.0".to_string(),
        "Start interactive debugger session".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "debugger::set_breakpoint".to_string(),
        "2.1.0".to_string(),
        "Set breakpoint at line number".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "debugger::step".to_string(),
        "2.1.0".to_string(),
        "Step to next instruction".to_string(),
    ));

    // ===== Version Functions (STABLE - NEW in v2.1.0) =====
    contract.register(ApiFunction::stable(
        "version::get_version".to_string(),
        "2.1.0".to_string(),
        "Get current Killer version".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "version::check_compatibility".to_string(),
        "2.1.0".to_string(),
        "Check version compatibility".to_string(),
    ));

    // ===== Exception Handling (STABLE) =====
    contract.register(ApiFunction::stable(
        "exception::try_catch".to_string(),
        "2.0.0".to_string(),
        "Execute code with exception handling".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "exception::throw".to_string(),
        "2.0.0".to_string(),
        "Throw exception with message".to_string(),
    ));

    // ===== Generator Functions (STABLE) =====
    contract.register(ApiFunction::stable(
        "generator::create".to_string(),
        "2.0.0".to_string(),
        "Create generator object".to_string(),
    ));

    contract.register(ApiFunction::stable(
        "generator::next".to_string(),
        "2.0.0".to_string(),
        "Get next value from generator".to_string(),
    ));

    // ===== Type Specialization (UNSTABLE) =====
    contract.register(ApiFunction::unstable(
        "specialization::infer_type".to_string(),
        "2.0.0".to_string(),
        "Infer type of expression (may change algorithm)".to_string(),
    ));

    contract.register(ApiFunction::unstable(
        "specialization::generate_code".to_string(),
        "2.0.0".to_string(),
        "Generate specialized code (experimental)".to_string(),
    ));

    contract
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_function_creation() {
        let func = ApiFunction::stable(
            "test_func".to_string(),
            "2.0.0".to_string(),
            "Test function".to_string(),
        );

        assert_eq!(func.name, "test_func");
        assert_eq!(func.stability, ApiStability::Stable);
        assert_eq!(func.introduced, "2.0.0");
    }

    #[test]
    fn test_api_contract_registration() {
        let mut contract = ApiContract::new(VersionInfo::parse("2.1.0").unwrap());
        let func = ApiFunction::stable(
            "func1".to_string(),
            "2.0.0".to_string(),
            "Test".to_string(),
        );

        contract.register(func);
        assert!(contract.exists("func1"));
        assert!(contract.is_stable("func1"));
    }

    #[test]
    fn test_api_contract_filtering() {
        let mut contract = ApiContract::new(VersionInfo::parse("2.1.0").unwrap());

        contract.register(ApiFunction::stable(
            "stable_func".to_string(),
            "2.0.0".to_string(),
            "Stable".to_string(),
        ));

        contract.register(ApiFunction::unstable(
            "unstable_func".to_string(),
            "2.1.0".to_string(),
            "Unstable".to_string(),
        ));

        assert_eq!(contract.get_stable_apis().len(), 1);
        assert_eq!(contract.get_unstable_apis().len(), 1);
    }

    #[test]
    fn test_api_contract_statistics() {
        let mut contract = ApiContract::new(VersionInfo::parse("2.1.0").unwrap());

        contract.register(ApiFunction::stable(
            "func1".to_string(),
            "2.0.0".to_string(),
            "Test1".to_string(),
        ));
        contract.register(ApiFunction::stable(
            "func2".to_string(),
            "2.0.0".to_string(),
            "Test2".to_string(),
        ));
        contract.register(ApiFunction::unstable(
            "func3".to_string(),
            "2.1.0".to_string(),
            "Test3".to_string(),
        ));

        let (stable, unstable, deprecated) = contract.count_by_stability();
        assert_eq!(stable, 2);
        assert_eq!(unstable, 1);
        assert_eq!(deprecated, 0);
    }

    #[test]
    fn test_backward_compatibility_aliases() {
        let mut compat = BackwardCompatibility::new();
        compat.add_alias("old_api".to_string(), "new_api".to_string());

        assert_eq!(compat.resolve_alias("old_api"), "new_api");
        assert_eq!(compat.resolve_alias("other"), "other");
    }

    #[test]
    fn test_migration_paths() {
        let mut compat = BackwardCompatibility::new();
        compat.add_migration_path("2.0.0".to_string(), "2.1.0".to_string());

        let path = compat.get_migration_path("2.0.0");
        assert!(path.is_some());
        assert_eq!(path.unwrap(), "2.1.0");
    }

    #[test]
    fn test_default_api_contract() {
        let contract = create_default_api_contract();
        let (stable, unstable, _deprecated) = contract.count_by_stability();

        // Should have multiple stable APIs from v2.0.0 and v2.1.0
        assert!(stable > 10);
        // Should have some unstable experimental APIs
        assert!(unstable > 0);
    }

    #[test]
    fn test_api_compatibility_report() {
        let contract = create_default_api_contract();
        let report = contract.compatibility_report();

        assert!(report.contains("Killer API Contract"));
        assert!(report.contains("Total Functions:"));
        assert!(report.contains("Stable:"));
    }
}
