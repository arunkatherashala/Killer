// Phase 19: Assassin Layer - Cgroups Resource Limiting
// Enforces resource quotas (CPU, memory, disk I/O)

use std::collections::HashMap;

/// Resource limits that can be enforced
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub memory_limit_mb: usize,      // Max memory in MB
    pub cpu_time_limit_ms: u64,      // Max CPU time in milliseconds
    pub disk_io_limit_mbps: usize,   // Max disk I/O in MB/s
    pub file_descriptor_limit: usize, // Max open files
    pub process_limit: usize,        // Max processes
}

/// Cgroup policy - defines resource constraints
#[derive(Debug, Clone)]
pub struct CgroupPolicy {
    pub name: String,
    pub limits: ResourceLimits,
    pub priority: CgroupPriority,
}

/// Priority level for resource scheduling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CgroupPriority {
    Minimal,    // Minimal resources (sandboxed untrusted code)
    Low,        // Low resources
    Normal,     // Normal resources
    High,       // High resources (trusted code)
    Unlimited,  // No limits
}

impl ResourceLimits {
    /// Create restrictive limits for untrusted code
    pub fn untrusted() -> Self {
        ResourceLimits {
            memory_limit_mb: 64,           // 64 MB max
            cpu_time_limit_ms: 5000,      // 5 seconds max
            disk_io_limit_mbps: 10,       // 10 MB/s max
            file_descriptor_limit: 10,    // Max 10 open files
            process_limit: 1,             // Single process only
        }
    }

    /// Create moderate limits for standard computation
    pub fn standard() -> Self {
        ResourceLimits {
            memory_limit_mb: 512,          // 512 MB max
            cpu_time_limit_ms: 60000,     // 60 seconds max
            disk_io_limit_mbps: 100,      // 100 MB/s max
            file_descriptor_limit: 100,   // Max 100 open files
            process_limit: 4,             // Up to 4 processes
        }
    }

    /// Create permissive limits for trusted code
    pub fn trusted() -> Self {
        ResourceLimits {
            memory_limit_mb: 4096,         // 4 GB max
            cpu_time_limit_ms: 600000,    // 10 minutes max
            disk_io_limit_mbps: 1000,     // 1000 MB/s max
            file_descriptor_limit: 1000,  // Max 1000 open files
            process_limit: 32,            // Up to 32 processes
        }
    }
}

impl CgroupPolicy {
    pub fn new(name: String, limits: ResourceLimits, priority: CgroupPriority) -> Self {
        CgroupPolicy {
            name,
            limits,
            priority,
        }
    }

    pub fn untrusted() -> Self {
        CgroupPolicy {
            name: "untrusted".to_string(),
            limits: ResourceLimits::untrusted(),
            priority: CgroupPriority::Minimal,
        }
    }

    pub fn standard() -> Self {
        CgroupPolicy {
            name: "standard".to_string(),
            limits: ResourceLimits::standard(),
            priority: CgroupPriority::Normal,
        }
    }

    pub fn trusted() -> Self {
        CgroupPolicy {
            name: "trusted".to_string(),
            limits: ResourceLimits::trusted(),
            priority: CgroupPriority::High,
        }
    }
}

/// Cgroups resource enforcement engine
pub struct CgroupManager {
    /// Active policies
    policies: HashMap<String, CgroupPolicy>,
    
    /// Resource usage tracking
    memory_used: HashMap<String, usize>,
    cpu_time_used: HashMap<String, u64>,
    disk_io_used: HashMap<String, usize>,
    
    /// Statistics
    policy_switches: usize,
    limit_violations: Vec<ResourceViolation>,
}

/// A resource limit violation
#[derive(Debug, Clone)]
pub struct ResourceViolation {
    pub policy_name: String,
    pub resource_type: String,
    pub limit: usize,
    pub current: usize,
    pub timestamp: u64,
}

impl CgroupManager {
    pub fn new() -> Self {
        let mut manager = CgroupManager {
            policies: HashMap::new(),
            memory_used: HashMap::new(),
            cpu_time_used: HashMap::new(),
            disk_io_used: HashMap::new(),
            policy_switches: 0,
            limit_violations: Vec::new(),
        };
        
        // Register standard policies
        manager.policies.insert("untrusted".to_string(), CgroupPolicy::untrusted());
        manager.policies.insert("standard".to_string(), CgroupPolicy::standard());
        manager.policies.insert("trusted".to_string(), CgroupPolicy::trusted());
        
        manager
    }

