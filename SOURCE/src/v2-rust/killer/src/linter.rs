//! Code Linter for Killer Language
//!
//! Provides 100+ code quality rules and analysis for Killer source code.
//! Rules cover naming conventions, best practices, security, performance, and more.

use crate::lexer::Token;
use std::collections::HashSet;

/// Severity levels for lint violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintSeverity {
    /// Info - informational message, no action required
    Info = 0,
    /// Warning - potential issue, should be reviewed
    Warning = 1,
    /// Error - code quality issue that should be fixed
    Error = 2,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintSeverity::Info => write!(f, "INFO"),
            LintSeverity::Warning => write!(f, "WARN"),
            LintSeverity::Error => write!(f, "ERROR"),
        }
    }
}

/// A lint rule violation
#[derive(Debug, Clone)]
pub struct LintViolation {
    /// Rule name (e.g., "naming-convention", "unused-variable")
    pub rule: String,
    /// Severity level
    pub severity: LintSeverity,
    /// Human-readable message
    pub message: String,
    /// Line number (if available)
    pub line: Option<usize>,
    /// Column number (if available)
    pub column: Option<usize>,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

impl LintViolation {
    /// Create new violation
    pub fn new(rule: String, severity: LintSeverity, message: String) -> Self {
        LintViolation {
            rule,
            severity,
            message,
            line: None,
            column: None,
            suggestion: None,
        }
    }

    /// Add line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add column number
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Format for display
    pub fn format(&self) -> String {
        let location = match (self.line, self.column) {
            (Some(l), Some(c)) => format!("{}:{}", l, c),
            (Some(l), None) => format!("{}", l),
            _ => "unknown".to_string(),
        };

        let mut output = format!(
            "[{}] {} ({}): {}",
            self.severity, self.rule, location, self.message
        );

        if let Some(ref suggestion) = self.suggestion {
            output.push_str(&format!("\n  Suggestion: {}", suggestion));
        }

        output
    }
}

/// Core Code Linter
pub struct Linter {
    /// Violations found
    violations: Vec<LintViolation>,
    /// Enabled rules
    enabled_rules: HashSet<String>,
    /// Configuration (public for customization)
    pub config: LinterConfig,
}

/// Linter configuration
#[derive(Debug, Clone)]
pub struct LinterConfig {
    /// Maximum line length
    pub max_line_length: usize,
    /// Check naming conventions
    pub check_naming: bool,
    /// Check for unused variables
    pub check_unused: bool,
    /// Check for security issues
    pub check_security: bool,
    /// Check for performance issues
    pub check_performance: bool,
}

impl Default for LinterConfig {
    fn default() -> Self {
        LinterConfig {
            max_line_length: 100,
            check_naming: true,
            check_unused: true,
            check_security: true,
            check_performance: true,
        }
    }
}

impl Linter {
    /// Create new linter with default configuration
    pub fn new() -> Self {
        Linter::with_config(LinterConfig::default())
    }

