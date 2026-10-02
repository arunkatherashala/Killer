//! Gradual type checker for the line-based Killer pipeline.
//!
//! Annotations are optional: `fn add(a: number, b: number) -> number {` and `x: string = "hi"`.
//! `process` strips them (so the line compiler sees plain code) and reports *definite* type
//! errors only. Anything it cannot determine is `any` and never produces an error. Source
//! without annotations is returned unchanged.

use crate::error::VmError;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Number,
    Str,
    Bool,
    Array,
    Dict,
    Any,
}

impl Ty {
    fn parse(s: &str) -> Option<Ty> {
        Some(match s.trim() {
            "number" => Ty::Number,
            "string" => Ty::Str,
            "bool" => Ty::Bool,
            "array" => Ty::Array,
            "dict" => Ty::Dict,
            "any" | "void" => Ty::Any,
            _ => return None,
        })
    }
    fn name(self) -> &'static str {
        match self {
            Ty::Number => "number",
            Ty::Str => "string",
            Ty::Bool => "bool",
            Ty::Array => "array",
            Ty::Dict => "dict",
            Ty::Any => "any",
        }
    }
    fn accepts(self, got: Ty) -> bool {
        self == Ty::Any || got == Ty::Any || self == got
    }
}

#[derive(Debug, Clone)]
struct FnSig {
    params: Vec<Ty>,
    /// parameters without a default: the minimum number of arguments a call must pass
    required: usize,
    ret: Ty,
    annotated: bool,
}

