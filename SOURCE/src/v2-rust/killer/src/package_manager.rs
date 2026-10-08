// Phase 10: Package Manager - Registry, dependencies, versioning, distribution
// Features: Package metadata, dependency resolution, version management, registry operations

use std::collections::{HashMap, BTreeMap};

/// Semantic version
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Version { major, minor, patch }
    }

    /// Parse from string "1.2.3"
    pub fn parse(version_str: &str) -> Result<Self, String> {
        let version_str = version_str.trim();
        let version_str = version_str.split('+').next().unwrap_or(version_str); // build metadata is ignored
        if version_str.contains('-') {
            return Err(format!("Invalid version '{}': pre-release versions are not supported", version_str));
        }
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version '{}': expected MAJOR.MINOR.PATCH", version_str));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| "Invalid major version".to_string())?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| "Invalid minor version".to_string())?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| "Invalid patch version".to_string())?;

        Ok(Version { major, minor, patch })
    }

    /// Format to string
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Check if compatible (same major version)
    pub fn is_compatible(&self, other: &Version) -> bool {
        self.major == other.major
    }

    /// Increment patch version
    pub fn bump_patch(mut self) -> Self {
        self.patch += 1;
        self
    }

    /// Increment minor version
    pub fn bump_minor(mut self) -> Self {
        self.minor += 1;
        self.patch = 0;
        self
    }

    /// Increment major version
    pub fn bump_major(mut self) -> Self {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Exact,
    Gt,
    Ge,
    Lt,
    Le,
    Caret,
    Tilde,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Comparator {
    op: Op,
    major: u32,
    minor: Option<u32>,
    patch: Option<u32>,
}

fn is_op_only(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| "^~<>=".contains(c))
}

/// A version requirement: comparators joined by `,` (or spaces), all of which must hold.
///
/// Syntax: `^1.2.3` (caret: compatible, 0.x aware), `~1.2.3` (tilde: patch updates only),
/// `>=1.0`, `>1`, `<2.0.0`, `<=1.5`, `=1.2.3` or a bare `1.2.3` (exact), `1.2` / `1.2.x` / `1.*`
/// (any patch / minor), `*` (anything), and ranges such as `>=1.0, <2.0` or `>=1.0 <2.0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionReq {
    comps: Vec<Comparator>,
    text: String,
}

impl VersionReq {
    pub fn any() -> Self {
        VersionReq { comps: Vec::new(), text: "*".to_string() }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let t = text.trim();
        if t.is_empty() || t == "*" {
            return Ok(Self::any());
        }
        // an operator may be separated from its version by a space (">= 1.0")
        let mut toks: Vec<String> = Vec::new();
        for part in t.split(',').flat_map(|p| p.split_whitespace()) {
            match toks.last_mut() {
                Some(last) if is_op_only(last) => last.push_str(part),
                _ => toks.push(part.to_string()),
            }
        }
        let bad = || format!("invalid version requirement '{}'", t);
        let mut comps = Vec::new();
        for tok in toks {
            let (op, rest) = if let Some(r) = tok.strip_prefix(">=") {
                (Op::Ge, r)
            } else if let Some(r) = tok.strip_prefix("<=") {
                (Op::Le, r)
            } else if let Some(r) = tok.strip_prefix('>') {
                (Op::Gt, r)
            } else if let Some(r) = tok.strip_prefix('<') {
                (Op::Lt, r)
            } else if let Some(r) = tok.strip_prefix('^') {
                (Op::Caret, r)
            } else if let Some(r) = tok.strip_prefix('~') {
                (Op::Tilde, r)
            } else if let Some(r) = tok.strip_prefix('=') {
                (Op::Exact, r)
            } else {
                (Op::Exact, tok.as_str())
            };
            if rest == "*" {
                continue;
            }
            let mut nums = [None, None, None];
            let parts: Vec<&str> = rest.split('.').collect();
            if parts.len() > 3 || rest.contains('-') {
                return Err(bad());
            }
            for (i, p) in parts.iter().enumerate() {
                if matches!(*p, "x" | "X" | "*") {
                    break;
                }
                nums[i] = Some(p.parse::<u32>().map_err(|_| bad())?);
            }
            let Some(major) = nums[0] else { return Err(bad()) };
            comps.push(Comparator { op, major, minor: nums[1], patch: nums[2] });
        }
        Ok(VersionReq { comps, text: t.to_string() })
    }

