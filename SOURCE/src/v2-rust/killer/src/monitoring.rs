// killer_rcore/src/monitoring.rs
// Monitoring and metrics framework for Killer compiler
// Tracks compilation performance, resource usage, and system health

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;

/// Represents a point-in-time measurement
#[derive(Clone, Debug)]
pub struct Measurement {
    /// Metric name
    pub name: String,
    /// Measured value
    pub value: f64,
    /// Unit (ms, bytes, %, etc.)
    pub unit: String,
    /// Timestamp
    pub timestamp: u64,
}

impl Measurement {
    /// Create a new measurement
    pub fn new(name: impl Into<String>, value: f64, unit: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Measurement {
            name: name.into(),
            value,
            unit: unit.into(),
            timestamp,
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {:.2} {}", self.name, self.value, self.unit)
    }
}

/// Compilation phase metrics
#[derive(Clone, Debug, Default)]
pub struct PhaseMetrics {
    /// Phase name
    pub phase: String,
    /// Time spent (ms)
    pub duration_ms: f64,
    /// Memory used (bytes)
    pub memory_bytes: u64,
    /// Items processed (files, functions, etc.)
    pub items_processed: u64,
    /// Errors encountered
    pub errors: u32,
    /// Warnings encountered
    pub warnings: u32,
}

impl PhaseMetrics {
    /// Create new phase metrics
    pub fn new(phase: impl Into<String>) -> Self {
        PhaseMetrics {
            phase: phase.into(),
            duration_ms: 0.0,
            memory_bytes: 0,
            items_processed: 0,
            errors: 0,
            warnings: 0,
        }
    }

    /// Calculate throughput (items per second)
    pub fn throughput(&self) -> f64 {
        if self.duration_ms <= 0.0 {
            return 0.0;
        }
        (self.items_processed as f64 / self.duration_ms) * 1000.0
    }

    /// Calculate memory per item
    pub fn memory_per_item(&self) -> f64 {
        if self.items_processed == 0 {
            return 0.0;
        }
        self.memory_bytes as f64 / self.items_processed as f64
    }
}

impl fmt::Display for PhaseMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {:.2}ms, {:.1} items/sec, {} bytes",
            self.phase,
            self.duration_ms,
            self.throughput(),
            self.memory_bytes
        )
    }
}

/// Resource monitoring
#[derive(Clone, Debug, Default)]
pub struct ResourceMetrics {
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Peak memory used
    pub peak_memory_bytes: u64,
    /// Disk I/O operations
    pub disk_ops: u32,
    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f32,
}

impl ResourceMetrics {
    /// Format memory with human-readable units
    pub fn memory_display(&self) -> String {
        format_bytes(self.memory_bytes)
    }

    /// Format peak memory
    pub fn peak_memory_display(&self) -> String {
        format_bytes(self.peak_memory_bytes)
    }
}

impl fmt::Display for ResourceMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CPU: {:.1}%, Memory: {} (peak: {}), Cache: {:.1}%",
            self.cpu_percent,
            self.memory_display(),
            self.peak_memory_display(),
            self.cache_hit_rate * 100.0
        )
    }
}

/// Health status of the monitoring system
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Critical => write!(f, "Critical"),
        }
    }
}

/// System health monitoring
#[derive(Clone, Debug)]
pub struct HealthMonitor {
    /// Overall health status
    pub status: HealthStatus,
    /// Last error message
    pub last_error: Option<String>,
    /// Compilation success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Average compilation time (ms)
    pub avg_compile_time_ms: f64,
    /// Slow compilations (> 1000ms)
    pub slow_compilations: u32,
    /// Failed compilations
    pub failed_compilations: u32,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new() -> Self {
        HealthMonitor {
            status: HealthStatus::Healthy,
            last_error: None,
            success_rate: 1.0,
            avg_compile_time_ms: 0.0,
            slow_compilations: 0,
            failed_compilations: 0,
        }
    }

    /// Update health based on compilation result
    pub fn record_compilation(&mut self, success: bool, duration_ms: f64) {
        if !success {
            self.failed_compilations += 1;
            self.last_error = Some("Compilation failed".to_string());
        }

        if duration_ms > 1000.0 {
            self.slow_compilations += 1;
        }

        self.recalculate_status();
    }

