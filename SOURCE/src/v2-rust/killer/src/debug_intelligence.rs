//! # Killer Debug Intelligence — "Developer Can Relax" System
//!
//! 10 autonomous debugging + AI-supportability builtins that make the language
//! self-healing.  The developer writes code; Killer catches, fixes, explains,
//! tests, and optimises it automatically.
//!
//! ## Builtin surface (exposed via builtin.rs)
//!
//! | Builtin                    | What it does                                              |
//! |----------------------------|-----------------------------------------------------------|
//! | `debug_check(code)`        | Static scan → structured issue list (errors / warnings)  |
//! | `auto_fix(code)`           | Rule-based fixer → ranked FixCandidates with confidence   |
//! | `explain_error(msg, ctx)`  | Plain-English error explanation (rule-based + LLM)        |
//! | `suggest_refactor(code)`   | Proactive improvement hints (long fn, duplication, …)     |
//! | `auto_test(code)`          | Generates Killer unit-test scaffold from kfn signatures   |
//! | `perf_profile(code)`       | Static perf hints (nested loops, I/O in loop, …)          |
//! | `ai_pair(task)`            | Write a description → get working Killer code back        |
//! | `killer_debug_agent(code)` | Autonomous fix-until-passes agent (up to 10 cycles)       |
//! | `watch(expr, value)`       | Track a runtime expression's value across executions      |
//! | `watch_report()`           | Dump the full watch log as a human-readable string        |
//!
//! ## Zero external crates — everything is pure std Rust.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// -----------------------------------------------------------------------------
// PUBLIC TYPES
// -----------------------------------------------------------------------------

/// Severity level of a detected issue.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Must fix before code will run.
    Error,
    /// Code runs but the result is probably wrong.
    Warning,
    /// Style / readability suggestion.
    Info,
    /// Performance opportunity.
    Perf,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error   => "error",
            Severity::Warning => "warning",
            Severity::Info    => "info",
            Severity::Perf    => "perf",
        }
    }
}

/// One detected issue in the source code.
#[derive(Debug, Clone)]
pub struct Issue {
    /// Short code like "E001", "W007", "P002".
    pub code: String,
    pub severity: Severity,
    /// 1-based line number (0 = file-level).
    pub line: usize,
    pub message: String,
    pub fix_hint: String,
    /// Whether `auto_fix` can repair this automatically.
    pub auto_fixable: bool,
}

/// A single line change produced by `auto_fix`.
#[derive(Debug, Clone)]
pub struct Change {
    pub line: usize,
    pub original: String,
    pub replacement: String,
    pub reason: String,
}

/// One candidate repair returned by `auto_fix`.
#[derive(Debug, Clone)]
pub struct FixCandidate {
    /// 0.0 – 1.0 confidence that this fix is correct.
    pub confidence: f32,
    pub description: String,
    pub fixed_code: String,
    pub changes: Vec<Change>,
}

/// One refactor suggestion returned by `suggest_refactor`.
#[derive(Debug, Clone)]
pub struct RefactorSuggestion {
    pub code: String,
    pub line: usize,
    pub title: String,
    pub description: String,
    pub priority: RefactorPriority,
}

#[derive(Debug, Clone)]
pub enum RefactorPriority {
    High,
    Medium,
    Low,
}

impl RefactorPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefactorPriority::High   => "high",
            RefactorPriority::Medium => "medium",
            RefactorPriority::Low    => "low",
        }
    }
}

/// One performance hint returned by `perf_profile`.
#[derive(Debug, Clone)]
pub struct PerfHint {
    pub line: usize,
    pub category: String,
    pub impact: Impact,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub enum Impact {
    High,
    Medium,
    Low,
}

impl Impact {
    pub fn as_str(&self) -> &'static str {
        match self {
            Impact::High   => "high",
            Impact::Medium => "medium",
            Impact::Low    => "low",
        }
    }
}

/// Final result of `killer_debug_agent`.
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub success: bool,
    pub fixed_code: String,
    pub cycles: usize,
    pub all_changes: Vec<Change>,
    pub final_issues: Vec<Issue>,
    pub summary: String,
}

/// One entry in the watch log.
#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub expr: String,
    pub value: String,
    pub call_count: usize,
    pub timestamp_ms: u128,
}

// -----------------------------------------------------------------------------
// LAYER 1 — STATIC CHECKER  (`debug_check`)
// -----------------------------------------------------------------------------

/// Scan Killer source code and return a list of detected issues.
///
/// Checks performed (pure text / brace-depth, no full parse needed):
///
/// | Code | Severity | Description                                  |
/// |------|----------|----------------------------------------------|
/// | E001 | Error    | `fn` / `func` / `function` instead of `kfn` |
/// | E002 | Error    | `let` / `var` / `const` assignment keyword   |
/// | E003 | Warning  | Trailing semicolon after `return` statement  |
/// | E004 | Error    | `print` / `println` without parentheses      |
/// | E005 | Warning  | Redundant `== true` / `== false` comparison  |
/// | E006 | Warning  | Unnecessary trailing semicolon               |
/// | E007 | Error    | Unclosed `{` brace(s) at end of file         |
/// | E008 | Error    | Unclosed `(` parenthesis at end of file      |
/// | E009 | Error    | Unexpected extra `}` (closes too many times) |
/// | W001 | Warning  | Variable assigned but never read             |
/// | W002 | Warning  | Shadowed variable (re-assigned same name)    |
/// | P001 | Perf     | String concat with `+` inside a loop         |
/// | P002 | Perf     | I/O call (readFile / http_get) inside a loop |
// -- Scope-aware variable entry ------------------------------------------------
/// Records a single variable binding inside one lexical scope.
#[derive(Clone)]
struct VarEntry {
    assign_line: usize,
    was_read:    bool,
    /// True when the variable is a kfn parameter — never report W001 for those.
    is_param:    bool,
}

