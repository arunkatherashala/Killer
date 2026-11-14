// Phase 21: Audit & Monitoring - Threat Intelligence Engine
// Detects and analyzes security threats

use std::collections::{HashMap, HashSet};

/// Threat severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatSeverity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl ThreatSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatSeverity::Low => "Low",
            ThreatSeverity::Medium => "Medium",
            ThreatSeverity::High => "High",
            ThreatSeverity::Critical => "Critical",
        }
    }
}

/// Threat type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThreatType {
    PrivilegeEscalation,    // Attempt to gain elevated privileges
    ResourceExhaustion,     // Resource consumption attack
    DenialOfService,        // DoS attempt
    UnauthorizedAccess,     // Unauthorized file/resource access
    MaliciousCode,          // Suspicious code execution
    SyscallViolation,       // Blocked syscall attempt
    DataExfiltration,       // Attempt to steal data
    ContainerBreakout,      // Container escape attempt
}

impl ThreatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatType::PrivilegeEscalation => "Privilege Escalation",
            ThreatType::ResourceExhaustion => "Resource Exhaustion",
            ThreatType::DenialOfService => "Denial of Service",
            ThreatType::UnauthorizedAccess => "Unauthorized Access",
            ThreatType::MaliciousCode => "Malicious Code",
            ThreatType::SyscallViolation => "Syscall Violation",
            ThreatType::DataExfiltration => "Data Exfiltration",
            ThreatType::ContainerBreakout => "Container Breakout",
        }
    }
}

/// Threat detection rule
#[derive(Debug, Clone)]
pub struct ThreatRule {
    pub name: String,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub pattern: String,
    pub threshold: usize,
    pub time_window: u64,  // seconds
    pub enabled: bool,
}

/// Detected threat
#[derive(Debug, Clone)]
pub struct DetectedThreat {
    pub rule_name: String,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub source: String,
    pub target: Option<String>,
    pub evidence: String,
    pub timestamp: u64,
    pub occurrence_count: usize,
}

/// Threat intelligence engine
pub struct ThreatIntelligence {
    rules: HashMap<String, ThreatRule>,
    threats: Vec<DetectedThreat>,
    threat_history: HashMap<String, Vec<u64>>,
    blocked_entities: HashSet<String>,
}

impl ThreatIntelligence {
    pub fn new() -> Self {
        let mut engine = ThreatIntelligence {
            rules: HashMap::new(),
            threats: Vec::new(),
            threat_history: HashMap::new(),
            blocked_entities: HashSet::new(),
        };

        // Register default rules
        engine.register_default_rules();

        engine
    }

    /// Register default threat detection rules
    fn register_default_rules(&mut self) {
        // Privilege escalation rule
        self.rules.insert("privesc".to_string(), ThreatRule {
            name: "Privilege Escalation Attempt".to_string(),
            threat_type: ThreatType::PrivilegeEscalation,
            severity: ThreatSeverity::Critical,
            pattern: "setuid|capset|prctl".to_string(),
            threshold: 1,
            time_window: 60,
            enabled: true,
        });

        // Resource exhaustion rule
        self.rules.insert("resource_exhaust".to_string(), ThreatRule {
            name: "Resource Exhaustion".to_string(),
            threat_type: ThreatType::ResourceExhaustion,
            severity: ThreatSeverity::High,
            pattern: "memory_alloc|cpu_spike".to_string(),
            threshold: 5,
            time_window: 30,
            enabled: true,
        });

        // Unauthorized access rule
        self.rules.insert("unauth_access".to_string(), ThreatRule {
            name: "Unauthorized File Access".to_string(),
            threat_type: ThreatType::UnauthorizedAccess,
            severity: ThreatSeverity::High,
            pattern: "open|read_denied".to_string(),
            threshold: 10,
            time_window: 60,
            enabled: true,
        });

        // Syscall violation rule
        self.rules.insert("syscall_violation".to_string(), ThreatRule {
            name: "Blocked Syscall Attempt".to_string(),
            threat_type: ThreatType::SyscallViolation,
            severity: ThreatSeverity::Medium,
            pattern: "execve|ptrace|fork".to_string(),
            threshold: 3,
            time_window: 60,
            enabled: true,
        });

        // Container breakout rule
        self.rules.insert("container_breakout".to_string(), ThreatRule {
            name: "Container Breakout Attempt".to_string(),
            threat_type: ThreatType::ContainerBreakout,
            severity: ThreatSeverity::Critical,
            pattern: "namespace_escape|mount_override".to_string(),
            threshold: 1,
            time_window: 60,
            enabled: true,
        });
    }

    /// Analyze event for threats
    pub fn analyze_event(&mut self, source: &str, action: &str, target: Option<&str>, evidence: &str) -> Vec<DetectedThreat> {
        let mut detected = Vec::new();

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check each rule
        for (rule_id, rule) in &self.rules {
            if !rule.enabled {
                continue;
            }

            // Pattern matching - check if pattern keywords exist in action or evidence
            let pattern_parts: Vec<&str> = rule.pattern.split('|').collect();
            let matches = pattern_parts.iter()
                .any(|p| action.contains(p) || evidence.contains(p));
            
            if matches {
                // Check history and threshold
                let history_key = format!("{}:{}", rule_id, source);
                let history = self.threat_history.entry(history_key.clone()).or_insert_with(Vec::new);

                // Clean old entries
                history.retain(|&timestamp| current_time - timestamp < rule.time_window);

                // Check threshold
                if history.len() >= rule.threshold - 1 {
                    let threat = DetectedThreat {
                        rule_name: rule.name.clone(),
                        threat_type: rule.threat_type.clone(),
                        severity: rule.severity,
                        source: source.to_string(),
                        target: target.map(|s| s.to_string()),
                        evidence: evidence.to_string(),
                        timestamp: current_time,
                        occurrence_count: history.len() + 1,
                    };

                    detected.push(threat.clone());
                    self.threats.push(threat);

                    // Block if critical
                    if rule.severity == ThreatSeverity::Critical {
                        self.blocked_entities.insert(source.to_string());
                    }
                }

                history.push(current_time);
            }
        }

        detected
    }

