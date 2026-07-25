// ================================================================
// DISTRIBUTED TRACING - Phase 27.5
// Spans, instrumentation, and context propagation
// ================================================================

use std::collections::HashMap;

/// Trace span for distributed tracing
#[derive(Clone, Debug)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ms: u64,
    pub status: String,
    pub tags: HashMap<String, String>,
    pub logs: Vec<(u64, String)>,
    pub service_name: String,
}

/// Trace context
#[derive(Clone, Debug)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub baggage: HashMap<String, String>,
}

/// Trace sample decision
#[derive(Clone)]
pub enum SamplingDecision {
    Sample,
    DoNotSample,
    Defer,
}

pub struct DistributedTracingSolver;

impl DistributedTracingSolver {
    // ================================================================
    // TRACE INITIALIZATION (1-12)
    // ================================================================

    /// Problem 1: Generate trace ID
    pub fn generate_trace_id() -> String {
        format!("trace_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0))
    }

    /// Problem 2: Generate span ID
    pub fn generate_span_id() -> String {
        format!("span_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0))
    }

    /// Problem 3: Create trace context
    pub fn create_trace_context(
        trace_id: &str,
        span_id: &str,
    ) -> TraceContext {
        TraceContext {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            baggage: HashMap::new(),
        }
    }

    /// Problem 4: Create child context
    pub fn create_child_context(
        parent: &TraceContext,
    ) -> TraceContext {
        TraceContext {
            trace_id: parent.trace_id.clone(),
            span_id: Self::generate_span_id(),
            parent_span_id: Some(parent.span_id.clone()),
            baggage: parent.baggage.clone(),
        }
    }

    /// Problem 5: Get trace ID from context
    pub fn get_trace_id(context: &TraceContext) -> String {
        context.trace_id.clone()
    }

    /// Problem 6: Get span ID from context
    pub fn get_span_id(context: &TraceContext) -> String {
        context.span_id.clone()
    }

    /// Problem 7: Get parent span ID
    pub fn get_parent_span_id(context: &TraceContext) -> Option<String> {
        context.parent_span_id.clone()
    }

    /// Problem 8: Set baggage item
    pub fn set_baggage_item(
        context: &mut TraceContext,
        key: &str,
        value: &str,
    ) {
        context.baggage.insert(key.to_string(), value.to_string());
    }

    /// Problem 9: Get baggage item
    pub fn get_baggage_item(context: &TraceContext, key: &str) -> Option<String> {
        context.baggage.get(key).cloned()
    }

    /// Problem 10: Serialize context for propagation
    pub fn serialize_context_for_propagation(context: &TraceContext) -> String {
        format!(
            "trace_id={},span_id={},parent_span_id={}",
            context.trace_id,
            context.span_id,
            context.parent_span_id.as_deref().unwrap_or("none")
        )
    }

    /// Problem 11: Deserialize context from propagation
    pub fn deserialize_context_from_propagation(serialized: &str) -> TraceContext {
        let parts: HashMap<&str, &str> = serialized
            .split(',')
            .filter_map(|part| {
                let kv: Vec<&str> = part.split('=').collect();
                if kv.len() == 2 {
                    Some((kv[0], kv[1]))
                } else {
                    None
                }
            })
            .collect();

        TraceContext {
            trace_id: parts.get("trace_id").map(|s| s.to_string()).unwrap_or_default(),
            span_id: parts.get("span_id").map(|s| s.to_string()).unwrap_or_default(),
            parent_span_id: parts
                .get("parent_span_id")
                .and_then(|s| if s != &"none" { Some(s.to_string()) } else { None }),
            baggage: HashMap::new(),
        }
    }

    /// Problem 12: Extract trace context from headers
    pub fn extract_trace_context_from_headers(
        headers: &HashMap<String, String>,
    ) -> Option<TraceContext> {
        let serialized = headers.get("X-Trace-Context")?;
        Some(Self::deserialize_context_from_propagation(serialized))
    }

    // ================================================================
    // SPAN CREATION AND MANAGEMENT (13-24)
    // ================================================================

    /// Problem 13: Start span
    pub fn start_span(
        context: &TraceContext,
        operation_name: &str,
        service_name: &str,
        now: u64,
    ) -> Span {
        Span {
            span_id: context.span_id.clone(),
            trace_id: context.trace_id.clone(),
            parent_span_id: context.parent_span_id.clone(),
            operation_name: operation_name.to_string(),
            start_time: now,
            end_time: 0,
            duration_ms: 0,
            status: "ACTIVE".to_string(),
            tags: HashMap::new(),
            logs: Vec::new(),
            service_name: service_name.to_string(),
        }
    }

    /// Problem 14: End span
    pub fn end_span(span: &mut Span, now: u64) {
        span.end_time = now;
        span.duration_ms = now - span.start_time;
        span.status = "FINISHED".to_string();
    }

    /// Problem 15: Set span tag
    pub fn set_span_tag(
        span: &mut Span,
        key: &str,
        value: &str,
    ) {
        span.tags.insert(key.to_string(), value.to_string());
    }

    /// Problem 16: Get span tag
    pub fn get_span_tag(span: &Span, key: &str) -> Option<String> {
        span.tags.get(key).cloned()
    }

    /// Problem 17: Add span log
    pub fn add_span_log(
        span: &mut Span,
        message: &str,
        timestamp: u64,
    ) {
        span.logs.push((timestamp, message.to_string()));
    }

    /// Problem 18: Record span error
    pub fn record_span_error(
        span: &mut Span,
        error_message: &str,
        timestamp: u64,
    ) {
        span.status = "ERROR".to_string();
        Self::set_span_tag(span, "error", "true");
        Self::set_span_tag(span, "error.message", error_message);
        Self::add_span_log(span, &format!("Error: {}", error_message), timestamp);
    }

    /// Problem 19: Set span kind
    pub fn set_span_kind(span: &mut Span, kind: &str) {
        Self::set_span_tag(span, "span.kind", kind);
    }

    /// Problem 20: Get span duration
    pub fn get_span_duration(span: &Span) -> u64 {
        span.duration_ms
    }

    /// Problem 21: Is span sampled
    pub fn is_span_sampled(span: &Span) -> bool {
        !span.trace_id.is_empty()
    }

    /// Problem 22: Get span status
    pub fn get_span_status(span: &Span) -> String {
        span.status.clone()
    }

    /// Problem 23: Set span status
    pub fn set_span_status(span: &mut Span, status: &str) {
        span.status = status.to_string();
    }

    /// Problem 24: Get span logs
    pub fn get_span_logs(span: &Span) -> Vec<(u64, String)> {
        span.logs.clone()
    }

    // ================================================================
    // INSTRUMENTATION (25-36)
    // ================================================================

    /// Problem 25: Instrument HTTP request
    pub fn instrument_http_request(
        span: &mut Span,
        method: &str,
        url: &str,
        status_code: u16,
    ) {
        Self::set_span_kind(span, "CLIENT");
        Self::set_span_tag(span, "http.method", method);
        Self::set_span_tag(span, "http.url", url);
        Self::set_span_tag(span, "http.status_code", &status_code.to_string());
    }

    /// Problem 26: Instrument database query
    pub fn instrument_database_query(
        span: &mut Span,
        db_type: &str,
        query: &str,
        rows_affected: u32,
    ) {
        Self::set_span_kind(span, "CLIENT");
        Self::set_span_tag(span, "db.type", db_type);
        Self::set_span_tag(span, "db.statement", query);
        Self::set_span_tag(span, "db.rows", &rows_affected.to_string());
    }

    /// Problem 27: Instrument RPC call
    pub fn instrument_rpc_call(
        span: &mut Span,
        service: &str,
        method: &str,
        status: &str,
    ) {
        Self::set_span_kind(span, "CLIENT");
        Self::set_span_tag(span, "rpc.service", service);
        Self::set_span_tag(span, "rpc.method", method);
        Self::set_span_tag(span, "rpc.status", status);
    }

    /// Problem 28: Instrument message queue
    pub fn instrument_message_queue(
        span: &mut Span,
        queue_name: &str,
        operation: &str,
        message_count: u32,
    ) {
        Self::set_span_tag(span, "messaging.system", "kafka");
        Self::set_span_tag(span, "messaging.destination", queue_name);
        Self::set_span_tag(span, "messaging.operation", operation);
        Self::set_span_tag(span, "messaging.message_count", &message_count.to_string());
    }

    /// Problem 29: Instrument cache operation
    pub fn instrument_cache_operation(
        span: &mut Span,
        cache_name: &str,
        operation: &str,
        hit: bool,
    ) {
        Self::set_span_tag(span, "cache.name", cache_name);
        Self::set_span_tag(span, "cache.operation", operation);
        Self::set_span_tag(span, "cache.hit", if hit { "true" } else { "false" });
    }

    /// Problem 30: Instrument service call
    pub fn instrument_service_call(
        span: &mut Span,
        service_name: &str,
        endpoint: &str,
    ) {
        span.service_name = service_name.to_string();
        Self::set_span_tag(span, "service.name", service_name);
        Self::set_span_tag(span, "service.endpoint", endpoint);
    }

    /// Problem 31: Add custom instrumentation
    pub fn add_custom_instrumentation(
        span: &mut Span,
        category: &str,
        name: &str,
        value: &str,
    ) {
        let key = format!("{}.{}", category, name);
        Self::set_span_tag(span, &key, value);
    }

    /// Problem 32: Get span instrumentation data
    pub fn get_span_instrumentation_data(span: &Span) -> HashMap<String, String> {
        span.tags.clone()
    }

    /// Problem 33: Record thread info
    pub fn record_thread_info(span: &mut Span) {
        Self::set_span_tag(span, "thread_name", "main");
        Self::set_span_tag(span, "thread_id", "1");
    }

    /// Problem 34: Record memory usage
    pub fn record_memory_usage(span: &mut Span, memory_mb: u32) {
        Self::set_span_tag(span, "process.memory_mb", &memory_mb.to_string());
    }

    /// Problem 35: Record CPU usage
    pub fn record_cpu_usage(span: &mut Span, cpu_percent: u8) {
        Self::set_span_tag(span, "process.cpu_percent", &cpu_percent.to_string());
    }

    /// Problem 36: Record resource metrics
    pub fn record_resource_metrics(
        span: &mut Span,
        memory_mb: u32,
        cpu_percent: u8,
    ) {
        Self::record_memory_usage(span, memory_mb);
        Self::record_cpu_usage(span, cpu_percent);
    }

    // ================================================================
    // CONTEXT PROPAGATION (37-45)
    // ================================================================

    /// Problem 37: Inject context into headers
    pub fn inject_context_into_headers(
        context: &TraceContext,
    ) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "X-Trace-Context".to_string(),
            Self::serialize_context_for_propagation(context),
        );
        headers.insert("X-Trace-ID".to_string(), context.trace_id.clone());
        headers.insert("X-Span-ID".to_string(), context.span_id.clone());
        headers
    }

