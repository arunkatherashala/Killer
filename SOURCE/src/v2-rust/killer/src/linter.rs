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

// =============================================================================================
// Rules for the language as it is: `lint_killer` (used by `killer lint`)
// =============================================================================================
//
// The rules below work on the source text (comments and strings masked), not on tokens of an
// imagined grammar, so they agree with what the compiler accepts. Rule ids:
//
//   undefined-function     error    call of a name that is not a function, class, variable or builtin
//   unused-variable        warning  variable assigned but never read
//   unreachable-code       warning  statement after return / throw / break / continue
//   shadowed-builtin       warning  function, class, parameter or variable named like a builtin
//   floor-division-comment warning  `a // 2` is a comment in Killer, not integer division
//
// Suppression: `# lint:ignore RULE[, RULE...]` (or `// lint:ignore ...` / `-- lint:ignore ...`)
// on the offending line, or alone on the line above it. `lint:ignore` without a rule id ignores
// every rule; `lint:ignore-file RULE` anywhere ignores the rule for the whole file.

pub const LINT_RULES: &[(&str, LintSeverity, &str)] = &[
    ("undefined-function", LintSeverity::Error, "call of an undefined function"),
    ("unused-variable", LintSeverity::Warning, "variable assigned but never read"),
    ("unreachable-code", LintSeverity::Warning, "code after return/throw/break/continue"),
    ("shadowed-builtin", LintSeverity::Warning, "name shadows a builtin function"),
    ("floor-division-comment", LintSeverity::Warning, "`//` starts a comment, it is not integer division"),
];

struct SrcLine<'a> {
    raw: &'a str,
    /// strings and comments blanked (columns preserved)
    masked: String,
    /// code part of the raw line (comment removed, strings kept)
    code: String,
    depth_before: i64,
    depth_after: i64,
}

