//! Distributed Tracing — Trace requests across services

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
}

impl TraceContext {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self {
            trace_id: format!("{:x}", now),
            span_id: format!("{:x}", now.wrapping_add(1)),
        }
    }
}
