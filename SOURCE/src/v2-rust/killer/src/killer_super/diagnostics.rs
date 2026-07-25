// killer_super/diagnostics.rs - Error reporting and diagnostics
// Comprehensive error and warning system for all 16 compiler stages

use std::fmt;

/// Severity level of a diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
    Fatal,
}

/// Represents a diagnostic (error, warning, or info message)
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub stage: &'static str,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn new(level: DiagnosticLevel, stage: &'static str, message: String) -> Self {
        Self {
            level,
            stage,
            message,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn error(stage: &'static str, message: String) -> Self {
        Self::new(DiagnosticLevel::Error, stage, message)
    }

    pub fn warning(stage: &'static str, message: String) -> Self {
        Self::new(DiagnosticLevel::Warning, stage, message)
    }

    pub fn fatal(stage: &'static str, message: String) -> Self {
        Self::new(DiagnosticLevel::Fatal, stage, message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match self.level {
            DiagnosticLevel::Info => "ℹ info",
            DiagnosticLevel::Warning => "⚠ warning",
            DiagnosticLevel::Error => "✗ error",
            DiagnosticLevel::Fatal => "✗✗ fatal",
        };

        write!(f, "[{}] {}: {}", level_str, self.stage, self.message)?;

        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, " ({}:{})", line, col)?;
        }

        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n  Suggestion: {}", suggestion)?;
        }

        Ok(())
    }
}

/// Collects and manages all diagnostics from compilation
#[allow(dead_code)]
pub struct DiagnosticsCollector {
    diagnostics: Vec<Diagnostic>,
    max_errors: usize,
}

impl DiagnosticsCollector {
    pub fn new() -> Self {
        Self {
            diagnostics: vec![],
            max_errors: 10,
        }
    }

    pub fn add(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn error(&mut self, stage: &'static str, message: String) {
        self.add(Diagnostic::error(stage, message));
    }

    pub fn warning(&mut self, stage: &'static str, message: String) {
        self.add(Diagnostic::warning(stage, message));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level >= DiagnosticLevel::Error)
    }

    pub fn has_fatals(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Fatal)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level >= DiagnosticLevel::Error)
            .count()
    }

    pub fn get_all(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn report(&self) -> String {
        let mut output = String::new();

        for diag in &self.diagnostics {
            output.push_str(&format!("{}\n", diag));
        }

        let error_count = self.error_count();
        if error_count > 0 {
            output.push_str(&format!(
                "\n{} error{} found during compilation\n",
                error_count,
                if error_count == 1 { "" } else { "s" }
            ));
        }

        output
    }
}

impl Default for DiagnosticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::new(
            DiagnosticLevel::Error,
            "Parser",
            "Unexpected token".to_string(),
        );
        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.stage, "Parser");
    }

    #[test]
    fn test_diagnostic_with_location() {
        let diag = Diagnostic::error("Lexer", "Invalid number".to_string())
            .with_location(42, 15);
        assert_eq!(diag.line, Some(42));
        assert_eq!(diag.column, Some(15));
    }

    #[test]
    fn test_diagnostics_collector() {
        let mut collector = DiagnosticsCollector::new();
        collector.error("Parser", "Expected ';'".to_string());
        collector.warning("Optimizer", "Unused variable".to_string());

        assert!(collector.has_errors());
        assert_eq!(collector.error_count(), 1);
        assert_eq!(collector.get_all().len(), 2);
    }
}