    /// Problem 38: Extract context from headers
    pub fn extract_context_from_headers(
        headers: &HashMap<String, String>,
    ) -> TraceContext {
        if let Some(context) = Self::extract_trace_context_from_headers(headers) {
            context
        } else {
            TraceContext {
                trace_id: Self::generate_trace_id(),
                span_id: Self::generate_span_id(),
                parent_span_id: None,
                baggage: HashMap::new(),
            }
        }
    }

    /// Problem 39: Propagate context to worker thread
    pub fn propagate_context_to_worker_thread(
        context: &TraceContext,
    ) -> TraceContext {
        Self::create_child_context(context)
    }

    /// Problem 40: Propagate context across services
    pub fn propagate_context_across_services(
        context: &TraceContext,
    ) -> String {
        Self::serialize_context_for_propagation(context)
    }

    /// Problem 41: B3 header format
    pub fn b3_header_format(context: &TraceContext) -> String {
        format!(
            "{}-{}-1-{}",
            context.trace_id,
            context.span_id,
            context.parent_span_id.as_deref().unwrap_or("0")
        )
    }

    /// Problem 42: Jaeger header format
    pub fn jaeger_header_format(context: &TraceContext) -> String {
        format!(
            "{}:{}:0:1",
            context.trace_id,
            context.span_id,
        )
    }

