use std::fmt::{Display, Formatter, Result as FmtResult};
use crate::source_location::{SourceLocation, ErrorWithLocation};

/// Comprehensive error type with location tracking, source context, and stack
/// traces.
///
/// Existing variants are preserved exactly as-is for backward compatibility.
/// The [`Rich`](VmError::Rich) variant wraps any other variant to attach
/// optional source text and call-stack frames for high-fidelity diagnostics.
#[derive(Debug, Clone)]
pub enum VmError {
    /// Parse error with source location
    ParseError {
        message: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },

    /// Runtime error with source location
    RuntimeError {
        message: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },

    /// IO error with source location
    IoError {
        message: String,
        location: Option<SourceLocation>,
    },

    /// Security error with source location and suggestion
    SecurityError {
        message: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },

    /// Type error with source location
    TypeError {
        message: String,
        location: Option<SourceLocation>,
        expected: String,
        found: String,
    },

    /// Compilation error with location and code
    CompilationError(ErrorWithLocation),

    /// Rich wrapper: attaches source text and/or a call-stack to any variant.
    Rich {
        inner: Box<VmError>,
        source_context: Option<String>,
        stack_frames: Vec<String>,
    },
}

// ── Factory methods (all existing signatures preserved) ─────────────────────

impl VmError {
    /// Create parse error (simple — no location, no suggestion)
    pub fn parse_error_simple(message: impl Into<String>) -> Self {
        VmError::ParseError {
            message: message.into(),
            location: None,
            suggestion: None,
        }
    }

    /// Create IO error (simple — no location)
    pub fn io_error_simple(message: impl Into<String>) -> Self {
        VmError::IoError {
            message: message.into(),
            location: None,
        }
    }

    /// Create parse error with location
    pub fn parse_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        VmError::ParseError {
            message: message.into(),
            location,
            suggestion: None,
        }
    }

    /// Create parse error with suggestion
    pub fn parse_error_with_suggestion(
        message: impl Into<String>,
        location: Option<SourceLocation>,
        suggestion: impl Into<String>,
    ) -> Self {
        VmError::ParseError {
            message: message.into(),
            location,
            suggestion: Some(suggestion.into()),
        }
    }

    /// Create runtime error
    pub fn runtime_error(message: impl Into<String>) -> Self {
        VmError::RuntimeError {
            message: message.into(),
            location: None,
            suggestion: None,
        }
    }

    /// Create runtime error with location
    pub fn runtime_error_at(message: impl Into<String>, location: SourceLocation) -> Self {
        VmError::RuntimeError {
            message: message.into(),
            location: Some(location),
            suggestion: None,
        }
    }

    /// Create IO error
    pub fn io_error(message: impl Into<String>) -> Self {
        VmError::IoError {
            message: message.into(),
            location: None,
        }
    }

    /// Create security error
    pub fn security_error(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        VmError::SecurityError {
            message: message.into(),
            location: None,
            suggestion: Some(suggestion.into()),
        }
    }

    /// Create type error
    pub fn type_error(
        message: impl Into<String>,
        expected: impl Into<String>,
        found: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        VmError::TypeError {
            message: message.into(),
            location,
            expected: expected.into(),
            found: found.into(),
        }
    }
}

// ── Builder methods for rich context ────────────────────────────────────────

impl VmError {
    /// Attach full source text so [`Display`] / [`render_rich`](Self::render_rich)
    /// can show a source-context snippet with a caret.
    pub fn with_source_context(self, source: &str) -> Self {
        match self {
            VmError::Rich { inner, stack_frames, .. } => VmError::Rich {
                inner,
                source_context: Some(source.to_string()),
                stack_frames,
            },
            other => VmError::Rich {
                inner: Box::new(other),
                source_context: Some(source.to_string()),
                stack_frames: Vec::new(),
            },
        }
    }