pub fn debug_check(code: &str) -> Vec<Issue> {
    let mut issues: Vec<Issue> = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    let mut brace_depth: i32 = 0;
    let mut paren_depth: i32 = 0;
    let mut loop_depth:  i32 = 0;

    // -- Scope stack for W001/W002 ---------------------------------------------
    // Each level is the set of variables defined in that brace scope.
    // When a `}` pops a scope we check W001 for everything in it.
    let mut scope_stack: Vec<HashMap<String, VarEntry>> = vec![HashMap::new()];

    // Helper closures via inline fn-items are not possible here, so we use
    // a small macro-style approach via helper functions defined below in
    // `debug_check` scope.
    //
    // mark_assign(name, line, is_param) — record or update an assignment
    // mark_read(name)                   — mark a name as read in the nearest scope
    // These operate on `scope_stack` via mutable borrow captured in the loop.

    for (i, &line) in lines.iter().enumerate() {
        let ln  = i + 1;
        let t   = line.trim();

        // Skip full-line comments
        if t.starts_with("//") || t.starts_with('#') {
            continue;
        }

        let code_part = strip_inline_comment(t);
        let first_word = code_part.split_whitespace().next().unwrap_or("");

        // -- E001: fn / func / function instead of kfn ------------------------
        if matches!(first_word, "fn" | "func" | "function") {
            issues.push(Issue {
                code: "E001".into(),
                severity: Severity::Error,
                line: ln,
                message: format!("Use 'kfn' instead of '{}' for function definitions", first_word),
                fix_hint: format!("Replace '{}' with 'kfn'", first_word),
                auto_fixable: true,
            });
        }

        // -- E002: let / var / const -------------------------------------------
        if matches!(first_word, "let" | "var" | "const") {
            issues.push(Issue {
                code: "E002".into(),
                severity: Severity::Error,
                line: ln,
                message: format!("No '{}' needed — assign directly: x = value", first_word),
                fix_hint: format!("Remove the '{}' keyword", first_word),
                auto_fixable: true,
            });
        }

        // -- E003: return x; ---------------------------------------------------
        if code_part.starts_with("return ") && code_part.ends_with(';') {
            issues.push(Issue {
                code: "E003".into(),
                severity: Severity::Warning,
                line: ln,
                message: "Trailing semicolon after 'return' is not needed in Killer".into(),
                fix_hint: "Remove the trailing ';'".into(),
                auto_fixable: true,
            });
        }

        // -- E004: print / println without parens -----------------------------
        {
            let fc = code_part.trim_start();
            if (fc.starts_with("print ") || fc.starts_with("println ")) && !fc.contains('(') {
                issues.push(Issue {
                    code: "E004".into(),
                    severity: Severity::Error,
                    line: ln,
                    message: "print() requires parentheses: print(value)".into(),
                    fix_hint: "Wrap the argument in parentheses".into(),
                    auto_fixable: true,
                });
            }
        }

        // -- E005: == true / == false ------------------------------------------
        if code_part.contains("== true") || code_part.contains("== false") {
            issues.push(Issue {
                code: "E005".into(),
                severity: Severity::Warning,
                line: ln,
                message: "Redundant boolean comparison".into(),
                fix_hint: "Use 'if x' instead of 'if x == true'".into(),
                auto_fixable: false,
            });
        }

        // -- E006: trailing semicolon (non-return) -----------------------------
        if code_part.ends_with(';') && !code_part.starts_with("return ") {
            issues.push(Issue {
                code: "E006".into(),
                severity: Severity::Warning,
                line: ln,
                message: "Unnecessary trailing semicolon".into(),
                fix_hint: "Remove the trailing ';'".into(),
                auto_fixable: true,
            });
        }

        // -- kfn parameter extraction ------------------------------------------
        // Parameters are declared in the new scope opened by the `{` on the
        // same line or the next line.  We extract them here and inject them
        // into the *current* scope (they are readable without an assignment).
        if first_word == "kfn" {
            if let (Some(po), Some(pc)) = (code_part.find('('), code_part.find(')')) {
                let params_str = &code_part[po + 1..pc];
                for param in params_str.split(',') {
                    let name = param.trim().split(':').next().unwrap_or("").trim();
                    if is_simple_identifier(name) {
                        if let Some(scope) = scope_stack.last_mut() {
                            scope.insert(name.to_string(), VarEntry {
                                assign_line: ln,
                                was_read:    true, // params are always "used" (caller reads them)
                                is_param:    true,
                            });
                        }
                    }
                }
            }
        }

        // -- Brace tracking + scope push/pop -----------------------------------
        // Count `{` and `}` on this line; for each `{` push a scope, for each
        // `}` pop a scope (emitting W001 for unused vars in the popped scope).
        let opens  = code_part.chars().filter(|&c| c == '{').count();
        let closes = code_part.chars().filter(|&c| c == '}').count();

        // Push new scopes for each `{`
        for _ in 0..opens {
            brace_depth += 1;
            scope_stack.push(HashMap::new());
        }

        // Pop scopes for each `}`, emitting W001 for unused vars
        for _ in 0..closes {
            brace_depth -= 1;
            if brace_depth < 0 {
                issues.push(Issue {
                    code: "E009".into(),
                    severity: Severity::Error,
                    line: ln,
                    message: "Unexpected extra closing brace '}'".into(),
                    fix_hint: "Remove this extra '}'".into(),
                    auto_fixable: false,
                });
                brace_depth = 0;
                // Don't pop scope — it was never pushed for this extra `}`
                continue;
            }
            if let Some(popped) = scope_stack.pop() {
                for (var, entry) in &popped {
                    if !entry.was_read && !entry.is_param {
                        issues.push(Issue {
                            code: "W001".into(),
                            severity: Severity::Warning,
                            line: entry.assign_line,
                            message: format!("Variable '{}' assigned but never read", var),
                            fix_hint: "Remove the assignment or use the variable somewhere".into(),
                            auto_fixable: false,
                        });
                    }
                }
                // Propagate reads upward: any name that was read in the inner
                // scope should mark the same name as read in the outer scope
                // (e.g. a variable declared in an outer scope, used in an if body).
                if let Some(outer) = scope_stack.last_mut() {
                    for (var, entry) in &popped {
                        if entry.was_read {
                            if let Some(outer_entry) = outer.get_mut(var) {
                                outer_entry.was_read = true;
                            }
                        }
                    }
                }
            }
        }

        // Paren tracking (E008)
        for ch in code_part.chars() {
            match ch {
                '(' => { paren_depth += 1; }
                ')' => { if paren_depth > 0 { paren_depth -= 1; } }
                _ => {}
            }
        }

        // -- Loop depth tracking -----------------------------------------------
        if first_word == "for" || first_word == "while" {
            loop_depth += 1;
        }
        // A lone `}` closes one loop level (heuristic — good enough for static scan)
        if t == "}" && loop_depth > 0 {
            loop_depth -= 1;
        }

        // -- P001: string concat in loop ---------------------------------------
        if loop_depth > 0
            && code_part.contains('+')
            && (code_part.contains('"') || code_part.contains("str("))
        {
            issues.push(Issue {
                code: "P001".into(),
                severity: Severity::Perf,
                line: ln,
                message: "String concatenation with '+' inside a loop — O(n²) allocations".into(),
                fix_hint: "Collect into an array, then call join() once after the loop".into(),
                auto_fixable: false,
            });
        }

        // -- P002: I/O call in loop --------------------------------------------
        if loop_depth > 0
            && (code_part.contains("readFile(")
                || code_part.contains("writeFile(")
                || code_part.contains("http_get(")
                || code_part.contains("http_post("))
        {
            issues.push(Issue {
                code: "P002".into(),
                severity: Severity::Perf,
                line: ln,
                message: "I/O call inside a loop — extremely slow for large datasets".into(),
                fix_hint: "Read data BEFORE the loop, then process in-memory".into(),
                auto_fixable: false,
            });
        }

        // -- Variable tracking: assignment (W001/W002) -------------------------
        // Only look at the part of the line BEFORE the first `{` so that the
        // variable on the LHS of `kfn foo() {` / `if cond {` is not mistaken
        // for an assignment.
        let assign_part = if let Some(b) = code_part.find('{') {
            &code_part[..b]
        } else {
            code_part
        };

        if let Some(var) = detect_assignment(assign_part) {
            // Search scopes from innermost outward
            let found_in_outer = scope_stack.iter().rev().skip(1)
                .any(|scope| scope.contains_key(&var));

            if let Some(current_scope) = scope_stack.last_mut() {
                if let Some(entry) = current_scope.get_mut(&var) {
                    // W002: re-assigned in same scope before being read
                    if !entry.was_read && !entry.is_param {
                        issues.push(Issue {
                            code: "W002".into(),
                            severity: Severity::Warning,
                            line: ln,
                            message: format!(
                                "Variable '{}' reassigned before it was ever read",
                                var
                            ),
                            fix_hint: "Check whether the first assignment is needed".into(),
                            auto_fixable: false,
                        });
                    }
                    entry.assign_line = ln;
                    entry.was_read    = false;
                } else if !found_in_outer {
                    // New variable in this scope
                    current_scope.insert(var, VarEntry {
                        assign_line: ln,
                        was_read:    false,
                        is_param:    false,
                    });
                } else {
                    // Assigned to an outer-scope variable — mark it read there
                    for scope in scope_stack.iter_mut().rev().skip(1) {
                        if let Some(entry) = scope.get_mut(&var) {
                            entry.was_read = true;
                            break;
                        }
                    }
                }
            }
        }

        // -- Mark reads --------------------------------------------------------
        // Everything to the RIGHT of `=` (or the whole line if no assignment)
        // is considered a read expression.
        let read_part = {
            // Find first non-comparison `=` to get the RHS
            let bytes = assign_part.as_bytes();
            let mut rhs_start = None;
            for (j, &b) in bytes.iter().enumerate() {
                if b == b'='
                    && j > 0
                    && !matches!(bytes[j - 1], b'!' | b'<' | b'>' | b'=')
                    && bytes.get(j + 1).copied() != Some(b'=')
                {
                    rhs_start = Some(j + 1);
                    break;
                }
            }
            if let Some(start) = rhs_start {
                &code_part[start.min(code_part.len())..]
            } else {
                code_part  // no assignment → whole line is a read context
            }
        };

        for token in tokenize_identifiers(read_part) {
            // Walk scopes from innermost outward, mark first match as read
            let mut marked = false;
            for scope in scope_stack.iter_mut().rev() {
                if let Some(entry) = scope.get_mut(&token) {
                    entry.was_read = true;
                    marked = true;
                    break;
                }
            }
            // Also scan the non-assign part of the line (function calls, conditions)
            let _ = marked;
        }
    }

    // -- E007: unclosed braces at EOF -----------------------------------------
    if brace_depth > 0 {
        issues.push(Issue {
            code: "E007".into(),
            severity: Severity::Error,
            line: lines.len(),
            message: format!("{} unclosed brace(s) — missing '}}' at end of file", brace_depth),
            fix_hint: format!("Add {} closing brace(s) '}}'", brace_depth),
            auto_fixable: true,
        });
    }

    // -- E008: unclosed parens at EOF -----------------------------------------
    if paren_depth > 0 {
        issues.push(Issue {
            code: "E008".into(),
            severity: Severity::Error,
            line: lines.len(),
            message: format!("{} unclosed parenthesis — missing ')'", paren_depth),
            fix_hint: format!("Add {} closing parenthesis ')'", paren_depth),
            auto_fixable: true,
        });
    }

    // -- W001 for outermost scope (module-level vars never used) --------------
    if let Some(top_scope) = scope_stack.last() {
        for (var, entry) in top_scope {
            if !entry.was_read && !entry.is_param {
                issues.push(Issue {
                    code: "W001".into(),
                    severity: Severity::Warning,
                    line: entry.assign_line,
                    message: format!("Variable '{}' assigned but never read", var),
                    fix_hint: "Remove the assignment or use the variable somewhere".into(),
                    auto_fixable: false,
                });
            }
        }
    }

    issues
}