    /// Problem 43: Parse B3 header
    pub fn parse_b3_header(header_value: &str) -> TraceContext {
        let parts: Vec<&str> = header_value.split('-').collect();
        TraceContext {
            trace_id: parts.get(0).map(|s| s.to_string()).unwrap_or_default(),
            span_id: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
            parent_span_id: parts.get(3).and_then(|s| if s != &"0" { Some(s.to_string()) } else { None }),
            baggage: HashMap::new(),
        }
    }

    /// Problem 44: Parse Jaeger header
    pub fn parse_jaeger_header(header_value: &str) -> TraceContext {
        let parts: Vec<&str> = header_value.split(':').collect();
        TraceContext {
            trace_id: parts.get(0).map(|s| s.to_string()).unwrap_or_default(),
            span_id: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
            parent_span_id: None,
            baggage: HashMap::new(),
        }
    }

    /// Problem 45: Link traces across systems
    pub fn link_traces_across_systems(
        local_trace_id: &str,
        external_trace_id: &str,
    ) -> String {
        format!("link_{}_to_{}", local_trace_id, external_trace_id)
    }

    // ================================================================
    // SAMPLING AND FILTERING (46-50)
    // ================================================================

    /// Problem 46: Apply sampling decision
    pub fn apply_sampling_decision(
        trace_id: &str,
        sampling_rate: f64,
    ) -> bool {
        let mut hash: u64 = 0;
        for byte in trace_id.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        (hash as f64 / u64::MAX as f64) < sampling_rate
    }

