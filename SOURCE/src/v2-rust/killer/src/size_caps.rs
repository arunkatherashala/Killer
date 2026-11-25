//! Result-size checks for builtins whose output can be far larger than their arguments: a separator joined
//! between many items, a long replacement text, splitting a string into characters or lines, overlapping
//! chunks, text encodings, byte concatenation.
//!
//! Each one checks BEFORE (or, for lists, while) it builds the result, against the fixed limits that the
//! operators already use (strings: [`crate::vm::MAX_STRING_BYTES`], 256 MB; lists: [`MAX_LIST_LEN`], the
//! `range` limit; byte buffers: [`MAX_BYTES_LEN`], the `bytes_new` limit) and against an active `eval` memory
//! cap ([`crate::memcap`]). Without the pre-check the builtin would build the whole result first, and the
//! evaluation's periodic cap check (every 256 instructions) could only notice afterwards, or never when the
//! builtin is the last thing the evaluation does.

use crate::error::VmError;
use crate::value::Value;
use crate::vm::{check_string_size, MAX_STRING_BYTES};
use std::borrow::Cow;

/// Most elements a builtin builds into one list from a string or a count (same as `range`).
pub const MAX_LIST_LEN: usize = crate::builtin::MAX_RANGE_LEN;
/// Largest byte buffer a builtin builds (same as `bytes_new`).
pub const MAX_BYTES_LEN: usize = 1 << 30;

/// How many list elements are added between two checks of the memory cap.
const CHECK_EVERY: usize = 4096;

#[cold]
fn list_too_large(what: &str) -> VmError {
    VmError::runtime_error(format!("{}: the result list is too large (limit {} elements)", what, MAX_LIST_LEN))
}

#[cold]
fn string_too_large(what: &str) -> VmError {
    VmError::runtime_error(format!("{}: the result string is too large (limit 256 MB)", what))
}

/// A string of `bytes` bytes may be built by `what` (fixed limit and memory cap); the error names the builtin.
pub fn check_string(what: &str, bytes: usize) -> Result<(), VmError> {
    if bytes > MAX_STRING_BYTES {
        return Err(string_too_large(what));
    }
    check_string_size(bytes)
}

/// A byte buffer of `bytes` bytes may be built by `what`.
pub fn check_bytes(what: &str, bytes: usize) -> Result<(), VmError> {
    if bytes > MAX_BYTES_LEN {
        return Err(VmError::runtime_error(format!("{}: the result is too large (limit 1 GB)", what)));
    }
    crate::memcap::reserve(bytes)
}

/// Collect `items` into a list, failing as soon as it would exceed [`MAX_LIST_LEN`] elements or the memory cap
/// (the cap is asked before every growth of the list buffer and checked every few thousand elements for the
/// memory the elements themselves use, so the overshoot is small).
pub fn collect_list<I: Iterator<Item = Value>>(what: &str, items: I) -> Result<Vec<Value>, VmError> {
    let mut out: Vec<Value> = Vec::new();
    for v in items {
        if out.len() == out.capacity() {
            if out.len() >= MAX_LIST_LEN {
                return Err(list_too_large(what));
            }
            let grow = out.capacity().saturating_mul(2).clamp(16, MAX_LIST_LEN);
            crate::memcap::reserve(grow.saturating_mul(std::mem::size_of::<Value>()))?;
            out.reserve_exact(grow - out.len());
        } else if out.len() % CHECK_EVERY == 0 {
            crate::memcap::check()?;
        }
        out.push(v);
    }
    if out.len() > MAX_LIST_LEN {
        return Err(list_too_large(what));
    }
    Ok(out)
}

/// The list of the characters of `s` (`split(s, "")`, `list(s)`); a too-long string is refused before any work.
pub fn chars_list(what: &str, s: &str) -> Result<Vec<Value>, VmError> {
    if s.len() > MAX_LIST_LEN && s.chars().count() > MAX_LIST_LEN {
        return Err(list_too_large(what));
    }
    collect_list(what, s.chars().map(|c| Value::Str(c.to_string())))
}