type Env = HashMap<String, Ty>;

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Index of the matching close bracket for the opener at `open`, skipping quoted text.
fn matching(s: &[char], open: usize) -> Option<usize> {
    let (o, c) = match s[open] {
        '(' => ('(', ')'),
        '[' => ('[', ']'),
        '{' => ('{', '}'),
        _ => return None,
    };
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut i = open;
    while i < s.len() {
        let ch = s[i];
        if let Some(q) = quote {
            if ch == '\\' {
                i += 1;
            } else if ch == q {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == o {
            depth += 1;
        } else if ch == c {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Split on top-level commas (outside quotes and brackets).
fn split_args(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if let Some(q) = quote {
            cur.push(ch);
            if ch == '\\' && i + 1 < chars.len() {
                i += 1;
                cur.push(chars[i]);
            } else if ch == q {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            cur.push(ch);
        } else if matches!(ch, '(' | '[' | '{') {
            depth += 1;
            cur.push(ch);
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
            cur.push(ch);
        } else if ch == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
        i += 1;
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Position of the lowest-precedence top-level binary operator (rightmost within its level).
fn find_binary(s: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = s.chars().collect();
    const LEVELS: &[&[&str]] = &[
        &["||", "&&"],
        &["==", "!=", "<=", ">=", "<", ">"],
        &["+", "-"],
        &["*", "/", "%"],
    ];
    for level in LEVELS {
        let mut depth = 0i32;
        let mut quote: Option<char> = None;
        let mut found: Option<(usize, usize)> = None;
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            if let Some(q) = quote {
                if ch == '\\' {
                    i += 1;
                } else if ch == q {
                    quote = None;
                }
                i += 1;
                continue;
            }
            if ch == '"' || ch == '\'' {
                quote = Some(ch);
            } else if matches!(ch, '(' | '[' | '{') {
                depth += 1;
            } else if matches!(ch, ')' | ']' | '}') {
                depth -= 1;
            } else if depth == 0 && i > 0 {
                for op in *level {
                    let oc: Vec<char> = op.chars().collect();
                    if chars[i..].starts_with(&oc) {
                        // reject unary minus/plus and multi-char operators split in half
                        let prev = chars[..i].iter().rev().find(|c| !c.is_whitespace());
                        let binary_ctx = matches!(prev, Some(p) if is_ident_char(*p) || matches!(p, ')' | ']' | '"' | '\''));
                        let next = chars.get(i + oc.len());
                        let splits_op = (oc.len() == 1 && matches!(next, Some('=' | '&' | '|')) && matches!(*op, "<" | ">"))
                            || (oc.len() == 1 && matches!(chars[i - 1], '=' | '!' | '<' | '>' | '&' | '|'));
                        if binary_ctx && !splits_op {
                            found = Some((i, oc.len()));
                        }
                        break;
                    }
                }
            }
            i += 1;
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn infer(expr: &str, env: &Env, fns: &HashMap<String, FnSig>) -> Ty {
    let e = expr.trim();
    if e.is_empty() {
        return Ty::Any;
    }
    if let Some((pos, len)) = find_binary(e) {
        let chars: Vec<char> = e.chars().collect();
        let l: String = chars[..pos].iter().collect();
        let r: String = chars[pos + len..].iter().collect();
        let op: String = chars[pos..pos + len].iter().collect();
        let (lt, rt) = (infer(&l, env, fns), infer(&r, env, fns));
        return match op.as_str() {
            "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||" => Ty::Bool,
            "+" => {
                if lt == Ty::Str || rt == Ty::Str {
                    Ty::Str
                } else if lt == Ty::Number && rt == Ty::Number {
                    Ty::Number
                } else {
                    Ty::Any
                }
            }
            _ => {
                if lt == Ty::Number && rt == Ty::Number {
                    Ty::Number
                } else {
                    Ty::Any
                }
            }
        };
    }
    let chars: Vec<char> = e.chars().collect();
    if chars[0] == '(' && matching(&chars, 0) == Some(chars.len() - 1) {
        let inner: String = chars[1..chars.len() - 1].iter().collect();
        return infer(&inner, env, fns);
    }
    if (chars[0] == '"' || chars[0] == '\'') && chars.len() >= 2 && chars[chars.len() - 1] == chars[0] {
        return Ty::Str;
    }
    if e == "true" || e == "false" {
        return Ty::Bool;
    }
    if chars[0] == '[' && matching(&chars, 0) == Some(chars.len() - 1) {
        return Ty::Array;
    }
    if chars[0] == '{' && matching(&chars, 0) == Some(chars.len() - 1) {
        return Ty::Dict;
    }
    if e.parse::<f64>().is_ok() {
        return Ty::Number;
    }
    if let Some(rest) = e.strip_prefix('-') {
        return match infer(rest, env, fns) {
            Ty::Number => Ty::Number,
            _ => Ty::Any,
        };
    }
    if e.chars().all(is_ident_char) {
        return env.get(e).copied().unwrap_or(Ty::Any);
    }
    if let Some(open) = e.find('(') {
        let name = &e[..open];
        if !name.is_empty() && name.chars().all(is_ident_char) && e.ends_with(')') {
            if let Some(sig) = fns.get(name) {
                return sig.ret;
            }
            return match name {
                "str" => Ty::Str,
                "len" | "sqrt" => Ty::Number,
                _ => Ty::Any,
            };
        }
    }
    Ty::Any
}

struct Header {
    keyword_len: usize, // chars up to and including the space after fn/kfn
    name: String,
    params: Vec<(String, Option<Ty>)>,
    ret: Option<Ty>,
    rebuilt: String, // header without annotations
    required: usize,
    annotated: bool,
}

/// Index of the first `=` outside quotes/brackets that is an assignment, not `==`, `<=`, `>=`, `!=`, `=>`.
fn find_top_level_eq(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let (mut depth, mut quote) = (0i32, 0u8);
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == 92 {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else {
            match c {
                34 | 39 => quote = c,
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b'=' if depth == 0 => {
                    let prev = if i > 0 { b[i - 1] } else { 0 };
                    let next = if i + 1 < b.len() { b[i + 1] } else { 0 };
                    if !matches!(prev, b'=' | b'<' | b'>' | b'!') && next != b'=' && next != b'>' {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Parse `fn name(a: t, b) -> t {` (indent already trimmed).
fn parse_header(trimmed: &str) -> Option<Result<Header, String>> {
    let kw = if trimmed.starts_with("kfn ") {
        4
    } else if trimmed.starts_with("fn ") {
        3
    } else {
        return None;
    };
    let chars: Vec<char> = trimmed.chars().collect();
    let open = chars.iter().position(|c| *c == '(')?;
    let name: String = chars[kw..open].iter().collect::<String>().trim().to_string();
    if name.is_empty() || !name.chars().all(is_ident_char) {
        return None;
    }
    let close = matching(&chars, open)?;
    let inner: String = chars[open + 1..close].iter().collect();
    let mut params = Vec::new();
    let mut rebuilt_params: Vec<String> = Vec::new();
    let mut required: Option<usize> = None;
    let mut annotated = false;
    for raw in split_args(&inner) {
        // `name: type = default` / `name = default`: peel the default off first, because the
        // default expression may itself contain ':' (a dict or a string)
        let (head, default) = match find_top_level_eq(&raw) {
            Some(i) => (raw[..i].trim().to_string(), Some(raw[i + 1..].trim().to_string())),
            None => (raw.trim().to_string(), None),
        };
        if default.is_some() && required.is_none() {
            required = Some(params.len());
        }
        let with_default = |name: &str| match &default {
            Some(d) => format!("{} = {}", name, d),
            None => name.to_string(),
        };
        match head.split_once(':') {
            Some((n, t)) if n.trim().chars().all(is_ident_char) && !n.trim().is_empty() => {
                let n = n.trim().to_string();
                match Ty::parse(t) {
                    Some(ty) => {
                        annotated = true;
                        rebuilt_params.push(with_default(&n));
                        params.push((n, Some(ty)));
                    }
                    None => return Some(Err(format!("unknown type '{}' for parameter '{}'", t.trim(), n))),
                }
            }
            _ => {
                let n = head.clone();
                rebuilt_params.push(with_default(&n));
                params.push((n, None));
            }
        }
    }
    let rest: String = chars[close + 1..].iter().collect();
    let rest_trim = rest.trim_start();
    let (ret, tail) = if let Some(after) = rest_trim.strip_prefix("->") {
        let end = after.find(|c| c == '{' || c == ':').unwrap_or(after.len());
        let tname = after[..end].trim();
        match Ty::parse(tname) {
            Some(t) => {
                annotated = true;
                (Some(t), after[end..].to_string())
            }
            None => return Some(Err(format!("unknown return type '{}'", tname))),
        }
    } else {
        (None, rest.clone())
    };
    let prefix: String = chars[..open].iter().collect();
    let tail = if ret.is_some() && !tail.is_empty() { format!(" {}", tail) } else { tail };
    let rebuilt = format!("{}({}){}", prefix, rebuilt_params.join(", "), tail);
    let required = required.unwrap_or(params.len());
    Some(Ok(Header { keyword_len: kw, name, params, ret, rebuilt, required, annotated }))
}

/// `name: type = expr` → (name, type, expr)
fn parse_var_annotation(trimmed: &str) -> Option<(String, Ty, String)> {
    let (lhs, rhs) = trimmed.split_once('=')?;
    if rhs.starts_with('=') {
        return None;
    }
    let (name, ty) = lhs.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(is_ident_char) {
        return None;
    }
    Some((name.to_string(), Ty::parse(ty)?, rhs.trim().to_string()))
}

/// `name = expr` (plain assignment, optional leading `let`) → (name, expr)
fn parse_assign(trimmed: &str) -> Option<(String, String)> {
    let t = trimmed.strip_prefix("let ").unwrap_or(trimmed);
    let eq = t.find('=')?;
    let (lhs, rhs) = (&t[..eq], &t[eq + 1..]);
    if rhs.starts_with('=') || lhs.is_empty() {
        return None;
    }
    let lhs = lhs.trim();
    if lhs.is_empty() || !lhs.chars().all(is_ident_char) {
        return None;
    }
    Some((lhs.to_string(), rhs.trim().to_string()))
}

fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

fn check_calls(line: &str, lineno: usize, env: &Env, fns: &HashMap<String, FnSig>, errs: &mut Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let ch = chars[i];
        if let Some(q) = quote {
            if ch == '\\' {
                i += 1;
            } else if ch == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            i += 1;
            continue;
        }
        if is_ident_char(ch) && (i == 0 || !(is_ident_char(chars[i - 1]) || chars[i - 1] == '.')) {
            let mut j = i;
            while j < chars.len() && is_ident_char(chars[j]) {
                j += 1;
            }
            let name: String = chars[i..j].iter().collect();
            if j < chars.len() && chars[j] == '(' {
                if let Some(sig) = fns.get(&name) {
                    if sig.annotated {
                        if let Some(close) = matching(&chars, j) {
                            let inner: String = chars[j + 1..close].iter().collect();
                            let args = split_args(&inner);
                            if args.len() < sig.required || args.len() > sig.params.len() {
                                errs.push(format!(
                                    "type error (line {}): {}() takes {} argument(s), got {}",
                                    lineno,
                                    name,
                                    if sig.required == sig.params.len() { sig.params.len().to_string() } else { format!("{} to {}", sig.required, sig.params.len()) },
                                    args.len()
                                ));
                            } else {
                                for (k, (a, p)) in args.iter().zip(&sig.params).enumerate() {
                                    let got = infer(a, env, fns);
                                    if !p.accepts(got) {
                                        errs.push(format!(
                                            "type error (line {}): argument {} of {}() expects {}, got {}",
                                            lineno, k + 1, name, p.name(), got.name()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
}

/// Strip annotations and collect type errors. Returns the plain source on success.
pub fn process(source: &str) -> Result<String, VmError> {
    let lines: Vec<&str> = source.lines().collect();

    // Pass 1: collect function signatures.
    let mut fns: HashMap<String, FnSig> = HashMap::new();
    let mut any_annotation = false;
    let mut errs: Vec<String> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        match parse_header(trimmed) {
            Some(Ok(h)) => {
                any_annotation |= h.annotated;
                fns.insert(
                    h.name.clone(),
                    FnSig {
                        params: h.params.iter().map(|(_, t)| t.unwrap_or(Ty::Any)).collect(),
                        required: h.required,
                        ret: h.ret.unwrap_or(Ty::Any),
                        annotated: h.annotated,
                    },
                );
            }
            Some(Err(m)) => errs.push(format!("type error (line {}): {}", idx + 1, m)),
            None => {
                if parse_var_annotation(trimmed).is_some() {
                    any_annotation = true;
                }
            }
        }
    }
    if !errs.is_empty() {
        return Err(VmError::parse_error_simple(errs.join("\n")));
    }
    if !any_annotation {
        return Ok(source.to_string());
    }

    // Pass 2: check bodies and emit stripped source.
    let mut out = String::with_capacity(source.len());
    let mut global: Env = HashMap::new();
    let mut global_declared: HashSet<String> = HashSet::new();
    let mut scope: Option<(usize, Ty, Env, HashSet<String>)> = None; // (fn indent, ret, env, declared)

    for (idx, line) in lines.iter().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        let indent = indent_of(line);

        if !trimmed.is_empty() {
            if let Some((fi, ..)) = &scope {
                if indent <= *fi && !trimmed.starts_with('}') {
                    scope = None;
                }
            }
        }

        let mut emitted = line.to_string();

        if let Some(Ok(h)) = parse_header(trimmed) {
            let mut env: Env = HashMap::new();
            let mut declared = HashSet::new();
            for (n, t) in &h.params {
                env.insert(n.clone(), t.unwrap_or(Ty::Any));
                if t.is_some() {
                    declared.insert(n.clone());
                }
            }
            let _ = h.keyword_len;
            scope = Some((indent, h.ret.unwrap_or(Ty::Any), env, declared));
            emitted = format!("{}{}", &line[..line.len() - trimmed.len()], h.rebuilt);
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
            let (env, declared, ret): (&mut Env, &mut HashSet<String>, Option<Ty>) = match scope.as_mut() {
                Some((_, r, e, d)) => (e, d, Some(*r)),
                None => (&mut global, &mut global_declared, None),
            };

            if let Some((name, ty, rhs)) = parse_var_annotation(trimmed) {
                let got = infer(&rhs, env, &fns);
                if !ty.accepts(got) {
                    errs.push(format!(
                        "type error (line {}): '{}' is declared {}, got {}",
                        lineno, name, ty.name(), got.name()
                    ));
                }
                env.insert(name.clone(), ty);
                declared.insert(name.clone());
                emitted = format!("{}{} = {}", &line[..line.len() - trimmed.len()], name, rhs);
                check_calls(&rhs, lineno, env, &fns, &mut errs);
            } else {
                check_calls(trimmed, lineno, env, &fns, &mut errs);
                if let Some(rest) = trimmed.strip_prefix("return ").or(if trimmed == "return" { Some("") } else { None }) {
                    if let Some(r) = ret {
                        let got = infer(rest, env, &fns);
                        if !rest.trim().is_empty() && !r.accepts(got) {
                            errs.push(format!(
                                "type error (line {}): function returns {}, got {}",
                                lineno, r.name(), got.name()
                            ));
                        }
                    }
                } else if let Some((name, rhs)) = parse_assign(trimmed) {
                    let got = infer(&rhs, env, &fns);
                    if declared.contains(&name) {
                        let want = env[&name];
                        if !want.accepts(got) {
                            errs.push(format!(
                                "type error (line {}): '{}' is {}, cannot assign {}",
                                lineno, name, want.name(), got.name()
                            ));
                        }
                    } else {
                        let merged = match env.get(&name) {
                            Some(prev) if *prev != got => Ty::Any,
                            _ => got,
                        };
                        env.insert(name, merged);
                    }
                }
            }
        }

        out.push_str(&emitted);
        out.push('\n');
    }

    if errs.is_empty() {
        Ok(out)
    } else {
        Err(VmError::parse_error_simple(errs.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(src: &str) -> String {
        match process(src) {
            Err(e) => format!("{:?}", e),
            Ok(_) => String::new(),
        }
    }

    #[test]
    fn unannotated_source_is_unchanged() {
        let src = "fn add(a, b) {\n  return a + b\n}\nprintln(add(1, \"x\"))\n";
        assert_eq!(process(src).unwrap(), src);
    }

    #[test]
    fn strips_annotations() {
        let src = "fn add(a: number, b: number) -> number {\n  return a + b\n}\nx: number = 5\nprintln(add(x, 2))\n";
        let out = process(src).unwrap();
        assert!(out.contains("fn add(a, b) {"), "{}", out);
        assert!(out.contains("x = 5"), "{}", out);
        assert!(!out.contains("->") && !out.contains(": number"));
    }

    #[test]
    fn catches_bad_argument() {
        let e = err("fn add(a: number, b: number) -> number {\n  return a + b\n}\nadd(1, \"two\")\n");
        assert!(e.contains("argument 2 of add() expects number, got string"), "{}", e);
    }

    #[test]
    fn catches_arity_and_return() {
        let e = err("fn f(a: number) -> string {\n  return a * 2\n}\nf(1, 2)\n");
        assert!(e.contains("takes 1 argument(s), got 2"), "{}", e);
        assert!(e.contains("function returns string, got number"), "{}", e);
    }

    #[test]
    fn catches_annotated_variable_misuse() {
        let e = err("n: number = \"hello\"\n");
        assert!(e.contains("'n' is declared number, got string"), "{}", e);
        let e = err("n: number = 1\nn = \"s\"\n");
        assert!(e.contains("cannot assign string"), "{}", e);
    }

    #[test]
    fn unknown_values_never_error() {
        let src = "fn f(a: number) -> number {\n  return a\n}\nv = readFile(\"x\")\nprintln(f(v))\nprintln(f(unknown_call(3)))\n";
        assert!(process(src).is_ok());
    }

    #[test]
    fn inference_through_variables_and_calls() {
        let e = err("fn dbl(a: number) -> number {\n  return a * 2\n}\ns = \"hi\"\ndbl(s)\n");
        assert!(e.contains("expects number, got string"), "{}", e);
        let ok = "fn dbl(a: number) -> number {\n  return a * 2\n}\nprintln(dbl(dbl(3) + 1))\n";
        assert!(process(ok).is_ok());
    }

    #[test]
    fn defaults_are_kept_and_make_arguments_optional() {
        let src = "fn greet(name: string, greeting: string = \"Hi: there\", opts = {\"a\": 1}) -> string {
  return greeting
}
greet(\"x\")
greet(\"x\", \"y\")
";
        let out = process(src).unwrap();
        assert!(out.contains("fn greet(name, greeting = \"Hi: there\", opts = {\"a\": 1}) {"), "{}", out);
        // omitting defaulted parameters is fine, omitting the required one is not
        assert!(err("fn f(a: number, b: number = 2) -> number {
  return a
}
f()
").contains("takes 1 to 2 argument(s), got 0"));
        assert!(err("fn f(a: number, b: number = 2) -> number {
  return a
}
f(1, 2, 3)
").contains("got 3"));
        assert!(process("fn f(a: number, b: number = 2) -> number {
  return a
}
f(1)
f(1, 5)
").is_ok());
    }

    #[test]
    fn rejects_unknown_type_name() {
        assert!(err("fn f(a: nubmer) {\n  return a\n}\n").contains("unknown type 'nubmer'"));
    }
}