// -----------------------------------------------------------------------------
// LAYER 2 — AUTO-FIX ENGINE  (`auto_fix`)
// -----------------------------------------------------------------------------

/// Apply all automatically-fixable issues and return ranked FixCandidates.
///
/// The first candidate (highest confidence) is the recommended apply.
/// A confidence of 1.0 means the fix is purely syntactic with no ambiguity.
pub fn auto_fix(code: &str) -> Vec<FixCandidate> {
    let issues = debug_check(code);
    let fixable: Vec<&Issue> = issues.iter().filter(|i| i.auto_fixable).collect();

    if fixable.is_empty() {
        return vec![FixCandidate {
            confidence: 1.0,
            description: "No auto-fixable issues found — code looks correct".into(),
            fixed_code: code.to_string(),
            changes: vec![],
        }];
    }

    let lines_orig: Vec<String> = code.lines().map(|l| l.to_string()).collect();
    let mut fixed_lines = lines_orig.clone();
    let mut changes: Vec<Change> = Vec::new();

    for issue in &fixable {
        let idx = issue.line.saturating_sub(1);

        // E007 / E008: append closing delimiters at end of file
        if issue.code == "E007" {
            let count = fixed_lines.iter()
                .flat_map(|l| l.chars())
                .filter(|&c| c == '{').count() as i32
                - fixed_lines.iter()
                .flat_map(|l| l.chars())
                .filter(|&c| c == '}').count() as i32;
            for _ in 0..count.max(0) {
                fixed_lines.push("}".into());
                changes.push(Change {
                    line: fixed_lines.len(),
                    original: String::new(),
                    replacement: "}".into(),
                    reason: "Add missing closing brace".into(),
                });
            }
            continue;
        }

        if idx >= fixed_lines.len() { continue; }

        let original = fixed_lines[idx].clone();
        let new_line = apply_fix_to_line(&original, &issue.code);

        if new_line != original {
            changes.push(Change {
                line: issue.line,
                original: original.clone(),
                replacement: new_line.clone(),
                reason: issue.message.clone(),
            });
            fixed_lines[idx] = new_line;
        }
    }

    let fixed_code = fixed_lines.join("\n");

    // Confidence: pure syntax fixes → 0.97; mixed (semantic) → 0.75
    let all_syntax = changes.iter().all(|c| {
        fixable.iter().any(|i| {
            i.line == c.line
                && matches!(i.code.as_str(), "E001" | "E002" | "E003" | "E004" | "E006" | "E007")
        })
    });
    let confidence = if all_syntax { 0.97 } else { 0.75 };

    vec![FixCandidate {
        confidence,
        description: format!(
            "Applied {} fix(es): {}",
            changes.len(),
            changes
                .iter()
                .map(|c| format!("line {}", c.line))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        fixed_code,
        changes,
    }]
}

fn apply_fix_to_line(line: &str, code: &str) -> String {
    let mut s = line.to_string();
    match code {
        "E001" => {
            // Replace fn/func/function with kfn (only at start of trimmed content)
            for kw in &["function ", "func ", "fn "] {
                if let Some(pos) = s.find(kw) {
                    s.replace_range(pos..pos + kw.len(), "kfn ");
                    break;
                }
            }
        }
        "E002" => {
            // Remove let/var/const
            for kw in &["const ", "let ", "var "] {
                if let Some(pos) = s.find(kw) {
                    s.replace_range(pos..pos + kw.len(), "");
                    break;
                }
            }
        }
        "E003" | "E006" => {
            // Remove trailing semicolon (preserving whitespace)
            let trimmed_end = s.trim_end();
            if trimmed_end.ends_with(';') {
                let new_end = trimmed_end.trim_end_matches(';');
                // keep trailing whitespace count from original
                let trailing_ws = s.len() - s.trim_end().len();
                s = format!("{}{}", new_end, " ".repeat(trailing_ws));
            }
        }
        "E004" => {
            // print "x"  →  print("x")
            // println "x" → println("x")
            for prefix in &["println ", "print "] {
                if let Some(pos) = s.find(prefix) {
                    let indent = &s[..pos];
                    let after  = s[pos + prefix.len()..].trim();
                    let fn_name = prefix.trim();
                    s = format!("{}{}({})", indent, fn_name, after);
                    break;
                }
            }
        }
        _ => {}
    }
    s
}

// -----------------------------------------------------------------------------
// LAYER 3 — EXPLAIN ERROR  (`explain_error`)
// -----------------------------------------------------------------------------

/// Return a plain-English explanation of an error message.
///
/// Strategy:
/// 1. Rule-based lookup for the 20 most common Killer errors (instant, offline)
/// 2. If no rule matched, escalate to local Ollama LLM (if running)
/// 3. If Ollama is unavailable, return a generic fallback
pub fn explain_error(error_msg: &str, code_context: &str) -> String {
    let ruled = rule_based_explanation(error_msg);
    if !ruled.is_empty() {
        return ruled;
    }
    try_llm_explain(error_msg, code_context)
        .unwrap_or_else(|| {
            format!(
                "[explain_error] No rule matched for: {}\n\
                 Tip: Check the Killer language reference at killer-lang.dev/docs\n\
                 Context:\n{}",
                error_msg, code_context
            )
        })
}

fn rule_based_explanation(msg: &str) -> String {
    let m = msg.to_lowercase();

    if m.contains("undefined variable") || m.contains("not found in scope") {
        let var = extract_quoted(msg).unwrap_or_else(|| "variable".into());
        return format!(
            "Variable '{}' is used but was never assigned.\n\
             → Check spelling (Killer is case-sensitive)\n\
             → Make sure you assigned it above this line:  {} = ...",
            var, var
        );
    }

    if m.contains("type error")
        && ((m.contains("string") && m.contains("int"))
            || (m.contains("str") && m.contains("num")))
    {
        return "Type mismatch: using a String where a number is expected.\n\
                → Use int(x) to convert a string to an integer\n\
                → Or str(x) to convert a number to a string"
            .into();
    }

    if m.contains("parse error") && m.contains("expected '{'") {
        return "Missing opening brace '{'.\n\
                → Killer blocks MUST start with '{'\n\
                → Example:  kfn foo() { ... }"
            .into();
    }

    if m.contains("expected 'kfn'") || m.contains("expected function") {
        return "Function definition must start with 'kfn'.\n\
                → Wrong:  fn add(a, b) { a + b }\n\
                → Right:  kfn add(a, b) { a + b }"
            .into();
    }

    if m.contains("stack overflow") || m.contains("recursion limit") {
        return "Stack overflow — infinite recursion detected.\n\
                → Make sure your recursive function has a base case:\n\
                   kfn fact(n) { if n <= 1 { 1 } else { n * fact(n - 1) } }"
            .into();
    }

    if m.contains("index out of bounds") || m.contains("index out of range") {
        return "Array index out of bounds.\n\
                → Arrays are 0-indexed: first element is arr[0]\n\
                → Maximum valid index is  len(arr) - 1\n\
                → Add a bounds check:  if i < len(arr) { ... }"
            .into();
    }

    if m.contains("divide by zero") || m.contains("division by zero") {
        return "Division by zero — the denominator is 0.\n\
                → Add a guard:  if divisor != 0 { result = a / divisor }"
            .into();
    }

    if m.contains("expected ')'") || m.contains("unclosed paren") {
        return "Missing closing parenthesis ')'.\n\
                → Count your '(' and ')' — they must match\n\
                → Common mistake: print(K\"text {var}\"  ← missing )"
            .into();
    }

    if m.contains("cannot call") && m.contains("not a function") {
        let name = extract_quoted(msg).unwrap_or_else(|| "x".into());
        return format!(
            "'{}' is not a function — you cannot call it with ().\n\
             → Check whether you spelled the function name correctly\n\
             → Make sure it was defined with 'kfn' before this line",
            name
        );
    }

    if m.contains("io error") || m.contains("no such file") || m.contains("file not found") {
        return "File I/O error — the file could not be opened.\n\
                → Use a relative path ('data.csv'), not an absolute path\n\
                → Check that the file exists in the working directory"
            .into();
    }

    String::new() // no rule matched
}

fn extract_quoted(s: &str) -> Option<String> {
    for delim in &['\'', '"'] {
        if let Some(start) = s.find(*delim) {
            let rest = &s[start + 1..];
            if let Some(end) = rest.find(*delim) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn try_llm_explain(error_msg: &str, code_context: &str) -> Option<String> {
    let prompt = format!(
        "You are an expert in the Killer programming language.\n\
         A developer received this error:\n  {}\n\n\
         Code context:\n{}\n\n\
         In 3-4 concise lines: what went wrong and exactly how to fix it.\n\
         Reply with plain text only, no markdown.",
        error_msg, code_context
    );
    crate::llm::ask(&crate::llm::LlmConfig::ollama("llama3.2"), &prompt).ok()
}

// -----------------------------------------------------------------------------
// LAYER 4 — SUGGEST REFACTOR  (`suggest_refactor`)
// -----------------------------------------------------------------------------

/// Return proactive refactor suggestions for the given source code.
pub fn suggest_refactor(code: &str) -> Vec<RefactorSuggestion> {
    let mut suggestions: Vec<RefactorSuggestion> = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    // -- S001: Long function (> 30 lines) -------------------------------------
    for (fn_name, length, start_line) in measure_function_lengths(&lines) {
        if length > 30 {
            suggestions.push(RefactorSuggestion {
                code: "S001".into(),
                line: start_line,
                title: format!("Function '{}' is {} lines — consider splitting", fn_name, length),
                description: "Functions over 30 lines are harder to test and understand. \
                               Extract logical sub-operations into smaller kfn helpers."
                    .into(),
                priority: RefactorPriority::Medium,
            });
        }
    }

    // -- S002: Repeated string literal (≥ 3 occurrences, len > 5) -------------
    let mut string_counts: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, &line) in lines.iter().enumerate() {
        for lit in extract_string_literals(line) {
            string_counts.entry(lit).or_default().push(i + 1);
        }
    }
    for (literal, occurrences) in &string_counts {
        if occurrences.len() >= 3 && literal.len() > 5 {
            suggestions.push(RefactorSuggestion {
                code: "S002".into(),
                line: occurrences[0],
                title: format!("Literal \"{}\" repeated {} times", literal, occurrences.len()),
                description: format!(
                    "Extract to a constant at the top:  CONST = \"{}\"",
                    literal
                ),
                priority: RefactorPriority::Low,
            });
        }
    }

    // -- S003: Manual for-loop + push that could use functional style ----------
    for (i, &line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("for ") && lines.get(i + 1).map(|l| l.contains("push(")).unwrap_or(false)
        {
            suggestions.push(RefactorSuggestion {
                code: "S003".into(),
                line: i + 1,
                title: "Manual for-loop + push — consider map/filter pattern".into(),
                description:
                    "If you are transforming every element, a functional style is cleaner:\n  \
                     result = items.map(kfn(x) { transform(x) })"
                        .into(),
                priority: RefactorPriority::Low,
            });
        }
    }

    // -- S004: Deep nesting (brace depth ≥ 4) ---------------------------------
    let mut depth: i32 = 0;
    for (i, &line) in lines.iter().enumerate() {
        let t = line.trim();
        depth += t.chars().filter(|&c| c == '{').count() as i32;
        depth -= t.chars().filter(|&c| c == '}').count() as i32;
        if depth >= 4 {
            suggestions.push(RefactorSuggestion {
                code: "S004".into(),
                line: i + 1,
                title: format!("Nesting depth {} — extract inner logic into a helper function", depth),
                description:
                    "Deep nesting is hard to read and test. \
                     Pull the inner block into a named kfn with a clear purpose."
                        .into(),
                priority: RefactorPriority::High,
            });
            break; // one warning per function is enough
        }
    }

    suggestions
}

// -----------------------------------------------------------------------------
// LAYER 5 — AUTO-TEST GENERATOR  (`auto_test`)
// -----------------------------------------------------------------------------

/// Generate a Killer unit-test scaffold from `kfn` function signatures.
///
/// Returns a string containing ready-to-run Killer test code.
pub fn auto_test(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let functions = extract_function_signatures(&lines);

    let mut out = String::from(
        "// -- Auto-generated tests (Killer Debug Intelligence) --------------\n\n",
    );

    if functions.is_empty() {
        out.push_str("// No kfn functions found — nothing to test\n");
        return out;
    }

    let mut pass_count = 0usize;
    let fail_count = 0usize;

    for (fn_name, params, _start_line) in &functions {
        let cases = generate_test_cases(fn_name, params);

        out.push_str(&format!("// Tests for: {}\n", fn_name));
        for (desc, inputs, expected) in &cases {
            out.push_str(&format!(
                "result_{fn} = {fn}({inputs})\n\
                 if result_{fn} == {exp} {{\n\
                   print(K\"PASS: {fn}({inputs}) = {exp}\")\n\
                 }} else {{\n\
                   print(K\"FAIL: {fn}({inputs}) expected {exp} got {{result_{fn}}}\")\n\
                 }}\n\n",
                fn = fn_name,
                inputs = inputs,
                exp = expected,
            ));
            let _ = desc;
        }
        pass_count += cases.len();
    }

    // Summary footer
    out.push_str(&format!(
        "// Generated {} test case(s) for {} function(s)\n",
        pass_count,
        functions.len()
    ));
    let _ = fail_count; // will be non-zero at runtime only

    out
}

// -----------------------------------------------------------------------------
// LAYER 6 — PERFORMANCE PROFILER  (`perf_profile`)
// -----------------------------------------------------------------------------

/// Return static performance hints for the given code.
pub fn perf_profile(code: &str) -> Vec<PerfHint> {
    let mut hints: Vec<PerfHint> = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    let mut loop_depth: i32 = 0;
    let mut reported_nested = false;

    for (i, &line) in lines.iter().enumerate() {
        let t   = line.trim();
        let ln  = i + 1;
        let cp  = strip_inline_comment(t);

        // Track loop depth
        let first = cp.split_whitespace().next().unwrap_or("");
        if first == "for" || first == "while" {
            loop_depth += 1;
            if loop_depth >= 2 && !reported_nested {
                hints.push(PerfHint {
                    line: ln,
                    category: "NestedLoop".into(),
                    impact: Impact::High,
                    message: format!(
                        "Nested loop (depth {}) — O(n^{}) time complexity",
                        loop_depth, loop_depth
                    ),
                    suggestion:
                        "Consider: pre-computed lookup table, early exit, or restructuring \
                         the outer loop to reduce iterations"
                            .into(),
                });
                reported_nested = true;
            }
        }

        if cp.trim() == "}" && loop_depth > 0 {
            loop_depth -= 1;
            if loop_depth < 2 { reported_nested = false; }
        }

        if loop_depth > 0 {
            // P001 — string concat
            if cp.contains('+') && (cp.contains('"') || cp.contains("str(")) {
                hints.push(PerfHint {
                    line: ln,
                    category: "StringBuild".into(),
                    impact: Impact::Medium,
                    message: "String concat with '+' inside loop — O(n²) allocations".into(),
                    suggestion:
                        "Collect strings into an array, call join() once after the loop".into(),
                });
            }

            // P002 — I/O in loop
            if cp.contains("readFile(")
                || cp.contains("writeFile(")
                || cp.contains("http_get(")
                || cp.contains("http_post(")
            {
                hints.push(PerfHint {
                    line: ln,
                    category: "IO_in_Loop".into(),
                    impact: Impact::High,
                    message: "I/O call inside loop — catastrophic for large datasets".into(),
                    suggestion: "Hoist I/O outside the loop; process data in-memory".into(),
                });
            }

            // P003 — array push in loop
            if cp.contains("push(") {
                hints.push(PerfHint {
                    line: ln,
                    category: "ArrayBuild".into(),
                    impact: Impact::Low,
                    message: "push() inside loop — consider pre-allocation or map pattern".into(),
                    suggestion:
                        "For element transformation:  result = items.map(kfn(x) { f(x) })".into(),
                });
            }
        }

        // P004 — repeated function call with same args in tight loop
        // (heuristic: same call site twice on consecutive lines)
        if i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if !cp.is_empty()
                && cp == next
                && (cp.contains('(') && cp.contains(')'))
                && loop_depth == 0
            {
                hints.push(PerfHint {
                    line: ln,
                    category: "RedundantCall".into(),
                    impact: Impact::Low,
                    message: "Identical function call on consecutive lines".into(),
                    suggestion:
                        "Cache the result in a variable: result = expensive_fn(x)".into(),
                });
            }
        }
    }

    hints
}

// -----------------------------------------------------------------------------
// LAYER 7 — AI PAIR PROGRAMMER  (`ai_pair`)
// -----------------------------------------------------------------------------

/// Describe a task in natural language; get working Killer code back.
///
/// Tries local Ollama first; falls back to template-based generation.
pub fn ai_pair(task_description: &str) -> String {
    try_llm_generate(task_description)
        .unwrap_or_else(|| template_generate(task_description))
}

fn try_llm_generate(task: &str) -> Option<String> {
    let prompt = format!(
        "Write a complete Killer language program for this task: {}\n\n\
         Killer syntax rules:\n\
         - Use 'kfn' for functions (NOT 'fn' / 'func' / 'function')\n\
         - No 'let'/'var'/'const' — assign directly: x = value\n\
         - String interpolation: K\"text {{var}}\" or K\"expr: {{a + b}}\"\n\
         - print() for output, println() for output with newline\n\
         - Implicit return (last expression in kfn is the return value)\n\
         - Arrays: [1, 2, 3]  Dicts: {{\"key\": value}}\n\n\
         Return ONLY the Killer code, no markdown, no explanation.",
        task
    );
    crate::llm::ask(&crate::llm::LlmConfig::ollama("llama3.2"), &prompt).ok()
}

fn template_generate(task: &str) -> String {
    let tl = task.to_lowercase();

    if tl.contains("hello") || tl.contains("greet") {
        return r#"kfn greet(name) {
  K"Hello, {name}!"
}

message = greet("World")
print(message)
"#
        .into();
    }

    if tl.contains("sort") {
        return r#"kfn bubble_sort(arr) {
  n = len(arr)
  i = 0
  while i < n {
    j = 0
    while j < n - i - 1 {
      if arr[j] > arr[j + 1] {
        temp = arr[j]
        arr[j] = arr[j + 1]
        arr[j + 1] = temp
      }
      j = j + 1
    }
    i = i + 1
  }
  arr
}

data = [64, 34, 25, 12, 22, 11, 90]
sorted = bubble_sort(data)
print(sorted)
"#
        .into();
    }

    if tl.contains("fibonacci") || tl.contains("fib") {
        return r#"kfn fib(n) {
  if n <= 1 { n }
  else { fib(n - 1) + fib(n - 2) }
}

i = 0
while i < 10 {
  print(fib(i))
  i = i + 1
}
"#
        .into();
    }

    if tl.contains("factorial") {
        return r#"kfn factorial(n) {
  if n <= 1 { 1 }
  else { n * factorial(n - 1) }
}

