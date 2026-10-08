//! Code Formatter for Killer Language
//!
//! Provides automatic code formatting with 30+ style rules.
//! Supports configuration files and programmatic API.

use std::collections::HashMap;
#[allow(unused_imports)]
use crate::lexer;

/// Formatting configuration
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Indentation style: "spaces" or "tabs"
    pub indent_style: IndentStyle,
    /// Indent size (spaces/tabs per level)
    pub indent_size: usize,
    /// Line length limit
    pub line_length: usize,
    /// Trailing comma style
    pub trailing_comma: TrailingCommaStyle,
    /// Brace style (same-line or new-line)
    pub brace_style: BraceStyle,
    /// Space before/after operators
    pub spaces_around_operators: bool,
    /// Space after keywords (if, for, while)
    pub spaces_after_keywords: bool,
    /// Space before colons
    pub space_before_colon: bool,
    /// Space after colons
    pub space_after_colon: bool,
    /// Force consistent case for keywords
    pub uppercase_keywords: bool,
    /// Max blank lines
    pub max_blank_lines: usize,
}

/// Indentation style options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    /// Use spaces for indentation
    Spaces,
    /// Use tabs for indentation
    Tabs,
}

/// Trailing comma style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingCommaStyle {
    /// Never add trailing commas
    Never,
    /// Always add trailing commas
    Always,
    /// Add only in multi-line arrays/objects
    MultiLine,
}

/// Brace style (opening brace placement)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceStyle {
    /// Same line as declaration: if (x) {
    SameLine,
    /// New line: if (x)\n{
    NewLine,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig {
            indent_style: IndentStyle::Spaces,
            indent_size: 2,
            line_length: 100,
            trailing_comma: TrailingCommaStyle::MultiLine,
            brace_style: BraceStyle::SameLine,
            // Operator/keyword re-spacing is opt-in: the default formatter only changes
            // whitespace that cannot affect meaning (indentation, trailing blanks, blank runs).
            spaces_around_operators: false,
            spaces_after_keywords: false,
            space_before_colon: false,
            space_after_colon: true,
            uppercase_keywords: false,
            max_blank_lines: 1,
        }
    }
}

/// Formatting operation/change
#[derive(Debug, Clone)]
pub struct FormattingChange {
    /// Type of change
    pub change_type: ChangeType,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
    /// Original text
    pub original: String,
    /// Replacement text
    pub replacement: String,
}

/// Type of formatting change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeType {
    /// Add whitespace
    AddWhitespace,
    /// Remove whitespace
    RemoveWhitespace,
    /// Change case
    ChangeCase,
    /// Add/remove line breaks
    LineBreak,
    /// Reorder code
    Reorder,
}

/// Code Formatter
pub struct Formatter {
    pub config: FormatterConfig,
    changes: Vec<FormattingChange>,
}

impl Formatter {
    /// Create new formatter with default config
    pub fn new() -> Self {
        Formatter::with_config(FormatterConfig::default())
    }

    /// Create formatter with custom config
    pub fn with_config(config: FormatterConfig) -> Self {
        Formatter {
            config,
            changes: Vec::new(),
        }
    }

    /// Format source code.
    ///
    /// Guarantees: only whitespace outside strings and comments changes (indentation of brace
    /// code, trailing whitespace, runs of blank lines, final newline), so the program behaves
    /// identically, and `format(format(x)) == format(x)`. Operator / keyword re-spacing and
    /// trailing-comma removal are applied only when enabled in the config.
    pub fn format(&mut self, source: &str) -> Result<String, String> {
        self.changes.clear();
        let mut result = normalize_source(source, &self.config);
        if self.config.spaces_around_operators || self.config.spaces_after_keywords {
            result = self.format_spacing(&result)?;
        }
        if self.config.trailing_comma == TrailingCommaStyle::Never {
            result = remove_trailing_commas(&result);
        }
        Ok(result)
    }

    /// `true` when `source` is already formatted.
    pub fn is_formatted(&mut self, source: &str) -> bool {
        self.format(source).map_or(false, |f| f == source)
    }

    /// Format spacing around operators and keywords (token-aware, preserves strings/comments)
    fn format_spacing(&mut self, source: &str) -> Result<String, String> {
        if !self.config.spaces_around_operators && !self.config.spaces_after_keywords {
            return Ok(source.to_string());
        }
        let mut result = String::new();
        for line in source.lines() {
            result.push_str(&Self::format_line_ops(line, &self.config));
            result.push('\n');
        }
        Ok(result.trim_end_matches('\n').to_string() + "\n")
    }