    /// Register a custom policy
    pub fn register_policy(&mut self, policy: CgroupPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    /// Check if memory usage is within limits
    pub fn check_memory(&mut self, policy_name: &str, current_mb: usize) -> bool {
        if let Some(policy) = self.policies.get(policy_name) {
            if current_mb > policy.limits.memory_limit_mb {
                self.limit_violations.push(ResourceViolation {
                    policy_name: policy_name.to_string(),
                    resource_type: "memory".to_string(),
                    limit: policy.limits.memory_limit_mb,
                    current: current_mb,
                    timestamp: 0,
                });
                return false;
            }
            self.memory_used.insert(policy_name.to_string(), current_mb);
            true
        } else {
            false
        }
    }

    /// Check if CPU time is within limits
    pub fn check_cpu_time(&mut self, policy_name: &str, elapsed_ms: u64) -> bool {
        if let Some(policy) = self.policies.get(policy_name) {
            if elapsed_ms > policy.limits.cpu_time_limit_ms {
                self.limit_violations.push(ResourceViolation {
                    policy_name: policy_name.to_string(),
                    resource_type: "cpu_time".to_string(),
                    limit: policy.limits.cpu_time_limit_ms as usize,
                    current: elapsed_ms as usize,
                    timestamp: 0,
                });
                return false;
            }
            self.cpu_time_used.insert(policy_name.to_string(), elapsed_ms);
            true
        } else {
            false
        }
    }

    /// Check if disk I/O is within limits
    pub fn check_disk_io(&mut self, policy_name: &str, current_mbps: usize) -> bool {
        if let Some(policy) = self.policies.get(policy_name) {
            if current_mbps > policy.limits.disk_io_limit_mbps {
                self.limit_violations.push(ResourceViolation {
                    policy_name: policy_name.to_string(),
                    resource_type: "disk_io".to_string(),
                    limit: policy.limits.disk_io_limit_mbps,
                    current: current_mbps,
                    timestamp: 0,
                });
                return false;
            }
            self.disk_io_used.insert(policy_name.to_string(), current_mbps);
            true
        } else {
            false
        }
    }

    /// Get resource usage for a policy
    pub fn get_usage(&self, policy_name: &str) -> Option<ResourceUsage> {
        Some(ResourceUsage {
            policy_name: policy_name.to_string(),
            memory_mb: self.memory_used.get(policy_name).copied(),
            cpu_time_ms: self.cpu_time_used.get(policy_name).copied(),
            disk_io_mbps: self.disk_io_used.get(policy_name).copied(),
        })
    }

    /// Get statistics
    pub fn get_stats(&self) -> CgroupStats {
        CgroupStats {
            policies: self.policies.len(),
            policy_switches: self.policy_switches,
            violations: self.limit_violations.len(),
            memory_violations: self.limit_violations.iter()
                .filter(|v| v.resource_type == "memory").count(),
            cpu_violations: self.limit_violations.iter()
                .filter(|v| v.resource_type == "cpu_time").count(),
            io_violations: self.limit_violations.iter()
                .filter(|v| v.resource_type == "disk_io").count(),
        }
    }

    /// Print cgroups report
    pub fn print_report(&self) {
        let stats = self.get_stats();
        println!("\n=== Cgroups Resource Limiting Report (Phase 19) ===");
        println!("Policies Registered: {}", stats.policies);
        println!("Policy Switches: {}", stats.policy_switches);
        println!("");
        println!("Resource Limit Violations:");
        println!("  Memory: {} violations", stats.memory_violations);
        println!("  CPU Time: {} violations", stats.cpu_violations);
        println!("  Disk I/O: {} violations", stats.io_violations);
        println!("  Total: {} violations", stats.violations);
        
        if stats.violations == 0 {
            println!("✅ All resources within limits");
        } else {
            println!("⚠️ Resource constraints enforced");
        }
    }
}

/// Resource usage statistics
#[derive(Debug)]
pub struct ResourceUsage {
    pub policy_name: String,
    pub memory_mb: Option<usize>,
    pub cpu_time_ms: Option<u64>,
    pub disk_io_mbps: Option<usize>,
}

/// Cgroups statistics
#[derive(Debug)]
pub struct CgroupStats {
    pub policies: usize,
    pub policy_switches: usize,
    pub violations: usize,
    pub memory_violations: usize,
    pub cpu_violations: usize,
    pub io_violations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_untrusted_limits() {
        let limits = ResourceLimits::untrusted();
        assert_eq!(limits.memory_limit_mb, 64);
        assert!(limits.cpu_time_limit_ms <= 5000);
    }

    #[test]
    fn test_standard_limits() {
        let limits = ResourceLimits::standard();
        assert_eq!(limits.memory_limit_mb, 512);
        assert!(limits.cpu_time_limit_ms <= 60000);
    }

    #[test]
    fn test_memory_limit_check() {
        let mut manager = CgroupManager::new();
        
        // Within limit: 50 MB < 64 MB
        assert!(manager.check_memory("untrusted", 50));
        
        // Over limit: 100 MB > 64 MB
        assert!(!manager.check_memory("untrusted", 100));
        
        let stats = manager.get_stats();
        assert_eq!(stats.memory_violations, 1);
    }

    #[test]
    fn test_cpu_time_limit_check() {
        let mut manager = CgroupManager::new();
        
        // Within limit: 1000 ms < 5000 ms
        assert!(manager.check_cpu_time("untrusted", 1000));
        
        // Over limit: 10000 ms > 5000 ms
        assert!(!manager.check_cpu_time("untrusted", 10000));
        
        let stats = manager.get_stats();
        assert_eq!(stats.cpu_violations, 1);
    }
}