    pub fn matches(&self, v: &Version) -> bool {
        self.comps.iter().all(|c| c.matches(v))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for VersionReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl Comparator {
    fn lower(&self) -> Version {
        Version::new(self.major, self.minor.unwrap_or(0), self.patch.unwrap_or(0))
    }
    /// First version above everything the partial version names ("1.2" -> 1.3.0, "1" -> 2.0.0).
    fn next_after_partial(&self) -> Version {
        match (self.minor, self.patch) {
            (None, _) => Version::new(self.major + 1, 0, 0),
            (Some(m), None) => Version::new(self.major, m + 1, 0),
            (Some(m), Some(p)) => Version::new(self.major, m, p + 1),
        }
    }
    fn matches(&self, v: &Version) -> bool {
        let lo = self.lower();
        match self.op {
            Op::Exact => *v >= lo && *v < self.next_after_partial(),
            Op::Ge => *v >= lo,
            Op::Gt => *v >= self.next_after_partial(),
            Op::Lt => *v < lo,
            Op::Le => *v < self.next_after_partial(),
            Op::Tilde => {
                let hi = match self.minor {
                    None => Version::new(self.major + 1, 0, 0),
                    Some(m) => Version::new(self.major, m + 1, 0),
                };
                *v >= lo && *v < hi
            }
            Op::Caret => {
                let hi = if self.major > 0 {
                    Version::new(self.major + 1, 0, 0)
                } else {
                    match (self.minor, self.patch) {
                        (None, _) => Version::new(1, 0, 0),
                        (Some(m), None) => Version::new(0, m + 1, 0),
                        (Some(m), Some(_)) if m > 0 => Version::new(0, m + 1, 0),
                        (Some(_), Some(p)) => Version::new(0, 0, p + 1),
                    }
                };
                *v >= lo && *v < hi
            }
        }
    }
}

/// Dependency specification
#[derive(Clone, Debug)]
pub struct Dependency {
    pub name: String,
    pub version_constraint: String, // "^1.0.0", "~1.2.0", "1.2.3", ">=1.0.0"
    pub optional: bool,
}

impl Dependency {
    pub fn new(name: String, version_constraint: String) -> Self {
        Dependency {
            name,
            version_constraint,
            optional: false,
        }
    }

    /// Mark as optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Check if version satisfies constraint (see [`VersionReq`] for the syntax; an
    /// unparsable constraint matches nothing).
    pub fn satisfies(&self, version: &Version) -> bool {
        VersionReq::parse(&self.version_constraint).map_or(false, |r| r.matches(version))
    }
}

/// Package metadata
#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub license: String,
    pub dependencies: Vec<Dependency>,
    pub keywords: Vec<String>,
    pub repository: String,
    pub downloads: u32,
}

impl Package {
    pub fn new(name: String, version: Version) -> Self {
        Package {
            name,
            version,
            description: String::new(),
            author: String::new(),
            license: String::new(),
            dependencies: Vec::new(),
            keywords: Vec::new(),
            repository: String::new(),
            downloads: 0,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set author
    pub fn with_author(mut self, author: String) -> Self {
        self.author = author;
        self
    }

    /// Add dependency
    pub fn add_dependency(mut self, dep: Dependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add keyword
    pub fn add_keyword(mut self, keyword: String) -> Self {
        self.keywords.push(keyword);
        self
    }

    /// Get package identifier
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version.to_string())
    }

    /// Increment download count
    pub fn increment_downloads(mut self) -> Self {
        self.downloads += 1;
        self
    }
}

/// Package registry
pub struct PackageRegistry {
    pub packages: HashMap<String, Vec<Package>>, // name -> versions
    pub index: BTreeMap<String, Vec<String>>,    // keyword -> package names
}

impl PackageRegistry {
    pub fn new() -> Self {
        PackageRegistry {
            packages: HashMap::new(),
            index: BTreeMap::new(),
        }
    }

