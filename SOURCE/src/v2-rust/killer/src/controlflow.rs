//! Lowers statement forms that have no bytecode of their own into the core `if` / `while` / `for`
//! the compiler already knows. Runs on brace-style source text, before the lambda pass (so that
//! the `=>` of `match` arms is never mistaken for an arrow function).
//!
//! ```text
//! match x { 1 => a()  2 | 3 => b()  n if n > 9 => c(n)  _ => d() }
//!     __m_0 = x ; if __m_0 == 1 { a() } else if (__m_0 == 2) || (__m_0 == 3) { b() } else if ... else { d() }
//! switch x { case 1: a()  case 2, 3: b()  default: d() }          (no fallthrough)
//! do { body } while cond          while true { <skip the test on the first pass> body }
//! for (i = 0; i < n; i++) { body } init ; while true { <step except on the first pass> if !cond break ; body }
//! for a, b in pairs { body }      for __it_0 in pairs { a = __it_0[0] ; b = __it_0[1] ; body }
//! ```
//! `continue` keeps working in `do`/`for(;;)` because the test/step sit at the top of the loop.
//! Strings and comments are never touched.

use crate::lambda::mask;

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

/// Index of the bracket matching the opener at `open` (masked text).
fn matching(m: &[u8], open: usize) -> Option<usize> {
    let (o, c) = match m[open] {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        b'{' => (b'{', b'}'),
        _ => return None,
    };
    let mut depth = 0i32;
    for (i, &ch) in m.iter().enumerate().skip(open) {
        if ch == o {
            depth += 1;
        } else if ch == c {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Positions of `target` at bracket depth 0 within `m[from..to]`.
fn top_level_positions(m: &[u8], from: usize, to: usize, target: u8) -> Vec<usize> {
    let mut depth = 0i32;
    let mut out = Vec::new();
    for i in from..to {
        match m[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            c if c == target && depth == 0 => out.push(i),
            _ => {}
        }
    }
    out
}

/// True when the statement starting at `pos` really is at the start of a statement.
fn at_statement_start(m: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i > 0 && (m[i - 1] == b' ' || m[i - 1] == b'\t') {
        i -= 1;
    }
    i == 0 || matches!(m[i - 1], b'\n' | b';' | b'{' | b'}')
}

fn keyword_at(m: &[u8], pos: usize, kw: &str) -> bool {
    let k = kw.as_bytes();
    m.len() > pos + k.len()
        && &m[pos..pos + k.len()] == k
        && !is_word(m[pos + k.len()])
        && (pos == 0 || !is_word(m[pos - 1]))
}

fn indent_of(src: &str, pos: usize) -> String {
    let line_start = src[..pos].rfind('\n').map_or(0, |p| p + 1);
    src[line_start..pos].chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// The `{` that opens the block whose header starts at `from`: the first top-level `{`.
fn find_block_open(m: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0i32;
    for i in from..m.len() {
        match m[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(i),
            b'\n' if depth == 0 => {
                // the brace may sit on the next line
                let mut j = i + 1;
                while j < m.len() && (m[j] == b' ' || m[j] == b'\t' || m[j] == b'\r' || m[j] == b'\n') {
                    j += 1;
                }
                return if j < m.len() && m[j] == b'{' { Some(j) } else { None };
            }
            _ => {}
        }
    }
    None
}

/// `(a + b)` -> `a + b` only when the parentheses enclose the whole text.
fn strip_outer_parens(s: &str) -> &str {
    let m = mask(s);
    if s.starts_with('(') && matching(&m, 0) == Some(s.len() - 1) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

struct Counter(usize);
impl Counter {
    fn next(&mut self) -> usize {
        self.0 += 1;
        self.0 - 1
    }
}

/// Lower every supported construct in `src`. Returns `src` unchanged when there are none.
pub fn lower(src: &str) -> String {
    let probe = mask(src);
    let text = String::from_utf8_lossy(&probe).to_string();
    let has_any = ["match", "switch", "do", "for"].iter().any(|k| text.contains(k));
    if !has_any {
        return src.to_string();
    }
    let mut counter = Counter(0);
    lower_with(src, &mut counter)
}

fn lower_with(src: &str, counter: &mut Counter) -> String {
    let m = mask(src);
    let mut out = String::with_capacity(src.len() + 64);
    let mut copied = 0usize; // everything before `copied` is already in `out`
    let mut i = 0usize;
    while i < src.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let c = m[i];
        if !(c == b'm' || c == b's' || c == b'd' || c == b'f') || !at_statement_start(&m, i) {
            i += 1;
            continue;
        }
        let lowered = if keyword_at(&m, i, "match") {
            lower_match(src, &m, i, counter)
        } else if keyword_at(&m, i, "switch") {
            lower_switch(src, &m, i, counter)
        } else if keyword_at(&m, i, "do") {
            lower_do_while(src, &m, i, counter)
        } else if keyword_at(&m, i, "for") {
            lower_for(src, &m, i, counter)
        } else {
            None
        };
        match lowered {
            Some((replacement, end)) => {
                out.push_str(&src[copied..i]);
                out.push_str(&replacement);
                copied = end;
                i = end;
            }
            None => i += 1,
        }
    }
    out.push_str(&src[copied..]);
    out
}

// ---------------------------------------------------------------------------------------------
// match

struct Arm {
    pattern: String,
    body: String,
}

/// Split a `match` body into arms: `pattern => statement` or `pattern => { block }`.
fn parse_arms(body: &str) -> Option<Vec<Arm>> {
    let m = mask(body);
    let mut arms = Vec::new();
    let mut i = 0;
    while i < body.len() {
        while i < body.len() && (m[i].is_ascii_whitespace() || m[i] == b',' || m[i] == b';') {
            i += 1;
        }
        if i >= body.len() {
            break;
        }
        // pattern up to the top-level `=>`
        let mut j = i;
        let mut depth = 0i32;
        let mut arrow = None;
        while j + 1 < body.len() {
            match m[j] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b'=' if depth == 0 && m[j + 1] == b'>' => {
                    arrow = Some(j);
                    break;
                }
                b'\n' if depth == 0 => return None, // a pattern must be followed by `=>` on its line
                _ => {}
            }
            j += 1;
        }
        let arrow = arrow?;
        let pattern = body[i..arrow].trim().to_string();
        let mut k = arrow + 2;
        while k < body.len() && (m[k] == b' ' || m[k] == b'\t') {
            k += 1;
        }
        let (stmt, next) = if k < body.len() && m[k] == b'{' {
            let close = matching(&m, k)?;
            (body[k + 1..close].to_string(), close + 1)
        } else {
            // a single statement: to the end of the line or the next top-level comma
            let mut e = k;
            let mut d = 0i32;
            while e < body.len() {
                match m[e] {
                    b'(' | b'[' | b'{' => d += 1,
                    b')' | b']' | b'}' => d -= 1,
                    b'\n' | b',' | b';' if d == 0 => break,
                    _ => {}
                }
                e += 1;
            }
            (body[k..e].trim().to_string(), e)
        };
        arms.push(Arm { pattern, body: stmt });
        i = next;
    }
    if arms.is_empty() {
        None
    } else {
        Some(arms)
    }
}

/// Replace whole-word occurrences of `name` in `text` by `with` (strings untouched).
fn replace_word(text: &str, name: &str, with: &str) -> String {
    let m = mask(text);
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if text.is_char_boundary(i)
            && m[i..].starts_with(name.as_bytes())
            && (i == 0 || !is_word(m[i - 1]))
            && (i + name.len() >= m.len() || !is_word(m[i + name.len()]))
        {
            out.push_str(with);
            i += name.len();
        } else {
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn lower_match(src: &str, m: &[u8], at: usize, counter: &mut Counter) -> Option<(String, usize)> {
    let header_start = at + "match".len();
    let open = find_block_open(m, header_start)?;
    let header = src[header_start..open].trim();
    let header = strip_outer_parens(header);
    if header.is_empty() || header.starts_with('=') || header.starts_with('.') {
        return None;
    }
    let close = matching(m, open)?;
    let arms = parse_arms(&src[open + 1..close])?;
    let indent = indent_of(src, at);
    let id = counter.next();
    let var = format!("__m_{}", id);

    let mut out = format!("{var} = {header}\n");
    let mut first = true;
    let mut default_body: Option<String> = None;
    for arm in &arms {
        let (pattern, guard) = match split_guard(&arm.pattern) {
            Some((p, g)) => (p, Some(g)),
            None => (arm.pattern.clone(), None),
        };
        let body = lower_with(&arm.body, counter);
        if pattern == "_" && guard.is_none() {
            default_body = Some(body);
            continue;
        }
        let (mut cond, bind) = if pattern == "_" {
            ("true".to_string(), None)
        } else if is_ident(&pattern) && !matches!(pattern.as_str(), "true" | "false" | "null" | "nil") {
            ("true".to_string(), Some(pattern.clone()))
        } else {
            let alts: Vec<String> = split_alternatives(&pattern)
                .into_iter()
                .map(|p| format!("({var} == {})", p.trim()))
                .collect();
            (alts.join(" || "), None)
        };
        if let Some(g) = guard {
            let g = match &bind {
                Some(name) => replace_word(&g, name, &var),
                None => g,
            };
            cond = if cond == "true" { format!("({g})") } else { format!("({cond}) && ({g})") };
        }
        let mut block = String::new();
        if let Some(name) = bind {
            block.push_str(&format!("{indent}  {name} = {var}\n"));
        }
        block.push_str(&format!("{indent}  {}\n", body.trim()));
        out.push_str(&format!(
            "{indent}{}if {cond} {{\n{block}{indent}}}",
            if first { "" } else { " else " }
        ));
        if first {
            first = false;
        }
    }
    if let Some(body) = default_body {
        if first {
            out.push_str(&format!("{indent}{}\n", body.trim()));
        } else {
            out.push_str(&format!(" else {{\n{indent}  {}\n{indent}}}", body.trim()));
        }
    }
    out.push('\n');
    // swallow the rest of the closing line's newline so line structure stays tidy
    Some((out.trim_end_matches('\n').to_string(), close + 1))
}

/// `pattern if guard` -> (pattern, guard)
fn split_guard(pattern: &str) -> Option<(String, String)> {
    let m = mask(pattern);
    let mut depth = 0i32;
    let b = pattern.as_bytes();
    for i in 0..b.len().saturating_sub(4) {
        match m[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b' ' if depth == 0 && m[i..].starts_with(b" if ") => {
                return Some((pattern[..i].trim().to_string(), pattern[i + 4..].trim().to_string()));
            }
            _ => {}
        }
    }
    None
}

/// `1 | 2 | 3` -> ["1", "2", "3"] (top-level single `|`)
fn split_alternatives(pattern: &str) -> Vec<String> {
    let m = mask(pattern);
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut last = 0;
    for i in 0..pattern.len() {
        match m[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'|' if depth == 0 && m.get(i + 1) != Some(&b'|') && (i == 0 || m[i - 1] != b'|') => {
                parts.push(pattern[last..i].to_string());
                last = i + 1;
            }
            _ => {}
        }
    }
    parts.push(pattern[last..].to_string());
    parts
}

// ---------------------------------------------------------------------------------------------
// switch

fn lower_switch(src: &str, m: &[u8], at: usize, counter: &mut Counter) -> Option<(String, usize)> {
    let header_start = at + "switch".len();
    let open = find_block_open(m, header_start)?;
    let header = src[header_start..open].trim();
    let header = strip_outer_parens(header);
    if header.is_empty() || header.starts_with('=') {
        return None;
    }
    let close = matching(m, open)?;
    let body = &src[open + 1..close];
    let bm = mask(body);

    // labels: `case a, b:` / `default:` at the start of a line
    struct Case {
        values: Vec<String>, // empty = default
        stmts: String,
    }
    let mut cases: Vec<Case> = Vec::new();
    let mut pos = 0;
    let mut cur: Option<Case> = None;
    for line in body.split_inclusive('\n') {
        let t = line.trim();
        let lm = &bm[pos..pos + line.len()];
        let lead = line.len() - line.trim_start().len();
        pos += line.len();
        let is_case = keyword_at(lm, lead, "case");
        let is_default = keyword_at(lm, lead, "default") && t.trim_start_matches("default").trim_start().starts_with(':');
        if is_case || is_default {
            if let Some(c) = cur.take() {
                cases.push(c);
            }
            let after = if is_case { &t["case".len()..] } else { &t["default".len()..] };
            let am = mask(after);
            let colons = top_level_positions(&am, 0, after.len(), b':');
            let colon = *colons.first()?;
            let values: Vec<String> = if is_case {
                let vm = &am[..colon];
                let commas = top_level_positions(vm, 0, vm.len(), b',');
                let mut vs = Vec::new();
                let mut last = 0;
                for c in commas {
                    vs.push(after[last..c].trim().to_string());
                    last = c + 1;
                }
                vs.push(after[last..colon].trim().to_string());
                vs
            } else {
                Vec::new()
            };
            let rest = after[colon + 1..].trim();
            cur = Some(Case { values, stmts: if rest.is_empty() { String::new() } else { format!("{rest}\n") } });
        } else if let Some(c) = cur.as_mut() {
            c.stmts.push_str(line);
        } else if !t.is_empty() {
            return None; // statements before the first label
        }
    }
    if let Some(c) = cur.take() {
        cases.push(c);
    }
    if cases.is_empty() {
        return None;
    }

    let indent = indent_of(src, at);
    let var = format!("__s_{}", counter.next());
    let mut out = format!("{var} = {header}\n");
    let mut first = true;
    let mut default_body: Option<String> = None;
    for case in &cases {
        // a trailing `break` ends the case (there is no fallthrough)
        let mut stmts = case.stmts.trim_end().to_string();
        if stmts.trim_end().ends_with("break") {
            let cut = stmts.trim_end().len() - "break".len();
            if cut == 0 || !is_word(stmts.as_bytes()[cut - 1]) {
                stmts.truncate(cut);
            }
        }
        let stmts = lower_with(stmts.trim(), counter);
        if case.values.is_empty() {
            default_body = Some(stmts);
            continue;
        }
        let cond: Vec<String> = case.values.iter().map(|v| format!("({var} == {v})")).collect();
        out.push_str(&format!(
            "{indent}{}if {} {{\n{indent}  {}\n{indent}}}",
            if first { "" } else { " else " },
            cond.join(" || "),
            stmts.replace('\n', &format!("\n{indent}  "))
        ));
        first = false;
    }
    if let Some(body) = default_body {
        if first {
            out.push_str(&format!("{indent}{}\n", body));
        } else {
            out.push_str(&format!(" else {{\n{indent}  {}\n{indent}}}", body.replace('\n', &format!("\n{indent}  "))));
        }
    }
    Some((out.trim_end_matches('\n').to_string(), close + 1))
}

// ---------------------------------------------------------------------------------------------
// do { } while cond

fn lower_do_while(src: &str, m: &[u8], at: usize, counter: &mut Counter) -> Option<(String, usize)> {
    let mut p = at + "do".len();
    while p < m.len() && (m[p] == b' ' || m[p] == b'\t' || m[p] == b'\n' || m[p] == b'\r') {
        p += 1;
    }
    if p >= m.len() || m[p] != b'{' {
        return None;
    }
    let close = matching(m, p)?;
    let mut q = close + 1;
    while q < m.len() && (m[q] == b' ' || m[q] == b'\t') {
        q += 1;
    }
    if !keyword_at(m, q, "while") {
        return None;
    }
    let cond_start = q + "while".len();
    let mut cond_end = cond_start;
    let mut depth = 0i32;
    while cond_end < m.len() {
        match m[cond_end] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'\n' | b';' if depth == 0 => break,
            _ => {}
        }
        cond_end += 1;
    }
    let cond = src[cond_start..cond_end].trim();
    if cond.is_empty() {
        return None;
    }
    let body = lower_with(&src[p + 1..close], counter);
    let indent = indent_of(src, at);
    let flag = format!("__dw_{}", counter.next());
    let out = format!(
        "{flag} = true\n{indent}while true {{\n{indent}  if {flag} {{\n{indent}    {flag} = false\n{indent}  }} else {{\n{indent}    if !({cond}) {{\n{indent}      break\n{indent}    }}\n{indent}  }}{}\n{indent}}}",
        body.trim_end_matches(|c| c == ' ' || c == '\t')
    );
    Some((out, cond_end))
}

// ---------------------------------------------------------------------------------------------
// for

fn lower_for(src: &str, m: &[u8], at: usize, counter: &mut Counter) -> Option<(String, usize)> {
    let header_start = at + "for".len();
    let open = find_block_open(m, header_start)?;
    let header = src[header_start..open].trim();
    let hm = mask(header);
    let close = matching(m, open)?;
    let indent = indent_of(src, at);

    // C-style: `for (init; cond; step)` or `for init; cond; step`
    let inner = if header.starts_with('(') && matching(&hm, 0) == Some(header.len() - 1) {
        &header[1..header.len() - 1]
    } else {
        header
    };
    let im = mask(inner);
    let semis = top_level_positions(&im, 0, inner.len(), b';');
    if semis.len() == 2 {
        let init = inner[..semis[0]].trim();
        let cond = inner[semis[0] + 1..semis[1]].trim();
        let step = inner[semis[1] + 1..].trim();
        let body = lower_with(&src[open + 1..close], counter);
        let flag = format!("__fs_{}", counter.next());
        let mut out = String::new();
        if !init.is_empty() {
            out.push_str(&format!("{init}\n{indent}"));
        }
        out.push_str(&format!("{flag} = true\n{indent}while true {{\n"));
        if step.is_empty() {
            out.push_str(&format!("{indent}  {flag} = false\n"));
        } else {
            out.push_str(&format!(
                "{indent}  if {flag} {{\n{indent}    {flag} = false\n{indent}  }} else {{\n{indent}    {}\n{indent}  }}\n",
                step
            ));
        }
        if !cond.is_empty() {
            out.push_str(&format!("{indent}  if !({cond}) {{\n{indent}    break\n{indent}  }}\n"));
        }
        out.push_str(body.trim_matches(|c| c == '\n' || c == '\r'));
        out.push_str(&format!("\n{indent}}}"));
        return Some((out, close + 1));
    }

    // destructuring: `for a, b in expr` / `for (a, b) of expr`
    let sep = [" in ", " of "].iter().filter_map(|s| hm_find(&hm, s)).min()?;
    let names_part = header[..sep].trim();
    let names_part = names_part.strip_prefix('(').and_then(|n| n.strip_suffix(')')).unwrap_or(names_part);
    let names: Vec<&str> = names_part.split(',').map(|n| n.trim()).collect();
    if names.len() < 2 || !names.iter().all(|n| is_ident(n)) {
        return None;
    }
    let iterable = header[sep + 4..].trim();
    let body = lower_with(&src[open + 1..close], counter);
    let tmp = format!("__it_{}", counter.next());
    let mut out = format!("for {tmp} in {iterable} {{\n");
    for (k, n) in names.iter().enumerate() {
        out.push_str(&format!("{indent}  {n} = {tmp}[{k}]\n"));
    }
    out.push_str(body.trim_matches(|c| c == '\n' || c == '\r'));
    out.push_str(&format!("\n{indent}}}"));
    Some((out, close + 1))
}

fn hm_find(masked: &[u8], pat: &str) -> Option<usize> {
    let p = pat.as_bytes();
    let mut depth = 0i32;
    for i in 0..masked.len() {
        match masked[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && masked[i..].starts_with(p) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_source_is_unchanged() {
        let s = "x = 1\nfor i in range(3) {\n  println(i)\n}\nwhile x < 3 {\n  x = x + 1\n}\n";
        assert_eq!(lower(s), s);
    }

    #[test]
    fn match_becomes_an_if_chain() {
        let out = lower("match x {\n  1 => println(\"one\")\n  2 | 3 => println(\"few\")\n  _ => println(\"other\")\n}\n");
        assert!(out.contains("__m_0 = x"), "{out}");
        assert!(out.contains("if (__m_0 == 1)"), "{out}");
        assert!(out.contains("else if (__m_0 == 2) || (__m_0 == 3)"), "{out}");
        assert!(out.contains("else {"), "{out}");
        assert!(!out.contains("=>"), "{out}");
    }

    #[test]
    fn match_binding_and_guard() {
        let out = lower("match v {\n  n if n > 9 => big(n)\n  _ => small()\n}\n");
        assert!(out.contains("if (__m_0 > 9)"), "{out}");
        assert!(out.contains("n = __m_0"), "{out}");
    }

    #[test]
    fn match_as_identifier_or_call_is_left_alone() {
        let s = "match = 5\nr = match(a, b)\n";
        assert_eq!(lower(s), s);
    }

    #[test]
    fn switch_has_no_fallthrough_and_drops_trailing_break() {
        let out = lower("switch x {\n  case 1:\n    a()\n    break\n  case 2, 3:\n    b()\n  default:\n    c()\n}\n");
        assert!(out.contains("if (__s_0 == 1)"), "{out}");
        assert!(out.contains("(__s_0 == 2) || (__s_0 == 3)"), "{out}");
        assert!(!out.contains("break"), "{out}");
        assert!(out.contains("else {"), "{out}");
    }

    #[test]
    fn do_while_tests_at_the_top_after_the_first_pass() {
        let out = lower("do {\n  i = i + 1\n} while i < 3\nprintln(i)\n");
        assert!(out.contains("while true {"), "{out}");
        assert!(out.contains("if !(i < 3)"), "{out}");
        assert!(out.contains("println(i)"), "{out}");
    }

    #[test]
    fn c_style_for_with_and_without_parentheses() {
        for header in ["for (i = 0; i < 4; i++) {", "for i = 0; i < 4; i++ {"] {
            let out = lower(&format!("{header}\n  t = t + i\n}}\n"));
            assert!(out.contains("i = 0\n"), "{out}");
            assert!(out.contains("while true {"), "{out}");
            assert!(out.contains("i++"), "{out}");
            assert!(out.contains("if !(i < 4)"), "{out}");
        }
    }

    #[test]
    fn for_destructuring() {
        let out = lower("for i, x in enumerate(xs) {\n  println(i)\n}\n");
        assert!(out.contains("for __it_0 in enumerate(xs) {"), "{out}");
        assert!(out.contains("i = __it_0[0]"), "{out}");
        assert!(out.contains("x = __it_0[1]"), "{out}");
    }

    #[test]
    fn nested_constructs_and_strings() {
        let out = lower("match a {\n  1 => {\n    do {\n      n = n + 1\n    } while n < 2\n  }\n  _ => println(\"match x { 1 => 2 }\")\n}\n");
        assert!(out.contains("while true"), "{out}");
        assert!(out.contains("\"match x { 1 => 2 }\""), "{out}");
    }
}
