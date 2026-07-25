// Phase 14: Analytics & Telemetry - metrics, traces, profiling, events
// Features: Metrics collection, trace aggregation, performance profiling, event streaming

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metric types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
    Custom(String),
}

impl MetricType {
    pub fn as_str(&self) -> &str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Timer => "timer",
            MetricType::Custom(name) => name,
        }
    }
}

/// Metric data point
#[derive(Clone, Debug)]
pub struct MetricDataPoint {
    pub timestamp: u64,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

impl MetricDataPoint {
    pub fn new(value: f64) -> Self {
        MetricDataPoint {
            timestamp: current_timestamp(),
            value,
            labels: HashMap::new(),
        }
    }

    /// Add label
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }
}

/// Metric
#[derive(Clone, Debug)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: String,
    pub data_points: Vec<MetricDataPoint>,
}

impl Metric {
    pub fn new(name: String, metric_type: MetricType) -> Self {
        Metric {
            name,
            metric_type,
            description: String::new(),
            unit: String::new(),
            data_points: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set unit
    pub fn with_unit(mut self, unit: String) -> Self {
        self.unit = unit;
        self
    }

    /// Record data point
    pub fn record(&mut self, value: f64) {
        self.data_points.push(MetricDataPoint::new(value));
    }

    /// Record with labels
    pub fn record_with_labels(&mut self, value: f64, labels: HashMap<String, String>) {
        let mut point = MetricDataPoint::new(value);
        point.labels = labels;
        self.data_points.push(point);
    }

    /// Get latest value
    pub fn latest_value(&self) -> Option<f64> {
        self.data_points.last().map(|p| p.value)
    }

    /// Get sum of all values
    pub fn sum(&self) -> f64 {
        self.data_points.iter().map(|p| p.value).sum()
    }

    /// Get average value
    pub fn average(&self) -> Option<f64> {
        if self.data_points.is_empty() {
            None
        } else {
            Some(self.sum() / self.data_points.len() as f64)
        }
    }

    /// Get min value
    pub fn min(&self) -> Option<f64> {
        self.data_points.iter()
            .map(|p| p.value)
            .fold(None, |acc, v| {
                Some(acc.map_or(v, |a| a.min(v)))
            })
    }

    /// Get max value
    pub fn max(&self) -> Option<f64> {
        self.data_points.iter()
            .map(|p| p.value)
            .fold(None, |acc, v| {
                Some(acc.map_or(v, |a| a.max(v)))
            })
    }

    /// Get data point count
    pub fn point_count(&self) -> usize {
        self.data_points.len()
    }
}

/// Metrics collector
#[derive(Clone, Debug)]
pub struct MetricsCollector {
    pub metrics: HashMap<String, Metric>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            metrics: HashMap::new(),
        }
    }

    /// Register metric
    pub fn register_metric(&mut self, metric: Metric) -> Result<(), String> {
        if self.metrics.contains_key(&metric.name) {
            return Err(format!("Metric {} already registered", metric.name));
        }
        self.metrics.insert(metric.name.clone(), metric);
        Ok(())
    }

    /// Unregister metric
    pub fn unregister_metric(&mut self, name: &str) -> Result<(), String> {
        if self.metrics.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("Metric {} not found", name))
        }
    }

    /// Record value
    pub fn record(&mut self, name: &str, value: f64) -> Result<(), String> {
        let metric = self.metrics.get_mut(name)
            .ok_or_else(|| format!("Metric {} not found", name))?;
        metric.record(value);
        Ok(())
    }

    /// Get metric
    pub fn get_metric(&self, name: &str) -> Option<Metric> {
        self.metrics.get(name).cloned()
    }

    /// List all metrics
    pub fn list_metrics(&self) -> Vec<Metric> {
        self.metrics.values().cloned().collect()
    }

    /// Metric count
    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }

    /// Get export snapshot
    pub fn export_snapshot(&self) -> HashMap<String, f64> {
        self.metrics.iter()
            .filter_map(|(name, metric)| {
                metric.latest_value().map(|v| (name.clone(), v))
            })
            .collect()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace span
#[derive(Clone, Debug)]
pub struct TraceSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub tags: HashMap<String, String>,
    pub events: Vec<TraceEvent>,
}

