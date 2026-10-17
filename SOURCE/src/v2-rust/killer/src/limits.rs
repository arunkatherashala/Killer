//! Structural limits checked before compilation.
//!
//! The compiler works on text and re-scans an expression at every level of nesting, so its cost
//! grows with the *product* of nesting depth and expression length (and its native recursion with
//! the depth). Real programs never come close to the limits below, but a hostile or generated
//! input (`((((...))))`, a 100,000-term sum, 500 nested lambdas) would otherwise take minutes or
//! overflow the native stack. They are refused up front with a clear error instead.

use crate::error::VmError;

/// Deepest nesting of `(`, `[` and `{` (blocks, calls, literals and lambdas all count).
pub const MAX_NESTING: usize = 200;
/// Most binary operators in one operand sequence (between commas / brackets / statement ends).
pub const MAX_OPERATORS: usize = 1000;

fn too_long(line: usize) -> VmError {
    VmError::parse_error_simple(format!(
        "Line {line}: expression is too long (more than {MAX_OPERATORS} operators in one expression); split it into several statements"
    ))
}

/// Check `source` against the structural limits. Strings and comments are ignored.
///
/// Operators counted per expression: arithmetic, bitwise, comparison (`<`, `>`, `==`, `!=`),
/// `&&` / `||` / `and` / `or`, `??` and `?`. The compiler splits chains of them by rescanning the
/// rest of the text, so an unbounded `a and b and c ...` chain was quadratic (8000 terms: 25 s).
/// The count restarts at every statement: at `;`, `,`, and at a line end that does not continue
/// the expression (inside `{ }` blocks too, so a long function body is not one expression).
pub fn check_source(source: &str) -> Result<(), VmError> {
    let masked = crate::lambda::mask(source);
    let mut depth: usize = 0;
    // operator count of each open group; the last entry is the group being scanned
    let mut counts: Vec<usize> = vec![0];
    // true for a `{` group that is a block (not a dict literal): its lines are separate statements
    let mut braces: Vec<bool> = vec![true];
    let mut line = 1usize;
    let mut prev: u8 = b'\n'; // previous non-space byte
    let mut prefix_run = 0usize; // length of the current run of `!` / `~`
    // a line ended: restart the count unless the next token continues the expression
    let mut pending_reset = false;
    let (mut wbuf, mut wlen) = ([0u8; 3], 0usize);
    for &c in &masked {
        // `and` / `or` as whole words
        if c.is_ascii_alphanumeric() || c == b'_' {
            if wlen < 3 {
                wbuf[wlen] = c;
            }
            wlen += 1;
        } else {
            if (wlen == 3 && wbuf == *b"and") || (wlen == 2 && wbuf[..2] == *b"or") {
                if let Some(last) = counts.last_mut() {
                    *last += 1;
                    if *last > MAX_OPERATORS {
                        return Err(too_long(line));
                    }
                }
            }
            wlen = 0;
        }
        if c == b'\n' {
            line += 1;
            let continues = matches!(prev, b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' | b'=' | b'?' | b'(' | b'[' | b',');
            if depth == 0 {
                counts[0] = 0;
                pending_reset = false;
            } else if !continues && braces.last() == Some(&true) {
                pending_reset = true;
            }
            prev = b'\n';
            continue;
        }
        if matches!(c, b' ' | b'\t' | b'\r') {
            continue;
        }
        if pending_reset {
            pending_reset = false;
            if !matches!(c, b'+' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' | b'?' | b'.' | b',') {
                if let Some(last) = counts.last_mut() {
                    *last = 0;
                }
            }
        }
        match c {
            b'(' | b'[' | b'{' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {line}: nesting is too deep (more than {MAX_NESTING} levels of brackets, blocks or lambdas)"
                    )));
                }
                counts.push(0);
                // `{` after an operand (`)`, a word, `else`) opens a block; after `=`, `(`, `,`, an
                // operator or at the start of a statement it is a dict literal
                braces.push(c == b'{' && !matches!(prev, b'=' | b'(' | b'[' | b',' | b':' | b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' | b'?' | b'!' | b'~'));
            }
            b')' | b']' | b'}' => {
                if depth > 0 {
                    depth -= 1;
                    counts.pop();
                    // a finished block ends the statement that opened it (`} else if x == 1 {`
                    // chains must not add up across branches)
                    if braces.pop() == Some(true) && c == b'}' {
                        if let Some(last) = counts.last_mut() {
                            *last = 0;
                        }
                    }
                }
            }
            b'!' | b'~' => {
                prefix_run = if prev == c { prefix_run + 1 } else { 1 };
                if prefix_run > MAX_NESTING {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {line}: too many `{}` operators in a row (limit {MAX_NESTING})",
                        c as char
                    )));
                }
            }
            b',' | b';' => {
                if let Some(last) = counts.last_mut() {
                    *last = 0;
                }
            }
            b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' | b'?' | b'=' => {
                let operand_before = prev.is_ascii_alphanumeric() || matches!(prev, b')' | b']' | b'}' | b'_' | b'"' | b'\'' | b'`');
                // `==` and `!=` count at their second character; a lone `=` is an assignment
                let counts_here = if c == b'=' { matches!(prev, b'=' | b'!') } else { operand_before };
                if counts_here {
                    if let Some(last) = counts.last_mut() {
                        *last += 1;
                        if *last > MAX_OPERATORS {
                            return Err(too_long(line));
                        }
                    }
                }
            }
            _ => {}
        }
        prev = c;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_code_passes() {
        assert!(check_source("fn f(a, b) {\n  return (a + b) * [1, 2, -3][0]\n}\nx = f(1, 2) - 4\n").is_ok());
        let long_literal = format!("x = [{}]\n", (0..100_000).map(|i| format!("-{i}")).collect::<Vec<_>>().join(", "));
        assert!(check_source(&long_literal).is_ok());
    }

    #[test]
    fn long_function_bodies_and_logical_chains() {
        // 3000 statements with an operator each, in one block: not one expression
        let body: String = (0..3000).map(|i| format!("  a = a + {i}
")).collect();
        assert!(check_source(&format!("fn f(a) {{
{body}  return a
}}
")).is_ok());
        // but one endless logical chain is refused, whatever the spelling
        for join in [" and ", " or ", " && ", " || ", " == ", " ?? "] {
            let e = check_source(&format!("x = {}
", vec!["a"; 3000].join(join))).unwrap_err().to_string();
            assert!(e.contains("expression is too long"), "{join}: {e}");
        }
        // across lines, a continuation does not restart the count
        let chain: String = (0..3000).map(|_| "  a +
").collect();
        assert!(check_source(&format!("fn f(a) {{
  x = {chain}  1
}}
")).is_err());
    }

    #[test]
    fn deep_nesting_is_refused() {
        let src = format!("x = {}1{}\n", "(".repeat(300), ")".repeat(300));
        let e = check_source(&src).unwrap_err().to_string();
        assert!(e.contains("nesting is too deep"), "{e}");
        assert!(check_source(&format!("x = {}1{}\n", "(".repeat(150), ")".repeat(150))).is_ok());
    }

    #[test]
    fn endless_operator_chains_are_refused() {
        let src = format!("x = {}\n", vec!["1"; 5000].join(" + "));
        let e = check_source(&src).unwrap_err().to_string();
        assert!(e.contains("expression is too long"), "{e}");
        assert!(check_source(&format!("x = {}\n", vec!["1"; 500].join(" + "))).is_ok());
    }

    #[test]
    fn long_prefix_operator_chains_are_refused() {
        let e = check_source(&format!("x = {}true
", "!".repeat(5000))).unwrap_err().to_string();
        assert!(e.contains("in a row"), "{e}");
        assert!(check_source("x = !!!true
y = a != b
").is_ok());
    }

    #[test]
    fn strings_and_comments_do_not_count() {
        let s = format!("x = \"{}\"\n// {}\n", "(".repeat(1000), "+ ".repeat(5000));
        assert!(check_source(&s).is_ok());
    }
}