    /// Publish package
    pub fn publish(&mut self, package: Package) -> Result<(), String> {
        let name = package.name.clone();
        
        // Check duplicate version
        if let Some(versions) = self.packages.get(&name) {
            if versions.iter().any(|p| p.version == package.version) {
                return Err(format!("Package {}@{} already exists", name, package.version.to_string()));
            }
        }

        // Index keywords
        for keyword in &package.keywords {
            self.index.entry(keyword.clone())
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        self.packages.entry(name.clone())
            .or_insert_with(Vec::new)
            .push(package);

        Ok(())
    }

    /// Unpublish package
    pub fn unpublish(&mut self, name: &str, version: &Version) -> Result<(), String> {
        if let Some(versions) = self.packages.get_mut(name) {
            versions.retain(|p| p.version != *version);
            if versions.is_empty() {
                self.packages.remove(name);
            }
            Ok(())
        } else {
            Err(format!("Package {} not found", name))
        }
    }

    /// Get latest version of package
    pub fn get_latest(&self, name: &str) -> Option<Package> {
        self.packages.get(name)
            .and_then(|versions| versions.iter().max_by(|a, b| a.version.cmp(&b.version)))
            .cloned()
    }

    /// Get specific version
    pub fn get_version(&self, name: &str, version: &Version) -> Option<Package> {
        self.packages.get(name)
            .and_then(|versions| versions.iter().find(|p| p.version == *version))
            .cloned()
    }

    /// Find compatible version
    pub fn find_compatible(&self, name: &str, constraint: &str) -> Option<Package> {
        self.packages.get(name)
            .and_then(|versions| {
                let dep = Dependency::new(name.to_string(), constraint.to_string());
                versions.iter().filter(|p| dep.satisfies(&p.version)).max_by(|a, b| a.version.cmp(&b.version))
            })
            .cloned()
    }

    /// Search packages by keyword
    pub fn search(&self, keyword: &str) -> Vec<Package> {
        if let Some(package_names) = self.index.get(keyword) {
            package_names.iter()
                .filter_map(|name| self.get_latest(name))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get package count
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Get total version count
    pub fn version_count(&self) -> usize {
        self.packages.values().map(|v| v.len()).sum()
    }

    /// List all packages
    pub fn list_all(&self) -> Vec<Package> {
        self.packages.values()
            .flat_map(|versions| versions.iter().cloned())
            .collect()
    }
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency resolver
pub struct DependencyResolver {
    pub registry: PackageRegistry,
    pub resolved: HashMap<String, Version>,
    pub conflicts: Vec<String>,
}

impl DependencyResolver {
    pub fn new(registry: PackageRegistry) -> Self {
        DependencyResolver {
            registry,
            resolved: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Resolve dependencies
    pub fn resolve(&mut self, package: &Package) -> Result<(), String> {
        self.resolved.clear();
        self.conflicts.clear();

        for dep in &package.dependencies {
            self.resolve_dependency(&dep)?;
        }

        Ok(())
    }

    /// Resolve single dependency. A version chosen earlier must also satisfy later requirements;
    /// otherwise the error names both the package and the requirement that failed.
    fn resolve_dependency(&mut self, dep: &Dependency) -> Result<(), String> {
        if let Some(existing_version) = self.resolved.get(&dep.name) {
            if !dep.satisfies(existing_version) {
                let msg = format!(
                    "Conflict: {} requires {}, but {} is already resolved",
                    dep.name, dep.version_constraint, existing_version.to_string()
                );
                self.conflicts.push(msg.clone());
                return Err(msg);
            }
            return Ok(());
        }

        let compatible = self.registry.find_compatible(&dep.name, &dep.version_constraint)
            .ok_or_else(|| format!("No version of {} matching {}", dep.name, dep.version_constraint))?;

        self.resolved.insert(dep.name.clone(), compatible.version.clone());

        for sub_dep in &compatible.dependencies {
            self.resolve_dependency(sub_dep)?;
        }

        Ok(())
    }

    /// Get resolution map
    pub fn get_resolution(&self) -> HashMap<String, String> {
        self.resolved.iter()
            .map(|(name, version)| (name.clone(), version.to_string()))
            .collect()
    }

    /// Has conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// Package installer
pub struct PackageInstaller {
    pub installed: HashMap<String, Version>,
    pub registry: PackageRegistry,
}

impl PackageInstaller {
    pub fn new(registry: PackageRegistry) -> Self {
        PackageInstaller {
            installed: HashMap::new(),
            registry,
        }
    }

    /// Install package
    pub fn install(&mut self, name: &str, version_constraint: &str) -> Result<Package, String> {
        let package = self.registry.find_compatible(name, version_constraint)
            .ok_or_else(|| format!("Package {} not found", name))?;

        self.installed.insert(name.to_string(), package.version.clone());
        Ok(package)
    }

    /// Uninstall package
    pub fn uninstall(&mut self, name: &str) -> Result<(), String> {
        if self.installed.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("Package {} not installed", name))
        }
    }

    /// Check if installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains_key(name)
    }

    /// Get installed version
    pub fn get_version(&self, name: &str) -> Option<Version> {
        self.installed.get(name).cloned()
    }

    /// List installed packages
    pub fn list_installed(&self) -> Vec<(String, Version)> {
        self.installed.iter()
            .map(|(name, version)| (name.clone(), version.clone()))
            .collect()
    }

    /// Count installed packages
    pub fn installed_count(&self) -> usize {
        self.installed.len()
    }
}

/// Package manager facade
pub struct PackageManager;

impl PackageManager {
    /// Create new registry
    pub fn new_registry() -> PackageRegistry {
        PackageRegistry::new()
    }

    /// Create new package
    pub fn new_package(name: String, version: String) -> Result<Package, String> {
        let version = Version::parse(&version)?;
        Ok(Package::new(name, version))
    }

    /// Create resolver
    pub fn new_resolver(registry: PackageRegistry) -> DependencyResolver {
        DependencyResolver::new(registry)
    }

    /// Create installer
    pub fn new_installer(registry: PackageRegistry) -> PackageInstaller {
        PackageInstaller::new(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_to_string() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 5, 0);
        assert!(v1.is_compatible(&v2));
    }

    #[test]
    fn test_version_bump_patch() {
        let v = Version::new(1, 2, 3).bump_patch();
        assert_eq!(v.patch, 4);
    }

    #[test]
    fn test_version_bump_minor() {
        let v = Version::new(1, 2, 3).bump_minor();
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_version_bump_major() {
        let v = Version::new(1, 2, 3).bump_major();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency::new("pkg".to_string(), "^1.0.0".to_string());
        assert_eq!(dep.name, "pkg");
    }

    #[test]
    fn test_dependency_satisfies_caret() {
        let dep = Dependency::new("pkg".to_string(), "^1.0.0".to_string());
        let v = Version::new(1, 5, 0);
        assert!(dep.satisfies(&v));
    }

    #[test]
    fn test_dependency_satisfies_tilde() {
        let dep = Dependency::new("pkg".to_string(), "~1.2.0".to_string());
        let v = Version::new(1, 2, 5);
        assert!(dep.satisfies(&v));
    }

    #[test]
    fn test_dependency_satisfies_exact() {
        let dep = Dependency::new("pkg".to_string(), "1.2.3".to_string());
        let v = Version::new(1, 2, 3);
        assert!(dep.satisfies(&v));
    }

    #[test]
    fn test_package_creation() {
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        assert_eq!(pkg.name, "test");
        assert_eq!(pkg.id(), "test@1.0.0");
    }

    #[test]
    fn test_package_with_metadata() {
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0))
            .with_author("Author".to_string())
            .add_keyword("keyword".to_string());
        assert_eq!(pkg.author, "Author");
        assert_eq!(pkg.keywords.len(), 1);
    }

    #[test]
    fn test_registry_publish() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        assert!(registry.publish(pkg).is_ok());
    }

    #[test]
    fn test_registry_get_latest() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        registry.publish(pkg).unwrap();
        
        let latest = registry.get_latest("test");
        assert!(latest.is_some());
    }

    #[test]
    fn test_registry_search() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0))
            .add_keyword("awesome".to_string());
        registry.publish(pkg).unwrap();
        
