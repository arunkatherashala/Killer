// Telemetry Collection System for Killer Language
// Purpose: Collect metrics, traces, and performance data
// Status: Production-ready

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Represents a metric value (counter, gauge, or histogram)
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),  // Stores samples for histogram
}

/// Histogram for tracking latency distributions
#[derive(Debug, Clone)]
pub struct Histogram {
    buckets: Vec<(f64, u64)>,  // (boundary_ms, count)
    total_count: u64,
    total_sum: f64,
}

impl Histogram {
    pub fn new() -> Self {
        Histogram {
            buckets: vec![
                (1.0, 0), (5.0, 0), (10.0, 0), (50.0, 0), (100.0, 0),
                (500.0, 0), (1000.0, 0), (5000.0, 0),
            ],
            total_count: 0,
            total_sum: 0.0,
        }
    }

    pub fn record(&mut self, value_ms: f64) {
        self.total_count += 1;
        self.total_sum += value_ms;

        for (boundary, count) in &mut self.buckets {
            if value_ms <= *boundary {
                *count += 1;
                break;
            }
        }
    }

    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        let target_index = ((self.total_count as f64 * p) / 100.0).ceil() as u64;
        let mut count = 0;

        for (boundary, bucket_count) in &self.buckets {
            count += bucket_count;
            if count >= target_index {
                return *boundary;
            }
        }

        self.buckets.last().map(|(b, _)| *b).unwrap_or(0.0)
    }

    pub fn avg(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.total_sum / self.total_count as f64
        }
    }
}

/// Application metrics
#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub latency_histogram: Histogram,
    pub success_rate: f64,
}

impl ApplicationMetrics {
    pub fn new() -> Self {
        ApplicationMetrics {
            request_count: 0,
            error_count: 0,
            latency_histogram: Histogram::new(),
            success_rate: 100.0,
        }
    }

    pub fn record_request(&mut self, duration_ms: f64, success: bool) {
        self.request_count += 1;
        self.latency_histogram.record(duration_ms);

        if !success {
            self.error_count += 1;
        }

        if self.request_count > 0 {
            self.success_rate = ((self.request_count - self.error_count) as f64 / self.request_count as f64) * 100.0;
        }
    }

    pub fn get_p50_latency(&self) -> f64 {
        self.latency_histogram.percentile(50.0)
    }

    pub fn get_p95_latency(&self) -> f64 {
        self.latency_histogram.percentile(95.0)
    }

    pub fn get_p99_latency(&self) -> f64 {
        self.latency_histogram.percentile(99.0)
    }
}

/// VM metrics
#[derive(Debug, Clone)]
pub struct VmMetrics {
    pub instructions_executed: u64,
    pub functions_called: u64,
    pub optimization_tier_transitions: u64,
    pub jit_compiles: u64,
    pub gc_runs: u64,
    pub gc_time_ms: f64,
}

impl VmMetrics {
    pub fn new() -> Self {
        VmMetrics {
            instructions_executed: 0,
            functions_called: 0,
            optimization_tier_transitions: 0,
            jit_compiles: 0,
            gc_runs: 0,
            gc_time_ms: 0.0,
        }
    }
}

/// Resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub network_io_bytes: u64,
    pub disk_usage_mb: u64,
}

impl ResourceMetrics {
    pub fn new() -> Self {
        ResourceMetrics {
            cpu_percent: 0.0,
            memory_mb: 0,
            network_io_bytes: 0,
            disk_usage_mb: 0,
        }
    }
}