/// `collect_list` for strings (`split`, `splitlines`, ...): each piece becomes a string value.
pub fn collect_strings<'a, I: Iterator<Item = &'a str>>(what: &str, pieces: I) -> Result<Vec<Value>, VmError> {
    collect_list(what, pieces.map(|p| Value::Str(p.to_string())))
}

/// `s` with every `old` replaced by `new`, after checking the size of the result.
pub fn replace(what: &str, s: &str, old: &str, new: &str) -> Result<String, VmError> {
    if new.len() > old.len() {
        // an empty pattern matches before every character and at the end
        let hits = if old.is_empty() { s.chars().count() + 1 } else { s.matches(old).count() };
        let total = s.len().saturating_add(hits.saturating_mul(new.len() - old.len()));
        check_string(what, total)?;
    }
    Ok(s.replace(old, new))
}

/// The printed form of every item joined with `sep`, after checking the size of the result.
pub fn join(what: &str, items: &[Value], sep: &str) -> Result<String, VmError> {
    let mut parts: Vec<Cow<'_, str>> = Vec::with_capacity(items.len());
    let mut total = 0usize;
    for (i, v) in items.iter().enumerate() {
        let p = match v {
            Value::Str(s) => Cow::Borrowed(s.as_str()),
            other => Cow::Owned(other.to_string()),
        };
        total = total.saturating_add(p.len()).saturating_add(if i > 0 { sep.len() } else { 0 });
        if total > MAX_STRING_BYTES {
            return Err(string_too_large(what));
        }
        parts.push(p);
    }
    check_string(what, total)?;
    let mut out = String::with_capacity(total);
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(p);
    }
    Ok(out)
}

/// A string that is built piece by piece (regular-expression replacement): `push` fails once the result would go
/// over the string limit, and asks the memory cap before every growth of its buffer.
pub struct StringBuilder<'w> {
    what: &'w str,
    out: String,
}

impl<'w> StringBuilder<'w> {
    pub fn new(what: &'w str) -> Self {
        StringBuilder { what, out: String::new() }
    }

    pub fn push_str(&mut self, s: &str) -> Result<(), VmError> {
        if self.out.len().saturating_add(s.len()) > MAX_STRING_BYTES {
            return Err(string_too_large(self.what));
        }
        let need = self.out.len() + s.len();
        if need > self.out.capacity() {
            // grow geometrically, and only after the memory cap agreed to the new buffer
            let grow = need.max(self.out.capacity().saturating_mul(2)).min(MAX_STRING_BYTES);
            crate::memcap::reserve(grow)?;
            self.out.reserve_exact(grow - self.out.len());
        }
        self.out.push_str(s);
        Ok(())
    }

    pub fn push(&mut self, c: char) -> Result<(), VmError> {
        let mut b = [0u8; 4];
        self.push_str(c.encode_utf8(&mut b))
    }

    pub fn finish(self) -> String {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_and_join_match_the_std_results() {
        assert_eq!(replace("t", "abcb", "b", "xyz").unwrap(), "axyzcxyz");
        assert_eq!(replace("t", "ab", "", "-").unwrap(), "-a-b-");
        assert_eq!(replace("t", "aaaa", "aa", "b").unwrap(), "bb");
        let items = vec![Value::Number(1.0), Value::Str("x".into()), Value::Bool(true)];
        assert_eq!(join("t", &items, ", ").unwrap(), "1, x, true");
        assert_eq!(join("t", &[], ", ").unwrap(), "");
    }

    #[test]
    fn oversized_results_are_refused_before_building() {
        let big_sep = "x".repeat(1 << 20);
        let items: Vec<Value> = (0..300).map(|i| Value::Number(i as f64)).collect();
        assert!(join("join", &items, &big_sep).unwrap_err().to_string().contains("too large"));
        assert!(replace("replace", &"a".repeat(300), "a", &big_sep).unwrap_err().to_string().contains("too large"));
        assert!(check_bytes("bytes_concat", (1 << 30) + 1).is_err());
        assert!(check_string("hex_encode", MAX_STRING_BYTES + 1).is_err());
        let mut b = StringBuilder::new("regex_replace");
        b.push_str("ab").unwrap();
        b.push('c').unwrap();
        assert_eq!(b.finish(), "abc");
        assert_eq!(collect_strings("split", "a,b".split(',')).unwrap().len(), 2);
    }
}