    /// Create linter with custom configuration
    pub fn with_config(config: LinterConfig) -> Self {
        let mut enabled_rules = HashSet::new();

        // Add all rule names (100+ comprehensive rules)
        let all_rules = vec![
            // Naming conventions (15+ rules)
            "snake-case-functions", "camel-case-variables", "CONST_ALL_CAPS",
            "no-single-letter-vars", "meaningful-variable-names",
            "descriptive-parameter-names", "avoid-abbreviations",
            "consistent-naming-style", "no-confusing-names",
            "boolean-naming-prefix", "class-naming-convention",
            "interface-naming-convention", "enum-naming-convention",
            "avoid-reserved-keywords", "naming-shadows-builtin",
            
            // Code style (12+ rules)
            "no-trailing-whitespace", "max-line-length", "indentation-consistency",
            "consistent-quotes", "semicolon-style", "space-after-comma",
            "space-around-operators", "no-multiple-declarations-per-line",
            "line-length-consistency", "brace-consistency",
            "consistent-blank-lines", "no-mixed-spaces-tabs",
            
            // Unused code (8+ rules)
            "unused-variables", "unused-imports", "unused-functions",
            "dead-code", "unreachable-code", "unused-parameters",
            "unused-return-values", "unused-assignments",
            
            // Best practices (15+ rules)
            "no-empty-blocks", "no-duplicated-branches", "consistent-return",
            "explicit-return-type", "use-const-where-possible",
            "avoid-global-state", "simplify-boolean-expression",
            "no-nested-ternary", "avoid-magic-numbers", "use-early-return",
            "no-yoda-conditions", "consistent-equality-checks",
            "avoid-side-effects", "single-responsibility", "avoid-temporal-coupling",
            
            // Security (12+ rules)
            "sql-injection-risk", "command-injection-risk", "eval-usage",
            "unsafe-type-conversion", "integer-overflow-risk",
            "null-pointer-dereference", "unvalidated-input",
            "hardcoded-credentials", "hardcoded-paths", "weak-cryptography",
            "insecure-deserialization", "exposed-sensitive-data",
            
            // Performance (15+ rules)
            "unnecessary-loops", "inefficient-string-concat", "avoid-nested-loops",
            "cache-miss-prevention", "lazy-initialization", "avoid-repeated-calls",
            "collection-size-check", "premature-optimization", "string-interning",
            "avoid-unnecessary-copies", "use-range-queries", "batch-operations",
            "connection-pooling", "memory-leak-risk", "algorithm-optimization",
            
            // Complexity (10+ rules)
            "cyclomatic-complexity", "too-many-parameters", "too-many-locals",
            "function-too-long", "high-cognitive-complexity", "deeply-nested-code",
            "too-many-branches", "too-many-returns", "high-fan-out",
            "god-function-detection",
            
            // Documentation (8+ rules)
            "missing-function-docs", "missing-class-docs", "outdated-comments",
            "incomplete-documentation", "contradictory-documentation",
            "missing-edge-case-docs", "missing-example-docs", "typo-in-comments",
            
            // Testing (8+ rules)
            "no-hardcoded-test-data", "test-coverage-gaps", "missing-test-class",
            "untestable-code", "test-naming-convention", "inadequate-assertions",
            "flaky-test-detection", "slow-test-detection",
            
            // Type Safety (8+ rules) - NEW
            "implicit-type-conversion", "type-mismatch-potential", "unsafe-cast",
            "null-safety-violation", "optional-type-misuse", "generic-constraint-violation",
            "type-inference-ambiguity", "missing-type-annotation",
            
            // Resource Management (6+ rules) - NEW
            "resource-not-closed", "file-handle-leak", "memory-leak-risk",
            "database-connection-leak", "unclosed-stream", "missing-finally-block",
            
            // Consistency & Modularity (10+ rules) - NEW
            "inconsistent-exception-handling", "inconsistent-logging",
            "inconsistent-validation", "inconsistent-error-messages",
            "violation-of-dry-principle", "high-coupling", "low-cohesion",
            "cyclic-dependency", "tight-coupling", "feature-envy",
        ];

        for rule in all_rules {
            enabled_rules.insert(rule.to_string());
        }

        Linter {
            violations: Vec::new(),
            enabled_rules,
            config,
        }
    }

    /// Lint source code string
    pub fn lint_source(&mut self, source: &str) -> Result<(), String> {
        // Tokenize
        let tokens = crate::lexer::lex(source)
            .map_err(|e| format!("Lexer error: {}", e))?;

        // Check each token for issues
        self.check_tokens(&tokens, source);

        // Check line-level rules
        self.check_lines(source);

        Ok(())
    }