print(factorial(10))
"#
        .into();
    }

    if tl.contains("palindrome") {
        return r#"kfn is_palindrome(s) {
  n = len(s)
  i = 0
  result = true
  while i < n / 2 {
    if charAt(s, i) != charAt(s, n - 1 - i) {
      result = false
    }
    i = i + 1
  }
  result
}

print(is_palindrome("racecar"))
print(is_palindrome("hello"))
"#
        .into();
    }

    if tl.contains("prime") {
        return r#"kfn is_prime(n) {
  if n < 2 { false }
  else {
    i = 2
    result = true
    while i * i <= n {
      if n - (n / i) * i == 0 { result = false }
      i = i + 1
    }
    result
  }
}

i = 2
while i <= 50 {
  if is_prime(i) { print(i) }
  i = i + 1
}
"#
        .into();
    }

    // Generic template
    format!(
        r#"// Generated scaffold for: {task}
// TODO: replace this template with your actual logic

kfn solve(input) {{
  result = input
  result
}}

output = solve("your input here")
print(K"Result: {{output}}")
"#,
        task = task
    )
}

// -----------------------------------------------------------------------------
// LAYER 8 — KILLER DEBUG AGENT  (`killer_debug_agent`)
// -----------------------------------------------------------------------------

/// Autonomous "fix-until-passes" agent.
///
/// Runs up to `MAX_CYCLES` of:
///   1. `debug_check` — are there any remaining errors?
///   2. If none → success!
///   3. `auto_fix` — apply the best available fix
///   4. Repeat
///
/// Returns the fixed code + a summary of what changed.
pub fn killer_debug_agent(code: &str) -> AgentResult {
    const MAX_CYCLES: usize = 10;

    let mut current = code.to_string();
    let mut all_changes: Vec<Change> = Vec::new();

    for cycle in 1..=MAX_CYCLES {
        let issues = debug_check(&current);
        let errors: Vec<&Issue> = issues
            .iter()
            .filter(|i| matches!(i.severity, Severity::Error))
            .collect();

        if errors.is_empty() {
            // -- Clean pass --------------------------------------------------
            let remaining_issues: Vec<Issue> = issues
                .into_iter()
                .filter(|i| !matches!(i.severity, Severity::Error))
                .collect();

            let warn_count = remaining_issues
                .iter()
                .filter(|i| matches!(i.severity, Severity::Warning))
                .count();
            let refactor_count = suggest_refactor(&current).len();
            let perf_count = perf_profile(&current).len();

            let summary = format!(
                "All errors resolved in {} cycle(s).\n\
                 {} warning(s) remain  |  \
                 {} refactor suggestion(s)  |  \
                 {} performance hint(s)\n\
                 Run suggest_refactor() and perf_profile() for further improvements.",
                cycle - 1,
                warn_count,
                refactor_count,
                perf_count
            );

            return AgentResult {
                success: true,
                fixed_code: current,
                cycles: cycle - 1,
                all_changes,
                final_issues: remaining_issues,
                summary,
            };
        }

        // -- Apply best fix --------------------------------------------------
        let candidates = auto_fix(&current);
        let best = candidates.into_iter().next();
        match best {
            Some(fix) if !fix.changes.is_empty() => {
                all_changes.extend(fix.changes);
                current = fix.fixed_code;
            }
            _ => break, // Nothing more the agent can fix automatically
        }
    }

    // -- Best-effort result --------------------------------------------------
    let remaining = debug_check(&current);
    let remaining_errors = remaining
        .iter()
        .filter(|i| matches!(i.severity, Severity::Error))
        .count();

    AgentResult {
        success: false,
        fixed_code: current,
        cycles: MAX_CYCLES,
        all_changes,
        final_issues: remaining.clone(),
        summary: format!(
            "Agent reached cycle limit ({}).\n\
             {} error(s) remain — manual review required.\n\
             Try explain_error() on each remaining issue for guidance.",
            MAX_CYCLES, remaining_errors
        ),
    }
}