    /// Problem 47: Filter sensitive data
    pub fn filter_sensitive_data(span: &mut Span) {
        let sensitive_keys = vec!["password", "api_key", "token", "secret"];
        for key in sensitive_keys {
            if let Some(_) = span.tags.get(key) {
                span.tags.insert(key.to_string(), "***REDACTED***".to_string());
            }
        }
    }

    /// Problem 48: Sample trace by operation
    pub fn sample_trace_by_operation(
        operation_name: &str,
        operation_samples: &HashMap<String, f64>,
    ) -> bool {
        operation_samples
            .get(operation_name)
            .map(|rate| Self::apply_sampling_decision(operation_name, *rate))
            .unwrap_or(true)
    }

    /// Problem 49: Tail-based sampling
    pub fn tail_based_sampling(
        spans: &[Span],
        error_threshold_percent: f64,
    ) -> bool {
        let error_count = spans.iter().filter(|s| s.status == "ERROR").count();
        let error_rate = (error_count as f64 / spans.len() as f64) * 100.0;
        error_rate > error_threshold_percent
    }

    /// Problem 50: Rate limit tracing
    pub fn rate_limit_tracing(
        spans_per_second: f64,
        elapsed_seconds: u64,
    ) -> u32 {
        ((spans_per_second * elapsed_seconds as f64) as u32).min(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trace_id() {
        let trace_id = DistributedTracingSolver::generate_trace_id();
        assert!(!trace_id.is_empty());
    }

    #[test]
    fn test_create_trace_context() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        assert_eq!(ctx.trace_id, "t1");
        assert_eq!(ctx.span_id, "s1");
    }

    #[test]
    fn test_create_child_context() {
        let parent = DistributedTracingSolver::create_trace_context("t1", "s1");
        let child = DistributedTracingSolver::create_child_context(&parent);
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
    }

    #[test]
    fn test_baggage() {
        let mut ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        DistributedTracingSolver::set_baggage_item(&mut ctx, "user_id", "123");
        let value = DistributedTracingSolver::get_baggage_item(&ctx, "user_id");
        assert_eq!(value, Some("123".to_string()));
    }

    #[test]
    fn test_span_lifecycle() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let mut span = DistributedTracingSolver::start_span(&ctx, "operation", "service", 1000);
        assert_eq!(span.status, "ACTIVE");
        DistributedTracingSolver::end_span(&mut span, 2000);
        assert_eq!(span.status, "FINISHED");
        assert_eq!(span.duration_ms, 1000);
    }

    #[test]
    fn test_span_tags() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let mut span = DistributedTracingSolver::start_span(&ctx, "operation", "service", 1000);
        DistributedTracingSolver::set_span_tag(&mut span, "key1", "value1");
        let value = DistributedTracingSolver::get_span_tag(&span, "key1");
        assert_eq!(value, Some("value1".to_string()));
    }

    #[test]
    fn test_span_error() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let mut span = DistributedTracingSolver::start_span(&ctx, "operation", "service", 1000);
        DistributedTracingSolver::record_span_error(&mut span, "Connection failed", 1500);
        assert_eq!(span.status, "ERROR");
    }

    #[test]
    fn test_inject_headers() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let headers = DistributedTracingSolver::inject_context_into_headers(&ctx);
        assert!(headers.contains_key("X-Trace-ID"));
        assert!(headers.contains_key("X-Span-ID"));
    }

    #[test]
    fn test_b3_format() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let b3 = DistributedTracingSolver::b3_header_format(&ctx);
        assert!(b3.contains("t1"));
        assert!(b3.contains("s1"));
    }

    #[test]
    fn test_instrumentation() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let mut span = DistributedTracingSolver::start_span(&ctx, "http_request", "service", 1000);
        DistributedTracingSolver::instrument_http_request(&mut span, "GET", "http://example.com", 200);
        assert_eq!(DistributedTracingSolver::get_span_tag(&span, "http.method"), Some("GET".to_string()));
    }

    #[test]
    fn test_sampling() {
        let sampled = DistributedTracingSolver::apply_sampling_decision("trace1", 1.0);
        assert!(sampled);

        let not_sampled = DistributedTracingSolver::apply_sampling_decision("trace1", 0.0);
        assert!(!not_sampled);
    }

    #[test]
    fn test_filter_sensitive_data() {
        let ctx = DistributedTracingSolver::create_trace_context("t1", "s1");
        let mut span = DistributedTracingSolver::start_span(&ctx, "op", "svc", 1000);
        DistributedTracingSolver::set_span_tag(&mut span, "password", "secret123");
        DistributedTracingSolver::filter_sensitive_data(&mut span);
        assert_eq!(DistributedTracingSolver::get_span_tag(&span, "password"), Some("***REDACTED***".to_string()));
    }
}
