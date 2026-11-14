// Phase 20: Isolation Architecture - Filesystem Sandbox
// Manages filesystem isolation and mount points

use std::collections::HashMap;
use std::path::PathBuf;

/// Filesystem permission level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPermission {
    None,       // No access
    Read,       // Read-only
    Write,      // Read-write
    ReadWrite,  // Full access with execute
    Execute,    // Execute-only
}

impl PathPermission {
    pub fn can_read(&self) -> bool {
        matches!(self, PathPermission::Read | PathPermission::Write | PathPermission::ReadWrite)
    }

    pub fn can_write(&self) -> bool {
        matches!(self, PathPermission::Write | PathPermission::ReadWrite)
    }

    pub fn can_execute(&self) -> bool {
        matches!(self, PathPermission::Execute | PathPermission::ReadWrite)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PathPermission::None => "none",
            PathPermission::Read => "r--",
            PathPermission::Write => "-w-",
            PathPermission::ReadWrite => "rw-",
            PathPermission::Execute => "--x",
        }
    }
}

/// Mount point configuration
#[derive(Debug, Clone)]
pub struct MountPoint {
    pub source: PathBuf,
    pub target: PathBuf,
    pub permission: PathPermission,
    pub read_only: bool,
    pub mount_type: MountType,
}

/// Type of mount
#[derive(Debug, Clone, Copy)]
pub enum MountType {
    Bind,       // Bind mount (same filesystem)
    Virtual,    // Virtual mount (tmpfs)
    Volume,     // Volume mount (external storage)
    Device,     // Device mount (special files)
}

impl MountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MountType::Bind => "bind",
            MountType::Virtual => "virtual",
            MountType::Volume => "volume",
            MountType::Device => "device",
        }
    }
}

/// Filesystem sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub name: String,
    pub root: PathBuf,
    pub mount_points: HashMap<String, MountPoint>,
    pub allowed_paths: HashMap<String, PathPermission>,
}

impl SandboxConfig {
    pub fn new(name: &str, root: PathBuf) -> Self {
        SandboxConfig {
            name: name.to_string(),
            root,
            mount_points: HashMap::new(),
            allowed_paths: HashMap::new(),
        }
    }

    /// Create restrictive sandbox (minimal paths)
    pub fn restrictive() -> Self {
        let mut config = SandboxConfig::new("restrictive", PathBuf::from("/sandbox"));
        config.allowed_paths.insert("/".to_string(), PathPermission::Read);
        config.allowed_paths.insert("/tmp".to_string(), PathPermission::ReadWrite);
        config
    }

    /// Create standard sandbox (common paths)
    pub fn standard() -> Self {
        let mut config = SandboxConfig::new("standard", PathBuf::from("/sandbox"));
        config.allowed_paths.insert("/".to_string(), PathPermission::Read);
        config.allowed_paths.insert("/tmp".to_string(), PathPermission::ReadWrite);
        config.allowed_paths.insert("/home".to_string(), PathPermission::ReadWrite);
        config.allowed_paths.insert("/var/tmp".to_string(), PathPermission::ReadWrite);
        config
    }

    /// Create permissive sandbox (broader access)
    pub fn permissive() -> Self {
        let mut config = SandboxConfig::new("permissive", PathBuf::from("/"));
        config.allowed_paths.insert("/".to_string(), PathPermission::ReadWrite);
        config
    }

    pub fn add_mount(&mut self, mount: MountPoint) {
        let target_str = mount.target.to_string_lossy().to_string();
        self.mount_points.insert(target_str, mount);
    }

    pub fn add_path(&mut self, path: &str, permission: PathPermission) {
        self.allowed_paths.insert(path.to_string(), permission);
    }
}

/// Filesystem sandbox enforcer
pub struct FilesystemSandbox {
    sandboxes: HashMap<String, SandboxConfig>,
    violations: Vec<SandboxViolation>,
}

/// Filesystem access violation
#[derive(Debug, Clone)]
pub struct SandboxViolation {
    pub container: String,
    pub path: PathBuf,
    pub attempted_access: PathPermission,
    pub allowed_access: Option<PathPermission>,
    pub timestamp: u64,
}

impl FilesystemSandbox {
    pub fn new() -> Self {
        FilesystemSandbox {
            sandboxes: HashMap::new(),
            violations: Vec::new(),
        }
    }

    /// Create a new sandbox
    pub fn create_sandbox(&mut self, config: SandboxConfig) -> Result<String, String> {
        let name = config.name.clone();
        if self.sandboxes.contains_key(&name) {
            return Err(format!("Sandbox '{}' already exists", name));
        }
        self.sandboxes.insert(name.clone(), config);
        Ok(name)
    }

