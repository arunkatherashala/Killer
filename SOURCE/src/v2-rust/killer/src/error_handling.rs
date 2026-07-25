/// Killer Language Error Handling System
/// Comprehensive error types, propagation, and recovery
/// 
/// Features:
/// - Rich error types (Parse, Runtime, Type, IO, etc.)
/// - Error context (file, line, column, message)
/// - Error recovery and suggestions
/// - Chainable error handling
/// - Pretty printing with colors
///
/// Usage:
/// ```ignore
/// fn parse_number(s: &str) -> Result<i64, KillerError> {
///     s.parse::<i64>()
///         .map_err(|_| KillerError::parse_error(
///             "Invalid number format",
///             "file.killer", 10, 5
///         ))
/// }
/// 
/// match parse_number("abc") {
///     Ok(n) => println!("Got: {}", n),
///     Err(e) => eprintln!("{}", e),  // Pretty printed with context
/// }
/// ```

use std::fmt;
use std::error::Error;
use std::io;

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Recoverable warning
    Warning,
    /// Non-fatal error
    Error,
    /// Compiler/runtime panic
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Error kinds for different failure modes
#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// Parse/syntax error
    ParseError {
        message: String,
        near_token: Option<String>,
    },
    /// Type checking error
    TypeError {
        expected: String,
        found: String,
        message: String,
    },
    /// Runtime error (division by zero, null deref, etc.)
    RuntimeError {
        message: String,
        operation: Option<String>,
    },
    /// File I/O error
    IoError {
        message: String,
        path: Option<String>,
        kind: io::ErrorKind,
    },
    /// Unresolved reference/name not found
    NameError {
        name: String,
        scope: String,
        suggestions: Vec<String>,
    },
    /// Function/method call mismatch
    CallError {
        name: String,
        expected_args: usize,
        found_args: usize,
        message: Option<String>,
    },
    /// Value out of range
    RangeError {
        value: String,
        min: Option<String>,
        max: Option<String>,
    },
    /// Assertion failure
    AssertionError {
        condition: String,
        message: Option<String>,
    },
    /// Timeout error
    TimeoutError {
        operation: String,
        duration_ms: u64,
    },
    /// Memory error
    MemoryError {
        message: String,
        requested_bytes: Option<usize>,
    },
    /// Import/module error
    ImportError {
        module: String,
        message: String,
        path: Option<String>,
    },
    /// Generic/unknown error
    Other {
        message: String,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::ParseError { message, near_token } => {
                write!(f, "{}", message)?;
                if let Some(token) = near_token {
                    write!(f, " near '{}'", token)?;
                }
                Ok(())
            }
            ErrorKind::TypeError { expected, found, message } => {
                write!(f, "type mismatch: {}\n  expected: {}\n  found: {}",
                       message, expected, found)
            }
            ErrorKind::RuntimeError { message, operation } => {
                write!(f, "{}", message)?;
                if let Some(op) = operation {
                    write!(f, " (in {})", op)?;
                }
                Ok(())
            }
            ErrorKind::IoError { message, path, .. } => {
                write!(f, "I/O error: {}", message)?;
                if let Some(p) = path {
                    write!(f, " (file: {})", p)?;
                }
                Ok(())
            }
            ErrorKind::NameError { name, scope, suggestions } => {
                write!(f, "undefined '{}' in {}", name, scope)?;
                if !suggestions.is_empty() {
                    write!(f, "\n  did you mean: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            ErrorKind::CallError { name, expected_args, found_args, message } => {
                write!(f, "call mismatch: expected {} args, found {}",
                       expected_args, found_args)?;
                if let Some(msg) = message {
                    write!(f, " ({})", msg)?;
                }
                write!(f, " in function '{}'", name)
            }
            ErrorKind::RangeError { value, min, max } => {
                write!(f, "value {} out of range", value)?;
                if let Some(m) = min {
                    write!(f, " (min: {})", m)?;
                }
                if let Some(m) = max {
                    write!(f, " (max: {})", m)?;
                }
                Ok(())
            }
            ErrorKind::AssertionError { condition, message } => {
                write!(f, "assertion failed: {}", condition)?;
                if let Some(msg) = message {
                    write!(f, " ({})", msg)?;
                }
                Ok(())
            }
            ErrorKind::TimeoutError { operation, duration_ms } => {
                write!(f, "timeout: {} exceeded {}ms", operation, duration_ms)
            }
            ErrorKind::MemoryError { message, requested_bytes } => {
                write!(f, "memory error: {}", message)?;
                if let Some(bytes) = requested_bytes {
                    write!(f, " (requested {} bytes)", bytes)?;
                }
                Ok(())
            }
            ErrorKind::ImportError { module, message, path } => {
                write!(f, "import error in '{}': {}", module, message)?;
                if let Some(p) = path {
                    write!(f, " (path: {})", p)?;
                }
                Ok(())
            }
            ErrorKind::Other { message } => write!(f, "{}", message),
        }
    }
}

