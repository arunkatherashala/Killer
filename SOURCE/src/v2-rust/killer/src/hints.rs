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

/// The message for a `//` that was meant as integer division.
pub const FLOOR_DIVISION_MESSAGE: &str = "// starts a comment in Killer; use floor(a / b) for integer division";

/// True when the text after a `//` reads like the right operand of a division: starts like an
/// operand and is not followed by a plain word (that would be prose: `// 2 steps left`).
fn operand_like(rest: &str) -> bool {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let Some(first) = words.first() else { return false };
    let c = first.chars().next().unwrap();
    if !(c.is_alphanumeric() || c == '_' || c == '(') {
        return false;
    }
    !words.get(1).map_or(false, |w| w.chars().next().map_or(false, |c| c.is_alphabetic()))
}

/// Keywords after which a `//` can only be a comment (they do not end an expression).
const NOT_EXPRESSION_END: &[&str] = &[
    "return", "else", "break", "continue", "try", "finally", "do", "then", "yield", "throw", "in", "of", "and", "or",
    "not", "if", "while", "for", "elif", "catch", "match", "switch", "case", "default", "fn", "kfn", "class", "let",
];

/// Scan a whole program for a `//` that is really floor division (`x // 2`), which would silently
/// become a comment. Returns the 1-based line of the first one.
///
/// Rule: a `//` is a comment when it starts a line, follows something that cannot end an
/// expression (an operator, `(`, `,`, a keyword), or is followed by prose (`// 2 steps left`,
/// `// TODO`). It is the mistake when an expression-ending token (name, number, `)`, `]`, string)
/// comes before it and an operand follows it: a number or `(` after the `//`, a lowercase name
/// glued to it (`a //b`), or a name that is already used earlier in the program (`a // b`).
/// `@lang { ... }` blocks are not Killer and are skipped.
pub fn silent_floor_division(source: &str) -> Option<usize> {
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut poly_depth = 0i32;
    let mut in_backtick = false;
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if poly_depth > 0 {
            poly_depth += trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            continue;
        }
        if trimmed.starts_with('@') && trimmed.contains('{') {
            let open = trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            if open > 0 {
                poly_depth = open;
            }
            continue;
        }
        let b = line.as_bytes();
        let mut quote: Option<u8> = if in_backtick { Some(b'`') } else { None };
        let mut i = 0;
        let mut found: Option<usize> = None;
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
                        break;
                    } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
                        found = Some(i);
                        break;
                    }
                }
            }
            i += 1;
        }
        in_backtick = quote == Some(b'`');
        let code_end = found.unwrap_or(line.len());
        let before = &line[..code_end];
        let flagged = found.map_or(false, |at| {
            let after = &line[at + 2..];
            let before_trim = before.trim_end();
            let last = before_trim.chars().last();
            let ends_expression =
                last.map_or(false, |c| c.is_alphanumeric() || matches!(c, '_' | ')' | ']' | '"' | '\''));
            let last_word: String = {
                let mut w: Vec<char> = before_trim.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                w.reverse();
                w.into_iter().collect()
            };
            if !ends_expression || NOT_EXPRESSION_END.contains(&last_word.as_str()) {
                return false;
            }
            let spaced = after.starts_with(char::is_whitespace);
            let rest = after.trim();
            if !operand_like(rest) {
                return false;
            }
            let first = rest.chars().next().unwrap();
            if first.is_ascii_digit() || first == '(' {
                return true;
            }
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if spaced {
                known.contains(&name)
            } else {
                first.is_lowercase() || first == '_'
            }
        });
        if flagged {
            return Some(index + 1);
        }
        // remember the names used so far (outside comments and strings)
        let mut word = String::new();
        let mut q: Option<char> = None;
        for c in before.chars().chain(std::iter::once(' ')) {
            match q {
                Some(qq) => {
                    if c == qq {
                        q = None;
                    }
                }
                None => {
                    if c == '"' || c == '\'' {
                        q = Some(c);
                    } else if c.is_alphanumeric() || c == '_' {
                        word.push(c);
                    } else if !word.is_empty() {
                        known.insert(std::mem::take(&mut word));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_floor_division_is_found() {
        assert_eq!(silent_floor_division("x = 10\ny = x // 2\n"), Some(2));
        assert_eq!(silent_floor_division("mid = (lo + hi) // 2"), Some(1));
        assert_eq!(silent_floor_division("a = 7\nb = 2\nc = a // b\n"), Some(3));
        assert_eq!(silent_floor_division("n = 9\nh = n //2\n"), Some(2));
        assert_eq!(silent_floor_division("h = xs[1] // (k + 1)"), Some(1));
    }

    #[test]
    fn comments_are_not_flagged_by_the_floor_division_scan() {
        assert_eq!(silent_floor_division("// whole line\nx = 1 // set x\n"), None);
        assert_eq!(silent_floor_division("x = 1 // 2 items\n"), None);
        assert_eq!(silent_floor_division("x = 1 //TODO later\n"), None);
        assert_eq!(silent_floor_division("x = 1 // increment\n"), None);
        assert_eq!(silent_floor_division("return // nothing\n"), None);
        assert_eq!(silent_floor_division("s = \"a // b\"\n"), None);
        assert_eq!(silent_floor_division("f(a, // first\n  b)\n"), None);
        assert_eq!(silent_floor_division("@python {\n  x = 4 // 2\n}\n"), None);
        assert_eq!(silent_floor_division("# 4 // 2\n"), None);
    }

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
