//! One tokenizer-aware mask for the string-based passes (line splitters, lambda lifting, control-flow lowering,
//! formatter, linter, language server, closure scan, hints).
//!
//! Several passes need to look at code without being fooled by text that only *looks* like code: a `]` inside a key,
//! a `//` or `--` inside a string, a quote inside a comment. Each used to carry its own little scanner, and they
//! disagreed (single quotes, escapes, backticks, where a `#` comment may start). This module is the single
//! definition of the lexical rules the passes agree on:
//!
//! * strings: `"..."` and `'...'` with backslash escapes, and backtick raw strings (no escapes); a triple-quoted
//!   string is just an empty string followed by a string followed by an empty one, so it needs no special case
//!   (and a multi-line string is handled by scanning the whole source at once, see [`mask_source`]);
//! * a `'` only opens a string when it is not an apostrophe inside a word (`don't`) and a closing `'` follows on the
//!   same line, so a stray apostrophe (a lifetime in a foreign-code block, say) cannot swallow later lines;
//! * comments: `//` and `--` anywhere outside a string, and `#` at the start of a line or after whitespace;
//!   a comment runs to the end of the line.
//!
//! The scan works on bytes (every delimiter is ASCII, and UTF-8 continuation bytes never equal an ASCII byte), so
//! offsets are valid byte offsets into the original text.

/// What a [`Span`] covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A string literal including its quotes (an unterminated one runs to the end of the text, or of the line for `'`).
    Str,
    /// A comment from its first character up to (not including) the line break.
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

fn prev_is_word(src: &str, i: usize) -> bool {
    src[..i].chars().next_back().map_or(false, |c| c.is_alphanumeric())
}

/// The strings and comments in `src`, in order. `src` may be one line or a whole file: a `"` or backtick string can
/// run over line breaks (so a triple-quoted block is covered), a `'` string and a comment end at the line break.
pub fn spans(src: &str) -> Vec<Span> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let comment = (c == b'#' && (i == 0 || b[i - 1].is_ascii_whitespace()))
            || (c == b'/' && b.get(i + 1) == Some(&b'/'))
            || (c == b'-' && b.get(i + 1) == Some(&b'-'));
        if comment {
            let end = b[i..].iter().position(|&x| x == b'\n').map_or(b.len(), |p| i + p);
            out.push(Span { start: i, end, kind: Kind::Comment });
            i = end;
            continue;
        }
        if (c == b'"' || c == b'\'') && b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c) {
            // triple-quoted: runs to the next triple of the same quote (over line breaks), or to the end
            let mut j = i + 3;
            let mut end = b.len();
            while j < b.len() {
                if b[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if b[j] == c && b.get(j + 1) == Some(&c) && b.get(j + 2) == Some(&c) {
                    end = j + 3;
                    break;
                }
                j += 1;
            }
            out.push(Span { start: i, end, kind: Kind::Str });
            i = end;
            continue;
        }
        if c == b'"' || c == b'`' || (c == b'\'' && !prev_is_word(src, i)) {
            let q = c;
            let mut j = i + 1;
            let mut end = None;
            while j < b.len() {
                let d = b[j];
                if d == b'\\' && q != b'`' {
                    j += 2;
                    continue;
                }
                if q == b'\'' && d == b'\n' {
                    break;
                }
                if d == q {
                    end = Some(j + 1);
                    break;
                }
                j += 1;
            }
            match end {
                Some(e) => {
                    out.push(Span { start: i, end: e, kind: Kind::Str });
                    i = e;
                }
                // an unterminated `'` is an apostrophe, not a string; the others run to the end of the text
                None if q == b'\'' => i += 1,
                None => {
                    out.push(Span { start: i, end: b.len(), kind: Kind::Str });
                    i = b.len();
                }
            }
            continue;
        }
        i += 1;
    }
    out
}

/// How [`mask`] rewrites the text.
#[derive(Debug, Clone, Copy)]
pub struct Opts {
    /// Replaces the characters inside strings.
    pub fill: char,
    /// Keep the opening and closing quote characters (otherwise they are blanked like the contents).
    pub keep_quotes: bool,
    /// Blank comments with spaces (otherwise comment text is left as it is).
    pub blank_comments: bool,
    /// One fill character per *byte* (the result has the byte length of the input, so byte offsets carry over),
    /// instead of one per character (the result has the character count of the input, so columns carry over).
    pub per_byte: bool,
}

