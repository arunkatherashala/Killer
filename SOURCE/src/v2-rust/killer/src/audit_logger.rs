// Phase 21: Audit & Monitoring - Comprehensive Audit Logger
// Records and analyzes all system activities

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

/// Audit log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLevel {
    Trace,      // Trace every operation
    Debug,      // Debug-level details
    Info,       // Informational events
    Warning,    // Warning level
    Error,      // Error level
    Critical,   // Critical events
}

impl AuditLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditLevel::Trace => "TRACE",
            AuditLevel::Debug => "DEBUG",
            AuditLevel::Info => "INFO",
            AuditLevel::Warning => "WARN",
            AuditLevel::Error => "ERRR",
            AuditLevel::Critical => "CRIT",
        }
    }

    pub fn to_priority(&self) -> u32 {
        match self {
            AuditLevel::Trace => 0,
            AuditLevel::Debug => 10,
            AuditLevel::Info => 20,
            AuditLevel::Warning => 30,
            AuditLevel::Error => 40,
            AuditLevel::Critical => 50,
        }
    }
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub level: AuditLevel,
    pub component: String,
    pub action: String,
    pub target: Option<String>,
    pub details: String,
    pub source: Option<String>,
    pub result: bool,
}

impl AuditEvent {
    pub fn new(level: AuditLevel, component: &str, action: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        AuditEvent {
            timestamp: ts,
            level,
            component: component.to_string(),
            action: action.to_string(),
            target: None,
            details: String::new(),
            source: None,
            result: true,
        }
    }

    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = details.to_string();
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    pub fn with_result(mut self, result: bool) -> Self {
        self.result = result;
        self
    }
}

/// Audit logger
pub struct AuditLogger {
    events: VecDeque<AuditEvent>,
    max_events: usize,
    min_level: AuditLevel,
    filters: HashMap<String, bool>,
    statistics: AuditStatistics,
}

impl AuditLogger {
    pub fn new(max_events: usize, min_level: AuditLevel) -> Self {
        AuditLogger {
            events: VecDeque::new(),
            max_events,
            min_level,
            filters: HashMap::new(),
            statistics: AuditStatistics::new(),
        }
    }

    /// Log an event
    pub fn log_event(&mut self, event: AuditEvent) {
        // Check minimum level
        if event.level.to_priority() < self.min_level.to_priority() {
            return;
        }

        // Check filters
        if let Some(&enabled) = self.filters.get(&event.component) {
            if !enabled {
                return;
            }
        }

        // Update statistics
        self.statistics.log_event(&event);

        // Add to queue
        self.events.push_back(event);

        // Enforce size limit
        while self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    /// Enable/disable component logging
    pub fn set_component_filter(&mut self, component: &str, enabled: bool) {
        self.filters.insert(component.to_string(), enabled);
    }

    /// Get events matching criteria
    pub fn get_events(&self, component: Option<&str>, level: Option<AuditLevel>) -> Vec<&AuditEvent> {
        self.events.iter()
            .filter(|e| {
                let comp_match = component.is_none() || component == Some(&e.component);
                let level_match = level.is_none() || level == Some(e.level);
                comp_match && level_match
            })
            .collect()
    }

    /// Get events for a specific target
    pub fn get_events_for_target(&self, target: &str) -> Vec<&AuditEvent> {
        self.events.iter()
            .filter(|e| e.target.as_deref() == Some(target))
            .collect()
    }

    /// Get failed operations
    pub fn get_failed_operations(&self) -> Vec<&AuditEvent> {
        self.events.iter()
            .filter(|e| !e.result)
            .collect()
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &AuditStatistics {
        &self.statistics
    }

    /// Export audit log to text format
    pub fn export_to_text(&self) -> String {
        let mut output = String::from("=== Audit Log Export ===\n\n");

        for event in self.events.iter() {
            output.push_str(&format!("[{}] {} {} - {}\n",
                event.timestamp,
                event.level.as_str(),
                event.component,
                event.action));

            if let Some(target) = &event.target {
                output.push_str(&format!("  Target: {}\n", target));
            }
            if !event.details.is_empty() {
                output.push_str(&format!("  Details: {}\n", event.details));
            }
            if let Some(source) = &event.source {
                output.push_str(&format!("  Source: {}\n", source));
            }
            output.push_str(&format!("  Result: {}\n\n", if event.result { "SUCCESS" } else { "FAILED" }));
        }

        output
    }

    /// Print audit report
    pub fn print_report(&self) {
        println!("\n=== Audit Logger Report (Phase 21) ===");
        println!("Max Events: {}", self.max_events);
        println!("Minimum Level: {:?}", self.min_level);
        println!("Current Events: {}", self.events.len());

        println!("\nStatistics:");
        println!("  Total Events: {}", self.statistics.total_events);
        println!("  Trace Events: {}", self.statistics.trace_count);
        println!("  Debug Events: {}", self.statistics.debug_count);
        println!("  Info Events: {}", self.statistics.info_count);
        println!("  Warning Events: {}", self.statistics.warning_count);
        println!("  Error Events: {}", self.statistics.error_count);
        println!("  Critical Events: {}", self.statistics.critical_count);

        let failed_count = self.get_failed_operations().len();
        if failed_count > 0 {
            println!("\n⚠ Failed Operations: {}", failed_count);
        }

        if !self.filters.is_empty() {
            println!("\nComponent Filters:");
            for (component, enabled) in &self.filters {
                println!("  {}: {}", component, if *enabled { "enabled" } else { "disabled" });
            }
        }

        println!("\nRecent Events (last 5):");
        for event in self.events.iter().rev().take(5) {
            println!("  [{}] {}: {}", 
                event.level.as_str(),
                event.component,
                event.action);
        }
    }
}

/// Audit statistics
#[derive(Debug, Clone)]
pub struct AuditStatistics {
    pub total_events: usize,
    pub trace_count: usize,
    pub debug_count: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub critical_count: usize,
}

impl AuditStatistics {
    pub fn new() -> Self {
        AuditStatistics {
            total_events: 0,
            trace_count: 0,
            debug_count: 0,
            info_count: 0,
            warning_count: 0,
            error_count: 0,
            critical_count: 0,
        }
    }

    fn log_event(&mut self, event: &AuditEvent) {
        self.total_events += 1;
        match event.level {
            AuditLevel::Trace => self.trace_count += 1,
            AuditLevel::Debug => self.debug_count += 1,
            AuditLevel::Info => self.info_count += 1,
            AuditLevel::Warning => self.warning_count += 1,
            AuditLevel::Error => self.error_count += 1,
            AuditLevel::Critical => self.critical_count += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditLevel::Info, "vm", "execute")
            .with_target("script.killer")
            .with_details("Function call");

        assert_eq!(event.component, "vm");
        assert_eq!(event.action, "execute");
        assert_eq!(event.target, Some("script.killer".to_string()));
    }

    #[test]
    fn test_audit_logger() {
        let mut logger = AuditLogger::new(100, AuditLevel::Info);

        let event = AuditEvent::new(AuditLevel::Info, "seccomp", "syscall_filtered")
            .with_target("execve")
            .with_result(false);

        logger.log_event(event);

        assert_eq!(logger.events.len(), 1);
        let failed = logger.get_failed_operations();
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_component_filter() {
        let mut logger = AuditLogger::new(100, AuditLevel::Trace);
        logger.set_component_filter("vm", false);

        let event = AuditEvent::new(AuditLevel::Info, "vm", "test");
        logger.log_event(event);

        assert_eq!(logger.events.len(), 0);
    }
}
