//! Lifts anonymous functions out of expressions.
//!
//! ```text
//! map(xs, fn(x) { return x * 2 })      map(xs, __lambda_0)      (+ a `fn __lambda_0(x) {...}` line above)
//! f = (a, b) => a + b                   f = __lambda_1
//! ys = filter(xs, n => n > 0)           ys = filter(xs, __lambda_2)
//! ```
//!
//! Each lambda becomes an ordinary named function defined on the line before the statement that
//! uses it, in the same block, so the compiler's closure capture (nested `fn` inside a function)
//! applies to lambdas for free. Lambdas nested inside lambdas are lifted innermost first.
//! Strings and comments are never touched.

/// Replace string contents and comments with `_`/spaces so structural scanning cannot be fooled,
/// keeping byte length (and line breaks) identical to the source.
pub(crate) fn mask(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) => {
                if c == b'\\' && i + 1 < b.len() {
                    out[i] = b'_';
                    if b[i + 1] != b'\n' {
                        out[i + 1] = b'_';
                    }
                    i += 2;
                    continue;
                }
                if c == q {
                    quote = None;
                } else if c != b'\n' {
                    out[i] = b'_';
                }
                i += 1;
            }
            None => {
                if c == b'"' || c == b'`' {
                    quote = Some(c);
                    i += 1;
                } else if (c == b'/' && b.get(i + 1) == Some(&b'/'))
                    || (c == b'-' && b.get(i + 1) == Some(&b'-'))
                    || (c == b'#' && b[..i].iter().rev().take_while(|x| **x != b'\n').all(|x| x.is_ascii_whitespace()))
                {
                    while i < b.len() && b[i] != b'\n' {
                        out[i] = b' ';
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
    out
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Index of the bracket matching the opener at `open`, in masked text.
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

fn skip_spaces(m: &[u8], mut i: usize) -> usize {
    while i < m.len() && (m[i] == b' ' || m[i] == b'\t') {
        i += 1;
    }
    i
}

/// One lambda found in the source.
struct Found {
    /// Byte range of the whole lambda expression.
    start: usize,
    end: usize,
    params: String,
    /// Function body text (statements), already wrapped for `fn name(params) { ... }`.
    body: String,
}

/// The lambda whose start is furthest right (so nested ones are handled innermost-first).
fn find_last(src: &str, m: &[u8]) -> Option<Found> {
    let mut best: Option<Found> = None;
    // `fn (params) { body }` with no name
    let mut i = 0;
    while i + 2 <= m.len() {
        if m[i] == b'f' && m[i + 1] == b'n' && (i == 0 || !is_word(m[i - 1])) {
            let p = skip_spaces(m, i + 2);
            if p < m.len() && m[p] == b'(' {
                // `async fn(` is not a lambda form we lift
                if let Some(close) = matching(m, p) {
                    let b = skip_spaces(m, close + 1);
                    if b < m.len() && m[b] == b'{' {
                        if let Some(end_brace) = matching(m, b) {
                            best = Some(Found {
                                start: i,
                                end: end_brace + 1,
                                params: src[p + 1..close].to_string(),
                                body: src[b + 1..end_brace].to_string(),
                            });
                        }
                    }
                }
            }
        }
        i += 1;
    }
    // arrow lambdas `(a, b) => expr` / `x => expr`
    let mut j = 0;
    while j + 1 < m.len() {
        if m[j] == b'=' && m[j + 1] == b'>' && (j == 0 || !matches!(m[j - 1], b'=' | b'<' | b'>' | b'!')) {
            if let Some(f) = arrow_at(src, m, j) {
                if best.as_ref().map_or(true, |b| f.start > b.start) {
                    best = Some(f);
                }
            }
        }
        j += 1;
    }
    best
}

fn arrow_at(src: &str, m: &[u8], arrow: usize) -> Option<Found> {
    // parameters: the token before `=>`
    let mut p = arrow;
    while p > 0 && (m[p - 1] == b' ' || m[p - 1] == b'\t') {
        p -= 1;
    }
    if p == 0 {
        return None;
    }
    let (start, params) = if m[p - 1] == b')' {
        let mut depth = 0i32;
        let mut k = p - 1;
        loop {
            match m[k] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            if k == 0 {
                return None;
            }
            k -= 1;
        }
        // `name(args) => ...` is not a lambda (e.g. a call followed by `=>` in a match arm)
        if k > 0 && is_word(m[k - 1]) {
            return None;
        }
        (k, src[k + 1..p - 1].to_string())
    } else if is_word(m[p - 1]) {
        let mut k = p;
        while k > 0 && is_word(m[k - 1]) {
            k -= 1;
        }
        (k, src[k..p].to_string())
    } else {
        return None;
    };
    // body: up to the first top-level `,` or closing bracket, or the end of the line
    let mut e = skip_spaces(m, arrow + 2);
    let body_start = e;
    let mut depth = 0i32;
    while e < m.len() {
        match m[e] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            b',' if depth == 0 => break,
            b'\n' if depth == 0 => break,
            _ => {}
        }
        e += 1;
    }
    let mut body_end = e;
    while body_end > body_start && (m[body_end - 1] == b' ' || m[body_end - 1] == b'\t' || m[body_end - 1] == b'\r') {
        body_end -= 1;
    }
    if body_end == body_start {
        return None;
    }
    // a block body `=> { ... }` is used as is
    let body = src[body_start..body_end].trim();
    let body_text = if body.starts_with('{') && body.ends_with('}') && matching(m, body_start) == Some(body_end - 1) {
        src[body_start + 1..body_end - 1].to_string()
    } else {
        format!("return {}", body)
    };
    Some(Found { start, end: body_end, params, body: body_text })
}

/// Lift every lambda in `src`. Returns the source unchanged when there are none.
pub fn lift(src: &str) -> String {
    lift_mapped(src).0
}

/// Like [`lift`], also returning for every output line the index of the input line it came from.
/// A lifted function's lines point at the lines of the lambda they were cut from; the statement
/// that held the lambda keeps pointing at its own first line.
pub fn lift_mapped(src: &str) -> (String, Vec<usize>) {
    let mut map: Vec<usize> = (0..crate::sourcemap::line_count(src)).collect();
    if !src.contains("=>") && !contains_anonymous_fn(src) {
        return (src.to_string(), map);
    }
    let mut text = src.to_string();
    let mut counter = 0usize;
    // guard against pathological input
    for _ in 0..10_000 {
        let m = mask(&text);
        let Some(found) = find_last(&text, &m) else { break };
        let name = format!("__lambda_{}", counter);
        counter += 1;
        // the line that holds the start of the lambda
        let line_start = text[..found.start].rfind('\n').map_or(0, |p| p + 1);
        let indent: String = text[line_start..].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let body = found.body.trim_matches(|c| c == '\n' || c == '\r');
        let definition = format!("{indent}fn {name}({}) {{\n{}\n{indent}}}\n", found.params.trim(), body);

        // line map: header -> first line of the lambda, body lines -> the lines they were cut
        // from, closing brace -> last line of the lambda; the statement line keeps its origin
        let start_line = text[..found.start].bytes().filter(|&b| b == b'\n').count();
        let end_line = start_line + text[found.start..found.end].bytes().filter(|&b| b == b'\n').count();
        let region: Vec<&str> = text[line_start..found.end].split('\n').collect();
        let mut def_map = vec![map[start_line.min(map.len() - 1)]];
        let mut cursor = 0usize;
        for bl in body.split('\n') {
            let t = bl.trim();
            let t = t.strip_prefix("return ").unwrap_or(t);
            if t.len() >= 2 {
                if let Some(off) = region[cursor..].iter().position(|r| r.contains(t)) {
                    cursor += off;
                }
            }
            def_map.push(map[(start_line + cursor).min(map.len() - 1)]);
        }
        def_map.push(map[end_line.min(map.len() - 1)]);
        let mut next_map: Vec<usize> = Vec::with_capacity(map.len() + def_map.len());
        next_map.extend_from_slice(&map[..start_line.min(map.len())]);
        next_map.extend(def_map);
        next_map.push(map[start_line.min(map.len() - 1)]);
        if end_line + 1 <= map.len() {
            next_map.extend_from_slice(&map[end_line + 1..]);
        }
        map = next_map;

        let mut next = String::with_capacity(text.len() + definition.len());
        next.push_str(&text[..line_start]);
        next.push_str(&definition);
        next.push_str(&text[line_start..found.start]);
        next.push_str(&name);
        next.push_str(&text[found.end..]);
        text = next;
    }
    (text, map)
}

fn contains_anonymous_fn(src: &str) -> bool {
    let m = mask(src);
    let mut i = 0;
    while i + 2 < m.len() {
        if m[i] == b'f' && m[i + 1] == b'n' && (i == 0 || !is_word(m[i - 1])) {
            let p = skip_spaces(&m, i + 2);
            if p < m.len() && m[p] == b'(' {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_map_follows_lifted_lambdas() {
        let (out, map) = lift_mapped("f = fn(a) {\n  return a * 3\n}\nprintln(f(4))\n");
        assert_eq!(out, "fn __lambda_0(a) {\n  return a * 3\n}\nf = __lambda_0\nprintln(f(4))\n");
        assert_eq!(map, vec![0, 1, 2, 0, 3, 4]);
        let (out, map) = lift_mapped("x = 1\nf = (a, b) => a + b\nprintln(f(1, 2))\n");
        assert_eq!(map.len(), crate::sourcemap::line_count(&out), "{out}");
        assert_eq!(map, vec![0, 1, 1, 1, 1, 2, 3]);
        // untouched source maps to itself
        let (_, map) = lift_mapped("a = 1\nb = 2\n");
        assert_eq!(map, vec![0, 1, 2]);
    }

    #[test]
    fn source_without_lambdas_is_untouched() {
        let s = "fn add(a, b) {\n  return a + b\n}\nx = a >= b\ny = \"fn(x) { }\"\n";
        assert_eq!(lift(s), s);
    }

    #[test]
    fn fn_expression_is_lifted_above_its_statement() {
        let out = lift("f = fn(a) {\n  return a * 3\n}\nprintln(f(4))\n");
        assert_eq!(out, "fn __lambda_0(a) {\n  return a * 3\n}\nf = __lambda_0\nprintln(f(4))\n");
    }

    #[test]
    fn arrow_forms() {
        let out = lift("f = (a, b) => a + b\ng = x => x * 2\nprintln(map(xs, n => n + 1))\n");
        assert!(out.contains("fn __lambda_"), "{out}");
        assert!(out.contains("return a + b"), "{out}");
        assert!(out.contains("return x * 2"), "{out}");
        assert!(out.contains("println(map(xs, __lambda_"), "{out}");
        assert!(!out.contains("=>"), "{out}");
    }

    #[test]
    fn arrow_body_stops_at_the_argument_boundary() {
        let out = lift("r = reduce(xs, (a, b) => a + b, 0)\n");
        assert!(out.contains("r = reduce(xs, __lambda_0, 0)"), "{out}");
    }

    #[test]
    fn nested_lambdas_lift_inner_first() {
        let out = lift("f = fn(a) {\n  return map(a, x => x + 1)\n}\n");
        assert!(!out.contains("=>"), "{out}");
        assert_eq!(out.matches("fn __lambda_").count(), 2, "{out}");
    }

    #[test]
    fn strings_and_comments_are_ignored() {
        let s = "s = \"a => b and fn(x) {}\"\n// f = x => x\n";
        assert_eq!(lift(s), s);
    }

    #[test]
    fn comparison_operators_are_not_arrows() {
        let s = "if a >= b {\n  c = a <= b\n}\nd = a == b\n";
        assert_eq!(lift(s), s);
    }
}