// -----------------------------------------------------------------------------
// LAYER 9 — WATCH  (`watch` / `watch_report`)
// -----------------------------------------------------------------------------

static WATCH_LOG: OnceLock<Mutex<Vec<WatchEntry>>> = OnceLock::new();

fn watch_log() -> &'static Mutex<Vec<WatchEntry>> {
    WATCH_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

/// Record or update a watched expression value.
///
/// Callable from builtins as `watch("expr_name", "value_string")`.
pub fn watch_value(expr: &str, value: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    if let Ok(mut log) = watch_log().lock() {
        if let Some(entry) = log.iter_mut().find(|e| e.expr == expr) {
            entry.value = value.to_string();
            entry.call_count += 1;
            entry.timestamp_ms = now;
        } else {
            log.push(WatchEntry {
                expr: expr.to_string(),
                value: value.to_string(),
                call_count: 1,
                timestamp_ms: now,
            });
        }
    }
}

/// Return a formatted dump of all watched expressions.
pub fn watch_report() -> String {
    match watch_log().lock() {
        Ok(log) if log.is_empty() => "No watch expressions active.".into(),
        Ok(log) => {
            let mut out = format!("Watch Report ({} expression(s)):\n", log.len());
            for e in log.iter() {
                out.push_str(&format!(
                    "  {} = {}  (updated {} time(s), last at {}ms)\n",
                    e.expr, e.value, e.call_count, e.timestamp_ms
                ));
            }
            out
        }
        Err(_) => "Watch log unavailable (lock poisoned).".into(),
    }
}