/// Main telemetry collector
pub struct TelemetryCollector {
    app_metrics: Arc<Mutex<ApplicationMetrics>>,
    vm_metrics: Arc<Mutex<VmMetrics>>,
    resource_metrics: Arc<Mutex<ResourceMetrics>>,
    start_time: SystemTime,
    enabled: bool,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        TelemetryCollector {
            app_metrics: Arc::new(Mutex::new(ApplicationMetrics::new())),
            vm_metrics: Arc::new(Mutex::new(VmMetrics::new())),
            resource_metrics: Arc::new(Mutex::new(ResourceMetrics::new())),
            start_time: SystemTime::now(),
            enabled: true,
        }
    }

    pub fn record_request(&self, duration_ms: f64, success: bool) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.app_metrics.lock() {
            metrics.record_request(duration_ms, success);
        }
    }

    pub fn record_vm_instruction(&self) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.vm_metrics.lock() {
            metrics.instructions_executed += 1;
        }
    }

    pub fn record_function_call(&self) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.vm_metrics.lock() {
            metrics.functions_called += 1;
        }
    }

    pub fn record_optimization_tier_transition(&self) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.vm_metrics.lock() {
            metrics.optimization_tier_transitions += 1;
        }
    }

    pub fn record_jit_compile(&self) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.vm_metrics.lock() {
            metrics.jit_compiles += 1;
        }
    }

    pub fn record_gc(&self, duration_ms: f64) {
        if !self.enabled {
            return;
        }

        if let Ok(mut metrics) = self.vm_metrics.lock() {
            metrics.gc_runs += 1;
            metrics.gc_time_ms += duration_ms;
        }
    }

    pub fn update_resource_metrics(&self, metrics: ResourceMetrics) {
        if !self.enabled {
            return;
        }

        if let Ok(mut resource_metrics) = self.resource_metrics.lock() {
            *resource_metrics = metrics;
        }
    }

    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let app = self.app_metrics.lock().ok().map(|g| g.clone()).unwrap_or_default();
        let vm = self.vm_metrics.lock().ok().map(|g| g.clone()).unwrap_or_default();
        let resources = self.resource_metrics.lock().ok().map(|g| g.clone()).unwrap_or_default();

        let uptime = self.start_time.elapsed().unwrap_or_default();

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            uptime,
            app_metrics: app,
            vm_metrics: vm,
            resource_metrics: resources,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn reset(&mut self) {
        self.app_metrics = Arc::new(Mutex::new(ApplicationMetrics::new()));
        self.vm_metrics = Arc::new(Mutex::new(VmMetrics::new()));
        self.resource_metrics = Arc::new(Mutex::new(ResourceMetrics::new()));
        self.start_time = SystemTime::now();
    }
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VmMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics snapshot for export
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub uptime: Duration,
    pub app_metrics: ApplicationMetrics,
    pub vm_metrics: VmMetrics,
    pub resource_metrics: ResourceMetrics,
}

impl MetricsSnapshot {
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::new();

        // Application metrics
        output.push_str("# HELP killer_requests_total Total requests processed\n");
        output.push_str("# TYPE killer_requests_total counter\n");
        output.push_str(&format!("killer_requests_total {}\n", self.app_metrics.request_count));

        output.push_str("# HELP killer_errors_total Total errors\n");
        output.push_str("# TYPE killer_errors_total counter\n");
        output.push_str(&format!("killer_errors_total {}\n", self.app_metrics.error_count));

        output.push_str("# HELP killer_success_rate Success rate percentage\n");
        output.push_str("# TYPE killer_success_rate gauge\n");
        output.push_str(&format!("killer_success_rate {}\n", self.app_metrics.success_rate));

        output.push_str("# HELP killer_latency_p50_ms P50 latency in milliseconds\n");
        output.push_str("# TYPE killer_latency_p50_ms gauge\n");
        output.push_str(&format!("killer_latency_p50_ms {}\n", self.app_metrics.get_p50_latency()));

        output.push_str("# HELP killer_latency_p95_ms P95 latency in milliseconds\n");
        output.push_str("# TYPE killer_latency_p95_ms gauge\n");
        output.push_str(&format!("killer_latency_p95_ms {}\n", self.app_metrics.get_p95_latency()));

        output.push_str("# HELP killer_latency_p99_ms P99 latency in milliseconds\n");
        output.push_str("# TYPE killer_latency_p99_ms gauge\n");
        output.push_str(&format!("killer_latency_p99_ms {}\n", self.app_metrics.get_p99_latency()));

        output.push_str("# HELP killer_latency_avg_ms Average latency in milliseconds\n");
        output.push_str("# TYPE killer_latency_avg_ms gauge\n");
        output.push_str(&format!("killer_latency_avg_ms {}\n", self.app_metrics.latency_histogram.avg()));

        // VM metrics
        output.push_str("# HELP killer_instructions_total Total instructions executed\n");
        output.push_str("# TYPE killer_instructions_total counter\n");
        output.push_str(&format!("killer_instructions_total {}\n", self.vm_metrics.instructions_executed));

        output.push_str("# HELP killer_gc_runs_total Total garbage collection runs\n");
        output.push_str("# TYPE killer_gc_runs_total counter\n");
        output.push_str(&format!("killer_gc_runs_total {}\n", self.vm_metrics.gc_runs));