/// Mask strings (and optionally comments) in `src`. Line breaks are always kept.
pub fn mask(src: &str, o: Opts) -> String {
    let sp = spans(src);
    let mut out = String::with_capacity(src.len());
    let mut k = 0;
    for (i, ch) in src.char_indices() {
        while k < sp.len() && sp[k].end <= i {
            k += 1;
        }
        let rep = if ch == '\n' {
            None
        } else {
            match sp.get(k).filter(|s| s.start <= i) {
                Some(s) if s.kind == Kind::Str => {
                    let sb = src.as_bytes();
                    let opener = sb[s.start];
                    let ql = if opener != b'`' && sb.get(s.start + 1) == Some(&opener) && sb.get(s.start + 2) == Some(&opener) { 3 } else { 1 };
                    let closed = s.end - s.start >= 2 * ql && sb[s.end - ql..s.end].iter().all(|&x| x == opener);
                    let is_quote = i < s.start + ql || (closed && i >= s.end - ql);
                    if o.keep_quotes && is_quote {
                        None
                    } else {
                        Some(o.fill)
                    }
                }
                Some(_) if o.blank_comments => Some(' '),
                _ => None,
            }
        };
        match rep {
            None => out.push(ch),
            Some(f) => {
                for _ in 0..(if o.per_byte { ch.len_utf8() } else { 1 }) {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// Strings and comments blanked with spaces, quotes included, one space per character (columns are kept): for
/// counting brackets and finding keywords on a line.
pub fn mask_code(line: &str) -> String {
    mask(line, Opts { fill: ' ', keep_quotes: false, blank_comments: true, per_byte: false })
}

/// Whole-source mask for structural scanning: string contents become `_`, comments become spaces, quotes and line
/// breaks stay, and the result has exactly the byte length of the source.
pub fn mask_source(src: &str) -> Vec<u8> {
    mask(src, Opts { fill: '_', keep_quotes: true, blank_comments: true, per_byte: true }).into_bytes()
}

/// String contents replaced by `fill` (quotes kept), comments left alone, one fill character per byte, so offsets
/// found in the result are valid in `line`.
pub fn mask_strings(line: &str, fill: char) -> String {
    mask(line, Opts { fill, keep_quotes: true, blank_comments: false, per_byte: true })
}

/// Byte offset of the first comment on `line` (a `//`, `--` or `#` comment outside strings), if any.
pub fn comment_start(line: &str) -> Option<usize> {
    spans(line).into_iter().find(|s| s.kind == Kind::Comment).map(|s| s.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(n: usize) -> String {
        " ".repeat(n)
    }

    fn code(s: &str) -> String {
        mask_code(s)
    }

    fn src_mask(s: &str) -> String {
        String::from_utf8(mask_source(s)).unwrap()
    }

    #[test]
    fn keeps_plain_code_and_columns() {
        assert_eq!(code("x = a[1] + f(2)"), "x = a[1] + f(2)");
        assert_eq!(code("é = 1"), "é = 1");
        assert_eq!(code(""), "");
    }

    #[test]
    fn brackets_in_strings_are_hidden() {
        assert_eq!(code("d[\"a]b\"] += 1"), format!("d[{}] += 1", sp(5)));
        assert_eq!(code("d['a[b'] += 1"), format!("d[{}] += 1", sp(5)));
        assert_eq!(code("f(\"(\", ')')"), format!("f({}, {})", sp(3), sp(3)));
        assert_eq!(code("x = `a]b`"), format!("x = {}", sp(5)));
    }

    #[test]
    fn comment_markers_inside_strings_are_not_comments() {
        assert_eq!(comment_start("s = \"a//b\""), None);
        assert_eq!(comment_start("s = 'a--b'"), None);
        assert_eq!(comment_start("s = \"#x\""), None);
        assert_eq!(comment_start("s = 1 // c"), Some(6));
        assert_eq!(comment_start("s = 1 -- c"), Some(6));
        assert_eq!(comment_start("s = 1 # c"), Some(6));
        assert_eq!(comment_start("# whole line"), Some(0));
        assert_eq!(comment_start("    # indented"), Some(4));
        // a `#` glued to a word is not a comment
        assert_eq!(comment_start("a#b"), None);
    }

    #[test]
    fn quotes_in_comments_do_not_open_strings() {
        assert_eq!(code("x = 1 // it's \"quoted"), format!("x = 1 {}", sp(15)));
        assert_eq!(code("y = [1] -- don't"), format!("y = [1] {}", sp(8)));
        assert_eq!(code("z = 2 # a \"b"), format!("z = 2 {}", sp(6)));
        // code after a string that contains comment markers is still visible
        assert_eq!(code("s = \"//\" + t // real"), format!("s = {} + t {}", sp(4), sp(7)));
    }

    #[test]
    fn escaped_quotes_stay_inside_the_string() {
        // s = "a\"b" + t[1]
        assert_eq!(code("s = \"a\\\"b\" + t[1]"), format!("s = {} + t[1]", sp(6)));
        // s = 'it\'s' + u
        assert_eq!(code("s = 'it\\'s' + u"), format!("s = {} + u", sp(7)));
        // an escaped backslash right before the closing quote does not escape it: s = "a\\" + v
        assert_eq!(code("s = \"a\\\\\" + v"), format!("s = {} + v", sp(5)));
    }

    #[test]
    fn backtick_strings_are_raw() {
        // no escapes: the backslash does not hide the closing backtick: s = `a\` + w
        assert_eq!(code("s = `a\\` + w"), format!("s = {} + w", sp(4)));
    }

    #[test]
    fn apostrophes_are_not_strings() {
        assert_eq!(code("don't = 1"), "don't = 1");
        // a lone quote that is never closed on its line is an apostrophe, not a string to the end of the text
        assert_eq!(code("fn f<'a>(x) { y }"), "fn f<'a>(x) { y }");
        assert_eq!(src_mask("x = 'a\ny = 2 // c"), format!("x = 'a\ny = 2 {}", sp(4)));
    }

    #[test]
    fn single_quote_strings_end_at_the_line_break() {
        assert_eq!(src_mask("a = 'x\ny'\nb = 'z'"), "a = 'x\ny'\nb = '_'");
    }

    #[test]
    fn triple_quoted_text_spans_lines() {
        let src = "n = 1\ndoc = \"\"\"line // one\nn += 5\n'x' -- y\n\"\"\"\nm = 2 // c";
        let want = format!("n = 1\ndoc = \"\"\"{}\n{}\n{}\n\"\"\"\nm = 2 {}", "_".repeat(11), "_".repeat(6), "_".repeat(8), sp(4));
        assert_eq!(src_mask(src), want);
        assert_eq!(src_mask(src).len(), src.len());
        // single-quote triples too, with the other quote and comment markers inside
        let t = "doc = '''a \"b\" // c\nd'''\nz = 1";
        assert_eq!(src_mask(t), format!("doc = '''{}\n_'''\nz = 1", "_".repeat(10)));
        // an unterminated triple runs to the end
        assert_eq!(src_mask("a = \"\"\"x\ny"), "a = \"\"\"_\n_");
    }

    #[test]
    fn unicode_is_masked_per_byte_or_per_char() {
        let line = "s = \"é→\" + t";
        // per byte: é is 2 bytes and → is 3, so 5 fill bytes and the byte length is unchanged
        assert_eq!(mask_strings(line, '\u{1}'), "s = \"\u{1}\u{1}\u{1}\u{1}\u{1}\" + t");
        assert_eq!(mask_strings(line, '\u{1}').len(), line.len());
        // per char: columns are unchanged
        assert_eq!(code(line), format!("s = {} + t", sp(4)));
        assert_eq!(code(line).chars().count(), line.chars().count());
        assert_eq!(src_mask(line).len(), line.len());
        // é (2 bytes) + " = " puts the string at byte 5; it is 4 bytes long, a space, then the comment at byte 10
        assert_eq!(comment_start("é = \"//\" // ü"), Some(10));
    }

    #[test]
    fn mask_strings_keeps_comments_and_quotes() {
        assert_eq!(mask_strings("x = \"a\" // \"b\"", '_'), "x = \"_\" // \"b\"");
        assert_eq!(mask_strings("x = 'a#b' # c", '_'), "x = '___' # c");
    }

    #[test]
    fn spans_report_kinds_and_offsets() {
        let s = spans("a = \"x\" // c");
        assert_eq!(s, vec![Span { start: 4, end: 7, kind: Kind::Str }, Span { start: 8, end: 12, kind: Kind::Comment }]);
    }
}
