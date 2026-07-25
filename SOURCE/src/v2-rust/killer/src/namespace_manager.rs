// Phase 20: Isolation Architecture - Linux Namespace Manager
// Manages process isolation through namespaces

use std::collections::HashMap;

/// Linux namespace types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamespaceType {
    Pid,        // PID namespace - process isolation
    Network,    // Network namespace - network stack isolation
    Mount,      // Mount namespace - filesystem view isolation
    Ipc,        // IPC namespace - inter-process communication isolation
    Uts,        // UTS namespace - hostname isolation
    User,       // User namespace - UID/GID mapping
    Cgroup,     // Cgroup namespace - resource namespace isolation
}

/// Namespace configuration for a container
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    pub pid_ns: bool,
    pub network_ns: bool,
    pub mount_ns: bool,
    pub ipc_ns: bool,
    pub uts_ns: bool,
    pub user_ns: bool,
    pub cgroup_ns: bool,
}

impl NamespaceConfig {
    /// Create isolated namespace configuration (all enabled)
    pub fn isolated() -> Self {
        NamespaceConfig {
            pid_ns: true,
            network_ns: true,
            mount_ns: true,
            ipc_ns: true,
            uts_ns: true,
            user_ns: true,
            cgroup_ns: true,
        }
    }

    /// Create partially isolated configuration (compute only)
    pub fn compute_only() -> Self {
        NamespaceConfig {
            pid_ns: true,
            network_ns: false,  // No network
            mount_ns: true,
            ipc_ns: true,
            uts_ns: false,      // Share hostname
            user_ns: true,
            cgroup_ns: true,
        }
    }

    /// Create minimal isolation (shared host)
    pub fn shared_host() -> Self {
        NamespaceConfig {
            pid_ns: false,
            network_ns: false,
            mount_ns: false,
            ipc_ns: false,
            uts_ns: false,
            user_ns: false,
            cgroup_ns: false,
        }
    }

    pub fn enabled_count(&self) -> usize {
        let mut count = 0;
        if self.pid_ns { count += 1; }
        if self.network_ns { count += 1; }
        if self.mount_ns { count += 1; }
        if self.ipc_ns { count += 1; }
        if self.uts_ns { count += 1; }
        if self.user_ns { count += 1; }
        if self.cgroup_ns { count += 1; }
        count
    }
}

/// User ID/GID mapping for namespace
#[derive(Debug, Clone)]
pub struct UserMapping {
    pub uid_map: Vec<(u32, u32, u32)>,  // inner_id, outer_id, range
    pub gid_map: Vec<(u32, u32, u32)>,
}

impl UserMapping {
    pub fn new() -> Self {
        UserMapping {
            uid_map: Vec::new(),
            gid_map: Vec::new(),
        }
    }

    /// Create a simple 1:1 mapping
    pub fn identity(uid: u32, gid: u32) -> Self {
        let mut mapping = UserMapping::new();
        mapping.uid_map.push((0, uid, 1));
        mapping.gid_map.push((0, gid, 1));
        mapping
    }
}

/// Namespace manager for container isolation
pub struct NamespaceManager {
    namespaces: HashMap<String, NamespaceConfig>,
    user_mappings: HashMap<String, UserMapping>,
    isolation_level: IsolationLevel,
}

/// Level of container isolation
#[derive(Debug, Clone, Copy)]
pub enum IsolationLevel {
    Host,       // No isolation
    Process,    // PID isolation only
    Network,    // PID + Network isolation
    Container,  // Full isolation
}

impl NamespaceManager {
    pub fn new(level: IsolationLevel) -> Self {
        NamespaceManager {
            namespaces: HashMap::new(),
            user_mappings: HashMap::new(),
            isolation_level: level,
        }
    }

    /// Create a named namespace container
    pub fn create_container(&mut self, name: &str, isolation: IsolationLevel) {
        let config = match isolation {
            IsolationLevel::Host => NamespaceConfig::shared_host(),
            IsolationLevel::Process => {
                let mut cfg = NamespaceConfig::shared_host();
                cfg.pid_ns = true;
                cfg
            },
            IsolationLevel::Network => NamespaceConfig::compute_only(),
            IsolationLevel::Container => NamespaceConfig::isolated(),
        };

        self.namespaces.insert(name.to_string(), config);
    }

    /// Create user namespace mapping
    pub fn map_user(&mut self, container: &str, uid: u32, gid: u32) {
        self.user_mappings.insert(
            container.to_string(),
            UserMapping::identity(uid, gid),
        );
    }