        output.push_str("# HELP killer_gc_time_ms Total GC time in milliseconds\n");
        output.push_str("# TYPE killer_gc_time_ms gauge\n");
        output.push_str(&format!("killer_gc_time_ms {}\n", self.vm_metrics.gc_time_ms));

        // Resource metrics
        output.push_str("# HELP killer_memory_mb Current memory usage in MB\n");
        output.push_str("# TYPE killer_memory_mb gauge\n");
        output.push_str(&format!("killer_memory_mb {}\n", self.resource_metrics.memory_mb));

        output.push_str("# HELP killer_uptime_seconds Uptime in seconds\n");
        output.push_str("# TYPE killer_uptime_seconds gauge\n");
        output.push_str(&format!("killer_uptime_seconds {}\n", self.uptime.as_secs()));

        output
    }

    pub fn to_json_format(&self) -> String {
        format!(
            r#"{{
  "timestamp": {},
  "uptime_seconds": {},
  "requests": {{
    "total": {},
    "errors": {},
    "success_rate": {}
  }},
  "latency": {{
    "p50_ms": {},
    "p95_ms": {},
    "p99_ms": {},
    "avg_ms": {}
  }},
  "vm": {{
    "instructions": {},
    "functions_called": {},
    "gc_runs": {},
    "gc_time_ms": {}
  }},
  "resources": {{
    "memory_mb": {},
    "cpu_percent": {}
  }}
}}"#,
            self.timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            self.uptime.as_secs(),
            self.app_metrics.request_count,
            self.app_metrics.error_count,
            self.app_metrics.success_rate,
            self.app_metrics.get_p50_latency(),
            self.app_metrics.get_p95_latency(),
            self.app_metrics.get_p99_latency(),
            self.app_metrics.latency_histogram.avg(),
            self.vm_metrics.instructions_executed,
            self.vm_metrics.functions_called,
            self.vm_metrics.gc_runs,
            self.vm_metrics.gc_time_ms,
            self.resource_metrics.memory_mb,
            self.resource_metrics.cpu_percent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_recording() {
        let mut hist = Histogram::new();
        hist.record(5.0);
        hist.record(10.0);
        hist.record(50.0);

        assert_eq!(hist.total_count, 3);
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut hist = Histogram::new();
        for i in 1..=100 {
            hist.record(i as f64);
        }

        let p50 = hist.percentile(50.0);
        let p99 = hist.percentile(99.0);

        assert!(p50 > 0.0);
        assert!(p99 >= p50);
    }

    #[test]
    fn test_application_metrics() {
        let mut metrics = ApplicationMetrics::new();
        metrics.record_request(10.0, true);
        metrics.record_request(20.0, true);
        metrics.record_request(30.0, false);

        assert_eq!(metrics.request_count, 3);
        assert_eq!(metrics.error_count, 1);
        assert!(metrics.success_rate > 60.0 && metrics.success_rate < 70.0);
    }

    #[test]
    fn test_telemetry_collector() {
        let collector = TelemetryCollector::new();

        collector.record_request(15.0, true);
        collector.record_vm_instruction();
        collector.record_function_call();

        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.app_metrics.request_count, 1);
        assert_eq!(snapshot.vm_metrics.instructions_executed, 1);
        assert_eq!(snapshot.vm_metrics.functions_called, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let collector = TelemetryCollector::new();
        collector.record_request(10.0, true);

        let snapshot = collector.get_metrics_snapshot();
        let prometheus_format = snapshot.to_prometheus_format();

        assert!(prometheus_format.contains("killer_requests_total 1"));
        assert!(prometheus_format.contains("killer_success_rate"));
    }

    #[test]
    fn test_json_export() {
        let collector = TelemetryCollector::new();
        collector.record_request(10.0, true);

        let snapshot = collector.get_metrics_snapshot();
        let json_format = snapshot.to_json_format();

        assert!(json_format.contains("\"total\": 1"));
        assert!(json_format.contains("\"success_rate\""));
    }

    #[test]
    fn test_enable_disable() {
        let mut collector = TelemetryCollector::new();

        collector.disable();
        collector.record_request(10.0, true);

        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.app_metrics.request_count, 0);

        collector.enable();
        collector.record_request(10.0, true);

        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.app_metrics.request_count, 1);
    }

    #[test]
    fn test_reset() {
        let mut collector = TelemetryCollector::new();
        collector.record_request(10.0, true);

        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.app_metrics.request_count, 1);

        collector.reset();

        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.app_metrics.request_count, 0);
    }
}