    /// Check access permission for a path
    pub fn check_access(&mut self, sandbox_name: &str, path: &str, access: PathPermission) -> Result<bool, String> {
        match self.sandboxes.get(sandbox_name) {
            Some(config) => {
                // Check direct path permission
                if let Some(perm) = config.allowed_paths.get(path) {
                    let allowed = match access {
                        PathPermission::Read => perm.can_read(),
                        PathPermission::Write => perm.can_write(),
                        PathPermission::ReadWrite => perm.can_read() && perm.can_write(),
                        PathPermission::Execute => perm.can_execute(),
                        PathPermission::None => true,
                    };

                    if !allowed {
                        self.record_violation(sandbox_name, path, access, Some(*perm));
                        return Ok(false);
                    }
                    return Ok(true);
                }

                // Check parent directories
                let path_buf = PathBuf::from(path);
                let mut current = path_buf.as_path();

                while let Some(parent) = current.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if let Some(perm) = config.allowed_paths.get(&parent_str) {
                        let allowed = match access {
                            PathPermission::Read => perm.can_read(),
                            PathPermission::Write => perm.can_write(),
                            PathPermission::ReadWrite => perm.can_read() && perm.can_write(),
                            PathPermission::Execute => perm.can_execute(),
                            PathPermission::None => true,
                        };

                        if !allowed {
                            self.record_violation(sandbox_name, path, access, Some(*perm));
                            return Ok(false);
                        }
                        return Ok(true);
                    }
                    current = parent;
                }

                // Default deny
                self.record_violation(sandbox_name, path, access, None);
                Ok(false)
            },
            None => Err(format!("Sandbox '{}' not found", sandbox_name)),
        }
    }

    /// Record a violation
    fn record_violation(&mut self, sandbox: &str, path: &str, access: PathPermission, allowed: Option<PathPermission>) {
        self.violations.push(SandboxViolation {
            container: sandbox.to_string(),
            path: PathBuf::from(path),
            attempted_access: access,
            allowed_access: allowed,
            timestamp: 0,
        });
    }

    /// Get sandbox configuration
    pub fn get_sandbox(&self, name: &str) -> Option<&SandboxConfig> {
        self.sandboxes.get(name)
    }

    /// Get violations for a sandbox
    pub fn get_violations(&self, sandbox_name: &str) -> Vec<&SandboxViolation> {
        self.violations.iter()
            .filter(|v| v.container == sandbox_name)
            .collect()
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> FilesystemStats {
        FilesystemStats {
            total_sandboxes: self.sandboxes.len(),
            total_mounts: self.sandboxes.values().map(|s| s.mount_points.len()).sum(),
            total_allowed_paths: self.sandboxes.values().map(|s| s.allowed_paths.len()).sum(),
            total_violations: self.violations.len(),
        }
    }

    /// Print filesystem report
    pub fn print_report(&self) {
        println!("\n=== Filesystem Sandbox Report (Phase 20) ===");

        let stats = self.get_stats();
        println!("Statistics:");
        println!("  Total Sandboxes: {}", stats.total_sandboxes);
        println!("  Total Mount Points: {}", stats.total_mounts);
        println!("  Total Allowed Paths: {}", stats.total_allowed_paths);
        println!("  Total Violations: {}", stats.total_violations);

        if !self.sandboxes.is_empty() {
            println!("\nSandbox Configurations:");
            for (name, config) in &self.sandboxes {
                println!("  {}:", name);
                println!("    Root: {}", config.root.display());
                println!("    Allowed Paths: {}", config.allowed_paths.len());
                println!("    Mount Points: {}", config.mount_points.len());
            }
        }

        if !self.violations.is_empty() {
            println!("\nViolations:");
            for violation in self.violations.iter().take(10) {
                println!("  {}: {} (attempted {})", 
                    violation.container, 
                    violation.path.display(),
                    violation.attempted_access.as_str());
            }
            if self.violations.len() > 10 {
                println!("  ... and {} more violations", self.violations.len() - 10);
            }
        }
    }
}

/// Filesystem statistics
#[derive(Debug)]
pub struct FilesystemStats {
    pub total_sandboxes: usize,
    pub total_mounts: usize,
    pub total_allowed_paths: usize,
    pub total_violations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_permission() {
        assert!(PathPermission::Read.can_read());
        assert!(!PathPermission::Read.can_write());
        
        assert!(PathPermission::ReadWrite.can_read());
        assert!(PathPermission::ReadWrite.can_write());
    }

    #[test]
    fn test_sandbox_creation() {
        let mut sandbox = FilesystemSandbox::new();
        let config = SandboxConfig::restrictive();
        
        assert!(sandbox.create_sandbox(config).is_ok());
        assert_eq!(sandbox.get_stats().total_sandboxes, 1);
    }

    #[test]
    fn test_access_control() {
        let mut sandbox = FilesystemSandbox::new();
        let mut config = SandboxConfig::restrictive();
        config.add_path("/tmp", PathPermission::ReadWrite);
        
        sandbox.create_sandbox(config).unwrap();
        
        assert!(sandbox.check_access("restrictive", "/", PathPermission::Read).unwrap());
        assert!(sandbox.check_access("restrictive", "/tmp", PathPermission::Write).unwrap());
    }

    #[test]
    fn test_violation_tracking() {
        let mut sandbox = FilesystemSandbox::new();
        let config = SandboxConfig::restrictive();
        
        sandbox.create_sandbox(config).unwrap();
        
        let _ = sandbox.check_access("restrictive", "/etc/passwd", PathPermission::Write);
        let violations = sandbox.get_violations("restrictive");
        assert_eq!(violations.len(), 1);
    }
}
