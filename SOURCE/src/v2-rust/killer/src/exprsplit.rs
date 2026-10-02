//! Top-level operator splitting for the line compiler's expression parser.
//!
//! Every function looks only at characters that sit at bracket depth 0 and outside string
//! literals, and returns slices of the original text. Precedence (lowest to highest):
//!
//! ```text
//! ?:  ·  X if C else Y   ternary (right associative)
//! ??                     null coalescing
//! ||
//! &&
//! == != < <= > >= in "not in"      comparison, chainable: a < b < c
//! |
//! ^
//! &
//! << >>
//! + -   then  * / % //   then  **   (handled by the compiler's own splitter)
//! ```

/// `mask[i]` is true when byte `i` is at bracket depth 0 and outside a string literal. The
/// brackets themselves and the quote characters are false.
pub fn top_level_mask(expr: &str) -> Vec<bool> {
    let b = expr.as_bytes();
    let mut mask = vec![false; b.len()];
    let (mut depth, mut in_str) = (0i32, false);
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {
                    if depth == 0 {
                        mask[i] = true;
                    }
                }
            }
        }
        i += 1;
    }
    mask
}

fn starts_at(b: &[u8], i: usize, pat: &str) -> bool {
    b.len() >= i + pat.len() && &b[i..i + pat.len()] == pat.as_bytes()
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// `cond ? a : b`  ->  (cond, a, b). The `:` matching the first `?` is found by counting nested `?`.
pub fn split_ternary(expr: &str) -> Option<(&str, &str, &str)> {
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    let mut q = None;
    let mut i = 0;
    while i < b.len() {
        if mask[i] && b[i] == b'?' {
            // `??` and `?.` are other operators
            if i + 1 < b.len() && (b[i + 1] == b'?' || b[i + 1] == b'.') {
                i += 2;
                continue;
            }
            q = Some(i);
            break;
        }
        i += 1;
    }
    let q = q?;
    let mut nest = 0;
    let mut j = q + 1;
    while j < b.len() {
        if mask[j] {
            if b[j] == b'?' {
                if j + 1 < b.len() && (b[j + 1] == b'?' || b[j + 1] == b'.') {
                    j += 2;
                    continue;
                }
                nest += 1;
            } else if b[j] == b':' {
                if nest == 0 {
                    let (cond, a, rest) = (expr[..q].trim(), expr[q + 1..j].trim(), expr[j + 1..].trim());
                    if cond.is_empty() || a.is_empty() || rest.is_empty() {
                        return None;
                    }
                    return Some((cond, a, rest));
                }
                nest -= 1;
            }
        }
        j += 1;
    }
    None
}

/// `a if cond else b`  ->  (cond, a, b). Right associative: the first ` if ` and the first
/// ` else ` after it are used, so `a if c else b if d else e` becomes `a if c else (b if d else e)`.
pub fn split_python_conditional(expr: &str) -> Option<(&str, &str, &str)> {
    let t = expr.trim_start();
    if t.starts_with("if ") {
        return None;
    }
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    let find = |from: usize, word: &str| -> Option<usize> {
        let mut i = from;
        while i + word.len() <= b.len() {
            if mask[i] && starts_at(b, i, word) {
                return Some(i);
            }
            i += 1;
        }
        None
    };
    let if_at = find(0, " if ")?;
    let else_at = find(if_at + 4, " else ")?;
    let value = expr[..if_at].trim();
    let cond = expr[if_at + 4..else_at].trim();
    let otherwise = expr[else_at + 6..].trim();
    if value.is_empty() || cond.is_empty() || otherwise.is_empty() {
        return None;
    }
    Some((cond, value, otherwise))
}

/// Rightmost occurrence of the lowest-precedence logical operator: `??`, then `||`, then `&&`.
pub fn split_logical_ordered(expr: &str) -> Option<(&str, &'static str, &str)> {
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    for op in ["??", "||", "&&"] {
        let mut found = None;
        let mut i = 0;
        while i + 2 <= b.len() {
            if mask[i] && starts_at(b, i, op) {
                // `???` / `|||` / `&&&` are not operators we know; take the rightmost pair
                found = Some(i);
                i += 2;
                continue;
            }
            i += 1;
        }
        if let Some(i) = found {
            let (l, r) = (expr[..i].trim(), expr[i + 2..].trim());
            if !l.is_empty() && !r.is_empty() {
                return Some((l, op, r));
            }
        }
    }
    None
}

/// A comparison chain `a < b <= c`, or a membership test `a in b` / `a not in b`.
/// Returns the operands and the operators between them.
pub fn split_comparison_chain(expr: &str) -> Option<(Vec<&str>, Vec<&'static str>)> {
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    let mut ops: Vec<(usize, usize, &'static str)> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !mask[i] {
            i += 1;
            continue;
        }
        let c = b[i];
        let next = if i + 1 < b.len() { b[i + 1] } else { 0 };
        let prev = if i > 0 { b[i - 1] } else { 0 };
        // two-character comparisons
        if (c == b'=' || c == b'!' || c == b'<' || c == b'>') && next == b'=' && !(c == b'=' && prev == b'=') {
            // `<=` `>=` `==` `!=`, but not part of `<<=`, `=>`, etc.
            if !(prev == b'<' && c == b'<') && !(prev == b'>' && c == b'>') {
                let op = match c {
                    b'=' => "==",
                    b'!' => "!=",
                    b'<' => "<=",
                    _ => ">=",
                };
                ops.push((i, 2, op));
                i += 2;
                continue;
            }
        }
        // shifts are not comparisons
        if (c == b'<' && next == b'<') || (c == b'>' && next == b'>') {
            i += 2;
            continue;
        }
        if c == b'<' || c == b'>' {
            // `->` and `=>` arrows
            if prev == b'-' || prev == b'=' {
                i += 1;
                continue;
            }
            ops.push((i, 1, if c == b'<' { "<" } else { ">" }));
            i += 1;
            continue;
        }
        // membership words need spaces around them
        if c == b' ' {
            if starts_at(b, i, " not in ") {
                ops.push((i, 8, "not in"));
                i += 8;
                continue;
            }
            if starts_at(b, i, " in ") {
                ops.push((i, 4, "in"));
                i += 4;
                continue;
            }
        }
        i += 1;
    }
    if ops.is_empty() {
        return None;
    }
    // `in` / `not in` do not chain; with one of them present only a single operator is allowed
    if ops.len() > 1 && ops.iter().any(|o| o.2 == "in" || o.2 == "not in") {
        return None;
    }
    let mut operands = Vec::with_capacity(ops.len() + 1);
    let mut start = 0;
    for (pos, len, _) in &ops {
        operands.push(expr[start..*pos].trim());
        start = pos + len;
    }
    operands.push(expr[start..].trim());
    if operands.iter().any(|o| o.is_empty()) {
        return None;
    }
    Some((operands, ops.into_iter().map(|o| o.2).collect()))
}