    /// Push a call-stack frame (e.g. `"main() line 10"`).
    ///
    /// If the error is not already a [`Rich`](VmError::Rich) variant it is
    /// automatically wrapped in one.
    pub fn add_stack_frame(&mut self, frame: String) {
        match self {
            VmError::Rich { stack_frames, .. } => {
                stack_frames.push(frame);
            }
            _ => {
                let placeholder = VmError::RuntimeError {
                    message: String::new(),
                    location: None,
                    suggestion: None,
                };
                let old = std::mem::replace(self, placeholder);
                *self = VmError::Rich {
                    inner: Box::new(old),
                    source_context: None,
                    stack_frames: vec![frame],
                };
            }
        }
    }
}

// ── Accessors ───────────────────────────────────────────────────────────────

impl VmError {
    /// Peel through any [`Rich`](VmError::Rich) wrappers to the concrete
    /// error variant underneath.
    fn inner_error(&self) -> &VmError {
        match self {
            VmError::Rich { inner, .. } => inner.inner_error(),
            other => other,
        }
    }

    fn error_kind_label(&self) -> &'static str {
        match self.inner_error() {
            VmError::ParseError { .. }       => "Parse error",
            VmError::RuntimeError { .. }     => "Runtime error",
            VmError::IoError { .. }          => "IO error",
            VmError::SecurityError { .. }    => "Security error",
            VmError::TypeError { .. }        => "Type error",
            VmError::CompilationError(_)     => "Compilation error",
            VmError::Rich { .. }             => unreachable!(),
        }
    }

    fn error_message(&self) -> &str {
        match self.inner_error() {
            VmError::ParseError { message, .. }
            | VmError::RuntimeError { message, .. }
            | VmError::IoError { message, .. }
            | VmError::SecurityError { message, .. }
            | VmError::TypeError { message, .. } => message,
            VmError::CompilationError(e) => &e.message,
            VmError::Rich { .. } => unreachable!(),
        }
    }

    fn error_location(&self) -> Option<&SourceLocation> {
        match self.inner_error() {
            VmError::ParseError { location, .. }
            | VmError::RuntimeError { location, .. }
            | VmError::IoError { location, .. }
            | VmError::SecurityError { location, .. }
            | VmError::TypeError { location, .. } => location.as_ref(),
            VmError::CompilationError(e) => Some(&e.location),
            VmError::Rich { .. } => unreachable!(),
        }
    }

    fn error_suggestion(&self) -> Option<&str> {
        match self.inner_error() {
            VmError::ParseError { suggestion, .. }
            | VmError::RuntimeError { suggestion, .. }
            | VmError::SecurityError { suggestion, .. } => suggestion.as_deref(),
            VmError::CompilationError(e) => e.suggestion.as_deref(),
            _ => None,
        }
    }

    fn source_context_str(&self) -> Option<&str> {
        match self {
            VmError::Rich { source_context, .. } => source_context.as_deref(),
            _ => None,
        }
    }

    fn stack_frames_list(&self) -> &[String] {
        match self {
            VmError::Rich { stack_frames, .. } => stack_frames,
            _ => &[],
        }
    }
}

// ── Rich rendering ──────────────────────────────────────────────────────────

