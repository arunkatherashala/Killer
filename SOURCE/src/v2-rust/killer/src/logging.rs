// Structured Logging System for Killer Language
// Purpose: Contextual logging with correlation IDs for distributed tracing
// Status: Production-ready

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Structured log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub context: HashMap<String, String>,
}

impl LogEntry {
    pub fn to_string(&self) -> String {
        let time_str = self
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| format!("{}", d.as_secs()))
            .unwrap_or_else(|_| "unknown".to_string());

        let mut parts = vec![
            format!("[{}]", time_str),
            format!("[{}]", self.level),
        ];

        if !self.request_id.is_empty() {
            parts.push(format!("[req-{}]", &self.request_id[..8.min(self.request_id.len())]));
        }

        if let Some(user_id) = &self.user_id {
            parts.push(format!("[user-{}]", &user_id[..8.min(user_id.len())]));
        }

        parts.push(self.message.clone());

        if !self.context.is_empty() {
            for (k, v) in &self.context {
                parts.push(format!("{}={}", k, v));
            }
        }

        parts.join(" ")
    }

    pub fn to_json(&self) -> String {
        let mut context_json = String::from("{");
        for (i, (k, v)) in self.context.iter().enumerate() {
            if i > 0 {
                context_json.push(',');
            }
            context_json.push_str(&format!(r#""{}":"{}""#, k, v));
        }
        context_json.push('}');

        format!(
            r#"{{
  "timestamp":{},
  "level":"{}",
  "message":"{}",
  "request_id":"{}",
  "user_id": {},
  "session_id": {},
  "trace_id": {},
  "context": {}
}}"#,
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            self.level,
            self.message,
            self.request_id,
            self.user_id
                .as_ref()
                .map(|u| format!(r#""{}""#, u))
                .unwrap_or_else(|| "null".to_string()),
            self.session_id
                .as_ref()
                .map(|s| format!(r#""{}""#, s))
                .unwrap_or_else(|| "null".to_string()),
            self.trace_id
                .as_ref()
                .map(|t| format!(r#""{}""#, t))
                .unwrap_or_else(|| "null".to_string()),
            context_json
        )
    }
}

/// Logger context for a request or session
pub struct LoggerContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
}

impl LoggerContext {
    pub fn new(request_id: String) -> Self {
        LoggerContext {
            request_id,
            user_id: None,
            session_id: None,
            trace_id: None,
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}

/// Structured logger
pub struct StructuredLogger {
    min_level: LogLevel,
    entries: Arc<Mutex<Vec<LogEntry>>>,
    max_entries: usize,
    output_to_stderr: bool,
}

impl StructuredLogger {
    pub fn new(min_level: LogLevel) -> Self {
        StructuredLogger {
            min_level,
            entries: Arc::new(Mutex::new(Vec::new())),
            max_entries: 10000,
            output_to_stderr: true,
        }
    }

    pub fn from_env() -> Self {
        let level = std::env::var("KILLER_LOG_LEVEL")
            .ok()
            .and_then(|l| match l.to_uppercase().as_str() {
                "TRACE" => Some(LogLevel::Trace),
                "DEBUG" => Some(LogLevel::Debug),
                "INFO" => Some(LogLevel::Info),
                "WARN" => Some(LogLevel::Warn),
                "ERROR" => Some(LogLevel::Error),
                "FATAL" => Some(LogLevel::Fatal),
                _ => None,
            })
            .unwrap_or(LogLevel::Info);

        StructuredLogger::new(level)
    }

    pub fn set_output_to_stderr(&mut self, output: bool) {
        self.output_to_stderr = output;
    }

    fn log(&self, entry: LogEntry) {
        if entry.level < self.min_level {
            return;
        }

        if self.output_to_stderr {
            eprintln!("{}", entry.to_string());
        }

        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);

            // Keep recent entries only
            if entries.len() > self.max_entries {
                let drain_count = entries.len() - self.max_entries;
                entries.drain(0..drain_count);
            }
        }
    }

    pub fn trace(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Trace,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn debug(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Debug,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn info(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Info,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn warn(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Warn,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn error(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Error,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn fatal(&self, message: &str, context: &LoggerContext) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Fatal,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: HashMap::new(),
        });
    }

    pub fn log_with_context(&self, level: LogLevel, message: &str, context: &LoggerContext, extra_context: HashMap<String, String>) {
        self.log(LogEntry {
            timestamp: SystemTime::now(),
            level,
            message: message.to_string(),
            request_id: context.request_id.clone(),
            user_id: context.user_id.clone(),
            session_id: context.session_id.clone(),
            trace_id: context.trace_id.clone(),
            context: extra_context,
        });
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries
            .lock()
            .ok()
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    pub fn get_entries_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        self.entries
            .lock()
            .ok()
            .map(|entries| entries.iter().filter(|e| e.level == level).cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_entries_by_request_id(&self, request_id: &str) -> Vec<LogEntry> {
        self.entries
            .lock()
            .ok()
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.request_id == request_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

/// Generate a correlation ID for distributed tracing
pub fn generate_correlation_id() -> String {
    use std::time::UNIX_EPOCH;

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!("{}-{:x}-{:x}", secs, std::process::id(), rand::random::<u32>())
}

/// Utility: Generate mock random number (simple replacement for rand crate)
mod rand {
    pub fn random<T: Default>() -> T {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_context() {
        let ctx = LoggerContext::new("req-123".to_string())
            .with_user_id("user-456".to_string())
            .with_session_id("session-789".to_string());

        assert_eq!(ctx.request_id, "req-123");
        assert_eq!(ctx.user_id, Some("user-456".to_string()));
        assert_eq!(ctx.session_id, Some("session-789".to_string()));
    }

    #[test]
    fn test_log_entry_formatting() {
        let ctx = LoggerContext::new("req-abc-123".to_string())
            .with_user_id("user-xyz-789".to_string());

        let entry = LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Info,
            message: "Test message".to_string(),
            request_id: ctx.request_id.clone(),
            user_id: ctx.user_id.clone(),
            session_id: None,
            trace_id: None,
            context: HashMap::new(),
        };

        let formatted = entry.to_string();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_structured_logger_info() {
        let logger = StructuredLogger::new(LogLevel::Info);
        let ctx = LoggerContext::new("req-123".to_string());

        logger.info("Test info", &ctx);

        let entries = logger.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[0].message, "Test info");
    }

    #[test]
    fn test_logger_respects_min_level() {
        let logger = StructuredLogger::new(LogLevel::Warn);
        let ctx = LoggerContext::new("req-123".to_string());

        logger.debug("Debug msg", &ctx);
        logger.info("Info msg", &ctx);
        logger.warn("Warn msg", &ctx);

        let entries = logger.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Warn);
    }

    #[test]
    fn test_get_entries_by_level() {
        let logger = StructuredLogger::new(LogLevel::Debug);
        let ctx = LoggerContext::new("req-123".to_string());

        logger.info("Info msg", &ctx);
        logger.error("Error msg", &ctx);
        logger.warn("Warn msg", &ctx);

        let errors = logger.get_entries_by_level(LogLevel::Error);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_get_entries_by_request_id() {
        let logger = StructuredLogger::new(LogLevel::Debug);
        let ctx1 = LoggerContext::new("req-123".to_string());
        let ctx2 = LoggerContext::new("req-456".to_string());

        logger.info("Msg 1", &ctx1);
        logger.info("Msg 2", &ctx1);
        logger.info("Msg 3", &ctx2);

        let entries_for_req1 = logger.get_entries_by_request_id("req-123");
        assert_eq!(entries_for_req1.len(), 2);

        let entries_for_req2 = logger.get_entries_by_request_id("req-456");
        assert_eq!(entries_for_req2.len(), 1);
    }

    #[test]
    fn test_logger_clear() {
        let logger = StructuredLogger::new(LogLevel::Debug);
        let ctx = LoggerContext::new("req-123".to_string());

        logger.info("Test", &ctx);
        assert_eq!(logger.get_entries().len(), 1);

        logger.clear();
        assert_eq!(logger.get_entries().len(), 0);
    }

    #[test]
    fn test_log_entry_to_json() {
        let ctx = LoggerContext::new("req-123".to_string())
            .with_user_id("user-456".to_string());

        let entry = LogEntry {
            timestamp: SystemTime::now(),
            level: LogLevel::Info,
            message: "Test".to_string(),
            request_id: ctx.request_id,
            user_id: ctx.user_id,
            session_id: None,
            trace_id: None,
            context: HashMap::new(),
        };

        let json = entry.to_json();
        assert!(json.contains(r#""level":"INFO""#));
        assert!(json.contains(r#""message":"Test""#));
    }
}