/// Bitwise operators, lowest precedence first: `|`, `^`, `&`, then shifts. Rightmost occurrence
/// of the lowest-precedence operator that is present (left associative).
pub fn split_bitwise(expr: &str) -> Option<(&str, &'static str, &str)> {
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    let levels: [&[&'static str]; 4] = [&["|"], &["^"], &["&"], &["<<", ">>"]];
    for level in levels {
        let mut found: Option<(usize, &'static str)> = None;
        let mut i = 0;
        while i < b.len() {
            if !mask[i] {
                i += 1;
                continue;
            }
            let mut hit = None;
            for op in level {
                if starts_at(b, i, op) {
                    hit = Some(*op);
                    break;
                }
            }
            match hit {
                Some(op) => {
                    let single = op.len() == 1;
                    let doubled = single && i + 1 < b.len() && b[i + 1] == b[i];
                    let after_same = single && i > 0 && b[i - 1] == b[i];
                    // `||` / `&&` are logical operators; `<<=` style forms are not handled
                    if single && (doubled || after_same) {
                        i += 2;
                        continue;
                    }
                    // an operator needs an operand on its left
                    if expr[..i].trim().is_empty() {
                        i += op.len();
                        continue;
                    }
                    found = Some((i, op));
                    i += op.len();
                }
                None => i += 1,
            }
        }
        if let Some((i, op)) = found {
            let (l, r) = (expr[..i].trim(), expr[i + op.len()..].trim());
            if !l.is_empty() && !r.is_empty() {
                return Some((l, op, r));
            }
        }
    }
    None
}

/// `(1, 2, 3)` is a tuple literal (compiled as an array). Returns the element texts.
pub fn tuple_elements(expr: &str) -> Option<Vec<&str>> {
    let e = expr.trim();
    if !(e.starts_with('(') && e.ends_with(')')) {
        return None;
    }
    let inner = &e[1..e.len() - 1];
    // the opening paren must close at the very end
    let mask_src = e;
    let b = mask_src.as_bytes();
    let (mut depth, mut in_str, mut i) = (0i32, false, 0);
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'(' || c == b'[' || c == b'{' {
            depth += 1;
        } else if c == b')' || c == b']' || c == b'}' {
            depth -= 1;
            if depth == 0 && i != b.len() - 1 {
                return None;
            }
        }
        i += 1;
    }
    let mask = top_level_mask(inner);
    let ib = inner.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    for k in 0..ib.len() {
        if mask[k] && ib[k] == b',' {
            parts.push(inner[start..k].trim());
            start = k + 1;
        }
    }
    if parts.is_empty() {
        return None; // no top-level comma: ordinary parentheses
    }
    let last = inner[start..].trim();
    if !last.is_empty() {
        parts.push(last); // `(1, 2,)` keeps a trailing comma legal
    }
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    Some(parts)
}

/// True when `expr` has a top-level operator that binds looser than a member access, so it cannot
/// be the receiver of `.name` / `.name(...)` (for example `1 + c`).
pub fn has_top_level_operator(expr: &str) -> bool {
    let b = expr.as_bytes();
    let mask = top_level_mask(expr);
    for i in 0..b.len() {
        if !mask[i] {
            continue;
        }
        match b[i] {
            b'+' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'<' | b'>' | b'=' | b'!' | b'?' => return true,
            b'-' => {
                // a leading unary minus is also looser than member access (`-c.n` is `-(c.n)`)
                return true;
            }
            b' ' => {
                if starts_at(b, i, " in ") || starts_at(b, i, " not in ") || starts_at(b, i, " if ") {
                    return true;
                }
            }
            _ => {}
        }
    }
    let t = expr.trim_start();
    t.starts_with("not ") || t.starts_with("await ") || t.starts_with("spawn ") || t.starts_with("new ") && false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ternary_forms() {
        assert_eq!(split_ternary("x > 2 ? \"big\" : \"small\""), Some(("x > 2", "\"big\"", "\"small\"")));
        assert_eq!(split_ternary("a ? b ? 1 : 2 : 3"), Some(("a", "b ? 1 : 2", "3")));
        assert_eq!(split_ternary("a ? 1 : b ? 2 : 3"), Some(("a", "1", "b ? 2 : 3")));
        assert_eq!(split_ternary("a ?? b"), None);
        assert_eq!(split_ternary("a?.b"), None);
        assert_eq!(split_ternary("f(a ? 1 : 2)"), None);
        assert_eq!(split_ternary("\"a ? b : c\""), None);
        assert_eq!(split_ternary("d[\"k\"] ? {\"a\": 1} : [1, 2]"), Some(("d[\"k\"]", "{\"a\": 1}", "[1, 2]")));
    }

    #[test]
    fn python_conditional() {
        assert_eq!(split_python_conditional("\"big\" if x > 2 else \"small\""), Some(("x > 2", "\"big\"", "\"small\"")));
        assert_eq!(split_python_conditional("a if c else b if d else e"), Some(("c", "a", "b if d else e")));
        assert_eq!(split_python_conditional("if x { y }"), None);
        assert_eq!(split_python_conditional("[x for x in xs if x > 1]"), None);
        assert_eq!(split_python_conditional("f(a if c else b)"), None);
        assert_eq!(split_python_conditional("\"a if b else c\""), None);
    }

    #[test]
    fn logical_precedence_and_associativity() {
        assert_eq!(split_logical_ordered("a && b || c"), Some(("a && b", "||", "c")));
        assert_eq!(split_logical_ordered("a || b && c"), Some(("a", "||", "b && c")));
        assert_eq!(split_logical_ordered("a || b || c"), Some(("a || b", "||", "c")));
        assert_eq!(split_logical_ordered("a && b && c"), Some(("a && b", "&&", "c")));
        assert_eq!(split_logical_ordered("a ?? b || c"), Some(("a", "??", "b || c")));
        assert_eq!(split_logical_ordered("f(a && b)"), None);
        assert_eq!(split_logical_ordered("\"a && b\""), None);
    }

    #[test]
    fn comparison_chains_and_membership() {
        assert_eq!(split_comparison_chain("a < b"), Some((vec!["a", "b"], vec!["<"])));
        assert_eq!(split_comparison_chain("1 < 2 < 3"), Some((vec!["1", "2", "3"], vec!["<", "<"])));
        assert_eq!(split_comparison_chain("a <= b == c"), Some((vec!["a", "b", "c"], vec!["<=", "=="])));
        assert_eq!(split_comparison_chain("x in xs"), Some((vec!["x", "xs"], vec!["in"])));
        assert_eq!(split_comparison_chain("x not in xs"), Some((vec!["x", "xs"], vec!["not in"])));
        assert_eq!(split_comparison_chain("a != b"), Some((vec!["a", "b"], vec!["!="])));
        assert_eq!(split_comparison_chain("1 << 4"), None);
        assert_eq!(split_comparison_chain("8 >> 1"), None);
        assert_eq!(split_comparison_chain("a + b"), None);
        assert_eq!(split_comparison_chain("[x for x in xs]"), None);
        assert_eq!(split_comparison_chain("f(a < b)"), None);
        assert_eq!(split_comparison_chain("\"a < b\""), None);
        assert_eq!(split_comparison_chain("a in b in c"), None);
    }

    #[test]
    fn bitwise_precedence() {
        assert_eq!(split_bitwise("a | b ^ c & d"), Some(("a", "|", "b ^ c & d")));
        assert_eq!(split_bitwise("a ^ b & c"), Some(("a", "^", "b & c")));
        assert_eq!(split_bitwise("a & b << 2"), Some(("a", "&", "b << 2")));
        assert_eq!(split_bitwise("1 << 2 >> 1"), Some(("1 << 2", ">>", "1")));
        assert_eq!(split_bitwise("a | b | c"), Some(("a | b", "|", "c")));
        assert_eq!(split_bitwise("a && b"), None);
        assert_eq!(split_bitwise("a || b"), None);
        assert_eq!(split_bitwise("f(a | b)"), None);
        assert_eq!(split_bitwise("6 & 3"), Some(("6", "&", "3")));
    }

    #[test]
    fn tuples() {
        assert_eq!(tuple_elements("(1, 2, 3)"), Some(vec!["1", "2", "3"]));
        assert_eq!(tuple_elements("(1, 2,)"), Some(vec!["1", "2"]));
        assert_eq!(tuple_elements("(f(1, 2), [3, 4])"), Some(vec!["f(1, 2)", "[3, 4]"]));
        assert_eq!(tuple_elements("(1 + 2)"), None);
        assert_eq!(tuple_elements("(a)(b, c)"), None);
        assert_eq!(tuple_elements("(1, 2) + (3, 4)"), None);
    }

    #[test]
    fn member_access_receivers() {
        assert!(has_top_level_operator("1 + c"));
        assert!(has_top_level_operator("true && c"));
        assert!(has_top_level_operator("len([1, 2]) + c"));
        assert!(has_top_level_operator("-c"));
        assert!(has_top_level_operator("a == b"));
        assert!(!has_top_level_operator("c"));
        assert!(!has_top_level_operator("a.b.c"));
        assert!(!has_top_level_operator("d[\"k\"]"));
        assert!(!has_top_level_operator("f(1 + 2)"));
        assert!(!has_top_level_operator("make(x).y"));
    }
}