    /// Recalculate health status
    fn recalculate_status(&mut self) {
        let total = self.failed_compilations + self.slow_compilations;

        if self.failed_compilations > 10 {
            self.status = HealthStatus::Critical;
        } else if total > 20 || self.failed_compilations > 3 {
            self.status = HealthStatus::Degraded;
        } else {
            self.status = HealthStatus::Healthy;
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HealthMonitor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Status: {}, Success: {:.1}%, Slow: {}, Failed: {}",
            self.status, self.success_rate * 100.0, self.slow_compilations, self.failed_compilations
        )
    }
}

/// Main monitoring framework
pub struct Monitor {
    /// Phase metrics
    phases: HashMap<String, PhaseMetrics>,
    /// Resource usage
    resources: ResourceMetrics,
    /// System health
    health: HealthMonitor,
    /// All measurements
    measurements: Vec<Measurement>,
    /// Enabled monitoring
    enabled: bool,
}

impl Monitor {
    /// Create a new monitor
    pub fn new() -> Self {
        Monitor {
            phases: HashMap::new(),
            resources: ResourceMetrics::default(),
            health: HealthMonitor::new(),
            measurements: Vec::new(),
            enabled: true,
        }
    }

    /// Enable/disable monitoring
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Record a measurement
    pub fn record(&mut self, name: impl Into<String>, value: f64, unit: impl Into<String>) {
        if !self.enabled {
            return;
        }
        self.measurements.push(Measurement::new(name, value, unit));
    }

    /// Record phase metrics
    pub fn record_phase(&mut self, metrics: PhaseMetrics) {
        if !self.enabled {
            return;
        }
        self.phases.insert(metrics.phase.clone(), metrics);
    }

    /// Record resource usage
    pub fn record_resources(&mut self, resources: ResourceMetrics) {
        if !self.enabled {
            return;
        }
        self.resources = resources;
    }

    /// Record compilation result
    pub fn record_compilation(&mut self, success: bool, duration_ms: f64) {
        if !self.enabled {
            return;
        }
        self.health.record_compilation(success, duration_ms);
    }

    /// Get all measurements
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    /// Get phase metrics for a specific phase
    pub fn phase(&self, name: &str) -> Option<&PhaseMetrics> {
        self.phases.get(name)
    }

    /// Get all phases
    pub fn phases(&self) -> &HashMap<String, PhaseMetrics> {
        &self.phases
    }

    /// Get resource metrics
    pub fn resources(&self) -> &ResourceMetrics {
        &self.resources
    }

    /// Get health status
    pub fn health(&self) -> &HealthMonitor {
        &self.health
    }

    /// Generate summary report
    pub fn summary(&self) -> String {
        let mut report = String::new();

        report.push_str("+============================================================+\n");
        report.push_str("|         KILLER COMPILER MONITORING SUMMARY                 |\n");
        report.push_str("+============================================================+\n\n");

        // Health Status
        report.push_str(&format!("📊 HEALTH STATUS: {}\n\n", self.health));

        // Phase Metrics
        if !self.phases.is_empty() {
            report.push_str("⏱️  COMPILATION PHASES:\n");
            let mut total_time = 0.0;
            for (_, phase) in &self.phases {
                report.push_str(&format!("   • {}\n", phase));
                total_time += phase.duration_ms;
            }
            report.push_str(&format!("   TOTAL: {:.2}ms\n\n", total_time));
        }

        // Resource Usage
        report.push_str(&format!("💾 RESOURCE USAGE:\n   {}\n\n", self.resources));

        // Recent Measurements
        if !self.measurements.is_empty() {
            report.push_str("📈 RECENT MEASUREMENTS:\n");
            for m in self.measurements.iter().take(10) {
                report.push_str(&format!("   • {}\n", m));
            }
            if self.measurements.len() > 10 {
                report.push_str(&format!("   ... and {} more\n", self.measurements.len() - 10));
            }
            report.push_str("\n");
        }

        report.push_str("+============================================================+\n");

        report
    }

