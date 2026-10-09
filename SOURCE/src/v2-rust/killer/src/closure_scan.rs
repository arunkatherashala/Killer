//! Text analysis behind shared closure variables.
//!
//! A local of an enclosing function that a nested function captures *and* that gets assigned
//! (by the nested function, or more than once by the enclosing one) must be one shared variable
//! instead of a per-closure copy. The compiler works on normalized lines (one statement per line,
//! `{` and `}` on lines of their own), so the decision is made here from the function's text before
//! its body is compiled: [`boxed_variables`] names the variables that live in a shared cell for the
//! whole call. Everything else keeps the plain copy-at-creation capture and the slot fast paths.

use std::collections::{HashMap, HashSet};

/// A function definition nested directly in the body being scanned.
pub(crate) struct Nested {
    /// Index of the `fn` header line in the scanned slice.
    pub start: usize,
    /// Index just past the line holding the closing `}`.
    pub end: usize,
    /// Parameter names (single-line headers).
    pub params: Vec<String>,
}

fn is_fn_header(line: &str) -> bool {
    line.starts_with("fn ") || line.starts_with("kfn ") || line.starts_with("async fn ") || line.starts_with("async kfn ")
}

fn header_params(header: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (header.find('('), header.rfind(')')) else { return Vec::new() };
    if close <= open {
        return Vec::new();
    }
    header[open + 1..close]
        .split(',')
        .filter_map(|p| {
            let name: String = p.trim().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.is_empty() { None } else { Some(name) }
        })
        .collect()
}

/// The functions defined directly in `body` (not those nested deeper: their text is inside).
pub(crate) fn nested_fns(body: &[(usize, String)]) -> Vec<Nested> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let t = body[i].1.trim();
        if is_fn_header(t) {
            let mut j = i;
            while j < body.len() && body[j].1.trim() != "{" {
                j += 1;
            }
            let end = crate::compiler::skip_braced_block(body, j.min(body.len().saturating_sub(1))).max(i + 1);
            out.push(Nested { start: i, end, params: header_params(&body[i..=j.min(body.len() - 1)].iter().map(|l| l.1.trim()).collect::<Vec<_>>().join(" ")) });
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// A line with the contents of string literals blanked out.
fn masked(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                    out.push(c);
                } else {
                    out.push('_');
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                }
                out.push(c);
            }
        }
    }
    out
}

/// Names a single statement line assigns (`x = ..`, `x += ..`, `x++`, `for x in ..`). With
/// `with_decls` a `let x = ..` counts too (it defines the enclosing function's own variable).
fn assigned_in_line(line: &str, with_decls: bool) -> Vec<String> {
    let m = masked(line);
    let mut t = m.trim();
    let mut out = Vec::new();
    if let Some(rest) = t.strip_prefix("for ") {
        let name: String = rest.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            out.push(name);
        }
        return out;
    }
    let mut decl = false;
    for kw in ["let ", "const ", "var "] {
        if let Some(rest) = t.strip_prefix(kw) {
            t = rest.trim_start();
            decl = true;
            break;
        }
    }
    if decl && !with_decls {
        return out;
    }
    let name: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    if name.is_empty() || name.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return out;
    }
    let rest = t[name.len()..].trim_start();
    let b = rest.as_bytes();
    let is_assign = match b {
        [b'=', next, ..] => *next != b'=' && *next != b'>',
        [b'=']  => true,
        [b'+', b'+', ..] | [b'-', b'-', ..] => true,
        [b':', ..] if decl => true, // `let x: type = ..`
        [op, b'=', next, ..] if b"+-*/%".contains(op) => *next != b'=',
        [op, b'='] if b"+-*/%".contains(op) => true,
        _ => false,
    };
    if is_assign {
        out.push(name);
    }
    out
}

/// Every name assigned anywhere in `lines` (nested bodies included), with `let` declarations only
/// counted when `with_decls`.
pub(crate) fn assigned_names(lines: &[(usize, String)], with_decls: bool) -> HashSet<String> {
    lines.iter().flat_map(|(_, l)| assigned_in_line(l, with_decls)).collect()
}

/// The shared-cell variables of one function: name -> source line of the first assignment that
/// defines it (0 for parameters, so they count as defined before any nested function).
///
/// `body` is the function's body (between its braces), `params` its parameters, `captured` the
/// names it got from its own enclosing function (never re-declared here), and `skip` names that
/// are not locals (`global x`).
pub(crate) fn boxed_variables(
    body: &[(usize, String)],
    params: &[String],
    captured: &HashSet<String>,
    skip: &HashSet<String>,
) -> HashMap<String, usize> {
    let nested = nested_fns(body);
    if nested.is_empty() {
        return HashMap::new();
    }
    // what the nested functions use / assign (their own parameters excluded)
    let mut nested_used: HashSet<String> = HashSet::new();
    let mut nested_assigned: HashSet<String> = HashSet::new();
    for n in &nested {
        let text = &body[n.start..n.end];
        let own: HashSet<&String> = n.params.iter().collect();
        for name in crate::compiler::identifiers_in_lines(text) {
            if !own.contains(&name) {
                nested_used.insert(name);
            }
        }
        for name in assigned_names(&text[1..], false) {
            if !own.contains(&name) {
                nested_assigned.insert(name);
            }
        }
    }
    // what this function assigns outside the nested definitions
    let mut own_count: HashMap<String, usize> = HashMap::new();
    let mut first_line: HashMap<String, usize> = HashMap::new();
    for p in params {
        *own_count.entry(p.clone()).or_insert(0) += 1;
        first_line.insert(p.clone(), 0);
    }
    let mut i = 0;
    let mut next = nested.iter().peekable();
    while i < body.len() {
        if let Some(n) = next.peek() {
            if i == n.start {
                i = n.end;
                next.next();
                continue;
            }
        }
        for name in assigned_in_line(&body[i].1, true) {
            *own_count.entry(name.clone()).or_insert(0) += 1;
            first_line.entry(name).or_insert(body[i].0);
        }
        i += 1;
    }
    let mut out = HashMap::new();
    for (name, count) in own_count {
        if captured.contains(&name) || skip.contains(&name) || !nested_used.contains(&name) {
            continue;
        }
        if nested_assigned.contains(&name) || count >= 2 {
            let line = first_line.get(&name).copied().unwrap_or(0);
            out.insert(name, line);
        }
    }
    out
}