// -----------------------------------------------------------------------------
// INTERNAL HELPERS
// -----------------------------------------------------------------------------

/// Strip `// inline comment` from a line of code.
fn strip_inline_comment(line: &str) -> &str {
    if let Some(pos) = line.find("//") {
        // Simple: ignore `//` inside strings (acceptable approximation)
        line[..pos].trim_end()
    } else {
        line
    }
}

/// Detect `name = expr` assignment (not `==`, not `+=`, etc.).
/// Returns `Some(variable_name)` if the line is a plain assignment.
fn detect_assignment(line: &str) -> Option<String> {
    // Look for a single `=` (not `==`, `!=`, `<=`, `>=`)
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'='
            && i > 0
            && !matches!(bytes[i - 1], b'!' | b'<' | b'>' | b'=')
            && bytes.get(i + 1).copied() != Some(b'=')
        {
            let lhs = line[..i].trim();
            if is_simple_identifier(lhs) {
                return Some(lhs.to_string());
            }
            break;
        }
    }
    None
}

fn is_simple_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !is_keyword(s)
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else"
            | "for"
            | "while"
            | "kfn"
            | "return"
            | "true"
            | "false"
            | "in"
            | "match"
            | "enum"
            | "struct"
            | "actor"
            | "handle"
            | "use"
            | "pub"
            | "mod"
            | "null"
    )
}

/// Extract all identifier-like tokens from a line (rough lexer).
fn tokenize_identifiers(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                let s = std::mem::take(&mut current);
                if is_simple_identifier(&s) {
                    tokens.push(s);
                }
            }
        }
    }
    if !current.is_empty() && is_simple_identifier(&current) {
        tokens.push(current);
    }
    tokens
}

fn measure_function_lengths(lines: &[&str]) -> Vec<(String, usize, usize)> {
    let mut result = Vec::new();
    let mut stack: Vec<(String, usize, i32)> = Vec::new(); // (name, start_line, base_depth)
    let mut depth: i32 = 0;

    for (i, &line) in lines.iter().enumerate() {
        let t = line.trim();
        let ln = i + 1;

        // Record function start
        if t.starts_with("kfn ") {
            if let Some(paren) = t.find('(') {
                let name = t[4..paren].trim().to_string();
                stack.push((name, ln, depth));
            }
        }

        let opens  = t.chars().filter(|&c| c == '{').count() as i32;
        let closes = t.chars().filter(|&c| c == '}').count() as i32;
        depth += opens - closes;

        // Check if any tracked function just ended
        stack.retain(|&(ref name, start, base)| {
            if depth <= base && ln > start {
                result.push((name.clone(), ln - start + 1, start));
                false
            } else {
                true
            }
        });
    }

    result
}

fn extract_string_literals(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        // Handle K"..." and "..."
        let is_k = ch == 'K' && chars.peek() == Some(&'"');
        let is_q = ch == '"';
        if is_k || is_q {
            if is_k { chars.next(); } // consume opening "
            let mut s = String::new();
            for c in chars.by_ref() {
                if c == '"' { break; }
                s.push(c);
            }
            if s.len() > 3 {
                result.push(s);
            }
        }
    }
    result
}

fn extract_function_signatures(lines: &[&str]) -> Vec<(String, Vec<String>, usize)> {
    let mut fns = Vec::new();
    for (i, &line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("kfn ") {
            if let (Some(po), Some(pc)) = (t.find('('), t.find(')')) {
                let name   = t[4..po].trim().to_string();
                let params_str = &t[po + 1..pc];
                let params: Vec<String> = if params_str.trim().is_empty() {
                    vec![]
                } else {
                    params_str
                        .split(',')
                        .map(|p| p.trim().split(':').next().unwrap_or("x").trim().to_string())
                        .collect()
                };
                fns.push((name, params, i + 1));
            }
        }
    }
    fns
}