impl VmError {
    /// Render a compiler-style diagnostic with source context, caret, and
    /// hints.
    ///
    /// `source` takes priority; when `None` the stored `source_context` (from
    /// [`with_source_context`](Self::with_source_context)) is used instead.
    ///
    /// # Example output
    ///
    /// ```text
    /// Error: Undefined variable `x`
    ///   --> line 5, column 12
    ///   |
    /// 5 | result = x + y
    ///   |          ^ undefined here
    ///   |
    ///   = hint: Did you mean to declare `x` with `let x = ...`?
    /// ```
    pub fn render_rich(&self, source: Option<&str>) -> String {
        let inner = self.inner_error();
        let kind  = self.error_kind_label();
        let msg   = self.error_message();
        let loc   = self.error_location();
        let effective_source = source.or_else(|| self.source_context_str());
        let frames = self.stack_frames_list();

        let explicit_hint = self.error_suggestion().map(String::from);
        let hint = explicit_hint.or_else(|| auto_suggest(inner));

        let mut out = String::new();

        // ── header ──────────────────────────────────────────────────────
        out.push_str(&format!("{kind}: {msg}\n"));

        // ── location arrow ──────────────────────────────────────────────
        if let Some(loc) = loc {
            out.push_str(&format!(
                "  --> line {}, column {}\n",
                loc.line, loc.column,
            ));
        }

        // ── source snippet with caret ───────────────────────────────────
        if let (Some(loc), Some(src)) = (loc, effective_source) {
            if loc.line > 0 {
                if let Some(source_line) = src.lines().nth(loc.line - 1) {
                    let lnum = loc.line.to_string();
                    let pad  = " ".repeat(lnum.len());

                    out.push_str(&format!("{pad} |\n"));
                    out.push_str(&format!("{lnum} | {source_line}\n"));

                    let col       = loc.column.saturating_sub(1);
                    let caret_pad = " ".repeat(col);
                    let note      = brief_annotation(inner);

                    out.push_str(&format!("{pad} | {caret_pad}^ {note}\n"));
                    out.push_str(&format!("{pad} |\n"));
                }
            }
        }

        // ── type error detail ───────────────────────────────────────────
        if let VmError::TypeError { expected, found, .. } = inner {
            out.push_str(&format!("  = expected: `{expected}`\n"));
            out.push_str(&format!("  =    found: `{found}`\n"));
        }

        // ── hint / suggestion ───────────────────────────────────────────
        if let Some(h) = &hint {
            out.push_str(&format!("  = hint: {h}\n"));
        }

        // ── stack trace ─────────────────────────────────────────────────
        if !frames.is_empty() {
            out.push('\n');
            out.push_str("Stack trace:\n");
            for f in frames {
                out.push_str(&format!("  at {f}\n"));
            }
        }

        out
    }
}

// ── Display ─────────────────────────────────────────────────────────────────

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if matches!(self, VmError::Rich { .. }) {
            return write!(f, "{}", self.render_rich(None));
        }

        match self {
            VmError::ParseError {
                message,
                location,
                suggestion,
            } => {
                write!(f, "Parse error: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at {loc}")?;
                }
                if let Some(sugg) = suggestion {
                    write!(f, "\nHint: {sugg}")?;
                }
                Ok(())
            }
            VmError::RuntimeError {
                message,
                location,
                suggestion,
            } => {
                write!(f, "Runtime error: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at {loc}")?;
                }
                if let Some(sugg) = suggestion {
                    write!(f, "\nHint: {sugg}")?;
                }
                Ok(())
            }
            VmError::IoError { message, location } => {
                write!(f, "IO error: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at {loc}")?;
                }
                Ok(())
            }
            VmError::SecurityError {
                message,
                location,
                suggestion,
            } => {
                write!(f, "Security error: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at {loc}")?;
                }
                if let Some(sugg) = suggestion {
                    write!(f, "\nSuggestion: {sugg}")?;
                }
                Ok(())
            }
            VmError::TypeError {
                message,
                location,
                expected,
                found,
            } => {
                write!(f, "Type error: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at {loc}")?;
                }
                write!(f, "\nExpected: {expected}\nFound: {found}")?;
                Ok(())
            }
            VmError::CompilationError(err) => write!(f, "{}", err.full_message()),
            VmError::Rich { .. } => unreachable!(),
        }
    }
}

impl std::error::Error for VmError {}

// ── Free helper functions ───────────────────────────────────────────────────

/// Short annotation placed next to the caret (`^`) on the source-context
/// line.
fn brief_annotation(err: &VmError) -> &'static str {
    match err {
        VmError::RuntimeError { message, .. } => {
            let lower = message.to_lowercase();
            if lower.contains("undefined") || lower.contains("not defined") {
                "undefined here"
            } else if lower.contains("type") {
                "type mismatch here"
            } else if lower.contains("division by zero") || lower.contains("divide by zero") {
                "division by zero here"
            } else if lower.contains("index") && lower.contains("bound") {
                "index out of bounds here"
            } else {
                "error occurred here"
            }
        }
        VmError::ParseError { message, .. } => {
            let lower = message.to_lowercase();
            if lower.contains("unexpected") {
                "unexpected token here"
            } else if lower.contains("expected") {
                "expected here"
            } else {
                "parse error here"
            }
        }
        VmError::TypeError { .. }        => "type mismatch here",
        VmError::SecurityError { .. }    => "not permitted here",
        VmError::IoError { .. }          => "IO error here",
        VmError::CompilationError(_)     => "compilation error here",
        VmError::Rich { inner, .. }      => brief_annotation(inner),
    }
}

