/// Source Location Information for Better Error Reporting
/// Purpose: Track source code positions for precise error messages and debugging
/// Part of v4.3 refactoring - HIGH priority fix

use std::fmt::{Display, Formatter};

/// Represents a location in source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// File path (relative or absolute)
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Source code context (up to 100 chars)
    pub context: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: String, line: usize, column: usize) -> Self {
        SourceLocation {
            file,
            line,
            column,
            context: None,
        }
    }

    /// Create with source context
    pub fn with_context(file: String, line: usize, column: usize, context: String) -> Self {
        SourceLocation {
            file,
            line,
            column,
            context: Some(context.chars().take(100).collect()),
        }
    }

    /// Create location from token position
    pub fn from_token(file: String, line: usize, column: usize, token_value: String) -> Self {
        SourceLocation {
            file,
            line,
            column,
            context: Some(token_value),
        }
    }
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)?;
        if let Some(ctx) = &self.context {
            write!(f, " ({})", ctx)?;
        }
        Ok(())
    }
}

/// Error type with precise location information
#[derive(Debug, Clone)]
pub struct ErrorWithLocation {
    /// Error message
    pub message: String,
    /// Where the error occurred
    pub location: SourceLocation,
    /// Optional remediation suggestion
    pub suggestion: Option<String>,
    /// Optional error code for categorization
    pub code: Option<String>,
}

impl ErrorWithLocation {
    /// Create error with location
    pub fn new(message: String, location: SourceLocation) -> Self {
        ErrorWithLocation {
            message,
            location,
            suggestion: None,
            code: None,
        }
    }

    /// Add suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Add error code
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    /// Full formatted error message with location and context
    pub fn full_message(&self) -> String {
        let mut msg = format!("{}: {}", self.location, self.message);
        if let Some(code) = &self.code {
            msg.push_str(&format!(" [{}]", code));
        }
        if let Some(sugg) = &self.suggestion {
            msg.push_str(&format!("\nHint: {}", sugg));
        }
        msg
    }
}

impl Display for ErrorWithLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_location_formatting() {
        let loc = SourceLocation::new("main.killer".to_string(), 42, 10);
        assert_eq!(loc.to_string(), "main.killer:42:10");
    }

    #[test]
    fn error_with_location_full_message() {
        let loc = SourceLocation::new("parser.killer".to_string(), 15, 5);
        let err = ErrorWithLocation::new("Invalid number".to_string(), loc)
            .with_code("E001".to_string())
            .with_suggestion("Use digits 0-9 only".to_string());

        let msg = err.full_message();
        assert!(msg.contains("Invalid number"));
        assert!(msg.contains("[E001]"));
        assert!(msg.contains("Use digits 0-9 only"));
    }

    #[test]
    fn source_location_with_context() {
        let loc = SourceLocation::with_context(
            "file.killer".to_string(),
            5,
            10,
            "let x = abc;".to_string(),
        );
        assert!(loc.context.is_some());
        assert_eq!(loc.context.unwrap(), "let x = abc;");
    }
}