fn generate_test_cases(fn_name: &str, params: &[String]) -> Vec<(String, String, String)> {
    let n  = params.len();
    let nm = fn_name.to_lowercase();
    let mut cases: Vec<(String, String, String)> = Vec::new();

    // -- Arithmetic ------------------------------------------------------------
    if nm.contains("add") || nm.contains("sum") || nm.contains("plus") {
        if n >= 2 {
            cases.push(("basic".into(),    "1, 2".into(),   "3".into()));
            cases.push(("zeros".into(),    "0, 0".into(),   "0".into()));
            cases.push(("negative".into(), "-1, 1".into(),  "0".into()));
            cases.push(("large".into(),    "100, 200".into(),"300".into()));
        }
    } else if nm.contains("sub") || nm.contains("minus") || nm.contains("diff") {
        if n >= 2 {
            cases.push(("basic".into(),    "5, 3".into(), "2".into()));
            cases.push(("zero".into(),     "7, 7".into(), "0".into()));
            cases.push(("negative".into(), "3, 5".into(), "-2".into()));
        }
    } else if nm.contains("mul") || nm.contains("product") || nm.contains("times") {
        if n >= 2 {
            cases.push(("basic".into(),  "3, 4".into(),  "12".into()));
            cases.push(("zero".into(),   "0, 5".into(),  "0".into()));
            cases.push(("one".into(),    "1, 7".into(),  "7".into()));
        }
    } else if nm.contains("div") || nm.contains("quot") {
        if n >= 2 {
            cases.push(("basic".into(),  "10, 2".into(),  "5".into()));
            cases.push(("one".into(),    "7, 1".into(),   "7".into()));
        }
    } else if nm.contains("mod") || nm.contains("remainder") {
        if n >= 2 {
            cases.push(("basic".into(),  "10, 3".into(),  "1".into()));
            cases.push(("zero".into(),   "9, 3".into(),   "0".into()));
        }
    } else if nm.contains("pow") || nm.contains("power") || nm.contains("exp") {
        if n >= 2 {
            cases.push(("basic".into(),  "2, 3".into(),  "8".into()));
            cases.push(("zero_exp".into(),"5, 0".into(), "1".into()));
        }
    } else if nm.contains("sqrt") || nm.contains("square_root") {
        if n >= 1 {
            cases.push(("four".into(),  "4".into(),   "2".into()));
            cases.push(("nine".into(),  "9".into(),   "3".into()));
            cases.push(("zero".into(),  "0".into(),   "0".into()));
        }
    } else if nm.contains("abs") || nm.contains("absolute") {
        if n >= 1 {
            cases.push(("positive".into(),  "5".into(),   "5".into()));
            cases.push(("negative".into(), "-3".into(),   "3".into()));
            cases.push(("zero".into(),      "0".into(),   "0".into()));
        }

    // -- Comparison -----------------------------------------------------------
    } else if nm.contains("max") {
        if n >= 2 {
            cases.push(("basic".into(),  "3, 7".into(),   "7".into()));
            cases.push(("equal".into(),  "5, 5".into(),   "5".into()));
            cases.push(("first".into(),  "9, 2".into(),   "9".into()));
        }
    } else if nm.contains("min") {
        if n >= 2 {
            cases.push(("basic".into(),  "3, 7".into(),   "3".into()));
            cases.push(("equal".into(),  "5, 5".into(),   "5".into()));
        }
    } else if nm.contains("clamp") {
        if n >= 3 {
            cases.push(("below".into(),  "0, 1, 10".into(),  "1".into()));
            cases.push(("above".into(),  "15, 1, 10".into(), "10".into()));
            cases.push(("inside".into(), "5, 1, 10".into(),  "5".into()));
        }

    // -- Boolean checks --------------------------------------------------------
    } else if nm.contains("is_prime") || nm.contains("isprime") || nm.contains("prime") {
        if n >= 1 {
            cases.push(("two".into(),    "2".into(),  "true".into()));
            cases.push(("three".into(),  "3".into(),  "true".into()));
            cases.push(("four".into(),   "4".into(),  "false".into()));
            cases.push(("one".into(),    "1".into(),  "false".into()));
        }
    } else if nm.contains("is_even") || nm.contains("even") {
        if n >= 1 {
            cases.push(("two".into(),    "2".into(),  "true".into()));
            cases.push(("three".into(),  "3".into(),  "false".into()));
            cases.push(("zero".into(),   "0".into(),  "true".into()));
        }
    } else if nm.contains("is_odd") || nm.contains("odd") {
        if n >= 1 {
            cases.push(("one".into(),    "1".into(),  "true".into()));
            cases.push(("two".into(),    "2".into(),  "false".into()));
        }
    } else if nm.contains("palindrome") {
        if n >= 1 {
            cases.push(("racecar".into(),  "\"racecar\"".into(),  "true".into()));
            cases.push(("hello".into(),    "\"hello\"".into(),    "false".into()));
            cases.push(("empty".into(),    "\"\"".into(),         "true".into()));
        }
    } else if nm.contains("contains") || nm.contains("includes") {
        if n >= 2 {
            cases.push(("found".into(),    "\"hello world\", \"world\"".into(), "true".into()));
            cases.push(("not_found".into(),"\"hello\", \"xyz\"".into(),         "false".into()));
        }
    } else if nm.contains("starts_with") || nm.contains("startswith") {
        if n >= 2 {
            cases.push(("match".into(),  "\"hello\", \"he\"".into(), "true".into()));
            cases.push(("no_match".into(),"\"hello\", \"lo\"".into(), "false".into()));
        }
    } else if nm.contains("ends_with") || nm.contains("endswith") {
        if n >= 2 {
            cases.push(("match".into(),  "\"hello\", \"lo\"".into(), "true".into()));
            cases.push(("no_match".into(),"\"hello\", \"he\"".into(), "false".into()));
        }

    // -- Sequences -------------------------------------------------------------
    } else if nm.contains("factorial") || nm.contains("fact") {
        if n >= 1 {
            cases.push(("zero".into(),  "0".into(), "1".into()));
            cases.push(("one".into(),   "1".into(), "1".into()));
            cases.push(("five".into(),  "5".into(), "120".into()));
            cases.push(("ten".into(),   "10".into(), "3628800".into()));
        }
    } else if nm.contains("fib") || nm.contains("fibonacci") {
        if n >= 1 {
            cases.push(("zero".into(),  "0".into(),  "0".into()));
            cases.push(("one".into(),   "1".into(),  "1".into()));
            cases.push(("six".into(),   "6".into(),  "8".into()));
            cases.push(("ten".into(),   "10".into(), "55".into()));
        }
    } else if nm.contains("gcd") || nm.contains("greatest_common") {
        if n >= 2 {
            cases.push(("basic".into(),  "12, 8".into(),   "4".into()));
            cases.push(("prime".into(),  "7, 5".into(),    "1".into()));
            cases.push(("same".into(),   "6, 6".into(),    "6".into()));
        }
    } else if nm.contains("lcm") || nm.contains("least_common") {
        if n >= 2 {
            cases.push(("basic".into(),  "4, 6".into(),    "12".into()));
        }

    // -- Array / list  ---------------------------------------------------------
    } else if nm.contains("reverse") || nm.contains("reversed") {
        if n >= 1 {
            cases.push(("basic".into(),  "[1, 2, 3]".into(),   "[3, 2, 1]".into()));
            cases.push(("single".into(), "[42]".into(),        "[42]".into()));
            cases.push(("empty".into(),  "[]".into(),          "[]".into()));
        }
    } else if nm.contains("sort") || nm.contains("sorted") {
        if n >= 1 {
            cases.push(("basic".into(),  "[3, 1, 2]".into(),   "[1, 2, 3]".into()));
            cases.push(("sorted".into(), "[1, 2, 3]".into(),   "[1, 2, 3]".into()));
            cases.push(("empty".into(),  "[]".into(),          "[]".into()));
        }
    } else if nm.contains("len") || nm.contains("length") || nm.contains("size") || nm.contains("count") {
        cases.push(("empty".into(),  "[]".into(),          "0".into()));
        cases.push(("single".into(), "[42]".into(),        "1".into()));
        cases.push(("three".into(),  "[1, 2, 3]".into(),   "3".into()));
    } else if nm.contains("sum") && n == 1 {
        cases.push(("basic".into(),  "[1, 2, 3]".into(),   "6".into()));
        cases.push(("empty".into(),  "[]".into(),           "0".into()));
    } else if nm.contains("flatten") {
        if n >= 1 {
            cases.push(("basic".into(), "[[1, 2], [3, 4]]".into(), "[1, 2, 3, 4]".into()));
            cases.push(("empty".into(), "[]".into(),                "[]".into()));
        }
    } else if nm.contains("unique") || nm.contains("dedup") {
        if n >= 1 {
            cases.push(("dups".into(),  "[1, 2, 2, 3, 3]".into(), "[1, 2, 3]".into()));
            cases.push(("clean".into(), "[1, 2, 3]".into(),        "[1, 2, 3]".into()));
        }
    } else if nm.contains("zip") {
        if n >= 2 {
            cases.push(("basic".into(), "[1, 2], [3, 4]".into(), "[[1, 3], [2, 4]]".into()));
        }

    // -- String operations -----------------------------------------------------
    } else if nm.contains("upper") || nm.contains("to_upper") || nm.contains("uppercase") {
        if n >= 1 {
            cases.push(("basic".into(),  "\"hello\"".into(),  "\"HELLO\"".into()));
            cases.push(("empty".into(),  "\"\"".into(),       "\"\"".into()));
        }
    } else if nm.contains("lower") || nm.contains("to_lower") || nm.contains("lowercase") {
        if n >= 1 {
            cases.push(("basic".into(),  "\"HELLO\"".into(),  "\"hello\"".into()));
        }
    } else if nm.contains("trim") || nm.contains("strip") {
        if n >= 1 {
            cases.push(("spaces".into(), "\"  hello  \"".into(), "\"hello\"".into()));
            cases.push(("clean".into(),  "\"hello\"".into(),     "\"hello\"".into()));
        }
    } else if nm.contains("repeat") {
        if n >= 2 {
            cases.push(("basic".into(),  "\"ab\", 3".into(), "\"ababab\"".into()));
            cases.push(("zero".into(),   "\"ab\", 0".into(), "\"\"".into()));
        }
    } else if nm.contains("concat") {
        if n >= 2 {
            cases.push(("basic".into(), "\"hello\", \" world\"".into(), "\"hello world\"".into()));
        }

    // -- Type conversion -------------------------------------------------------
    } else if nm == "to_str" || nm == "to_string" || nm.contains("stringify") {
        if n >= 1 {
            cases.push(("number".into(), "42".into(),    "\"42\"".into()));
            cases.push(("zero".into(),   "0".into(),     "\"0\"".into()));
        }
    } else if nm == "to_int" || nm == "parse_int" || nm.contains("to_number") {
        if n >= 1 {
            cases.push(("basic".into(), "\"42\"".into(),  "42".into()));
            cases.push(("zero".into(),  "\"0\"".into(),   "0".into()));
        }

    // -- Rounding / precision --------------------------------------------------
    } else if nm.contains("round") {
        if n >= 1 {
            cases.push(("up".into(),    "2.6".into(), "3".into()));
            cases.push(("down".into(),  "2.4".into(), "2".into()));
        }
    } else if nm.contains("floor") {
        if n >= 1 {
            cases.push(("basic".into(), "2.9".into(), "2".into()));
        }
    } else if nm.contains("ceil") {
        if n >= 1 {
            cases.push(("basic".into(), "2.1".into(), "3".into()));
        }

    // -- Generic fallback: try Ollama, otherwise emit labelled TODO ------------
    } else {
        // Try LLM for unknown function names
        if let Some(ai_cases) = generate_test_cases_via_llm(fn_name, params) {
            return ai_cases;
        }
        // Plain scaffold with clearly labelled placeholders
        let inputs: String = match n {
            0 => String::new(),
            1 => "/* arg1 */".into(),
            2 => "/* arg1 */, /* arg2 */".into(),
            3 => "/* arg1 */, /* arg2 */, /* arg3 */".into(),
            _ => (0..n).map(|j| format!("/* arg{} */", j + 1)).collect::<Vec<_>>().join(", "),
        };
        cases.push(("smoke".into(), inputs, "/* expected_result */".into()));
    }

    cases
}