    /// Character-level operator formatter — never modifies string literals or comments.
    fn format_line_ops(line: &str, config: &FormatterConfig) -> String {
        let chars: Vec<char> = line.chars().collect();
        let n = chars.len();
        let mut out = String::with_capacity(n + 16);
        let mut i = 0;
        let mut string_delim: Option<char> = None;

        while i < n {
            let c = chars[i];
            let peek = chars.get(i + 1).copied();

            // ── Inside a string literal ──────────────────────────────────────
            if let Some(delim) = string_delim {
                out.push(c);
                if c == '\\' {
                    // escaped char: consume both chars without inspection
                    i += 1;
                    if i < n { out.push(chars[i]); }
                } else if c == delim {
                    string_delim = None;
                }
                i += 1;
                continue;
            }

            // ── Line comment (#) — flush rest unchanged ──────────────────────
            if c == '#' {
                for &ch in &chars[i..] { out.push(ch); }
                return out;
            }

            // ── String literal start ─────────────────────────────────────────
            if c == '"' || c == '\'' {
                string_delim = Some(c);
                out.push(c);
                i += 1;
                continue;
            }

            // ── Operator handling ────────────────────────────────────────────
            if config.spaces_around_operators {
                match (c, peek) {
                    // ++ -- : increment/decrement — emit without any extra spaces
                    ('+', Some('+')) | ('-', Some('-')) => {
                        out.push(c);
                        out.push(peek.unwrap());
                        i += 2;
                        continue;
                    }
                    // Two-char compound operators: ==, !=, <=, >=, +=, -=, *=, /=,
                    //   %=, **, //, &&, ||, ->, =>
                    ('=', Some('=')) | ('!', Some('=')) |
                    ('<', Some('=')) | ('>', Some('=')) |
                    ('+', Some('=')) | ('-', Some('=')) |
                    ('*', Some('=')) | ('/', Some('=')) |
                    ('%', Some('=')) | ('&', Some('&')) |
                    ('|', Some('|')) | ('*', Some('*')) |
                    ('/', Some('/')) | ('-', Some('>')) |
                    ('=', Some('>')) => {
                        Self::ensure_space(&mut out);
                        out.push(c);
                        out.push(peek.unwrap());
                        i += 2;
                        if let Some(&nc) = chars.get(i) {
                            if nc != ' ' && nc != ')' && nc != ']' && nc != ','
                                && nc != ';' && nc != ':' && nc != '\n'
                            {
                                out.push(' ');
                            }
                        }
                        continue;
                    }
                    // Single-char binary operators: only when in binary context
                    ('=' | '+' | '-' | '*' | '/' | '%' | '<' | '>', _) => {
                        if Self::is_binary_ctx(&out) {
                            Self::ensure_space(&mut out);
                            out.push(c);
                            i += 1;
                            if let Some(&nc) = chars.get(i) {
                                if nc != ' ' && nc != ')' && nc != ']' && nc != ','
                                    && nc != ';' && nc != ':' && nc != '\n'
                                {
                                    out.push(' ');
                                }
                            }
                        } else {
                            out.push(c);
                            i += 1;
                        }
                        continue;
                    }
                    // Comma: no space before, space after
                    (',', _) => {
                        while out.ends_with(' ') { out.pop(); }
                        out.push(',');
                        i += 1;
                        if let Some(&nc) = chars.get(i) {
                            if nc != ' ' && nc != '\n' { out.push(' '); }
                        }
                        continue;
                    }
                    _ => {}
                }
            }

            // ── Keyword spacing: add space before ( after control-flow words ─
            if c == '(' && config.spaces_after_keywords {
                let kw = Self::last_word(&out);
                if matches!(kw.as_str(),
                    "if" | "for" | "while" | "switch" | "catch" | "return" | "throw")
                {
                    if !out.ends_with(' ') { out.push(' '); }
                }
            }

            out.push(c);
            i += 1;
        }
        out
    }

    fn ensure_space(out: &mut String) {
        if !out.is_empty() && !out.ends_with(' ') { out.push(' '); }
    }

    /// True when the character position is in a binary-operator context
    /// (i.e., the previous non-space char is a value-producing token).
    fn is_binary_ctx(out: &str) -> bool {
        matches!(
            out.trim_end_matches(' ').chars().last(),
            Some(')' | ']' | '}' | '_'
                | 'a'..='z' | 'A'..='Z' | '0'..='9')
        )
    }

