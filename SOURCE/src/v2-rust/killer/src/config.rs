//! Configuration file support for Killer tools
//!
//! Provides `.killerrc` configuration file parsing and management.
//! Supports TOML format with sections for linter and formatter.

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Configuration for Killer tools
#[derive(Debug, Clone)]
pub struct KillerConfig {
    /// Linter configuration
    pub linter: LinterConfig,
    /// Formatter configuration
    pub formatter: FormatterConfig,
    /// Project root path
    pub project_root: PathBuf,
}

/// Linter settings from config
#[derive(Debug, Clone)]
pub struct LinterConfig {
    /// Max line length
    pub max_line_length: usize,
    /// Enable naming rules
    pub check_naming: bool,
    /// Enable unused code rules
    pub check_unused: bool,
    /// Enable security rules
    pub check_security: bool,
    /// Enable performance rules
    pub check_performance: bool,
    /// Severity level (info=0, warning=1, error=2)
    pub min_severity: u8,
    /// Rules to disable
    pub disabled_rules: Vec<String>,
}

/// Formatter settings from config
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Indentation style: "spaces" or "tabs"
    pub indent_style: String,
    /// Indent size
    pub indent_size: usize,
    /// Line length limit
    pub line_length: usize,
    /// Trailing comma style: "never", "always", "multiline"
    pub trailing_comma: String,
    /// Brace style: "same-line" or "new-line"
    pub brace_style: String,
    /// Spaces around operators
    pub spaces_around_operators: bool,
    /// Space after keywords
    pub spaces_after_keywords: bool,
    /// Uppercase keywords
    pub uppercase_keywords: bool,
    /// Max blank lines
    pub max_blank_lines: usize,
}

impl Default for LinterConfig {
    fn default() -> Self {
        LinterConfig {
            max_line_length: 100,
            check_naming: true,
            check_unused: true,
            check_security: true,
            check_performance: true,
            min_severity: 0,
            disabled_rules: Vec::new(),
        }
    }
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig {
            indent_style: "spaces".to_string(),
            indent_size: 2,
            line_length: 100,
            trailing_comma: "multiline".to_string(),
            brace_style: "same-line".to_string(),
            spaces_around_operators: true,
            spaces_after_keywords: true,
            uppercase_keywords: false,
            max_blank_lines: 2,
        }
    }
}

impl Default for KillerConfig {
    fn default() -> Self {
        KillerConfig {
            linter: LinterConfig::default(),
            formatter: FormatterConfig::default(),
            project_root: PathBuf::from("."),
        }
    }
}

impl KillerConfig {
    /// Load configuration from .killerrc file
    /// Searches from given path up to root
    pub fn load_from_path(start_path: &Path) -> Result<Self, ConfigError> {
        let config_file = find_config_file(start_path)?;
        Self::load_from_file(&config_file)
    }