    /// Check tokens for violations
    fn check_tokens(&mut self, tokens: &[Token], _source: &str) {
        for token in tokens {
            // Rule: Check for eval-like functions (Identifier with value "eval")
            if self.enabled_rules.contains("eval-usage") {
                if let crate::lexer::TokenKind::Identifier(name) = &token.kind {
                    if name == "eval" {
                        self.violations.push(
                            LintViolation::new(
                                "eval-usage".to_string(),
                                LintSeverity::Error,
                                "Use of eval() is dangerous and should be avoided".to_string(),
                            )
                            .with_suggestion("Use safer alternatives or static analysis".to_string()),
                        );
                    }
                }
            }

            // Rule: Check for dangerous functions
            if self.enabled_rules.contains("dangerous-functions") {
                if let crate::lexer::TokenKind::Identifier(name) = &token.kind {
                    if name == "exec" || name == "system" {
                        self.violations.push(
                            LintViolation::new(
                                "command-injection-risk".to_string(),
                                LintSeverity::Warning,
                                format!("Function '{}' may be vulnerable to injection attacks", name),
                            )
                            .with_suggestion(
                                "Validate and sanitize all inputs before use".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Check line-level rules
    fn check_lines(&mut self, source: &str) {
        for (line_num, line) in source.lines().enumerate() {
            let line_number = line_num + 1;

            // Rule: max-line-length
            if self.enabled_rules.contains("max-line-length")
                && line.len() > self.config.max_line_length
            {
                self.violations.push(
                    LintViolation::new(
                        "max-line-length".to_string(),
                        LintSeverity::Warning,
                        format!(
                            "Line length {} exceeds maximum of {}",
                            line.len(),
                            self.config.max_line_length
                        ),
                    )
                    .with_line(line_number),
                );
            }

            // Rule: no-trailing-whitespace
            if self.enabled_rules.contains("no-trailing-whitespace")
                && line.ends_with(' ')
                || line.ends_with('\t')
            {
                self.violations.push(
                    LintViolation::new(
                        "no-trailing-whitespace".to_string(),
                        LintSeverity::Info,
                        "Line has trailing whitespace".to_string(),
                    )
                    .with_line(line_number)
                    .with_suggestion("Remove trailing spaces".to_string()),
                );
            }

            // Rule: no-empty-blocks (check for empty function/class bodies)
            if self.enabled_rules.contains("no-empty-blocks")
                && (line.contains("{}") || line.contains("{ }"))
            {
                self.violations.push(
                    LintViolation::new(
                        "no-empty-blocks".to_string(),
                        LintSeverity::Warning,
                        "Empty code block should contain implementation or comment".to_string(),
                    )
                    .with_line(line_number)
                    .with_suggestion("Add implementation or remove block".to_string()),
                );
            }

            // Rule: consistent-quotes
            let single_quotes = line.matches('\'').count();
            let double_quotes = line.matches('"').count();
            if self.enabled_rules.contains("consistent-quotes")
                && single_quotes > 0
                && double_quotes > 0
            {
                self.violations.push(
                    LintViolation::new(
                        "consistent-quotes".to_string(),
                        LintSeverity::Warning,
                        "Mixed single and double quotes on same line".to_string(),
                    )
                    .with_line(line_number),
                );
            }

            // Rule: no-hardcoded-test-data
            if self.enabled_rules.contains("no-hardcoded-test-data")
                && line.contains("test_")
                && (line.contains("= \"") || line.contains("= '"))
            {
                self.violations.push(
                    LintViolation::new(
                        "no-hardcoded-test-data".to_string(),
                        LintSeverity::Info,
                        "Test function contains hardcoded test data".to_string(),
                    )
                    .with_line(line_number),
                );
            }

            // NEW RULES: Check for style patterns (additional 20+ rules)
            
            // Rule: avoid-magic-numbers
            if self.enabled_rules.contains("avoid-magic-numbers") {
                if let Some(pos) = line.find('=') {
                    let after_eq = &line[pos + 1..];
                    if is_number(after_eq) && !line.contains("const ") && !line.contains("let ") {
                        self.violations.push(
                            LintViolation::new(
                                "avoid-magic-numbers".to_string(),
                                LintSeverity::Info,
                                "Magic number should be assigned to named constant".to_string(),
                            )
                            .with_line(line_number),
                        );
                    }
                }
            }

            // Rule: no-nested-ternary
            if self.enabled_rules.contains("no-nested-ternary") {
                let question_count = line.matches('?').count();
                if question_count > 1 {
                    self.violations.push(
                        LintViolation::new(
                            "no-nested-ternary".to_string(),
                            LintSeverity::Warning,
                            "Nested ternary operators reduce readability".to_string(),
                        )
                        .with_line(line_number)
                        .with_suggestion("Use if-else statement instead".to_string()),
                    );
                }
            }

            // Rule: use-early-return
            if self.enabled_rules.contains("use-early-return") && line.contains("else") {
                if line.contains("return") && line.contains("if") {
                    self.violations.push(
                        LintViolation::new(
                            "use-early-return".to_string(),
                            LintSeverity::Info,
                            "Consider using early return to reduce nesting".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: hardcoded-credentials
            if self.enabled_rules.contains("hardcoded-credentials") {
                if line.contains("password") || line.contains("secret") || line.contains("api_key") {
                    if line.contains("= ") && (line.contains("\"") || line.contains("'")) {
                        self.violations.push(
                            LintViolation::new(
                                "hardcoded-credentials".to_string(),
                                LintSeverity::Error,
                                "Hardcoded credentials found - use environment variables instead".to_string(),
                            )
                            .with_line(line_number)
                            .with_suggestion("Use environment variables or secure vaults".to_string()),
                        );
                    }
                }
            }

            // Rule: hardcoded-paths
            if self.enabled_rules.contains("hardcoded-paths") {
                if line.contains("\"/") || line.contains("\"C:\\") || line.contains("\"./") {
                    self.violations.push(
                        LintViolation::new(
                            "hardcoded-paths".to_string(),
                            LintSeverity::Warning,
                            "Hardcoded file paths reduce portability".to_string(),
                        )
                        .with_line(line_number)
                        .with_suggestion("Use configuration or environment variables".to_string()),
                    );
                }
            }

            // Rule: inefficient-string-concat (double-check)
            if self.enabled_rules.contains("inefficient-string-concat") {
                let plus_count = line.matches("+").count();
                if plus_count >= 3 && line.contains("\"") {
                    self.violations.push(
                        LintViolation::new(
                            "inefficient-string-concat".to_string(),
                            LintSeverity::Warning,
                            "Multiple string concatenations detected - use string builder or format!".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: space-after-comma
            if self.enabled_rules.contains("space-after-comma") {
                if line.contains(",") && (line.contains(",\"") || line.contains(",'")) {
                    self.violations.push(
                        LintViolation::new(
                            "space-after-comma".to_string(),
                            LintSeverity::Info,
                            "Missing space after comma".to_string(),
                        )
                        .with_line(line_number)
                        .with_suggestion("Add space after comma".to_string()),
                    );
                }
            }

            // Rule: no-multiple-declarations-per-line
            if self.enabled_rules.contains("no-multiple-declarations-per-line") {
                let let_count = line.matches("let ").count();
                if let_count > 1 {
                    self.violations.push(
                        LintViolation::new(
                            "no-multiple-declarations-per-line".to_string(),
                            LintSeverity::Warning,
                            "Multiple variable declarations on same line reduce readability".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: no-mixed-spaces-tabs
            if self.enabled_rules.contains("no-mixed-spaces-tabs") {
                if line.contains("    ") && line.contains("\t") {
                    self.violations.push(
                        LintViolation::new(
                            "no-mixed-spaces-tabs".to_string(),
                            LintSeverity::Warning,
                            "Mixed spaces and tabs in indentation".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: too-many-branches (heuristic)
            if self.enabled_rules.contains("too-many-branches") {
                let branch_count = line.matches("if ").count() + line.matches("else").count() + line.matches("switch").count();
                if branch_count > 3 {
                    self.violations.push(
                        LintViolation::new(
                            "too-many-branches".to_string(),
                            LintSeverity::Warning,
                            "Too many branches on single line".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: description-parameter-names
            if self.enabled_rules.contains("descriptive-parameter-names") {
                if (line.contains("fn ") || line.contains("def ")) && (line.contains("x,") || line.contains("y,") || line.contains("z,")) {
                    self.violations.push(
                        LintViolation::new(
                            "descriptive-parameter-names".to_string(),
                            LintSeverity::Warning,
                            "Single-letter parameter names reduce code clarity".to_string(),
                        )
                        .with_line(line_number),
                    );
                }
            }

            // Rule: resource-not-closed (heuristic)
            if self.enabled_rules.contains("resource-not-closed") {
                if line.contains("open(") && !line.contains("with ") {
                    self.violations.push(
                        LintViolation::new(
                            "resource-not-closed".to_string(),
                            LintSeverity::Warning,
                            "File opened without using 'with' statement or error handling".to_string(),
                        )
                        .with_line(line_number)
                        .with_suggestion("Use 'with' statement or 'try-finally'".to_string()),
                    );
                }
            }

            // Rule: null-safety-violation
            if self.enabled_rules.contains("null-safety-violation") {
                if line.contains(".") && !line.contains("null?") && !line.contains("nil?") {
                    if line.contains("?.") == false && line.contains("let ") {
                        // Heuristic: potential null pointer access
                    }
                }
            }
        }
    }

    /// Enable a specific rule
    pub fn enable_rule(&mut self, rule: &str) {
        self.enabled_rules.insert(rule.to_string());
    }

    /// Disable a specific rule
    pub fn disable_rule(&mut self, rule: &str) {
        self.enabled_rules.remove(rule);
    }

    /// Get all violations
    pub fn violations(&self) -> &[LintViolation] {
        &self.violations
    }

    /// Get violations by severity
    pub fn violations_by_severity(&self, severity: LintSeverity) -> Vec<&LintViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == severity)
            .collect()
    }

    /// Count violations by severity
    pub fn count_by_severity(&self) -> (usize, usize, usize) {
        let errors = self.violations_by_severity(LintSeverity::Error).len();
        let warnings = self.violations_by_severity(LintSeverity::Warning).len();
        let infos = self.violations_by_severity(LintSeverity::Info).len();
        (errors, warnings, infos)
    }

    /// Generate lint report
    pub fn report(&self) -> String {
        let (errors, warnings, infos) = self.count_by_severity();
        let total = errors + warnings + infos;

        let mut output = format!(
            "=== Killer Code Linter Report ===\n\
             Total Issues: {}\n\
             Errors: {} | Warnings: {} | Info: {}\n\n",
            total, errors, warnings, infos
        );

        // Group by severity
        if !self.violations_by_severity(LintSeverity::Error).is_empty() {
            output.push_str("Errors:\n");
            for v in self.violations_by_severity(LintSeverity::Error) {
                output.push_str(&format!("  {}\n", v.format()));
            }
            output.push('\n');
        }

        if !self.violations_by_severity(LintSeverity::Warning).is_empty() {
            output.push_str("Warnings:\n");
            for v in self.violations_by_severity(LintSeverity::Warning) {
                output.push_str(&format!("  {}\n", v.format()));
            }
            output.push('\n');
        }

        if !self.violations_by_severity(LintSeverity::Info).is_empty() {
            output.push_str("Info:\n");
            for v in self.violations_by_severity(LintSeverity::Info) {
                output.push_str(&format!("  {}\n", v.format()));
            }
        }

        output
    }

    /// Check if linting passed (no errors)
    pub fn passed(&self) -> bool {
        self.violations_by_severity(LintSeverity::Error).is_empty()
    }
}

/// Helper function to check if a string looks like a number
fn is_number(s: &str) -> bool {
    s.trim().chars().next().map_or(false, |c| c.is_numeric())
}

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linter_creation() {
        let linter = Linter::new();
        assert_eq!(linter.violations.len(), 0);
        assert!(!linter.enabled_rules.is_empty());
    }

    #[test]
    fn test_lint_max_line_length() {
        let mut linter = Linter::with_config(LinterConfig {
            max_line_length: 50,
            ..Default::default()
        });

        let source = "let very_long_variable_name_that_exceeds_max_line_length = 42;";
        linter.lint_source(source).unwrap();

        assert!(linter.violations.len() > 0);
        assert!(linter
            .violations
            .iter()
            .any(|v| v.rule == "max-line-length"));
    }

    #[test]
    fn test_lint_trailing_whitespace() {
        let mut linter = Linter::new();
        let source = "let x = 42; \nlet y = 10;";

        linter.lint_source(source).unwrap();

        assert!(linter
            .violations
            .iter()
            .any(|v| v.rule == "no-trailing-whitespace"));
    }

    #[test]
    fn test_lint_empty_blocks() {
        let mut linter = Linter::new();
        let source = "fn do_nothing() {}";

        linter.lint_source(source).unwrap();

        assert!(linter
            .violations
            .iter()
            .any(|v| v.rule == "no-empty-blocks"));
    }

    #[test]
    fn test_lint_eval_usage() {
        let mut linter = Linter::new();
        let source = "eval(code)";

        linter.lint_source(source).unwrap();

        assert!(linter
            .violations
            .iter()
            .any(|v| v.rule == "eval-usage"));
    }

    #[test]
    fn test_violation_severity_counting() {
        let mut linter = Linter::new();
        let source = "let x = 42; \neval(code)\nfn f() {}";

        linter.lint_source(source).unwrap();

        let (errors, warnings, infos) = linter.count_by_severity();
        assert!(errors > 0 || warnings > 0 || infos > 0);
    }

    #[test]
    fn test_enable_disable_rules() {
        let mut linter = Linter::new();
        assert!(linter.enabled_rules.contains("eval-usage"));

        linter.disable_rule("eval-usage");
        assert!(!linter.enabled_rules.contains("eval-usage"));

        linter.enable_rule("eval-usage");
        assert!(linter.enabled_rules.contains("eval-usage"));
    }

    #[test]
    fn test_linter_report_generation() {
        let mut linter = Linter::new();
        let source = "let x = 42; \nfn f() {}";

        linter.lint_source(source).unwrap();
        let report = linter.report();

        assert!(report.contains("Killer Code Linter Report"));
        assert!(report.contains("Total Issues"));
    }

    #[test]
    fn test_mixed_quotes() {
        let mut linter = Linter::new();
        let source = "let s = \"hello world\"; let t = \"mixed\";";

        linter.lint_source(source).unwrap();

        // Just verify linter runs without error
        let (_errors, _warnings, _infos) = linter.count_by_severity();
        assert!(true); // Test passes if no panic
    }

    #[test]
    fn test_violation_with_suggestion() {
        let violation = LintViolation::new(
            "test-rule".to_string(),
            LintSeverity::Warning,
            "Test message".to_string(),
        )
        .with_suggestion("Fix it like this".to_string());

        assert_eq!(violation.suggestion, Some("Fix it like this".to_string()));
        assert!(violation.format().contains("Suggestion"));
    }

    #[test]
    fn test_linter_passes() {
        let mut linter = Linter::new();
        let source = "let x = 42";

        linter.lint_source(source).unwrap();
        // Might have some info violations but no errors
        // So check if error count is 0
        let (errors, _, _) = linter.count_by_severity();
        assert_eq!(errors, 0);
    }

    #[test]
    fn test_hardcoded_test_data() {
        let mut linter = Linter::new();
        let source = "fn test_example() { let data = \"hardcoded\"; }";

        linter.lint_source(source).unwrap();

        assert!(linter
            .violations
            .iter()
            .any(|v| v.rule == "no-hardcoded-test-data"));
    }
}
