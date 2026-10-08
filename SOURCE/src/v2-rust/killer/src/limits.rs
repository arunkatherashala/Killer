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

/// Check `source` against the structural limits. Strings and comments are ignored.
pub fn check_source(source: &str) -> Result<(), VmError> {
    let masked = crate::lambda::mask(source);
    let mut depth: usize = 0;
    // operator count of each open group; the last entry is the group being scanned
    let mut counts: Vec<usize> = vec![0];
    let mut line = 1usize;
    let mut prev: u8 = b'\n'; // previous non-space byte
    let mut prefix_run = 0usize; // length of the current run of `!` / `~`
    for &c in &masked {
        match c {
            b'\n' => {
                line += 1;
                if depth == 0 {
                    counts[0] = 0;
                }
                prev = b'\n';
                continue;
            }
            b' ' | b'\t' | b'\r' => continue,
            b'(' | b'[' | b'{' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {line}: nesting is too deep (more than {MAX_NESTING} levels of brackets, blocks or lambdas)"
                    )));
                }
                counts.push(0);
            }
            b')' | b']' | b'}' => {
                if depth > 0 {
                    depth -= 1;
                    counts.pop();
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
            b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' => {
                let operand_before = prev.is_ascii_alphanumeric() || matches!(prev, b')' | b']' | b'}' | b'_' | b'"' | b'\'' | b'`');
                if operand_before {
                    if let Some(last) = counts.last_mut() {
                        *last += 1;
                        if *last > MAX_OPERATORS {
                            return Err(VmError::parse_error_simple(format!(
                                "Line {line}: expression is too long (more than {MAX_OPERATORS} operators in one expression); split it into several statements"
                            )));
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
