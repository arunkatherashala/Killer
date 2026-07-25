#![allow(unsafe_code)]

//! Version management and compatibility tracking
//! 
//! Killer VM follows semantic versioning: MAJOR.MINOR.PATCH
//! - MAJOR: Breaking API changes
//! - MINOR: New features (backward compatible)
//! - PATCH: Bug fixes only

/// Current version of Killer VM
pub const VERSION: &str = "2.1.0";

/// Killer Standard Library / Algorithm Library version
/// v1.2.0 adds: Native HashMap (O(1)), Dijkstra shortest path,
///              BST, Dynamic Programming (fib/knapsack/LCS/coin_change)
pub const STDLIB_VERSION: &str = "1.2.0";

/// Algorithm library codename for this release
pub const STDLIB_CODENAME: &str = "Enterprise";

/// Minimum compatible version for backward compatibility
pub const MIN_COMPATIBLE_VERSION: &str = "2.0.0";

/// API stability levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Stable - guaranteed backward compatible across versions
    Stable,
    /// Unstable - may change without notice
    Unstable,
    /// Deprecated - will be removed in next major version
    Deprecated,
}

/// Version information structure
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub stability: &'static str,
}

impl VersionInfo {
    /// Parse version string (e.g., "2.1.0")
    pub fn parse(version_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", version_str));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(VersionInfo {
            major,
            minor,
            patch,
            stability: "stable",
        })
    }

    /// Check if this version is compatible with another
    pub fn is_compatible_with(&self, other: &VersionInfo) -> bool {
        // Compatible if: same MAJOR, MINOR >= other.MINOR
        self.major == other.major && self.minor >= other.minor
    }

    /// Format as version string
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Global version information
pub fn get_version() -> VersionInfo {
    VersionInfo::parse(VERSION).expect("Invalid built-in version")
}

/// Check compatibility with minimum version
pub fn check_compatibility(required_version: &str) -> Result<(), String> {
    let current = get_version();
    let required = VersionInfo::parse(required_version)?;

    if current.is_compatible_with(&required) {
        Ok(())
    } else {
        Err(format!(
            "Version mismatch: required {}, have {}",
            required_version, VERSION
        ))
    }
}

/// Feature availability based on version
pub fn feature_available(_feature: &str, since_version: &str) -> bool {
    let current = match get_version().to_string().as_str() {
        v => {
            if let Ok(ver) = VersionInfo::parse(v) {
                ver
            } else {
                return false;
            }
        }
    };

    match VersionInfo::parse(since_version) {
        Ok(required) => current.is_compatible_with(&required),
        Err(_) => false,
    }
}

/// Deprecation information for tracking API changes
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    /// Name of the deprecated item
    pub name: String,
    /// Version when deprecated
    pub deprecated_since: String,
    /// Suggested replacement
    pub replacement: Option<String>,
    /// Migration notes
    pub migration_notes: Option<String>,
    /// Version when it will be removed
    pub removal_version: Option<String>,
}

impl DeprecationInfo {
    /// Create new deprecation info
    pub fn new(name: String, deprecated_since: String) -> Self {
        DeprecationInfo {
            name,
            deprecated_since,
            replacement: None,
            migration_notes: None,
            removal_version: None,
        }
    }

    /// Set replacement
    pub fn with_replacement(mut self, replacement: String) -> Self {
        self.replacement = Some(replacement);
        self
    }

    /// Set migration notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.migration_notes = Some(notes);
        self
    }

    /// Set removal version
    pub fn with_removal(mut self, version: String) -> Self {
        self.removal_version = Some(version);
        self
    }

    /// Generate deprecation warning message
    pub fn warning_message(&self) -> String {
        let mut msg = format!(
            "⚠️  DEPRECATED: '{}' is deprecated since v{}",
            self.name, self.deprecated_since
        );

        if let Some(ref replacement) = self.replacement {
            msg.push_str(&format!("\n   → Use '{}' instead", replacement));
        }

        if let Some(ref notes) = self.migration_notes {
            msg.push_str(&format!("\n   → Migration: {}", notes));
        }

        if let Some(ref removal) = self.removal_version {
            msg.push_str(&format!("\n   → Will be removed in v{}", removal));
        }

        msg
    }
}

/// Deprecation registry for tracking deprecated items
pub struct DeprecationRegistry {
    items: Vec<DeprecationInfo>,
}

impl DeprecationRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        DeprecationRegistry {
            items: Vec::new(),
        }
    }

    /// Register a deprecated item
    pub fn register(&mut self, info: DeprecationInfo) {
        self.items.push(info);
    }

    /// Get deprecation info by name
    pub fn get(&self, name: &str) -> Option<&DeprecationInfo> {
        self.items.iter().find(|item| item.name == name)
    }

    /// Get all deprecated items for a version
    pub fn get_by_version(&self, version: &str) -> Vec<&DeprecationInfo> {
        self.items.iter()
            .filter(|item| item.deprecated_since == version)
            .collect()
    }

    /// Check if item is deprecated
    pub fn is_deprecated(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Count deprecated items
    pub fn count(&self) -> usize {
        self.items.len()
    }
}