        let results = registry.search("awesome");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_dependency_resolver() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        registry.publish(pkg).unwrap();
        
        let mut resolver = DependencyResolver::new(registry);
        let root = Package::new("root".to_string(), Version::new(1, 0, 0))
            .add_dependency(Dependency::new("test".to_string(), "^1.0.0".to_string()));
        
        assert!(resolver.resolve(&root).is_ok());
        assert!(resolver.resolved.contains_key("test"));
    }

    #[test]
    fn test_package_installer() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        registry.publish(pkg).unwrap();
        
        let mut installer = PackageInstaller::new(registry);
        assert!(installer.install("test", "^1.0.0").is_ok());
        assert!(installer.is_installed("test"));
    }

    #[test]
    fn test_package_installer_uninstall() {
        let mut registry = PackageRegistry::new();
        let pkg = Package::new("test".to_string(), Version::new(1, 0, 0));
        registry.publish(pkg).unwrap();
        
        let mut installer = PackageInstaller::new(registry);
        installer.install("test", "^1.0.0").unwrap();
        assert!(installer.uninstall("test").is_ok());
        assert!(!installer.is_installed("test"));
    }

    #[test]
    fn test_registry_package_count() {
        let mut registry = PackageRegistry::new();
        let pkg1 = Package::new("pkg1".to_string(), Version::new(1, 0, 0));
        let pkg2 = Package::new("pkg2".to_string(), Version::new(1, 0, 0));
        registry.publish(pkg1).unwrap();
        registry.publish(pkg2).unwrap();
        
        assert_eq!(registry.package_count(), 2);
    }

    fn req_matches(req: &str, v: &str) -> bool {
        VersionReq::parse(req).unwrap().matches(&Version::parse(v).unwrap())
    }

    #[test]
    fn requirement_syntax() {
        assert!(req_matches("^1.2.3", "1.9.0") && !req_matches("^1.2.3", "2.0.0") && !req_matches("^1.2.3", "1.2.2"));
        assert!(req_matches("^0.2.3", "0.2.9") && !req_matches("^0.2.3", "0.3.0"));
        assert!(req_matches("^0.0.3", "0.0.3") && !req_matches("^0.0.3", "0.0.4"));
        assert!(req_matches("~1.2.3", "1.2.9") && !req_matches("~1.2.3", "1.3.0"));
        assert!(req_matches("~1", "1.9.9") && !req_matches("~1", "2.0.0"));
        assert!(req_matches(">=1.0", "1.0.0") && req_matches(">1", "2.0.0") && !req_matches(">1", "1.9.9"));
        assert!(req_matches("<2.0.0", "1.99.0") && !req_matches("<2.0.0", "2.0.0"));
        assert!(req_matches("<=1.5", "1.5.9") && !req_matches("<=1.5", "1.6.0"));
        assert!(req_matches("1.2.3", "1.2.3") && !req_matches("1.2.3", "1.2.4"));
        assert!(req_matches("=1.2.3", "1.2.3"));
        assert!(req_matches("1.2", "1.2.7") && !req_matches("1.2", "1.3.0"));
        assert!(req_matches("1.*", "1.7.0") && req_matches("*", "9.9.9"));
        assert!(req_matches(">=1.0, <2.0", "1.5.0") && !req_matches(">=1.0, <2.0", "2.0.0"));
        assert!(req_matches(">= 1.0 < 2.0", "1.5.0") && !req_matches(">= 1.0 < 2.0", "0.9.0"));
        assert!(VersionReq::parse("^x").is_err() && VersionReq::parse("1.2.3.4").is_err());
    }

    #[test]
    fn latest_is_the_highest_version_not_the_last_published() {
        let mut r = PackageRegistry::new();
        r.publish(Package::new("p".into(), Version::new(1, 2, 0))).unwrap();
        r.publish(Package::new("p".into(), Version::new(1, 10, 0))).unwrap();
        r.publish(Package::new("p".into(), Version::new(1, 3, 0))).unwrap();
        assert_eq!(r.get_latest("p").unwrap().version, Version::new(1, 10, 0));
        assert_eq!(r.find_compatible("p", "~1.2.0").unwrap().version, Version::new(1, 2, 0));
        assert_eq!(r.find_compatible("p", "^1").unwrap().version, Version::new(1, 10, 0));
    }
}