    /// Export metrics as JSON-like format
    pub fn export_json(&self) -> String {
        let mut json = String::from("{\n  \"monitoring\": {\n");

        // Health
        json.push_str("    \"health\": {\n");
        json.push_str(&format!("      \"status\": \"{}\",\n", self.health.status));
        json.push_str(&format!("      \"success_rate\": {},\n", self.health.success_rate));
        json.push_str(&format!("      \"failed_compilations\": {}\n", self.health.failed_compilations));
        json.push_str("    },\n");

        // Resources
        json.push_str("    \"resources\": {\n");
        json.push_str(&format!("      \"cpu_percent\": {},\n", self.resources.cpu_percent));
        json.push_str(&format!("      \"memory_bytes\": {},\n", self.resources.memory_bytes));
        json.push_str(&format!("      \"cache_hit_rate\": {}\n", self.resources.cache_hit_rate));
        json.push_str("    },\n");

        // Phases
        json.push_str("    \"phases\": {\n");
        for (name, phase) in &self.phases {
            json.push_str(&format!("      \"{}\": {{\n", name));
            json.push_str(&format!("        \"duration_ms\": {},\n", phase.duration_ms));
            json.push_str(&format!("        \"items_processed\": {}\n", phase.items_processed));
            json.push_str("      },\n");
        }
        json.push_str("    }\n");

        json.push_str("  }\n}\n");

        json
    }

    /// Clear all metrics
    pub fn reset(&mut self) {
        self.phases.clear();
        self.measurements.clear();
        self.resources = ResourceMetrics::default();
        self.health = HealthMonitor::new();
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Format bytes with human-readable units
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_creation() {
        let m = Measurement::new("test_metric", 42.5, "ms");
        assert_eq!(m.name, "test_metric");
        assert_eq!(m.value, 42.5);
        assert_eq!(m.unit, "ms");
    }

    #[test]
    fn test_phase_metrics() {
        let mut phase = PhaseMetrics::new("lexing");
        phase.duration_ms = 10.0;
        phase.items_processed = 100;

        let throughput = phase.throughput();
        assert!(throughput > 0.0);
        assert_eq!(phase.memory_per_item(), 0.0);
    }

    #[test]
    fn test_resource_metrics_display() {
        let mut res = ResourceMetrics::default();
        res.memory_bytes = 1024 * 1024;
        res.peak_memory_bytes = 2 * 1024 * 1024;

        assert_eq!(res.memory_display(), "1.00 MB");
        assert_eq!(res.peak_memory_display(), "2.00 MB");
    }

    #[test]
    fn test_health_monitor() {
        let mut health = HealthMonitor::new();
        assert_eq!(health.status, HealthStatus::Healthy);

        health.record_compilation(false, 500.0);
        assert_eq!(health.failed_compilations, 1);
    }

    #[test]
    fn test_monitor_recording() {
        let mut monitor = Monitor::new();
        monitor.record("test", 42.0, "ms");
        assert_eq!(monitor.measurements().len(), 1);

        monitor.set_enabled(false);
        monitor.record("ignored", 10.0, "ms");
        assert_eq!(monitor.measurements().len(), 1);
    }

    #[test]
    fn test_monitor_phases() {
        let mut monitor = Monitor::new();
        let phase = PhaseMetrics::new("parsing");
        monitor.record_phase(phase.clone());

        assert!(monitor.phase("parsing").is_some());
        assert_eq!(monitor.phases().len(), 1);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_monitor_summary() {
        let mut monitor = Monitor::new();
        monitor.record_compilation(true, 100.0);
        let summary = monitor.summary();

        assert!(summary.contains("KILLER COMPILER MONITORING SUMMARY"));
        assert!(summary.contains("HEALTH STATUS"));
    }

    #[test]
    fn test_monitor_json_export() {
        let monitor = Monitor::new();
        let json = monitor.export_json();

        assert!(json.contains("\"monitoring\""));
        assert!(json.contains("\"health\""));
        assert!(json.contains("\"resources\""));
    }
}