/// Main error type with context
#[derive(Debug, Clone)]
pub struct KillerError {
    /// Error classification
    pub kind: ErrorKind,
    /// Severity (warning/error/critical)
    pub severity: ErrorSeverity,
    /// Source file
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column (1-indexed)
    pub column: usize,
    /// Full error context
    pub context: Option<String>,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Chain of errors (for error propagation)
    pub source_error: Option<Box<KillerError>>,
}

impl KillerError {
    /// Create parse error
    pub fn parse_error(message: impl Into<String>, file: impl Into<String>, line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::ParseError {
                message: message.into(),
                near_token: None,
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Create type error
    pub fn type_error(expected: impl Into<String>, found: impl Into<String>, 
                      message: impl Into<String>, file: impl Into<String>, 
                      line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::TypeError {
                expected: expected.into(),
                found: found.into(),
                message: message.into(),
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Create runtime error
    pub fn runtime_error(message: impl Into<String>, file: impl Into<String>, 
                        line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::RuntimeError {
                message: message.into(),
                operation: None,
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Create I/O error
    pub fn io_error(message: impl Into<String>, path: Option<String>, 
                    kind: io::ErrorKind, file: impl Into<String>, 
                    line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::IoError {
                message: message.into(),
                path,
                kind,
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Create name error with suggestions
    pub fn name_error(name: impl Into<String>, scope: impl Into<String>, 
                     suggestions: Vec<String>, file: impl Into<String>, 
                     line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::NameError {
                name: name.into(),
                scope: scope.into(),
                suggestions,
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Create call error
    pub fn call_error(name: impl Into<String>, expected_args: usize, 
                     found_args: usize, file: impl Into<String>, 
                     line: usize, col: usize) -> Self {
        KillerError {
            kind: ErrorKind::CallError {
                name: name.into(),
                expected_args,
                found_args,
                message: None,
            },
            severity: ErrorSeverity::Error,
            file: file.into(),
            line,
            column: col,
            context: None,
            suggestion: None,
            source_error: None,
        }
    }

    /// Set severity level
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add context (source line, etc.)
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add suggestion for fixing
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Chain error for multi-level error propagation
    pub fn with_source(mut self, source: KillerError) -> Self {
        self.source_error = Some(Box::new(source));
        self
    }

    /// Get error code (for scripting/error matching)
    pub fn code(&self) -> &'static str {
        match self.kind {
            ErrorKind::ParseError { .. } => "E001",
            ErrorKind::TypeError { .. } => "E002",
            ErrorKind::RuntimeError { .. } => "E003",
            ErrorKind::IoError { .. } => "E004",
            ErrorKind::NameError { .. } => "E005",
            ErrorKind::CallError { .. } => "E006",
            ErrorKind::RangeError { .. } => "E007",
            ErrorKind::AssertionError { .. } => "E008",
            ErrorKind::TimeoutError { .. } => "E009",
            ErrorKind::MemoryError { .. } => "E010",
            ErrorKind::ImportError { .. } => "E011",
            ErrorKind::Other { .. } => "E999",
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.severity != ErrorSeverity::Critical
    }

    /// Get full error chain
    pub fn chain(&self) -> Vec<&KillerError> {
        let mut chain = vec![self];
        let mut current = self;
        while let Some(source) = &current.source_error {
            chain.push(source);
            current = source;
        }
        chain
    }
}

impl fmt::Display for KillerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Pretty error format with colors (if terminal supports it)
        writeln!(f, "[{}] {} at {}:{}:{}",
                 self.code(),
                 self.severity,
                 self.file,
                 self.line,
                 self.column)?;
        
        // Main error message
        writeln!(f, "  {}", self.kind)?;
        
        // Context (source line)
        if let Some(ctx) = &self.context {
            writeln!(f, "  | {}", ctx)?;
            writeln!(f, "  | {}{}", " ".repeat(self.column.saturating_sub(1)), "^")?;
        }
        
        // Suggestion
        if let Some(sugg) = &self.suggestion {
            writeln!(f, "  → try: {}", sugg)?;
        }
        
        // Error chain
        if let Some(source) = &self.source_error {
            writeln!(f, "\ncaused by:")?;
            writeln!(f, "  {}", source)?;
        }
        
        Ok(())
    }
}

impl Error for KillerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source_error.as_ref().map(|e| e.as_ref() as &dyn Error)
    }
}

/// Result type for Killer operations
pub type Result<T> = std::result::Result<T, KillerError>;

/// Error recovery trait - implement on types that can validate themselves
pub trait ErrorRecovery {
    /// Try to recover from error, return true if successful
    fn recover_from(&mut self, error: &KillerError) -> bool;
    
    /// Validate state, return Ok if valid
    fn validate(&self) -> Result<()>;
}

/// Utility: Create suggestions based on string similarity
pub fn suggest_similar(invalid: &str, valid_options: &[&str]) -> Vec<String> {
    let mut suggestions: Vec<_> = valid_options
        .iter()
        .map(|opt| {
            let dist = levenshtein_distance(invalid, opt);
            (opt, dist)
        })
        .filter(|(_, dist)| *dist <= 2)  // Max 2 character edits
        .collect();
    
    suggestions.sort_by_key(|(_, dist)| *dist);
    suggestions.into_iter()
        .take(3)  // Top 3 suggestions
        .map(|(opt, _)| opt.to_string())
        .collect()
}

/// Levenshtein distance for typo suggestions
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(
                    matrix[i][j + 1] + 1,    // deletion
                    matrix[i + 1][j] + 1     // insertion
                ),
                matrix[i][j] + cost          // substitution
            );
        }
    }
    
    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_creation() {
        let err = KillerError::parse_error("unexpected token", "test.killer", 5, 10);
        assert_eq!(err.severity, ErrorSeverity::Error);
        assert_eq!(err.code(), "E001");
        assert_eq!(err.line, 5);
        assert_eq!(err.column, 10);
    }

    #[test]
    fn test_type_error() {
        let err = KillerError::type_error("i64", "String", "incompatible types", 
                                         "test.killer", 3, 8);
        assert_eq!(err.code(), "E002");
        assert!(err.to_string().contains("type mismatch"));
    }

    #[test]
    fn test_name_error_suggestions() {
        let sugg = vec!["count".to_string(), "counter".to_string()];
        let err = KillerError::name_error("cnt", "global", sugg, "test.killer", 1, 1)
            .with_suggestion("use 'count' instead");
        assert!(err.to_string().contains("undefined"));
        assert!(err.to_string().contains("did you mean"));
    }

    #[test]
    fn test_error_chain() {
        let e1 = KillerError::runtime_error("inner error", "test.killer", 1, 1);
        let e2 = KillerError::runtime_error("outer error", "test.killer", 2, 1)
            .with_source(e1);
        assert_eq!(e2.chain().len(), 2);
    }

    #[test]
    fn test_suggest_similar() {
        let valid = vec!["println", "print", "parse", "push"];
        let sugg = suggest_similar("prnt", &valid);
        assert!(!sugg.is_empty());
        assert!(sugg.contains(&"print".to_string()));
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(KillerError::parse_error("x", "f", 1, 1).code(), "E001");
        assert_eq!(KillerError::type_error("a", "b", "m", "f", 1, 1).code(), "E002");
        assert_eq!(KillerError::runtime_error("x", "f", 1, 1).code(), "E003");
    }

    #[test]
    fn test_error_severity() {
        let err = KillerError::parse_error("error", "f", 1, 1)
            .with_severity(ErrorSeverity::Critical);
        assert_eq!(err.severity, ErrorSeverity::Critical);
        assert!(!err.is_recoverable());
    }
}
