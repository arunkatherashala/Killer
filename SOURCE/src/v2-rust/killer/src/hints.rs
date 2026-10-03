//! Friendlier explanations for compile errors that have a well-known cause.

/// Byte offset of a `//` that starts a comment (not inside a string or backticks).
fn comment_start(line: &str) -> Option<usize> {
    let b = line.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) => {
                if c == b'\\' && q != b'`' {
                    i += 1;
                } else if c == q {
                    quote = None;
                }
            }
            None => {
                if c == b'"' || c == b'\'' || c == b'`' {
                    quote = Some(c);
                } else if c == b'#' {
                    return None;
                } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

/// True when `rest` (the text after `//`) reads like the right operand of a division
/// (`2`, `(n + 1)`, `step)`) rather than a sentence.
fn looks_like_operand(rest: &str) -> bool {
    let r = rest.trim();
    let Some(first) = r.chars().next() else { return false };
    if !(first.is_alphanumeric() || first == '_' || first == '(') {
        return false;
    }
    let chars: Vec<char> = r.chars().collect();
    // two words in a row ("first arg") means prose
    !chars.windows(3).any(|w| w[0].is_alphanumeric() && w[1] == ' ' && w[2].is_alphanumeric())
}

/// If `line` looks like it used `//` as integer division (the comment swallowed an operand and
/// left brackets open or a dangling operator), return the explanation.
pub fn floor_division_hint(line: &str) -> Option<&'static str> {
    let at = comment_start(line)?;
    let before = &line[..at];
    let after = &line[at + 2..];
    if before.trim().is_empty() || !looks_like_operand(after) {
        return None;
    }
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in before.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            },
        }
    }
    let dangling = before.trim_end().ends_with(['+', '-', '*', '/', '%', '=', ',', '<', '>', '&', '|']);
    if depth > 0 || dangling {
        Some("// starts a comment in Killer; use floor(a / b) for integer division")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_floor_division_that_left_brackets_open() {
        assert!(floor_division_hint("x = (a // 2) + 1").is_some());
        assert!(floor_division_hint("fn half(n) { return n // 2 }").is_some());
        assert!(floor_division_hint("mid = (lo + hi) // 2").is_none()); // balanced: nothing broke
    }

    #[test]
    fn ordinary_comments_are_not_flagged() {
        assert!(floor_division_hint("fn f(a, // first arg").is_none());
        assert!(floor_division_hint("x = (1 + // sum of the parts").is_none());
        assert!(floor_division_hint("// just a comment").is_none());
        assert!(floor_division_hint("s = \"a // b\" + (1").is_none());
    }
}
