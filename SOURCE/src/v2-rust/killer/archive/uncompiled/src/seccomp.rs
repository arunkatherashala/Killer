// Phase 19: Assassin Layer - Seccomp Syscall Filtering
// Restricts which system calls the Killer VM can make

use std::collections::{HashMap, HashSet};

/// A system call that can be allowed or denied
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallType {
    // Safe I/O operations
    Read,
    Write,
    OpenRead,      // open() for reading only
    Close,
    
    // Memory operations
    Mmap,          // Memory mapping (controlled)
    Mprotect,      // Protected memory regions
    
    // Process control
    Exit,
    ExitGroup,
    
    // No file system writes
    // No network operations
    // No privilege escalation
    // Placeholder for others
    Other(u32),
}

/// Seccomp profile - defines which syscalls are allowed
#[derive(Debug, Clone)]
pub struct SeccompProfile {
    pub name: String,
    pub description: String,
    pub allowed_syscalls: HashSet<SyscallType>,
    pub denied_syscalls: HashSet<SyscallType>,
    pub log_violations: bool,
    pub audit_level: AuditLevel,
}

/// How much to log about syscall usage
#[derive(Debug, Clone, Copy)]
pub enum AuditLevel {
    Silent,    // Don't log anything
    Warnings,  // Log only denials
    Verbose,   // Log all syscalls
}

/// Seccomp enforcement engine
pub struct SeccompEnforcer {
    /// Current profile
    active_profile: SeccompProfile,
    
    /// Statistics
    syscalls_allowed: HashMap<String, usize>,
    syscalls_denied: HashMap<String, usize>,
    violation_log: Vec<SyscallViolation>,
    
    /// Configuration
    enforce: bool,  // Actually enforce or just log?
}

/// A syscall violation event
#[derive(Debug, Clone)]
pub struct SyscallViolation {
    pub syscall: String,
    pub timestamp: u64,
    pub process_id: u32,
    pub reason: String,
}

impl SeccompProfile {
    /// Create a minimal "read-only" profile for safety
    pub fn read_only() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(SyscallType::Read);
        allowed.insert(SyscallType::OpenRead);
        allowed.insert(SyscallType::Close);
        allowed.insert(SyscallType::Exit);
        allowed.insert(SyscallType::ExitGroup);
        allowed.insert(SyscallType::Mmap);
        allowed.insert(SyscallType::Mprotect);
        
        SeccompProfile {
            name: "read_only".to_string(),
            description: "Allow only safe read operations and memory management".to_string(),
            allowed_syscalls: allowed,
            denied_syscalls: HashSet::new(),
            log_violations: true,
            audit_level: AuditLevel::Warnings,
        }
    }

    /// Create a "safe I/O" profile for typical computation
    pub fn safe_io() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(SyscallType::Read);
        allowed.insert(SyscallType::Write);
        allowed.insert(SyscallType::OpenRead);
        allowed.insert(SyscallType::Close);
        allowed.insert(SyscallType::Exit);
        allowed.insert(SyscallType::ExitGroup);
        allowed.insert(SyscallType::Mmap);
        allowed.insert(SyscallType::Mprotect);
        
        SeccompProfile {
            name: "safe_io".to_string(),
            description: "Allow read, write, and memory operations".to_string(),
            allowed_syscalls: allowed,
            denied_syscalls: HashSet::new(),
            log_violations: true,
            audit_level: AuditLevel::Warnings,
        }
    }

    /// Create a locked-down "compute-only" profile
    pub fn compute_only() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(SyscallType::Exit);
        allowed.insert(SyscallType::ExitGroup);
        allowed.insert(SyscallType::Mmap);
        allowed.insert(SyscallType::Mprotect);
        
        SeccompProfile {
            name: "compute_only".to_string(),
            description: "Allow only computation, no I/O".to_string(),
            allowed_syscalls: allowed,
            denied_syscalls: HashSet::new(),
            log_violations: true,
            audit_level: AuditLevel::Verbose,
        }
    }

    /// Check if a syscall is allowed
    pub fn is_allowed(&self, syscall: SyscallType) -> bool {
        self.allowed_syscalls.contains(&syscall)
    }
}

impl SeccompEnforcer {
    pub fn new(profile: SeccompProfile) -> Self {
        SeccompEnforcer {
            active_profile: profile,
            syscalls_allowed: HashMap::new(),
            syscalls_denied: HashMap::new(),
            violation_log: Vec::new(),
            enforce: true,
        }
    }