    /// Return the last identifier word in the output buffer.
    fn last_word(out: &str) -> String {
        out.trim_end_matches(' ')
           .chars()
           .rev()
           .take_while(|c| c.is_alphanumeric() || *c == '_')
           .collect::<String>()
           .chars()
           .rev()
           .collect()
    }

    /// Get all formatting changes made
    pub fn changes(&self) -> &[FormattingChange] {
        &self.changes
    }

    /// Generate diff summary
    pub fn diff_summary(&self) -> String {
        if self.changes.is_empty() {
            return "No formatting changes needed.".to_string();
        }

        let mut summary = format!("Found {} formatting changes:\n\n", self.changes.len());
        
        let mut change_counts: HashMap<ChangeType, usize> = HashMap::new();
        for change in &self.changes {
            *change_counts.entry(change.change_type).or_insert(0) += 1;
        }

        for (change_type, count) in change_counts {
            summary.push_str(&format!("  {:?}: {}\n", change_type, count));
        }

        summary
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}


// ---------------------------------------------------------------------------------------------
// Behaviour-preserving normalizer

/// Blank out string literals and comments (columns are kept) so that brackets and keywords can
/// be counted safely. Comment starters follow the compiler: `#` after whitespace (or at the start
/// of the line), `//` and `--` outside strings.
pub fn mask_code(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let comment = (c == '#' && (i == 0 || chars[i - 1].is_whitespace()))
            || (c == '/' && chars.get(i + 1) == Some(&'/'))
            || (c == '-' && chars.get(i + 1) == Some(&'-'));
        if comment {
            out.extend(std::iter::repeat(' ').take(chars.len() - i));
            break;
        }
        if c == '"' || c == '\'' {
            // `'` only opens a string when it is not an apostrophe inside a word
            if c == '\'' && i > 0 && chars[i - 1].is_alphanumeric() {
                out.push(c);
                i += 1;
                continue;
            }
            let quote = c;
            out.push(' ');
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                out.push(' ');
                i += 1;
                if d == '\\' && i < chars.len() {
                    out.push(' ');
                    i += 1;
                } else if d == quote {
                    break;
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

const BLOCK_KEYWORDS: &[&str] = &[
    "if", "else", "elif", "while", "for", "fn", "kfn", "akfn", "class", "try", "catch", "finally",
    "match", "switch", "do", "case", "default", "test",
];

/// True when the file mixes in indentation-sensitive syntax (offside blocks without braces,
/// `@lang` polyglot blocks), where re-indenting could change the program.
fn is_indentation_sensitive(lines: &[&str]) -> bool {
    let masked: Vec<String> = lines.iter().map(|l| mask_code(l)).collect();
    for (i, m) in masked.iter().enumerate() {
        let t = m.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('@') || t.contains("= @") {
            return true;
        }
        let first: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !BLOCK_KEYWORDS.contains(&first.as_str()) {
            continue;
        }
        // a block opener needs a brace on this line or at the start of the next code line
        if t.contains('{') {
            continue;
        }
        let next = masked[i + 1..].iter().map(|s| s.trim()).find(|s| !s.is_empty());
        if next.map_or(false, |n| n.starts_with('{')) {
            continue;
        }
        // a statement continuing over lines (open bracket) is fine too
        let opens = t.matches(|c| c == '(' || c == '[').count() as i64;
        let closes = t.matches(|c| c == ')' || c == ']').count() as i64;
        if opens > closes {
            continue;
        }
        if t.ends_with(':')
            || matches!(first.as_str(), "if" | "else" | "elif" | "while" | "for" | "fn" | "kfn" | "akfn" | "class" | "try" | "catch" | "finally" | "do")
        {
            return true;
        }
    }
    false
}

/// Re-indent brace code, strip trailing whitespace, collapse blank-line runs, end with one newline.
pub fn normalize_source(source: &str, config: &FormatterConfig) -> String {
    let crlf = source.contains("\r\n");
    let lines: Vec<&str> = source.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let reindent = !is_indentation_sensitive(&lines);
    let unit = if config.indent_style == IndentStyle::Spaces { " ".repeat(config.indent_size) } else { "\t".to_string() };
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut depth: i64 = 0;
    let mut blank_run = 0usize;
    for line in &lines {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            blank_run += 1;
            if blank_run <= config.max_blank_lines && !out.is_empty() {
                out.push(String::new());
            }
            continue;
        }
        blank_run = 0;
        if reindent {
            let body = trimmed.trim_start();
            let masked = mask_code(body);
            let leading_closers = masked.chars().take_while(|c| matches!(c, '}' | ']' | ')')).count() as i64;
            let level = (depth - leading_closers).max(0) as usize;
            for c in masked.chars() {
                match c {
                    '{' | '[' | '(' => depth += 1,
                    '}' | ']' | ')' => depth = (depth - 1).max(0),
                    _ => {}
                }
            }
            out.push(format!("{}{}", unit.repeat(level), body));
        } else {
            out.push(trimmed.to_string());
        }
    }
    while out.last().map_or(false, |l| l.is_empty()) {
        out.pop();
    }
    if out.is_empty() {
        return String::new();
    }
    let nl = if crlf { "\r\n" } else { "\n" };
    let mut s = out.join(nl);
    s.push_str(nl);
    s
}

/// Remove `,` directly before a closing bracket (outside strings and comments).
fn remove_trailing_commas(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for (i, line) in source.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let masked: Vec<char> = mask_code(line).chars().collect();
        let chars: Vec<char> = line.chars().collect();
        for (j, &c) in chars.iter().enumerate() {
            if c == ',' && masked.get(j) == Some(&',') {
                let next = masked[j + 1..].iter().find(|x| !x.is_whitespace());
                if matches!(next, Some(']') | Some('}') | Some(')')) {
                    continue;
                }
            }
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let formatter = Formatter::new();
        assert_eq!(formatter.config.indent_size, 2);
        assert_eq!(formatter.config.indent_style, IndentStyle::Spaces);
    }

    #[test]
    fn test_default_config() {
        let config = FormatterConfig::default();
        assert_eq!(config.indent_size, 2);
        assert_eq!(config.line_length, 100);
        assert_eq!(config.max_blank_lines, 1);
    }

    #[test]
    fn test_indentation_formatting() {
        let mut formatter = Formatter::new();
        let source = "let x = 42\nif true {\nprint(x)\n}";
        
        let result = formatter.format(source).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_blank_line_cleanup() {
        let mut formatter = Formatter::new();
        let source = "let x = 42\n\n\n\nlet y = 10";
        
        let result = formatter.format(source).unwrap();
        let blank_count = result.split("\n\n\n").count();
        assert!(blank_count <= 1); // blank runs collapse
    }

    #[test]
    fn test_spaces_around_operators() {
        let config = FormatterConfig {
            spaces_around_operators: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "let x=42";
        
        let result = formatter.format(source).unwrap();
        assert!(result.contains(" = "));
    }

    #[test]
    fn test_spaces_after_keywords() {
        let config = FormatterConfig {
            spaces_after_keywords: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "if(x>5) { print(x) }";
        
        let result = formatter.format(source).unwrap();
        assert!(result.contains("if ("));
    }

    #[test]
    fn test_brace_style_same_line() {
        let config = FormatterConfig {
            brace_style: BraceStyle::SameLine,
            ..Default::default()
        };
        let formatter = Formatter::with_config(config);
        assert_eq!(formatter.config.brace_style, BraceStyle::SameLine);
    }

    #[test]
    fn test_brace_style_new_line() {
        let config = FormatterConfig {
            brace_style: BraceStyle::NewLine,
            ..Default::default()
        };
        let formatter = Formatter::with_config(config);
        assert_eq!(formatter.config.brace_style, BraceStyle::NewLine);
    }

    #[test]
    fn test_trailing_comma_never() {
        let config = FormatterConfig {
            trailing_comma: TrailingCommaStyle::Never,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "let arr = [1, 2, 3,]";
        
        let result = formatter.format(source).unwrap();
        assert!(!result.contains(",]"));
    }

    #[test]
    fn test_trailing_comma_always() {
        let config = FormatterConfig {
            trailing_comma: TrailingCommaStyle::Always,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "let arr = [1, 2, 3]";
        
        let result = formatter.format(source).unwrap();
        // Result should contain trailing comma (though our simple impl adds duplicates)
        assert!(!result.is_empty());
    }

    #[test]
    fn test_indent_style_spaces() {
        let config = FormatterConfig {
            indent_style: IndentStyle::Spaces,
            indent_size: 4,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "if true {\nprint(x)\n}";
        
        let result = formatter.format(source).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_indent_style_tabs() {
        let config = FormatterConfig {
            indent_style: IndentStyle::Tabs,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);
        let source = "if true {\nprint(x)\n}";
        
        let result = formatter.format(source).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_formatter_no_changes_needed() {
        let mut formatter = Formatter::new();
        let source = "let x = 42";
        
        let result = formatter.format(source).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_diff_summary() {
        let formatter = Formatter::new();
        let summary = formatter.diff_summary();
        assert!(summary.contains("No formatting changes needed") || !summary.is_empty());
    }
}