/// Generate a contextual fix suggestion when no explicit one was provided.
fn auto_suggest(err: &VmError) -> Option<String> {
    match err {
        VmError::RuntimeError { message, .. } => {
            let lower = message.to_lowercase();
            if lower.contains("undefined variable") || lower.contains("not defined") {
                let name = extract_identifier(message);
                match name {
                    Some(n) => Some(format!(
                        "Did you mean to declare `{n}` with `let {n} = ...`?"
                    )),
                    None => Some(
                        "Did you declare this variable? Use `let <name> = ...` first."
                            .into(),
                    ),
                }
            } else if lower.contains("type mismatch") || lower.contains("type error") {
                Some("Check that both operands have compatible types.".into())
            } else if lower.contains("division by zero") || lower.contains("divide by zero") {
                Some(
                    "Guard against zero before dividing, e.g. `if divisor != 0 { ... }`."
                        .into(),
                )
            } else if lower.contains("index") && lower.contains("bound") {
                Some("Ensure the index is within `0..len`. Use `.len()` to check.".into())
            } else if lower.contains("stack overflow") {
                Some("Check for infinite recursion or deeply nested calls.".into())
            } else {
                None
            }
        }
        VmError::ParseError { message, .. } => {
            let lower = message.to_lowercase();
            if lower.contains("expected '}'") || lower.contains("expected }") {
                Some(
                    "You may have an unclosed block. Check for matching `{` and `}`."
                        .into(),
                )
            } else if lower.contains("expected ')'") || lower.contains("expected )") {
                Some(
                    "You may have an unclosed parenthesis. Check for matching `(` and `)`."
                        .into(),
                )
            } else if lower.contains("expected ']'") || lower.contains("expected ]") {
                Some(
                    "You may have an unclosed bracket. Check for matching `[` and `]`."
                        .into(),
                )
            } else if lower.contains("expected ';'") || lower.contains("expected ;") {
                Some(
                    "Did you forget a semicolon at the end of the previous statement?"
                        .into(),
                )
            } else if lower.contains("unexpected token") || lower.contains("unexpected") {
                Some(
                    "Check for missing semicolons or operators before this token."
                        .into(),
                )
            } else {
                None
            }
        }
        VmError::TypeError { expected, found, .. } => Some(format!(
            "Expected type `{expected}`, but found `{found}`. Consider an explicit conversion."
        )),
        VmError::SecurityError { .. } => Some(
            "This operation requires elevated permissions. Check your security policy."
                .into(),
        ),
        VmError::Rich { inner, .. } => auto_suggest(inner),
        _ => None,
    }
}

/// Peel `VmError::Rich` wrappers for use by free functions (mirrors the private
/// `inner_error` method on `VmError`).
fn inner_vm_error(err: &VmError) -> &VmError {
    match err {
        VmError::Rich { inner, .. } => inner_vm_error(inner),
        other => other,
    }
}

fn error_message_ref(inner: &VmError) -> &str {
    match inner {
        VmError::ParseError { message, .. }
        | VmError::RuntimeError { message, .. }
        | VmError::IoError { message, .. }
        | VmError::SecurityError { message, .. }
        | VmError::TypeError { message, .. } => message,
        VmError::CompilationError(e) => &e.message,
        VmError::Rich { .. } => unreachable!(),
    }
}