impl TraceSpan {
    pub fn new(span_id: String, trace_id: String, operation_name: String) -> Self {
        TraceSpan {
            span_id,
            trace_id,
            parent_span_id: None,
            operation_name,
            start_time: current_timestamp(),
            end_time: None,
            tags: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Set parent span
    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_span_id = Some(parent_id);
        self
    }

    /// Add tag
    pub fn add_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Add event
    pub fn add_event(mut self, event: TraceEvent) -> Self {
        self.events.push(event);
        self
    }

    /// End span
    pub fn end(mut self) -> Self {
        self.end_time = Some(current_timestamp());
        self
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> Option<u64> {
        self.end_time.map(|end| end - self.start_time)
    }

    /// Check if finished
    pub fn is_finished(&self) -> bool {
        self.end_time.is_some()
    }
}

/// Trace event
#[derive(Clone, Debug)]
pub struct TraceEvent {
    pub timestamp: u64,
    pub name: String,
    pub message: String,
}

impl TraceEvent {
    pub fn new(name: String, message: String) -> Self {
        TraceEvent {
            timestamp: current_timestamp(),
            name,
            message,
        }
    }
}

/// Trace collector
#[derive(Clone, Debug)]
pub struct TraceCollector {
    pub spans: HashMap<String, TraceSpan>,
    pub traces: HashMap<String, Vec<String>>, // trace_id -> span_ids
}

impl TraceCollector {
    pub fn new() -> Self {
        TraceCollector {
            spans: HashMap::new(),
            traces: HashMap::new(),
        }
    }

    /// Record span
    pub fn record_span(&mut self, span: TraceSpan) -> Result<(), String> {
        if self.spans.contains_key(&span.span_id) {
            return Err(format!("Span {} already exists", span.span_id));
        }

        self.traces.entry(span.trace_id.clone())
            .or_insert_with(Vec::new)
            .push(span.span_id.clone());

        self.spans.insert(span.span_id.clone(), span);
        Ok(())
    }

    /// Get span
    pub fn get_span(&self, span_id: &str) -> Option<TraceSpan> {
        self.spans.get(span_id).cloned()
    }

    /// Get trace spans
    pub fn get_trace(&self, trace_id: &str) -> Vec<TraceSpan> {
        self.traces.get(trace_id)
            .iter()
            .flat_map(|span_ids| {
                span_ids.iter()
                    .filter_map(|id| self.spans.get(id).cloned())
            })
            .collect()
    }

    /// List all traces
    pub fn list_traces(&self) -> Vec<String> {
        self.traces.keys().cloned().collect()
    }

    /// Span count
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Trace count
    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance profile
#[derive(Clone, Debug)]
pub struct PerformanceProfile {
    pub function_name: String,
    pub total_calls: u64,
    pub total_time_ms: u64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
}

impl PerformanceProfile {
    pub fn new(function_name: String) -> Self {
        PerformanceProfile {
            function_name,
            total_calls: 0,
            total_time_ms: 0,
            min_time_ms: u64::MAX,
            max_time_ms: 0,
            avg_time_ms: 0.0,
        }
    }

    /// Record call
    pub fn record_call(mut self, time_ms: u64) -> Self {
        self.total_calls += 1;
        self.total_time_ms += time_ms;
        self.min_time_ms = self.min_time_ms.min(time_ms);
        self.max_time_ms = self.max_time_ms.max(time_ms);
        self.avg_time_ms = self.total_time_ms as f64 / self.total_calls as f64;
        self
    }
}

/// Profiler
#[derive(Clone, Debug)]
pub struct Profiler {
    pub profiles: HashMap<String, PerformanceProfile>,
}

impl Profiler {
    pub fn new() -> Self {
        Profiler {
            profiles: HashMap::new(),
        }
    }

    /// Start profiling function
    pub fn start_function(&mut self, function_name: String) -> u64 {
        if !self.profiles.contains_key(&function_name) {
            let profile = PerformanceProfile::new(function_name.clone());
            self.profiles.insert(function_name, profile);
        }
        current_timestamp()
    }

    /// End profiling function
    pub fn end_function(&mut self, function_name: &str, start_time: u64) -> Result<(), String> {
        let elapsed = current_timestamp() - start_time;
        let profile = self.profiles.get_mut(function_name)
            .ok_or_else(|| format!("Function {} not profiled", function_name))?;

        *profile = profile.clone().record_call(elapsed);
        Ok(())
    }

    /// Get profile
    pub fn get_profile(&self, function_name: &str) -> Option<PerformanceProfile> {
        self.profiles.get(function_name).cloned()
    }

    /// List all profiles
    pub fn list_profiles(&self) -> Vec<PerformanceProfile> {
        self.profiles.values().cloned().collect()
    }

    /// Profile count
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    /// Get slowest function
    pub fn get_slowest(&self) -> Option<PerformanceProfile> {
        self.profiles.values()
            .max_by(|a, b| a.avg_time_ms.partial_cmp(&b.avg_time_ms).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }

    /// Get most called function
    pub fn get_most_called(&self) -> Option<PerformanceProfile> {
        self.profiles.values()
            .max_by_key(|p| p.total_calls)
            .cloned()
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Event types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    Info,
    Warning,
    Error,
    Debug,
    Trace,
    Custom(String),
}

impl EventType {
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Info => "info",
            EventType::Warning => "warning",
            EventType::Error => "error",
            EventType::Debug => "debug",
            EventType::Trace => "trace",
            EventType::Custom(name) => name,
        }
    }
}

/// Log event
#[derive(Clone, Debug)]
pub struct LogEvent {
    pub event_id: String,
    pub timestamp: u64,
    pub event_type: EventType,
    pub source: String,
    pub message: String,
    pub context: HashMap<String, String>,
}

impl LogEvent {
    pub fn new(event_type: EventType, source: String, message: String) -> Self {
        LogEvent {
            event_id: format!("{}-{}", source, current_timestamp()),
            timestamp: current_timestamp(),
            event_type,
            source,
            message,
            context: HashMap::new(),
        }
    }

    /// Add context
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
}

/// Event stream
#[derive(Clone, Debug)]
pub struct EventStream {
    pub events: Vec<LogEvent>,
    pub filters: HashMap<String, EventType>,
}

impl EventStream {
    pub fn new() -> Self {
        EventStream {
            events: Vec::new(),
            filters: HashMap::new(),
        }
    }

    /// Add event
    pub fn add_event(&mut self, event: LogEvent) {
        self.events.push(event);
    }

    /// Filter by type
    pub fn get_events_by_type(&self, event_type: &EventType) -> Vec<LogEvent> {
        self.events.iter()
            .filter(|e| &e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Filter by source
    pub fn get_events_by_source(&self, source: &str) -> Vec<LogEvent> {
        self.events.iter()
            .filter(|e| e.source == source)
            .cloned()
            .collect()
    }

    /// Get recent events
    pub fn get_recent(&self, count: usize) -> Vec<LogEvent> {
        self.events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Error count
    pub fn error_count(&self) -> usize {
        self.get_events_by_type(&EventType::Error).len()
    }

    /// Warning count
    pub fn warning_count(&self) -> usize {
        self.get_events_by_type(&EventType::Warning).len()
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new()
    }
}

/// Telemetry aggregator
pub struct TelemetryAggregator {
    pub metrics: MetricsCollector,
    pub traces: TraceCollector,
    pub profiler: Profiler,
    pub events: EventStream,
}

impl TelemetryAggregator {
    pub fn new() -> Self {
        TelemetryAggregator {
            metrics: MetricsCollector::new(),
            traces: TraceCollector::new(),
            profiler: Profiler::new(),
            events: EventStream::new(),
        }
    }

    /// Get metrics snapshot
    pub fn metrics_snapshot(&self) -> HashMap<String, f64> {
        self.metrics.export_snapshot()
    }

    /// Get performance report
    pub fn performance_report(&self) -> Vec<PerformanceProfile> {
        self.profiler.list_profiles()
    }

    /// Get event summary
    pub fn event_summary(&self) -> (usize, usize, usize) {
        (
            self.events.event_count(),
            self.events.error_count(),
            self.events.warning_count(),
        )
    }

    /// Export all telemetry
    pub fn export(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            timestamp: current_timestamp(),
            metrics: self.metrics_snapshot(),
            profile_count: self.profiler.profile_count(),
            trace_count: self.traces.trace_count(),
            event_count: self.events.event_count(),
            error_count: self.events.error_count(),
        }
    }
}

impl Default for TelemetryAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Telemetry snapshot for export
#[derive(Clone, Debug)]
pub struct TelemetrySnapshot {
    pub timestamp: u64,
    pub metrics: HashMap<String, f64>,
    pub profile_count: usize,
    pub trace_count: usize,
    pub event_count: usize,
    pub error_count: usize,
}

/// Helper to get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_as_str() {
        assert_eq!(MetricType::Counter.as_str(), "counter");
        assert_eq!(MetricType::Timer.as_str(), "timer");
    }

    #[test]
    fn test_metric_data_point_creation() {
        let point = MetricDataPoint::new(42.5);
        assert_eq!(point.value, 42.5);
    }

    #[test]
    fn test_metric_data_point_with_label() {
        let point = MetricDataPoint::new(10.0)
            .with_label("env".to_string(), "prod".to_string());
        assert_eq!(point.labels.get("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_metric_creation() {
        let metric = Metric::new("requests".to_string(), MetricType::Counter)
            .with_description("Total requests".to_string());
        assert_eq!(metric.name, "requests");
    }

    #[test]
    fn test_metric_record() {
        let mut metric = Metric::new("temp".to_string(), MetricType::Gauge);
        metric.record(23.5);
        metric.record(24.0);
        assert_eq!(metric.point_count(), 2);
    }

    #[test]
    fn test_metric_latest_value() {
        let mut metric = Metric::new("temp".to_string(), MetricType::Gauge);
        metric.record(20.0);
        metric.record(25.0);
        assert_eq!(metric.latest_value(), Some(25.0));
    }

    #[test]
    fn test_metric_sum() {
        let mut metric = Metric::new("count".to_string(), MetricType::Counter);
        metric.record(10.0);
        metric.record(20.0);
        metric.record(30.0);
        assert_eq!(metric.sum(), 60.0);
    }

    #[test]
    fn test_metric_average() {
        let mut metric = Metric::new("value".to_string(), MetricType::Gauge);
        metric.record(10.0);
        metric.record(20.0);
        assert_eq!(metric.average(), Some(15.0));
    }

    #[test]
    fn test_metric_min_max() {
        let mut metric = Metric::new("value".to_string(), MetricType::Gauge);
        metric.record(5.0);
        metric.record(10.0);
        metric.record(15.0);
        assert_eq!(metric.min(), Some(5.0));
        assert_eq!(metric.max(), Some(15.0));
    }

    #[test]
    fn test_metrics_collector_register() {
        let mut collector = MetricsCollector::new();
        let metric = Metric::new("test".to_string(), MetricType::Counter);
        assert!(collector.register_metric(metric).is_ok());
        assert_eq!(collector.metric_count(), 1);
    }

    #[test]
    fn test_metrics_collector_record() {
        let mut collector = MetricsCollector::new();
        let metric = Metric::new("test".to_string(), MetricType::Counter);
        collector.register_metric(metric).unwrap();
        assert!(collector.record("test", 100.0).is_ok());
    }

    #[test]
    fn test_metrics_collector_unregister() {
        let mut collector = MetricsCollector::new();
        let metric = Metric::new("test".to_string(), MetricType::Counter);
        collector.register_metric(metric).unwrap();
        assert!(collector.unregister_metric("test").is_ok());
        assert_eq!(collector.metric_count(), 0);
    }

    #[test]
    fn test_trace_span_creation() {
        let span = TraceSpan::new(
            "span1".to_string(),
            "trace1".to_string(),
            "operation".to_string(),
        );
        assert_eq!(span.span_id, "span1");
        assert!(!span.is_finished());
    }

    #[test]
    fn test_trace_span_with_parent() {
        let span = TraceSpan::new(
            "span2".to_string(),
            "trace1".to_string(),
            "operation".to_string(),
        ).with_parent("span1".to_string());
        assert_eq!(span.parent_span_id, Some("span1".to_string()));
    }

    #[test]
    fn test_trace_span_add_tag() {
        let span = TraceSpan::new(
            "span1".to_string(),
            "trace1".to_string(),
            "operation".to_string(),
        ).add_tag("status".to_string(), "success".to_string());
        assert_eq!(span.tags.get("status"), Some(&"success".to_string()));
    }

    #[test]
    fn test_trace_span_end() {
        let span = TraceSpan::new(
            "span1".to_string(),
            "trace1".to_string(),
            "operation".to_string(),
        ).end();
        assert!(span.is_finished());
        assert!(span.duration_ms().is_some());
    }

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new("startup".to_string(), "Service started".to_string());
        assert_eq!(event.name, "startup");
    }

    #[test]
    fn test_trace_collector_record_span() {
        let mut collector = TraceCollector::new();
        let span = TraceSpan::new(
            "span1".to_string(),
            "trace1".to_string(),
            "operation".to_string(),
        );
        assert!(collector.record_span(span).is_ok());
        assert_eq!(collector.span_count(), 1);
    }

    #[test]
    fn test_trace_collector_get_trace() {
        let mut collector = TraceCollector::new();
        let span1 = TraceSpan::new("span1".to_string(), "trace1".to_string(), "op1".to_string());
        let span2 = TraceSpan::new("span2".to_string(), "trace1".to_string(), "op2".to_string());
        collector.record_span(span1).unwrap();
        collector.record_span(span2).unwrap();

        let trace = collector.get_trace("trace1");
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn test_performance_profile_creation() {
        let profile = PerformanceProfile::new("test_func".to_string());
        assert_eq!(profile.function_name, "test_func");
        assert_eq!(profile.total_calls, 0);
    }

    #[test]
    fn test_performance_profile_record_call() {
        let mut profile = PerformanceProfile::new("func".to_string());
        profile = profile.record_call(10);
        profile = profile.record_call(20);
        assert_eq!(profile.total_calls, 2);
        assert_eq!(profile.min_time_ms, 10);
        assert_eq!(profile.max_time_ms, 20);
    }

    #[test]
    fn test_profiler_start_end() {
        let mut profiler = Profiler::new();
        let start = profiler.start_function("func".to_string());
        assert!(profiler.end_function("func", start).is_ok());
        assert_eq!(profiler.profile_count(), 1);
    }

    #[test]
    fn test_profiler_get_slowest() {
        let mut profiler = Profiler::new();
        let start1 = profiler.start_function("fast".to_string());
        let start2 = profiler.start_function("slow".to_string());
        profiler.end_function("fast", start1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        profiler.end_function("slow", start2).unwrap();

        let slowest = profiler.get_slowest();
        assert!(slowest.is_some());
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::Info.as_str(), "info");
        assert_eq!(EventType::Error.as_str(), "error");
    }

    #[test]
    fn test_log_event_creation() {
        let event = LogEvent::new(
            EventType::Info,
            "system".to_string(),
            "Started".to_string(),
        );
        assert_eq!(event.event_type, EventType::Info);
    }

    #[test]
    fn test_log_event_with_context() {
        let event = LogEvent::new(
            EventType::Error,
            "db".to_string(),
            "Connection failed".to_string(),
        ).with_context("code".to_string(), "500".to_string());
        assert_eq!(event.context.get("code"), Some(&"500".to_string()));
    }

    #[test]
    fn test_event_stream_add_event() {
        let mut stream = EventStream::new();
        let event = LogEvent::new(EventType::Info, "app".to_string(), "Started".to_string());
        stream.add_event(event);
        assert_eq!(stream.event_count(), 1);
    }

    #[test]
    fn test_event_stream_get_by_type() {
        let mut stream = EventStream::new();
        stream.add_event(LogEvent::new(EventType::Info, "app".to_string(), "msg1".to_string()));
        stream.add_event(LogEvent::new(EventType::Error, "app".to_string(), "msg2".to_string()));
        
        let errors = stream.get_events_by_type(&EventType::Error);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_event_stream_get_by_source() {
        let mut stream = EventStream::new();
        stream.add_event(LogEvent::new(EventType::Info, "app".to_string(), "msg1".to_string()));
        stream.add_event(LogEvent::new(EventType::Info, "db".to_string(), "msg2".to_string()));
        
        let app_events = stream.get_events_by_source("app");
        assert_eq!(app_events.len(), 1);
    }

    #[test]
    fn test_telemetry_aggregator_creation() {
        let agg = TelemetryAggregator::new();
        assert_eq!(agg.metrics.metric_count(), 0);
        assert_eq!(agg.traces.trace_count(), 0);
    }

    #[test]
    fn test_telemetry_aggregator_export() {
        let agg = TelemetryAggregator::new();
        let snapshot = agg.export();
        assert_eq!(snapshot.event_count, 0);
        assert_eq!(snapshot.error_count, 0);
    }
}