    /// Load configuration from specific file
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(format!("Cannot read config: {}", e)))?;

        Self::parse_toml(&contents, path.parent().unwrap_or(Path::new(".")))
    }

    /// Parse TOML configuration
    fn parse_toml(content: &str, project_root: &Path) -> Result<Self, ConfigError> {
        // Simple TOML parser (not full-featured)
        let mut config = KillerConfig {
            project_root: project_root.to_path_buf(),
            ..Default::default()
        };

        let mut in_linter_section = false;
        let mut in_formatter_section = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Section headers
            if trimmed == "[linter]" {
                in_linter_section = true;
                in_formatter_section = false;
                continue;
            }
            if trimmed == "[formatter]" {
                in_formatter_section = true;
                in_linter_section = false;
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_linter_section = false;
                in_formatter_section = false;
                continue;
            }

            // Parse key = value pairs
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();

                if in_linter_section {
                    Self::set_linter_config(&mut config.linter, key, value)?;
                } else if in_formatter_section {
                    Self::set_formatter_config(&mut config.formatter, key, value)?;
                }
            }
        }

        Ok(config)
    }

    /// Set linter configuration from key-value pair
    fn set_linter_config(config: &mut LinterConfig, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "max_line_length" => {
                config.max_line_length = parse_usize(value)?;
            }
            "check_naming" => {
                config.check_naming = parse_bool(value)?;
            }
            "check_unused" => {
                config.check_unused = parse_bool(value)?;
            }
            "check_security" => {
                config.check_security = parse_bool(value)?;
            }
            "check_performance" => {
                config.check_performance = parse_bool(value)?;
            }
            "min_severity" => {
                config.min_severity = parse_u8(value)?;
            }
            "disabled_rules" => {
                config.disabled_rules = parse_string_list(value);
            }
            _ => {
                return Err(ConfigError::UnknownKey(format!("Unknown linter key: {}", key)));
            }
        }
        Ok(())
    }

    /// Set formatter configuration from key-value pair
    fn set_formatter_config(config: &mut FormatterConfig, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "indent_style" => {
                let style = unquote(value);
                if style != "spaces" && style != "tabs" {
                    return Err(ConfigError::InvalidValue(
                        format!("indent_style must be 'spaces' or 'tabs', got '{}'", style)
                    ));
                }
                config.indent_style = style;
            }
            "indent_size" => {
                config.indent_size = parse_usize(value)?;
            }
            "line_length" => {
                config.line_length = parse_usize(value)?;
            }
            "trailing_comma" => {
                let style = unquote(value);
                if !["never", "always", "multiline"].contains(&style.as_str()) {
                    return Err(ConfigError::InvalidValue(
                        format!("trailing_comma must be 'never', 'always', or 'multiline', got '{}'", style)
                    ));
                }
                config.trailing_comma = style;
            }
            "brace_style" => {
                let style = unquote(value);
                if style != "same-line" && style != "new-line" {
                    return Err(ConfigError::InvalidValue(
                        format!("brace_style must be 'same-line' or 'new-line', got '{}'", style)
                    ));
                }
                config.brace_style = style;
            }
            "spaces_around_operators" => {
                config.spaces_around_operators = parse_bool(value)?;
            }
            "spaces_after_keywords" => {
                config.spaces_after_keywords = parse_bool(value)?;
            }
            "uppercase_keywords" => {
                config.uppercase_keywords = parse_bool(value)?;
            }
            "max_blank_lines" => {
                config.max_blank_lines = parse_usize(value)?;
            }
            _ => {
                return Err(ConfigError::UnknownKey(format!("Unknown formatter key: {}", key)));
            }
        }
        Ok(())
    }

    /// Create a default config file
    pub fn create_default_file(path: &Path) -> Result<(), ConfigError> {
        let content = r#"# Killer Project Configuration
# See docs/CONFIG.md for all available options

[linter]
max_line_length = 100
check_naming = true
check_unused = true
check_security = true
check_performance = true
min_severity = 0
disabled_rules = []

[formatter]
indent_style = "spaces"
indent_size = 2
line_length = 100
trailing_comma = "multiline"
brace_style = "same-line"
spaces_around_operators = true
spaces_after_keywords = true
uppercase_keywords = false
max_blank_lines = 2
"#;

        fs::write(path, content)
            .map_err(|e| ConfigError::IoError(format!("Cannot write config: {}", e)))
    }
}

/// Configuration error types
#[derive(Debug)]
pub enum ConfigError {
    /// No config file found in hierarchy
    NotFound,
    /// IO error reading config
    IoError(String),
    /// Invalid syntax
    SyntaxError(String),
    /// Unknown configuration key
    UnknownKey(String),
    /// Invalid configuration value
    InvalidValue(String),
    /// Parse error
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "Config file not found"),
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::SyntaxError(e) => write!(f, "Syntax error: {}", e),
            ConfigError::UnknownKey(k) => write!(f, "Unknown key: {}", k),
            ConfigError::InvalidValue(v) => write!(f, "Invalid value: {}", v),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Find .killerrc file in directory hierarchy
fn find_config_file(start_path: &Path) -> Result<PathBuf, ConfigError> {
    let mut current = start_path.to_path_buf();

    loop {
        let config_path = current.join(".killerrc");
        if config_path.exists() {
            return Ok(config_path);
        }

        // Move to parent directory
        if let Some(parent) = current.parent() {
            if parent == current {
                // Reached filesystem root
                return Err(ConfigError::NotFound);
            }
            current = parent.to_path_buf();
        } else {
            return Err(ConfigError::NotFound);
        }
    }
}