/// Ask Ollama to generate 3 concrete test cases for an unknown function.
/// Returns None if Ollama is unavailable (no network penalty — pure TCP timeout).
fn generate_test_cases_via_llm(
    fn_name: &str,
    params: &[String],
) -> Option<Vec<(String, String, String)>> {
    let param_list = params.join(", ");
    let prompt = format!(
        "For a Killer language function named '{fn_name}' with parameters ({param_list}), \
         give exactly 3 test cases in this format, one per line:\n\
         CASE: description | input_args | expected_output\n\
         Use simple literal values (numbers, strings, arrays). No explanation.",
    );
    let response = crate::llm::ask(&crate::llm::LlmConfig::ollama("llama3.2"), &prompt).ok()?;

    let mut cases = Vec::new();
    for line in response.lines() {
        let line = line.trim();
        if !line.starts_with("CASE:") { continue; }
        let parts: Vec<&str> = line["CASE:".len()..].splitn(3, '|').collect();
        if parts.len() == 3 {
            cases.push((
                parts[0].trim().to_string(),
                parts[1].trim().to_string(),
                parts[2].trim().to_string(),
            ));
        }
    }
    if cases.is_empty() { None } else { Some(cases) }
}

// -----------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_check_fn_keyword() {
        let code = "fn add(a, b) {\n  a + b\n}";
        let issues = debug_check(code);
        assert!(issues.iter().any(|i| i.code == "E001"));
    }

    #[test]
    fn test_debug_check_let_keyword() {
        let code = "let x = 5\nprint(x)";
        let issues = debug_check(code);
        assert!(issues.iter().any(|i| i.code == "E002"));
    }

    #[test]
    fn test_debug_check_unclosed_brace() {
        let code = "kfn foo() {\n  x = 1\n";
        let issues = debug_check(code);
        assert!(issues.iter().any(|i| i.code == "E007"));
    }

    #[test]
    fn test_auto_fix_fn_to_kfn() {
        let code = "fn add(a, b) {\n  a + b\n}";
        let fixes = auto_fix(code);
        assert!(!fixes.is_empty());
        assert!(fixes[0].fixed_code.contains("kfn add"));
    }

    #[test]
    fn test_auto_fix_let_removal() {
        let code = "let x = 5\nprint(x)";
        let fixes = auto_fix(code);
        assert!(!fixes.is_empty());
        assert!(!fixes[0].fixed_code.contains("let "));
    }

    #[test]
    fn test_auto_fix_no_changes_on_valid_code() {
        let code = "kfn add(a, b) {\n  a + b\n}\nprint(add(1, 2))";
        let fixes = auto_fix(code);
        assert_eq!(fixes[0].changes.len(), 0);
        assert_eq!(fixes[0].confidence, 1.0);
    }

    #[test]
    fn test_killer_debug_agent_fixes_fn() {
        let bad = "fn greet(name) {\n  K\"Hello {name}\"\n}";
        let result = killer_debug_agent(bad);
        assert!(result.fixed_code.contains("kfn"));
        assert!(result.cycles <= 10);
    }

    #[test]
    fn test_auto_test_generates_for_add() {
        let code = "kfn add(a, b) {\n  a + b\n}";
        let tests = auto_test(code);
        assert!(tests.contains("add(1, 2)"));
        assert!(tests.contains("PASS") || tests.contains("FAIL"));
    }

    #[test]
    fn test_perf_profile_nested_loop() {
        let code = "for i in range(10) {\n  for j in range(10) {\n    x = i + j\n  }\n}";
        let hints = perf_profile(code);
        assert!(hints.iter().any(|h| h.category == "NestedLoop"));
    }

    #[test]
    fn test_suggest_refactor_repeated_string() {
        let code = concat!(
            "print(\"hello world\")\n",
            "x = \"hello world\"\n",
            "y = \"hello world\"\n",
        );
        let suggestions = suggest_refactor(code);
        assert!(suggestions.iter().any(|s| s.code == "S002"));
    }

    #[test]
    fn test_watch_value_and_report() {
        watch_value("total", "42");
        watch_value("total", "99");
        let report = watch_report();
        assert!(report.contains("total"));
        assert!(report.contains("99"));
    }

    #[test]
    fn test_explain_error_undefined_variable() {
        let msg = "undefined variable 'counter'";
        let exp = explain_error(msg, "");
        assert!(exp.contains("counter"));
    }

    #[test]
    fn test_ai_pair_hello_template() {
        let code = ai_pair("hello world greeter");
        assert!(code.contains("kfn") || code.contains("greet"));
    }
}