impl Default for DeprecationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global deprecation registry
static mut DEPRECATION_REGISTRY: Option<DeprecationRegistry> = Some(
    DeprecationRegistry {
        items: Vec::new(),
    }
);

/// Get or create global deprecation registry
#[allow(static_mut_refs)]
pub fn get_deprecation_registry() -> &'static mut DeprecationRegistry {
    unsafe {
        DEPRECATION_REGISTRY.as_mut().expect("Deprecation registry uninitialized")
    }
}

/// Check if a feature is deprecated
pub fn is_deprecated(name: &str) -> bool {
    get_deprecation_registry().is_deprecated(name)
}

/// Get deprecation info for a feature
pub fn get_deprecation_info(name: &str) -> Option<DeprecationInfo> {
    get_deprecation_registry().get(name).cloned()
}

/// Register deprecation (for internal use)
pub fn register_deprecation(info: DeprecationInfo) {
    get_deprecation_registry().register(info);
}

/// API function stability marker
#[macro_export]
macro_rules! api_stable {
    ($name:expr, $since:expr) => {
        const _: () = {
            const _API_STABLE: &str = $name;
            const _SINCE: &str = $since;
        };
    };
}

/// Mark function as deprecated
#[macro_export]
macro_rules! api_deprecated {
    ($name:expr, $since:expr, $replacement:expr) => {
        #[deprecated(
            since = $since,
            note = concat!("use `", $replacement, "` instead")
        )]
        fn _deprecated_marker() {}
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = VersionInfo::parse("2.1.0").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = VersionInfo::parse("2.1.0").unwrap();
        let v2 = VersionInfo::parse("2.0.5").unwrap();
        assert!(v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_version_incompatibility() {
        let v1 = VersionInfo::parse("2.1.0").unwrap();
        let v2 = VersionInfo::parse("3.0.0").unwrap();
        assert!(!v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_feature_available() {
        assert!(feature_available("repl", "2.0.0"));
        assert!(feature_available("debugger", "2.1.0"));
    }

    #[test]
    fn test_deprecation_info_creation() {
        let dep = DeprecationInfo::new("old_fn".to_string(), "2.1.0".to_string())
            .with_replacement("new_fn".to_string())
            .with_notes("Use new_fn for better performance".to_string())
            .with_removal("3.0.0".to_string());

        assert_eq!(dep.name, "old_fn");
        assert_eq!(dep.deprecated_since, "2.1.0");
        assert_eq!(dep.replacement, Some("new_fn".to_string()));
        assert_eq!(dep.removal_version, Some("3.0.0".to_string()));
    }

    #[test]
    fn test_deprecation_warning_message() {
        let dep = DeprecationInfo::new("old_fn".to_string(), "2.1.0".to_string())
            .with_replacement("new_fn".to_string())
            .with_removal("3.0.0".to_string());

        let warning = dep.warning_message();
        assert!(warning.contains("DEPRECATED"));
        assert!(warning.contains("old_fn"));
        assert!(warning.contains("new_fn"));
        assert!(warning.contains("3.0.0"));
    }

    #[test]
    fn test_deprecation_registry() {
        let mut registry = DeprecationRegistry::new();
        
        let dep1 = DeprecationInfo::new("func1".to_string(), "2.1.0".to_string())
            .with_replacement("func1_new".to_string());
        let dep2 = DeprecationInfo::new("func2".to_string(), "2.1.0".to_string());

        registry.register(dep1);
        registry.register(dep2);

        assert_eq!(registry.count(), 2);
        assert!(registry.is_deprecated("func1"));
        assert!(registry.is_deprecated("func2"));
        assert!(!registry.is_deprecated("func3"));
    }

    #[test]
    fn test_deprecation_registry_by_version() {
        let mut registry = DeprecationRegistry::new();
        
        let dep1 = DeprecationInfo::new("func1".to_string(), "2.1.0".to_string());
        let dep2 = DeprecationInfo::new("func2".to_string(), "2.2.0".to_string());
        let dep3 = DeprecationInfo::new("func3".to_string(), "2.1.0".to_string());

        registry.register(dep1);
        registry.register(dep2);
        registry.register(dep3);

        let v210 = registry.get_by_version("2.1.0");
        assert_eq!(v210.len(), 2);
        
        let v220 = registry.get_by_version("2.2.0");
        assert_eq!(v220.len(), 1);
    }

    #[test]
    fn test_global_deprecation_functions() {
        let dep = DeprecationInfo::new("test_func".to_string(), "2.1.0".to_string())
            .with_replacement("test_func_v2".to_string());
        
        register_deprecation(dep);
        assert!(is_deprecated("test_func"));
        
        let info = get_deprecation_info("test_func");
        assert!(info.is_some());
        if let Some(info) = info {
            assert_eq!(info.replacement, Some("test_func_v2".to_string()));
        }
    }
}