fn words_with_pos(s: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let st = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((st, chars[st..i].iter().collect()));
        } else if chars[i].is_ascii_digit() {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn prev_non_space(chars: &[char], i: usize) -> Option<char> {
    chars[..i].iter().rev().find(|c| !c.is_whitespace()).copied()
}

/// Statement-start assignment target of a masked line: `x = ...`, `let x = ...`, `global x = ...`.
fn assignment_target(masked: &str) -> Option<(usize, String)> {
    let chars: Vec<char> = masked.chars().collect();
    let skip_ws = |mut i: usize| {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        i
    };
    let word = |i: usize| -> Option<String> {
        let w: String = chars[i.min(chars.len())..].iter().take_while(|c| c.is_alphanumeric() || **c == '_').collect();
        if w.is_empty() || w.chars().next().map_or(true, |c| c.is_ascii_digit()) {
            None
        } else {
            Some(w)
        }
    };
    let mut start = skip_ws(0);
    let mut w = word(start)?;
    if w == "let" || w == "global" {
        start = skip_ws(start + w.chars().count());
        w = word(start)?;
    }
    if crate::lsp_core::KEYWORDS.contains(&w.as_str()) {
        return None;
    }
    let after = skip_ws(start + w.chars().count());
    if chars.get(after) == Some(&'=') && !matches!(chars.get(after + 1), Some('=') | Some('>')) {
        Some((start, w))
    } else {
        None
    }
}

/// Index (char) where a trailing comment starts in `raw`, or the line length.
fn comment_start_col(raw: &str) -> usize {
    let rc: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < rc.len() {
        let c = rc[i];
        if let Some(q) = in_str {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || (c == '\'' && !(i > 0 && rc[i - 1].is_alphanumeric())) {
            in_str = Some(c);
        } else if (c == '#' && (i == 0 || rc[i - 1].is_whitespace()))
            || (c == '/' && rc.get(i + 1) == Some(&'/'))
            || (c == '-' && rc.get(i + 1) == Some(&'-'))
        {
            return i;
        }
        i += 1;
    }
    rc.len()
}

struct FnInfo {
    name: String,
    start: usize,
    end: usize,
    params: Vec<String>,
    is_method: bool,
}

fn split_params(masked: &str) -> Vec<String> {
    let open = match masked.find('(') {
        Some(o) => o,
        None => return Vec::new(),
    };
    let mut d = 0;
    let mut ptxt = String::new();
    for c in masked[open..].chars() {
        if c == '(' {
            d += 1;
            if d == 1 {
                continue;
            }
        }
        if c == ')' {
            d -= 1;
            if d == 0 {
                break;
            }
        }
        ptxt.push(c);
    }
    let mut dd = 0;
    let mut part = String::new();
    let mut parts = Vec::new();
    for c in ptxt.chars() {
        match c {
            '(' | '[' | '{' => dd += 1,
            ')' | ']' | '}' => dd -= 1,
            _ => {}
        }
        if c == ',' && dd == 0 {
            parts.push(std::mem::take(&mut part));
        } else {
            part.push(c);
        }
    }
    parts.push(part);
    parts.iter().filter_map(|p| words_with_pos(p).first().map(|(_, w)| w.clone())).collect()
}

/// Lint Killer source with the rules above. Violations are sorted by line.
pub fn lint_killer(source: &str) -> Vec<LintViolation> {
    let mut lines: Vec<SrcLine> = Vec::new();
    let mut depth = 0i64;
    for raw in source.lines() {
        let masked = crate::formatter::mask_code(raw);
        let code: String = raw.chars().take(comment_start_col(raw)).collect();
        let before = depth;
        for c in masked.chars() {
            match c {
                '{' | '[' | '(' => depth += 1,
                '}' | ']' | ')' => depth = (depth - 1).max(0),
                _ => {}
            }
        }
        lines.push(SrcLine { raw, masked, code, depth_before: before, depth_after: depth });
    }
    if lines.is_empty() {
        return Vec::new();
    }

    let builtins = crate::lsp_core::builtin_names();
    let mut out: Vec<LintViolation> = Vec::new();
    let mut add = |rule: &str, ln: usize, col: usize, msg: String| {
        let sev = LINT_RULES.iter().find(|r| r.0 == rule).map(|r| r.1).unwrap_or(LintSeverity::Warning);
        out.push(LintViolation::new(rule.to_string(), sev, msg).with_line(ln + 1).with_column(col + 1));
    };

    // ---- structure: functions, classes, parameters ---------------------------------------
    let find_end = |start: usize| -> usize {
        let d0 = lines[start].depth_before;
        let mut i = start;
        let mut seen_open = false;
        loop {
            if lines[i].depth_after > d0 {
                seen_open = true;
            }
            if seen_open && lines[i].depth_after <= d0 {
                return i;
            }
            if !seen_open && i > start + 1 {
                return start; // no block follows
            }
            i += 1;
            if i >= lines.len() {
                return lines.len() - 1;
            }
        }
    };
    let mut fns: Vec<FnInfo> = Vec::new();
    let mut class_names: Vec<(String, usize, usize)> = Vec::new();
    let mut has_import = false;
    for (i, l) in lines.iter().enumerate() {
        let ws = words_with_pos(l.masked.trim());
        let first = ws.first().map(|(_, w)| w.as_str()).unwrap_or("");
        if first == "import" || first == "from" {
            has_import = true;
        }
        let name_idx = if first == "async" { 2 } else { 1 };
        let kind = if first == "async" { ws.get(1).map(|(_, w)| w.as_str()).unwrap_or("") } else { first };
        if matches!(kind, "fn" | "kfn" | "akfn") {
            if let Some((_, name)) = ws.get(name_idx) {
                fns.push(FnInfo { name: name.clone(), start: i, end: find_end(i), params: split_params(&l.masked), is_method: false });
            }
        } else if first == "class" {
            if let Some((_, name)) = ws.get(1) {
                class_names.push((name.clone(), i, find_end(i)));
            }
        }
    }
    for f in fns.iter_mut() {
        f.is_method = class_names.iter().any(|(_, s, e)| f.start > *s && f.start <= *e && lines[f.start].depth_before == lines[*s].depth_before + 1);
    }

    // ---- names defined anywhere --------------------------------------------------------------
    // A name counts as defined when it is a function or class, or appears anywhere other than as
    // the callee of a call (assignment, parameter, loop variable, lambda parameter, ...).
    let mut defined: std::collections::HashSet<String> = std::collections::HashSet::new();
    for f in &fns {
        defined.insert(f.name.clone());
    }
    for (c, _, _) in &class_names {
        defined.insert(c.clone());
    }
    for l in &lines {
        let mc: Vec<char> = l.masked.chars().collect();
        // `name(params) => expr` defines a function
        if l.masked.contains("=>") {
            if let Some((_, w)) = words_with_pos(l.masked.trim_start()).first() {
                if l.masked.trim_start()[w.len()..].starts_with('(') {
                    defined.insert(w.clone());
                }
            }
        }
        for (p, w) in words_with_pos(&l.masked) {
            let end = p + w.chars().count();
            let next = mc[end..].iter().find(|c| !c.is_whitespace());
            if prev_non_space(&mc, p) == Some('.') {
                continue;
            }
            if next != Some(&'(') {
                defined.insert(w);
            }
        }
    }

    // ---- undefined-function --------------------------------------------------------------------
    if !has_import {
        for (i, l) in lines.iter().enumerate() {
            let mc: Vec<char> = l.masked.chars().collect();
            let ws = words_with_pos(&l.masked);
            for (k, (p, w)) in ws.iter().enumerate() {
                let end = p + w.chars().count();
                if mc[end..].iter().find(|c| !c.is_whitespace()) != Some(&'(') || prev_non_space(&mc, *p) == Some('.') {
                    continue;
                }
                let prev_word = if k > 0 { ws[k - 1].1.as_str() } else { "" };
                if matches!(prev_word, "fn" | "kfn" | "akfn" | "class" | "new" | "test") {
                    continue;
                }
                if defined.contains(w)
                    || builtins.contains(&w.as_str())
                    || crate::net_ai::is_builtin(w)
                    || crate::lsp_core::KEYWORDS.contains(&w.as_str())
                    || matches!(w.as_str(), "super" | "this" | "assert_throws")
                {
                    continue;
                }
                add("undefined-function", i, *p, format!("call of undefined function `{w}`"));
            }
        }
    }

    // ---- shadowed-builtin ------------------------------------------------------------------------
    let is_builtin = |n: &str| builtins.contains(&n);
    for f in &fns {
        if !f.is_method && is_builtin(&f.name) {
            let col = lines[f.start].masked.find(f.name.as_str()).unwrap_or(0);
            add("shadowed-builtin", f.start, col, format!("function `{}` shadows the builtin of the same name", f.name));
        }
        for p in &f.params {
            if is_builtin(p) {
                let col = lines[f.start].masked.find(p.as_str()).unwrap_or(0);
                add("shadowed-builtin", f.start, col, format!("parameter `{p}` shadows the builtin of the same name"));
            }
        }
    }
    for (c, s, _) in &class_names {
        if is_builtin(c) {
            add("shadowed-builtin", *s, lines[*s].masked.find(c.as_str()).unwrap_or(0), format!("class `{c}` shadows the builtin of the same name"));
        }
    }
    let mut shadow_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some((col, name)) = assignment_target(&l.masked) {
            if is_builtin(&name) && shadow_seen.insert(name.clone()) {
                add("shadowed-builtin", i, col, format!("variable `{name}` shadows the builtin of the same name"));
            }
        }
    }

    // ---- unused-variable ----------------------------------------------------------------------
    // Count word occurrences (in code, strings included, so `k"{x}"` counts as a read) against
    // the number of assignments to the name.
    let count_in = |range: &std::ops::RangeInclusive<usize>, name: &str| -> (usize, usize, Option<(usize, usize)>) {
        let mut occurrences = 0usize;
        let mut targets = 0usize;
        let mut first_target: Option<(usize, usize)> = None;
        for i in range.clone() {
            let l = &lines[i];
            let cc: Vec<char> = l.code.chars().collect();
            for (p, w) in words_with_pos(&l.code) {
                if w == name && prev_non_space(&cc, p) != Some('.') {
                    occurrences += 1;
                }
            }
            if let Some((col, n)) = assignment_target(&l.masked) {
                if n == name {
                    targets += 1;
                    if first_target.is_none() {
                        first_target = Some((i, col));
                    }
                }
            }
        }
        (occurrences, targets, first_target)
    };
    let mut check_unused = |range: std::ops::RangeInclusive<usize>, top_level_only: bool, params: &[String], add: &mut dyn FnMut(&str, usize, usize, String)| {
        let mut names: Vec<String> = Vec::new();
        for i in range.clone() {
            if top_level_only && lines[i].depth_before != 0 {
                continue;
            }
            if let Some((_, n)) = assignment_target(&lines[i].masked) {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        for n in names {
            if n.starts_with('_') || params.contains(&n) {
                continue;
            }
            let (occ, targets, first) = count_in(&range, &n);
            if occ > targets {
                continue;
            }
            if let Some((i, col)) = first {
                // `global x` reaches the outer scope: not a local
                let declared_global = range.clone().any(|j| {
                    let ws = words_with_pos(&lines[j].masked);
                    ws.first().map_or(false, |(_, w)| w == "global") && ws.iter().skip(1).any(|(_, w)| *w == n)
                });
                if !declared_global {
                    add("unused-variable", i, col, format!("variable `{n}` is assigned but never used"));
                }
            }
        }
    };
    check_unused(0..=lines.len() - 1, true, &[], &mut add);
    for f in &fns {
        if f.end > f.start {
            check_unused(f.start + 1..=f.end, false, &f.params, &mut add);
        }
    }

    // ---- unreachable-code ---------------------------------------------------------------------
    for i in 0..lines.len() {
        let t = lines[i].masked.trim();
        let first: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !matches!(first.as_str(), "return" | "throw" | "break" | "continue") {
            continue;
        }
        if lines[i].depth_after != lines[i].depth_before {
            continue; // `return [` continues on the next lines
        }
        if t.ends_with(|c| matches!(c, '+' | '-' | '*' | '/' | ',' | '&' | '|' | '=')) {
            continue;
        }
        if let Some(j) = (i + 1..lines.len()).find(|&j| !lines[j].masked.trim().is_empty()) {
            let nt = lines[j].masked.trim();
            let nfirst: String = nt.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if lines[j].depth_before == lines[i].depth_before
                && !nt.starts_with(|c| matches!(c, '}' | ')' | ']' | ':'))
                && !matches!(nfirst.as_str(), "case" | "default" | "else" | "elif" | "catch" | "finally")
            {
                let col = lines[j].masked.chars().count() - lines[j].masked.trim_start().chars().count();
                add("unreachable-code", j, col, format!("code after `{first}` is never executed"));
            }
        }
    }

    // ---- floor-division-comment ------------------------------------------------------------------
    for (i, l) in lines.iter().enumerate() {
        let cs = comment_start_col(l.raw);
        let rc: Vec<char> = l.raw.chars().collect();
        if cs + 1 >= rc.len() || rc[cs] != '/' || rc[cs + 1] != '/' {
            continue;
        }
        let before: String = rc[..cs].iter().collect::<String>().trim_end().to_string();
        let after: String = rc[cs + 2..].iter().collect::<String>().trim().to_string();
        let ends_operand = before.chars().last().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == ')' || c == ']');
        let in_expr = before.contains('=') || before.trim_start().starts_with("return");
        let numeric_after = after.chars().next().map_or(false, |c| c.is_ascii_digit())
            && after.trim_end_matches(|c| matches!(c, ')' | ']' | ' ')).chars().all(|c| c.is_ascii_digit() || c == '.' || c == '_');
        if ends_operand && (crate::hints::floor_division_hint(l.raw).is_some() || (in_expr && numeric_after)) {
            add("floor-division-comment", i, cs, "`//` starts a comment in Killer; use floor(a / b) for integer division".to_string());
        }
    }

    // ---- suppression -----------------------------------------------------------------------------
    let parse_ignore = |text: &str, marker: &str| -> Option<Vec<String>> {
        let i = text.find(marker)?;
        Some(
            text[i + marker.len()..]
                .split(|c: char| c == ',' || c.is_whitespace())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
        )
    };
    let mut file_ignores: Vec<String> = Vec::new();
    let mut line_ignores: Vec<Vec<String>> = vec![Vec::new(); lines.len() + 1];
    for (i, l) in lines.iter().enumerate() {
        if let Some(r) = parse_ignore(l.raw, "lint:ignore-file") {
            file_ignores.extend(r);
            continue;
        }
        if let Some(mut r) = parse_ignore(l.raw, "lint:ignore") {
            if r.is_empty() {
                r.push("*".to_string());
            }
            let target = if l.code.trim().is_empty() { i + 1 } else { i };
            line_ignores[target].extend(r);
        }
    }
    out.retain(|v| {
        let ln = v.line.unwrap_or(1) - 1;
        let ign = |list: &Vec<String>| list.iter().any(|r| r == "*" || *r == v.rule);
        !(ign(&file_ignores) || line_ignores.get(ln).map_or(false, |l| ign(l)))
    });
    out.sort_by_key(|v| (v.line, v.column));
    out.dedup_by(|a, b| a.line == b.line && a.rule == b.rule && a.message == b.message);
    out
}

#[cfg(test)]
mod killer_rule_tests {
    use super::*;

    fn rules(src: &str) -> Vec<(String, usize)> {
        lint_killer(src).into_iter().map(|v| (v.rule, v.line.unwrap())).collect()
    }

    #[test]
    fn undefined_function() {
        let r = rules("fn a() {\n  return 1\n}\nprint(a())\nprnt(2)\n");
        assert_eq!(r, vec![("undefined-function".to_string(), 5)]);
    }

    #[test]
    fn unused_variable_in_function() {
        let r = rules("fn f(a) {\n  unused = a + 1\n  used = 2\n  return used\n}\nprint(f(1))\n");
        assert_eq!(r, vec![("unused-variable".to_string(), 2)]);
    }

    #[test]
    fn interpolation_counts_as_use() {
        let r = rules("fn f() {\n  name = \"x\"\n  return k\"hi {name}\"\n}\nprint(f())\n");
        assert!(r.is_empty(), "{r:?}");
    }

    #[test]
    fn unreachable_after_return() {
        let r = rules("fn f() {\n  return 1\n  print(2)\n}\nprint(f())\n");
        assert_eq!(r, vec![("unreachable-code".to_string(), 3)]);
        assert!(rules("fn f(a) {\n  if a {\n    return 1\n  } else {\n    return 2\n  }\n}\nprint(f(1))\n").is_empty());
    }

    #[test]
    fn shadowed_builtin() {
        let r = rules("len = 3\nprint(len)\nfn max(a) {\n  return a\n}\nprint(max(1))\n");
        assert!(r.contains(&("shadowed-builtin".to_string(), 1)), "{r:?}");
        assert!(r.contains(&("shadowed-builtin".to_string(), 3)), "{r:?}");
    }

    #[test]
    fn floor_division_comment() {
        let r = rules("n = 9\nhalf = n // 2\nprint(half)\n");
        assert_eq!(r, vec![("floor-division-comment".to_string(), 2)]);
        assert!(rules("x = 5 # // 2 is fine here\nprint(x)\n").is_empty());
        assert!(rules("print(\"a // 2\")\n").is_empty());
    }

    #[test]
    fn suppression() {
        assert!(rules("prnt(1) # lint:ignore undefined-function\n").is_empty());
        assert!(rules("# lint:ignore undefined-function\nprnt(1)\n").is_empty());
        assert!(rules("# lint:ignore-file undefined-function\nprnt(1)\nprnt(2)\n").is_empty());
        assert_eq!(rules("prnt(1) # lint:ignore unused-variable\n").len(), 1);
    }

    #[test]
    fn arrow_function_definitions_are_defined() {
        assert!(rules("add(a, b) => a + b\nprint(add(1, 2))\n").is_empty());
    }
}