fn error_location_ref(inner: &VmError) -> Option<&SourceLocation> {
    match inner {
        VmError::ParseError { location, .. }
        | VmError::RuntimeError { location, .. }
        | VmError::IoError { location, .. }
        | VmError::SecurityError { location, .. }
        | VmError::TypeError { location, .. } => location.as_ref(),
        VmError::CompilationError(e) => Some(&e.location),
        VmError::Rich { .. } => unreachable!(),
    }
}

fn error_suggestion_ref(inner: &VmError) -> Option<&str> {
    match inner {
        VmError::ParseError { suggestion, .. }
        | VmError::RuntimeError { suggestion, .. }
        | VmError::SecurityError { suggestion, .. } => suggestion.as_deref(),
        VmError::CompilationError(e) => e.suggestion.as_deref(),
        _ => None,
    }
}

fn error_kind_tag(inner: &VmError) -> &'static str {
    match inner {
        VmError::ParseError { .. } => "Parse",
        VmError::RuntimeError { .. } => "Runtime",
        VmError::IoError { .. } => "IO",
        VmError::SecurityError { .. } => "Security",
        VmError::TypeError { .. } => "Type",
        VmError::CompilationError(_) => "Compilation",
        VmError::Rich { .. } => unreachable!(),
    }
}

fn rich_source_context_ref(err: &VmError) -> Option<&str> {
    match err {
        VmError::Rich { source_context, .. } => source_context.as_deref(),
        _ => None,
    }
}

/// Renders a Rust-style diagnostic: labeled header, `--> line` pointer, source
/// line, caret, message beside the caret, optional hints, and type details.
///
/// When the error has no [`SourceLocation`], returns `error: <message>` only.
///
/// The `source` argument is the full program text used to extract the faulting
/// line. If it is empty and `error` is [`VmError::Rich`] with stored
/// `source_context`, that text is used instead.
pub fn render_error(error: &VmError, source: &str) -> String {
    let inner = inner_vm_error(error);
    let msg = error_message_ref(inner);
    let loc = error_location_ref(inner);

    let Some(loc) = loc else {
        return format!("error: {msg}");
    };

    let effective_source = if !source.is_empty() {
        source
    } else {
        rich_source_context_ref(error).unwrap_or(source)
    };

    let kind = error_kind_tag(inner);
    let explicit_hint = error_suggestion_ref(inner).map(String::from);
    let hint = explicit_hint.or_else(|| auto_suggest(inner));

    let mut out = String::new();
    out.push_str(&format!("error[{kind}]: {msg}\n"));
    out.push_str(&format!("  --> line {}, column {}\n", loc.line, loc.column));
    out.push_str("   |\n");

    let lnum = loc.line;
    let pad = " ".repeat(lnum.to_string().len().max(1));

    if loc.line == 0 || effective_source.is_empty() {
        out.push_str(&format!("{pad} |\n"));
        out.push_str(&format!(
            "{pad} | = note: {}",
            if effective_source.is_empty() {
                "no source text available for this diagnostic"
            } else {
                "invalid line number (line must be >= 1)"
            }
        ));
        out.push('\n');
        out.push_str(&format!("{pad} |\n"));
    } else if let Some(source_line) = effective_source.lines().nth(loc.line.saturating_sub(1)) {
        let lnum_str = lnum.to_string();
        out.push_str(&format!("{lnum_str} | {source_line}\n"));

        let col = loc.column.saturating_sub(1);
        let caret_pad = " ".repeat(col);
        out.push_str(&format!("{pad} | {caret_pad}^ {msg}\n"));
        out.push_str(&format!("{pad} |\n"));
    } else {
        out.push_str(&format!("{pad} |\n"));
        out.push_str(&format!(
            "{pad} | = note: line {} is past the end of the source ({} line(s))\n",
            loc.line,
            effective_source.lines().count()
        ));
        out.push_str(&format!("{pad} |\n"));
    }

    if let VmError::TypeError { expected, found, .. } = inner {
        out.push_str(&format!("   = expected: `{expected}`\n"));
        out.push_str(&format!("   = found: `{found}`\n"));
    }

    if let Some(h) = &hint {
        out.push_str(&format!("   = hint: {h}\n"));
    }

    out
}