    /// Check if a syscall is permitted
    pub fn check_syscall(&mut self, syscall: SyscallType, syscall_name: &str) -> bool {
        if self.active_profile.is_allowed(syscall) {
            *self.syscalls_allowed.entry(syscall_name.to_string()).or_insert(0) += 1;
            
            if matches!(self.active_profile.audit_level, AuditLevel::Verbose) {
                println!("[SECCOMP] Allowed: {} ({})", syscall_name, self.syscalls_allowed.len());
            }
            
            true
        } else {
            *self.syscalls_denied.entry(syscall_name.to_string()).or_insert(0) += 1;
            
            if self.active_profile.log_violations {
                self.log_violation(syscall_name);
            }
            
            if self.enforce {
                println!("[SECCOMP] DENIED: {} - not permitted by profile '{}'", 
                    syscall_name, self.active_profile.name);
                false
            } else {
                // Just log, don't enforce
                true
            }
        }
    }

    fn log_violation(&mut self, syscall_name: &str) {
        let violation = SyscallViolation {
            syscall: syscall_name.to_string(),
            timestamp: 0,  // Would be real timestamp in production
            process_id: std::process::id(),
            reason: format!("Syscall not in allowed set for profile '{}'", 
                self.active_profile.name),
        };
        self.violation_log.push(violation);
    }

    /// Set enforcement mode
    pub fn set_enforcement(&mut self, enforce: bool) {
        self.enforce = enforce;
    }

    /// Get statistics
    pub fn get_stats(&self) -> SeccompStats {
        SeccompStats {
            profile_name: self.active_profile.name.clone(),
            allowed_syscalls: self.syscalls_allowed.len(),
            denied_syscalls: self.syscalls_denied.len(),
            total_allowed: self.syscalls_allowed.values().sum(),
            total_denied: self.syscalls_denied.values().sum(),
            violations: self.violation_log.len(),
        }
    }

    /// Print seccomp report
    pub fn print_report(&self) {
        let stats = self.get_stats();
        println!("\n=== Seccomp Filtering Report (Phase 19) ===");
        println!("Profile: {}", stats.profile_name);
        println!("Description: {}", self.active_profile.description);
        println!("");
        println!("Allowed Syscalls: {} types, {} total calls", 
            stats.allowed_syscalls, stats.total_allowed);
        println!("Denied Syscalls: {} types, {} total attempts", 
            stats.denied_syscalls, stats.total_denied);
        
        if stats.violations > 0 {
            println!("⚠️ Violations: {}", stats.violations);
            println!("Recent violations:");
            for violation in self.violation_log.iter().rev().take(3) {
                println!("  - {}: {}", violation.syscall, violation.reason);
            }
        } else {
            println!("✅ No violations");
        }
    }
}

/// Seccomp statistics
#[derive(Debug)]
pub struct SeccompStats {
    pub profile_name: String,
    pub allowed_syscalls: usize,
    pub denied_syscalls: usize,
    pub total_allowed: usize,
    pub total_denied: usize,
    pub violations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_profile() {
        let profile = SeccompProfile::read_only();
        assert!(profile.is_allowed(SyscallType::Read));
        assert!(profile.is_allowed(SyscallType::Exit));
        assert!(!profile.is_allowed(SyscallType::Write));
    }

    #[test]
    fn test_safe_io_profile() {
        let profile = SeccompProfile::safe_io();
        assert!(profile.is_allowed(SyscallType::Read));
        assert!(profile.is_allowed(SyscallType::Write));
        assert!(!profile.is_allowed(SyscallType::Other(1000)));
    }

    #[test]
    fn test_seccomp_enforcement() {
        let profile = SeccompProfile::read_only();
        let mut enforcer = SeccompEnforcer::new(profile);
        
        assert!(enforcer.check_syscall(SyscallType::Read, "read"));
        assert!(!enforcer.check_syscall(SyscallType::Write, "write"));
        
        let stats = enforcer.get_stats();
        assert_eq!(stats.total_allowed, 1);
        assert_eq!(stats.total_denied, 1);
    }

    #[test]
    fn test_compute_only_profile() {
        let profile = SeccompProfile::compute_only();
        assert!(profile.is_allowed(SyscallType::Exit));
        assert!(!profile.is_allowed(SyscallType::Read));
        assert!(!profile.is_allowed(SyscallType::Write));
    }
}
