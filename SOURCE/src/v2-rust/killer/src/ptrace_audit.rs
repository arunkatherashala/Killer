// Phase 19: Assassin Layer - Ptrace Syscall Auditing
// Monitors and audits system calls for security analysis

use std::collections::HashMap;

/// A recorded syscall event
#[derive(Debug, Clone)]
pub struct SyscallAuditEntry {
    pub timestamp: u64,
    pub syscall_name: String,
    pub arguments: Vec<String>,
    pub return_value: i64,
    pub process_id: u32,
    pub severity: SyscallSeverity,
    pub blocked: bool,
}

/// Severity level of a syscall
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallSeverity {
    Safe,        // Safe - usual operation
    Warning,     // Suspicious - needs review
    Dangerous,   // Dangerous - should be denied
    Critical,    // Critical security issue
}

/// Ptrace auditing and monitoring system
pub struct PtraceAuditor {
    /// Audit log of all syscalls
    audit_log: Vec<SyscallAuditEntry>,
    
    /// Count by syscall type
    syscall_counts: HashMap<String, usize>,
    
    /// Count by severity
    severity_counts: HashMap<String, usize>,
    
    /// Blocked calls
    blocked_calls: usize,
    
    /// Configuration
    audit_level: AuditLevel,
    record_arguments: bool,
}

/// Audit detail level
#[derive(Debug, Clone, Copy)]
pub enum AuditLevel {
    Minimal,   // Only critical calls
    Standard,  // All dangerous calls
    Verbose,   // All syscalls
    Debug,     // All syscalls with full details
}

impl SyscallSeverity {
    pub fn classify(syscall_name: &str) -> Self {
        match syscall_name {
            // Safe syscalls
            "read" | "write" | "close" | "exit" | "exit_group" |
            "mmap" | "mprotect" | "munmap" => SyscallSeverity::Safe,
            
            // Potentially suspicious
            "open" | "openat" | "stat" | "fstat" |
            "dup" | "dup2" | "dup3" => SyscallSeverity::Warning,
            
            // Dangerous syscalls
            "execve" | "fork" | "vfork" | "clone" |
            "ptrace" | "process_vm_readv" => SyscallSeverity::Dangerous,
            
            // Critical security syscalls
            "prctl" | "capset" | "capget" | "getuid" |
            "setuid" | "setgid" | "seteuid" => SyscallSeverity::Critical,
            
            // Unknown - treat as warning
            _ => SyscallSeverity::Warning,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SyscallSeverity::Safe => "SAFE",
            SyscallSeverity::Warning => "WARNING",
            SyscallSeverity::Dangerous => "DANGEROUS",
            SyscallSeverity::Critical => "CRITICAL",
        }
    }
}

impl PtraceAuditor {
    pub fn new(audit_level: AuditLevel) -> Self {
        PtraceAuditor {
            audit_log: Vec::new(),
            syscall_counts: HashMap::new(),
            severity_counts: HashMap::new(),
            blocked_calls: 0,
            audit_level,
            record_arguments: matches!(audit_level, AuditLevel::Debug),
        }
    }

    /// Record a syscall execution
    pub fn record_syscall(
        &mut self,
        syscall_name: &str,
        arguments: Vec<String>,
        return_value: i64,
        blocked: bool,
    ) {
        let severity = SyscallSeverity::classify(syscall_name);
        
        // Check if we should log this call
        let should_log = match self.audit_level {
            AuditLevel::Minimal => severity == SyscallSeverity::Critical,
            AuditLevel::Standard => severity == SyscallSeverity::Dangerous || severity == SyscallSeverity::Critical,
            AuditLevel::Verbose => true,
            AuditLevel::Debug => true,
        };
        
        if should_log {
            let entry = SyscallAuditEntry {
                timestamp: 0,  // Would be real timestamp
                syscall_name: syscall_name.to_string(),
                arguments: if self.record_arguments { arguments } else { Vec::new() },
                return_value,
                process_id: std::process::id(),
                severity,
                blocked,
            };
            
            self.audit_log.push(entry);
            
            // Only update statistics for logged syscalls
            *self.severity_counts.entry(severity.as_str().to_string()).or_insert(0) += 1;
            
            if blocked {
                self.blocked_calls += 1;
            }
        }
        
        // Always update syscall counts (for statistics on what was called, not what was logged)
        *self.syscall_counts.entry(syscall_name.to_string()).or_insert(0) += 1;
    }

    /// Get syscall statistics
    pub fn get_stats(&self) -> PtraceStats {
        PtraceStats {
            total_syscalls: self.syscall_counts.values().sum(),
            unique_syscalls: self.syscall_counts.len(),
            blocked_calls: self.blocked_calls,
            audit_log_size: self.audit_log.len(),
            safe_count: self.severity_counts.get("SAFE").copied().unwrap_or(0),
            warning_count: self.severity_counts.get("WARNING").copied().unwrap_or(0),
            dangerous_count: self.severity_counts.get("DANGEROUS").copied().unwrap_or(0),
            critical_count: self.severity_counts.get("CRITICAL").copied().unwrap_or(0),
        }
    }

