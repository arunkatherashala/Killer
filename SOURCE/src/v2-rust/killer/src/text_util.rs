//! Small UTF-8 safe string helpers.

/// Largest index <= `max_bytes` that lies on a char boundary of `s`.
pub fn floor_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut i = max_bytes;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Truncate `s` to at most `max_bytes` bytes without splitting a UTF-8 char.
pub fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    &s[..floor_char_boundary(s, max_bytes)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundary() {
        let s = "a\u{e9}"; // 'e acute' occupies bytes 1..3
        assert_eq!(truncate_utf8(s, 2), "a");
        assert_eq!(truncate_utf8(s, 3), "a\u{e9}");
        assert_eq!(truncate_utf8(s, 100), "a\u{e9}");
        assert_eq!(truncate_utf8(s, 0), "");
    }
}