    /// Check if namespace type is enabled for container
    pub fn is_enabled(&self, container: &str, ns_type: NamespaceType) -> bool {
        match self.namespaces.get(container) {
            Some(config) => match ns_type {
                NamespaceType::Pid => config.pid_ns,
                NamespaceType::Network => config.network_ns,
                NamespaceType::Mount => config.mount_ns,
                NamespaceType::Ipc => config.ipc_ns,
                NamespaceType::Uts => config.uts_ns,
                NamespaceType::User => config.user_ns,
                NamespaceType::Cgroup => config.cgroup_ns,
            },
            None => false,
        }
    }

    /// Get isolation statistics
    pub fn get_stats(&self) -> NamespaceStats {
        let total_containers = self.namespaces.len();
        let mut fully_isolated = 0;
        let mut partially_isolated = 0;
        let mut shared = 0;

        for config in self.namespaces.values() {
            let count = config.enabled_count();
            if count == 7 {
                fully_isolated += 1;
            } else if count > 0 {
                partially_isolated += 1;
            } else {
                shared += 1;
            }
        }

        NamespaceStats {
            total_containers,
            fully_isolated,
            partially_isolated,
            shared,
        }
    }

    /// Validate namespace configuration
    pub fn validate_isolation(&self, container: &str) -> Vec<String> {
        let mut warnings = Vec::new();

        match self.namespaces.get(container) {
            Some(config) => {
                if !config.pid_ns {
                    warnings.push("PID namespace not isolated".to_string());
                }
                if !config.network_ns {
                    warnings.push("Network namespace not isolated".to_string());
                }
                if !config.user_ns {
                    warnings.push("User namespace not isolated".to_string());
                }
                if config.enabled_count() < 4 {
                    warnings.push("Less than 4 namespaces isolated - weak isolation".to_string());
                }
            },
            None => warnings.push(format!("Container '{}' not found", container)),
        }

        warnings
    }

    /// Print isolation report
    pub fn print_report(&self) {
        println!("\n=== Namespace Isolation Report (Phase 20) ===");
        println!("Isolation Level: {:?}", self.isolation_level);
        
        let stats = self.get_stats();
        println!("\nContainer Statistics:");
        println!("  Total Containers: {}", stats.total_containers);
        println!("  Fully Isolated: {}", stats.fully_isolated);
        println!("  Partially Isolated: {}", stats.partially_isolated);
        println!("  Shared Host: {}", stats.shared);

        if !self.namespaces.is_empty() {
            println!("\nContainer Details:");
            for (name, config) in &self.namespaces {
                println!("  {}:", name);
                println!("    PID Namespace: {}", if config.pid_ns { "✓" } else { "✗" });
                println!("    Network Namespace: {}", if config.network_ns { "✓" } else { "✗" });
                println!("    Mount Namespace: {}", if config.mount_ns { "✓" } else { "✗" });
                println!("    IPC Namespace: {}", if config.ipc_ns { "✓" } else { "✗" });
                println!("    UTS Namespace: {}", if config.uts_ns { "✓" } else { "✗" });
                println!("    User Namespace: {}", if config.user_ns { "✓" } else { "✗" });
                println!("    Cgroup Namespace: {}", if config.cgroup_ns { "✓" } else { "✗" });
            }
        }
    }
}

/// Namespace statistics
#[derive(Debug)]
pub struct NamespaceStats {
    pub total_containers: usize,
    pub fully_isolated: usize,
    pub partially_isolated: usize,
    pub shared: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_config_creation() {
        let isolated = NamespaceConfig::isolated();
        assert_eq!(isolated.enabled_count(), 7);

        let compute = NamespaceConfig::compute_only();
        assert_eq!(compute.enabled_count(), 5);

        let shared = NamespaceConfig::shared_host();
        assert_eq!(shared.enabled_count(), 0);
    }

    #[test]
    fn test_namespace_manager() {
        let mut manager = NamespaceManager::new(IsolationLevel::Container);
        
        manager.create_container("app1", IsolationLevel::Container);
        manager.create_container("app2", IsolationLevel::Process);
        
        assert!(manager.is_enabled("app1", NamespaceType::Pid));
        assert!(manager.is_enabled("app1", NamespaceType::Network));
        assert!(manager.is_enabled("app2", NamespaceType::Pid));
        assert!(!manager.is_enabled("app2", NamespaceType::Network));
    }

    #[test]
    fn test_user_mapping() {
        let mapping = UserMapping::identity(1000, 1000);
        assert_eq!(mapping.uid_map.len(), 1);
        assert_eq!(mapping.gid_map.len(), 1);
    }

    #[test]
    fn test_isolation_validation() {
        let mut manager = NamespaceManager::new(IsolationLevel::Process);
        manager.create_container("weak", IsolationLevel::Process);
        
        let warnings = manager.validate_isolation("weak");
        assert!(!warnings.is_empty());
    }
}