    /// Check if entity is blocked
    pub fn is_blocked(&self, entity: &str) -> bool {
        self.blocked_entities.contains(entity)
    }

    /// Block an entity
    pub fn block_entity(&mut self, entity: &str) {
        self.blocked_entities.insert(entity.to_string());
    }

    /// Unblock an entity
    pub fn unblock_entity(&mut self, entity: &str) {
        self.blocked_entities.remove(entity);
    }

    /// Get threats by severity
    pub fn get_threats_by_severity(&self, severity: ThreatSeverity) -> Vec<&DetectedThreat> {
        self.threats.iter()
            .filter(|t| t.severity == severity)
            .collect()
    }

    /// Get threats for a source
    pub fn get_threats_for_source(&self, source: &str) -> Vec<&DetectedThreat> {
        self.threats.iter()
            .filter(|t| t.source == source)
            .collect()
    }

    /// Get threat statistics
    pub fn get_statistics(&self) -> ThreatStatistics {
        let mut stats = ThreatStatistics::new();

        for threat in &self.threats {
            stats.total_threats += 1;
            match threat.severity {
                ThreatSeverity::Low => stats.low_severity += 1,
                ThreatSeverity::Medium => stats.medium_severity += 1,
                ThreatSeverity::High => stats.high_severity += 1,
                ThreatSeverity::Critical => stats.critical_severity += 1,
            }
        }

        stats.blocked_entities = self.blocked_entities.len();

        stats
    }

    /// Generate threat report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Threat Intelligence Report ===\n\n");

        let stats = self.get_statistics();
        report.push_str(&format!("Total Threats Detected: {}\n", stats.total_threats));
        report.push_str(&format!("  Critical: {}\n", stats.critical_severity));
        report.push_str(&format!("  High: {}\n", stats.high_severity));
        report.push_str(&format!("  Medium: {}\n", stats.medium_severity));
        report.push_str(&format!("  Low: {}\n\n", stats.low_severity));

        report.push_str(&format!("Blocked Entities: {}\n\n", stats.blocked_entities));

        if !self.threats.is_empty() {
            report.push_str("Recent Threats:\n");
            for threat in self.threats.iter().rev().take(10) {
                report.push_str(&format!("  [{}] {} - {} ({})\n",
                    threat.timestamp,
                    threat.rule_name,
                    threat.source,
                    threat.severity.as_str()));
            }
        }

        report
    }

    /// Print threat report
    pub fn print_report(&self) {
        println!("\n=== Threat Intelligence Report (Phase 21) ===");
        
        let stats = self.get_statistics();
        println!("Total Threats: {}", stats.total_threats);
        println!("  Critical: {}", stats.critical_severity);
        println!("  High: {}", stats.high_severity);
        println!("  Medium: {}", stats.medium_severity);
        println!("  Low: {}", stats.low_severity);

        println!("\nBlocked Entities: {}", stats.blocked_entities);

        println!("\nActive Rules: {}", self.rules.iter().filter(|(_, r)| r.enabled).count());

        if !self.threats.is_empty() {
            println!("\nRecent Threats (last 5):");
            for threat in self.threats.iter().rev().take(5) {
                println!("  {}: {} ({})", threat.rule_name, threat.severity.as_str(), threat.source);
            }
        }
    }
}

/// Threat statistics
#[derive(Debug)]
pub struct ThreatStatistics {
    pub total_threats: usize,
    pub critical_severity: usize,
    pub high_severity: usize,
    pub medium_severity: usize,
    pub low_severity: usize,
    pub blocked_entities: usize,
}

impl ThreatStatistics {
    pub fn new() -> Self {
        ThreatStatistics {
            total_threats: 0,
            critical_severity: 0,
            high_severity: 0,
            medium_severity: 0,
            low_severity: 0,
            blocked_entities: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_intelligence_init() {
        let engine = ThreatIntelligence::new();
        assert!(!engine.rules.is_empty());
    }

    #[test]
    fn test_threat_detection() {
        let mut engine = ThreatIntelligence::new();
        
        let threats = engine.analyze_event("container1", "setuid", None, "Privilege escalation attempt");
        assert!(!threats.is_empty());
        assert_eq!(threats[0].severity, ThreatSeverity::Critical);
    }

    #[test]
    fn test_entity_blocking() {
        let mut engine = ThreatIntelligence::new();
        
        engine.block_entity("malicious_app");
        assert!(engine.is_blocked("malicious_app"));
        
        engine.unblock_entity("malicious_app");
        assert!(!engine.is_blocked("malicious_app"));
    }

    #[test]
    fn test_threat_threshold() {
        let mut engine = ThreatIntelligence::new();
        
        // Single occurrence below threshold
        let threats1 = engine.analyze_event("container1", "execve", None, "Blocked syscall");
        
        // Multiple occurrences should trigger
        let threats2 = engine.analyze_event("container1", "execve", None, "Blocked syscall");
        let threats3 = engine.analyze_event("container1", "execve", None, "Blocked syscall");
        
        assert!(!threats3.is_empty());
    }
}