    /// Get top syscalls
    pub fn get_top_syscalls(&self, limit: usize) -> Vec<(String, usize)> {
        let mut syscalls: Vec<_> = self.syscall_counts.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        syscalls.sort_by(|a, b| b.1.cmp(&a.1));
        syscalls.into_iter().take(limit).collect()
    }

    /// Get suspicious activity
    pub fn get_suspicious_activity(&self) -> Vec<&SyscallAuditEntry> {
        self.audit_log.iter()
            .filter(|e| e.severity == SyscallSeverity::Dangerous || e.severity == SyscallSeverity::Critical)
            .collect()
    }

    /// Export audit log for analysis
    pub fn export_audit_log(&self) -> String {
        let mut output = String::from("=== Ptrace Audit Log ===\n");
        
        for entry in &self.audit_log {
            output.push_str(&format!(
                "[{}] {} (severity: {}, return: {}, blocked: {})\n",
                entry.process_id,
                entry.syscall_name,
                entry.severity.as_str(),
                entry.return_value,
                entry.blocked
            ));
            
            if !entry.arguments.is_empty() {
                output.push_str(&format!("  Args: {}\n", entry.arguments.join(", ")));
            }
        }
        
        output
    }

    /// Print audit report
    pub fn print_report(&self) {
        let stats = self.get_stats();
        println!("\n=== Ptrace Syscall Auditing Report (Phase 19) ===");
        println!("Total Syscalls: {}", stats.total_syscalls);
        println!("Unique Syscalls: {}", stats.unique_syscalls);
        println!("Blocked Calls: {}", stats.blocked_calls);
        println!("Audit Log Entries: {}", stats.audit_log_size);
        println!("");
        println!("Severity Breakdown:");
        println!("  Safe: {}", stats.safe_count);
        println!("  Warning: {}", stats.warning_count);
        println!("  Dangerous: {}", stats.dangerous_count);
        println!("  Critical: {}", stats.critical_count);
        
        if stats.dangerous_count > 0 || stats.critical_count > 0 {
            println!("\n⚠️ Suspicious Activity Detected:");
            for entry in self.get_suspicious_activity().iter().take(5) {
                println!("  - {}: {}", entry.syscall_name, entry.severity.as_str());
            }
        } else {
            println!("\n✅ No suspicious activity detected");
        }
        
        if !self.get_top_syscalls(3).is_empty() {
            println!("\nMost Common Syscalls:");
            for (syscall, count) in self.get_top_syscalls(3) {
                println!("  - {}: {} calls", syscall, count);
            }
        }
    }
}

/// Ptrace audit statistics
#[derive(Debug)]
pub struct PtraceStats {
    pub total_syscalls: usize,
    pub unique_syscalls: usize,
    pub blocked_calls: usize,
    pub audit_log_size: usize,
    pub safe_count: usize,
    pub warning_count: usize,
    pub dangerous_count: usize,
    pub critical_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_classification() {
        assert_eq!(SyscallSeverity::classify("read"), SyscallSeverity::Safe);
        assert_eq!(SyscallSeverity::classify("open"), SyscallSeverity::Warning);
        assert_eq!(SyscallSeverity::classify("execve"), SyscallSeverity::Dangerous);
        assert_eq!(SyscallSeverity::classify("ptrace"), SyscallSeverity::Dangerous);
    }

    #[test]
    fn test_audit_recording() {
        let mut auditor = PtraceAuditor::new(AuditLevel::Verbose);
        
        auditor.record_syscall("read", vec!["fd: 3, size: 1024".to_string()], 1024, false);
        auditor.record_syscall("execve", vec!["/bin/sh".to_string()], -1, true);
        
        let stats = auditor.get_stats();
        assert_eq!(stats.safe_count, 1);
        assert_eq!(stats.dangerous_count, 1);
        assert_eq!(stats.blocked_calls, 1);
    }

    #[test]
    fn test_suspicious_activity_detection() {
        let mut auditor = PtraceAuditor::new(AuditLevel::Standard);
        
        auditor.record_syscall("ptrace", vec![], 0, false);
        auditor.record_syscall("setuid", vec![], 0, false);
        
        let suspicious = auditor.get_suspicious_activity();
        assert!(suspicious.len() > 0);
    }

    #[test]
    fn test_audit_level_filtering() {
        let mut auditor = PtraceAuditor::new(AuditLevel::Minimal);
        
        // These won't be logged at Minimal level
        auditor.record_syscall("read", vec![], 0, false);
        auditor.record_syscall("warning_call", vec![], 0, false);
        
        let stats = auditor.get_stats();
        // At Minimal level, only Critical is logged
        assert_eq!(stats.safe_count, 0);
    }
}