/// Parse boolean value
fn parse_bool(value: &str) -> Result<bool, ConfigError> {
    let cleaned = remove_inline_comment(value);
    match unquote(&cleaned).to_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(ConfigError::ParseError(format!("Invalid boolean: {}", value)))
    }
}

/// Parse unsigned integer
fn parse_usize(value: &str) -> Result<usize, ConfigError> {
    let cleaned = remove_inline_comment(value);
    unquote(&cleaned).parse()
        .map_err(|_| ConfigError::ParseError(format!("Invalid number: {}", value)))
}

/// Parse unsigned 8-bit integer
fn parse_u8(value: &str) -> Result<u8, ConfigError> {
    let cleaned = remove_inline_comment(value);
    unquote(&cleaned).parse()
        .map_err(|_| ConfigError::ParseError(format!("Invalid number: {}", value)))
}

/// Parse comma-separated list
fn parse_string_list(value: &str) -> Vec<String> {
    let cleaned = remove_inline_comment(value);
    unquote(&cleaned)
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Remove inline comments from value (e.g., "100 # comment" -> "100")
fn remove_inline_comment(value: &str) -> String {
    if let Some(pos) = value.find('#') {
        value[..pos].to_string()
    } else {
        value.to_string()
    }
}

/// Remove quotes from string value
fn unquote(value: &str) -> String {
    let s = value.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KillerConfig::default();
        assert_eq!(config.linter.max_line_length, 100);
        assert_eq!(config.formatter.indent_size, 2);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("yes").unwrap(), true);
        assert_eq!(parse_bool("no").unwrap(), false);
    }

    #[test]
    fn test_parse_usize() {
        assert_eq!(parse_usize("100").unwrap(), 100);
        assert_eq!(parse_usize("42").unwrap(), 42);
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("'world'"), "world");
        assert_eq!(unquote("plain"), "plain");
    }

    #[test]
    fn test_parse_string_list() {
        let list = parse_string_list("\"rule1, rule2, rule3\"");
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], "rule1");
    }

    #[test]
    fn test_parse_config_basic() {
        let content = r#"
[linter]
max_line_length = 120
check_naming = true

[formatter]
indent_size = 4
line_length = 120
"#;
        let config = KillerConfig::parse_toml(content, Path::new(".")).unwrap();
        assert_eq!(config.linter.max_line_length, 120);
        assert_eq!(config.formatter.indent_size, 4);
    }

    #[test]
    fn test_linter_config_defaults() {
        let config = LinterConfig::default();
        assert_eq!(config.check_naming, true);
        assert_eq!(config.check_security, true);
    }

    #[test]
    fn test_formatter_config_defaults() {
        let config = FormatterConfig::default();
        assert_eq!(config.indent_style, "spaces");
        assert_eq!(config.trailing_comma, "multiline");
    }

    #[test]
    fn test_invalid_indent_style() {
        let content = r#"
[formatter]
indent_style = "spaces or tabs"
"#;
        let result = KillerConfig::parse_toml(content, Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_trailing_comma_style() {
        let content = r#"
[formatter]
trailing_comma = "invalid"
"#;
        let result = KillerConfig::parse_toml(content, Path::new("."));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_comments() {
        let content = r#"
# Configuration file
[linter]
max_line_length = 100  # Set max line length
check_naming = true
"#;
        let config = KillerConfig::parse_toml(content, Path::new(".")).unwrap();
        assert_eq!(config.linter.max_line_length, 100);
    }

    #[test]
    fn test_disabled_rules() {
        let content = r#"
[linter]
disabled_rules = "rule1, rule2, rule3"
"#;
        let config = KillerConfig::parse_toml(content, Path::new(".")).unwrap();
        assert_eq!(config.linter.disabled_rules.len(), 3);
        assert_eq!(config.linter.disabled_rules[0], "rule1");
    }
}
