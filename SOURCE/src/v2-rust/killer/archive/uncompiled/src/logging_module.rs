// Logging Module for Killer Language
// Structured logging with levels, filtering, and formatting
// Version: 2.1.0

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::SystemTime;

/// Log levels in order of severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub source: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String, source: Option<String>) -> Self {
        LogEntry {
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            level,
            message,
            source,
        }
    }
    
    pub fn format(&self) -> String {
        match &self.source {
            Some(src) => format!("[{}] {} | {}: {}", 
                self.timestamp, 
                self.level.as_str(), 
                src, 
                self.message),
            None => format!("[{}] {} | {}", 
                self.timestamp, 
                self.level.as_str(), 
                self.message),
        }
    }
    
    pub fn format_json(&self) -> String {
        format!(
            r#"{{"timestamp":{},"level":"{}","message":"{}","source":"{}"}}"#,
            self.timestamp,
            self.level.as_str(),
            self.message.replace("\"", "\\\""),
            self.source.as_ref().unwrap_or(&String::new())
        )
    }
}

/// Global logger with thread-safe access
pub struct Logger {
    min_level: LogLevel,
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
    current_source: Option<String>,
}

impl Logger {
    /// Create new logger with minimum level
    /// Logger::new(LogLevel::Debug) => logger that shows debug and above
    pub fn new(min_level: LogLevel) -> Self {
        Logger {
            min_level,
            entries: Arc::new(Mutex::new(VecDeque::new())),
            max_entries: 10000,
            current_source: None,
        }
    }
    
    /// Create logger with custom capacity
    pub fn with_capacity(min_level: LogLevel, capacity: usize) -> Self {
        Logger {
            min_level,
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            max_entries: capacity,
            current_source: None,
        }
    }
    
    /// Set minimum log level (below this level are filtered out)
    pub fn set_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }
    
    /// Set current source (context) for logs
    pub fn set_source(&mut self, source: &str) {
        self.current_source = Some(source.to_string());
    }
    
    /// Clear current source
    pub fn clear_source(&mut self) {
        self.current_source = None;
    }
    
    /// Log trace message
    /// logger.trace("Starting initialization") => logs with TRACE level
    pub fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
    
    /// Log debug message
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    /// Log info message
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    /// Log warning message
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }
    
    /// Log error message
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
    
    /// Internal logging function (respects min_level)
    fn log(&self, level: LogLevel, message: &str) {
        if level < self.min_level {
            return;
        }
        
        let entry = LogEntry::new(level, message.to_string(), self.current_source.clone());
        
        if let Ok(mut entries) = self.entries.lock() {
            entries.push_back(entry);
            
            // Maintain max_entries limit
            while entries.len() > self.max_entries {
                entries.pop_front();
            }
        }
    }
    
    /// Get formatted log messages matching level
    /// logger.logs(LogLevel::Error) => all error messages
    pub fn logs(&self, level: LogLevel) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|e| e.level == level)
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all formatted log messages
    pub fn all_logs(&self) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get logs since certain level (inclusive)
    /// logger.logs_since(LogLevel::Warn) => WARN and ERROR messages
    pub fn logs_since(&self, level: LogLevel) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|e| e.level >= level)
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all logs as JSON array
    pub fn all_logs_json(&self) -> String {
        if let Ok(entries) = self.entries.lock() {
            let json_entries: Vec<String> = entries.iter()
                .map(|e| e.format_json())
                .collect();
            format!("[{}]", json_entries.join(","))
        } else {
            "[]".to_string()
        }
    }
    
    /// Get number of entries
    pub fn count(&self) -> usize {
        self.entries.as_ref().lock().map(|e| e.len()).unwrap_or(0)
    }
    
    /// Get entries by level
    pub fn count_by_level(&self, level: LogLevel) -> usize {
        if let Ok(entries) = self.entries.lock() {
            entries.iter().filter(|e| e.level == level).count()
        } else {
            0
        }
    }
    
    /// Clear all log entries
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
    
    /// Get first N entries
    pub fn first(&self, n: usize) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .take(n)
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get last N entries
    pub fn last(&self, n: usize) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .rev()
                .take(n)
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Search logs by message content
    pub fn search(&self, query: &str) -> Vec<String> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|e| e.message.contains(query))
                .map(|e| e.format())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Filter logs with custom predicate
    pub fn filter_logs(&self, level: LogLevel) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|e| e.level == level)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Static logging module with common functions
pub struct LoggingModule;

impl LoggingModule {
    /// Parse log level from string
    pub fn parse_level(s: &str) -> Option<LogLevel> {
        LogLevel::from_str(s)
    }
    
    /// Format timestamp as readable string
    pub fn format_timestamp(ts: u64) -> String {
        format!("{}", ts)
    }
    
    /// Create summary statistics
    pub fn summary(logger: &Logger) -> String {
        let trace = logger.count_by_level(LogLevel::Trace);
        let debug = logger.count_by_level(LogLevel::Debug);
        let info = logger.count_by_level(LogLevel::Info);
        let warn = logger.count_by_level(LogLevel::Warn);
        let error = logger.count_by_level(LogLevel::Error);
        
        format!(
            "Trace: {}, Debug: {}, Info: {}, Warn: {}, Error: {}",
            trace, debug, info, warn, error
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
    }
    
    #[test]
    fn test_logger_creation() {
        let logger = Logger::new(LogLevel::Info);
        assert_eq!(logger.count(), 0);
    }
    
    #[test]
    fn test_logging_basic() {
        let logger = Logger::new(LogLevel::Debug);
        logger.debug("Test message");
        assert_eq!(logger.count(), 1);
        assert_eq!(logger.count_by_level(LogLevel::Debug), 1);
    }
    
    #[test]
    fn test_log_filtering() {
        let logger = Logger::new(LogLevel::Info);
        logger.debug("Debug msg");
        logger.info("Info msg");
        logger.error("Error msg");
        
        assert_eq!(logger.count(), 2); // Debug filtered out
        assert_eq!(logger.count_by_level(LogLevel::Error), 1);
    }
    
    #[test]
    fn test_log_source() {
        let logger = Logger::new(LogLevel::Debug);
        let msg = logger.logs(LogLevel::Debug);
        assert_eq!(msg.len(), 0);
    }
    
    #[test]
    fn test_logs_since() {
        let logger = Logger::new(LogLevel::Trace);
        logger.trace("Trace");
        logger.debug("Debug");
        logger.warn("Warn");
        logger.error("Error");
        
        let warn_and_above = logger.logs_since(LogLevel::Warn);
        assert_eq!(warn_and_above.len(), 2);
    }
    
    #[test]
    fn test_search() {
        let logger = Logger::new(LogLevel::Debug);
        logger.debug("Database connection opened");
        logger.debug("Cache initialized");
        logger.debug("Database query executed");
        
        let db_logs = logger.search("Database");
        assert_eq!(db_logs.len(), 2);
    }
    
    #[test]
    fn test_summary() {
        let logger = Logger::new(LogLevel::Trace);
        logger.trace("T");
        logger.debug("D");
        logger.info("I");
        logger.warn("W");
        logger.error("E");
        
        let summary = LoggingModule::summary(&logger);
        assert!(summary.contains("Trace: 1"));
        assert!(summary.contains("Error: 1"));
    }
}