/// Formats a call stack as indented `at …` lines (newlines between frames,
/// trailing newline if non-empty).
pub fn format_call_stack(stack: &[String]) -> String {
    if stack.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for frame in stack {
        out.push_str("  at ");
        out.push_str(frame);
        out.push('\n');
    }
    out
}

/// Try to extract an identifier from backtick- or single-quote-delimited
/// text inside an error message (e.g. `` Undefined variable `x` ``).
fn extract_identifier(msg: &str) -> Option<String> {
    for delimiter in ['`', '\''] {
        if let Some(start) = msg.find(delimiter) {
            let rest = &msg[start + 1..];
            if let Some(end) = rest.find(delimiter) {
                let name = &rest[..end];
                if !name.is_empty() && !name.contains(' ') {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backward_compatible_display_runtime() {
        let err = VmError::runtime_error("something broke");
        assert_eq!(err.to_string(), "Runtime error: something broke");
    }

    #[test]
    fn backward_compatible_display_parse() {
        let err = VmError::parse_error_simple("unexpected token");
        assert_eq!(err.to_string(), "Parse error: unexpected token");
    }

    #[test]
    fn backward_compatible_display_type_error() {
        let err = VmError::type_error("cannot add", "Number", "String", None);
        let s = err.to_string();
        assert!(s.contains("Type error: cannot add"));
        assert!(s.contains("Expected: Number"));
        assert!(s.contains("Found: String"));
    }

    #[test]
    fn rich_render_with_source_context() {
        let source = "let y = 10\nresult = x + y\nprint(result)";
        let loc = SourceLocation::new("main.killer".to_string(), 2, 10);
        let err = VmError::RuntimeError {
            message: "Undefined variable `x`".to_string(),
            location: Some(loc),
            suggestion: None,
        }
        .with_source_context(source);

        let rendered = err.render_rich(None);
        assert!(rendered.contains("Undefined variable `x`"));
        assert!(rendered.contains("line 2, column 10"));
        assert!(rendered.contains("result = x + y"));
        assert!(rendered.contains("^ undefined here"));
        assert!(rendered.contains("hint:"));
        assert!(rendered.contains("let x = ..."));
    }

    #[test]
    fn rich_render_explicit_source_overrides_stored() {
        let stored  = "stored line\n";
        let runtime = "actual line\n";
        let loc = SourceLocation::new("f.killer".to_string(), 1, 1);
        let err = VmError::runtime_error_at("oops", loc)
            .with_source_context(stored);

        let rendered = err.render_rich(Some(runtime));
        assert!(rendered.contains("actual line"));
        assert!(!rendered.contains("stored line"));
    }

    #[test]
    fn stack_trace_rendering() {
        let mut err = VmError::runtime_error("something went wrong");
        err.add_stack_frame("main() line 10".to_string());
        err.add_stack_frame("calculate() line 5".to_string());
        err.add_stack_frame("helper() line 2".to_string());

        let rendered = err.render_rich(None);
        assert!(rendered.contains("Stack trace:"));
        assert!(rendered.contains("at main() line 10"));
        assert!(rendered.contains("at calculate() line 5"));
        assert!(rendered.contains("at helper() line 2"));
    }

    #[test]
    fn display_uses_rich_for_rich_variant() {
        let source = "let a = 1\nlet b = a + c\n";
        let loc = SourceLocation::new("test.killer".to_string(), 2, 13);
        let err = VmError::RuntimeError {
            message: "Undefined variable `c`".to_string(),
            location: Some(loc),
            suggestion: None,
        }
        .with_source_context(source);

        let display = err.to_string();
        assert!(display.contains("^ undefined here"));
        assert!(display.contains("let c = ..."));
    }

    #[test]
    fn with_source_context_is_builder() {
        let err = VmError::parse_error_simple("unexpected token")
            .with_source_context("let x = @\n");
        assert!(matches!(err, VmError::Rich { .. }));
    }

    #[test]
    fn add_stack_frame_wraps_non_rich() {
        let mut err = VmError::io_error("disk full");
        err.add_stack_frame("save() line 42".to_string());
        assert!(matches!(err, VmError::Rich { .. }));
        assert_eq!(err.stack_frames_list().len(), 1);
    }

    #[test]
    fn auto_suggest_parse_expected_brace() {
        let err = VmError::parse_error_simple("Expected '}' after block");
        let rendered = err.with_source_context("if true {\n  x\n").render_rich(None);
        assert!(rendered.contains("unclosed block"));
    }

    #[test]
    fn auto_suggest_type_error() {
        let err = VmError::type_error("cannot add", "Number", "String", None)
            .with_source_context("let x = 1 + \"hello\"\n");
        let rendered = err.render_rich(None);
        assert!(rendered.contains("expected: `Number`"));
        assert!(rendered.contains("found: `String`"));
        assert!(rendered.contains("explicit conversion"));
    }

    #[test]
    fn explicit_suggestion_takes_priority_over_auto() {
        let err = VmError::parse_error_with_suggestion(
            "Expected '}'",
            None,
            "Close the block on line 3",
        )
        .with_source_context("");

        let rendered = err.render_rich(None);
        assert!(rendered.contains("Close the block on line 3"));
        assert!(!rendered.contains("unclosed block"));
    }

    #[test]
    fn extract_identifier_backtick() {
        assert_eq!(extract_identifier("Undefined variable `foo`"), Some("foo".into()));
    }

    #[test]
    fn extract_identifier_single_quote() {
        assert_eq!(extract_identifier("Variable 'bar' not found"), Some("bar".into()));
    }

    #[test]
    fn extract_identifier_none() {
        assert_eq!(extract_identifier("Something went wrong"), None);
    }

    #[test]
    fn security_error_unchanged() {
        let err = VmError::security_error("blocked", "use sandbox mode");
        let s = err.to_string();
        assert!(s.contains("Security error: blocked"));
        assert!(s.contains("Suggestion: use sandbox mode"));
    }

    #[test]
    fn render_error_shows_line_caret_and_message() {
        let source = "let y = 10\nresult = x + 1\n";
        let loc = SourceLocation::new("main.killer".to_string(), 2, 11);
        let err = VmError::RuntimeError {
            message: "Undefined variable `x`".to_string(),
            location: Some(loc),
            suggestion: None,
        };
        let s = render_error(&err, source);
        assert!(s.contains("error[Runtime]: Undefined variable `x`"));
        assert!(s.contains("  --> line 2, column 11"));
        assert!(s.contains("result = x + 1"));
        assert!(s.contains("^ Undefined variable `x`"));
        assert!(s.contains("= hint:"));
    }

    #[test]
    fn render_error_without_location_is_plain() {
        let err = VmError::runtime_error("oops");
        assert_eq!(render_error(&err, ""), "error: oops");
    }

    #[test]
    fn render_error_line_out_of_range() {
        let loc = SourceLocation::new("f.killer".to_string(), 99, 1);
        let err = VmError::runtime_error_at("bad line", loc);
        let s = render_error(&err, "one line only\n");
        assert!(s.contains("error[Runtime]: bad line"));
        assert!(s.contains("past the end of the source"));
    }

    #[test]
    fn render_error_falls_back_to_rich_source_when_arg_empty() {
        let source = "a\nb\n";
        let loc = SourceLocation::new("x.killer".to_string(), 2, 1);
        let inner = VmError::RuntimeError {
            message: "e".to_string(),
            location: Some(loc),
            suggestion: None,
        };
        let err = inner.with_source_context(source);
        let s = render_error(&err, "");
        assert!(s.contains("2 | b"));
    }

    #[test]
    fn format_call_stack_empty() {
        assert_eq!(format_call_stack(&[]), "");
    }

    #[test]
    fn format_call_stack_frames() {
        let s = format_call_stack(&[
            "main() line 10".to_string(),
            "compute() line 25".to_string(),
            "helper() line 42".to_string(),
        ]);
        assert_eq!(
            s,
            "  at main() line 10\n  at compute() line 25\n  at helper() line 42\n"
        );
    }
}
