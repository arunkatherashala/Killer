//! Front-end pass for `@lang { ... }` blocks: runs on the source right after imports are resolved,
//! BEFORE any other rewriting (sugar, indentation, comment stripping, quote rewriting). It takes the
//! body of every block verbatim from the untouched text, removes only the common leading
//! indentation (like Python's `textwrap.dedent`), and replaces the whole block with one ordinary
//! call expression:
//!
//! ```text
//! @python { ... }            ->  print(polyglot_exec("python", "<encoded body>"))   (statement form)
//! x = @python { ... }        ->  x = polyglot_exec("python", "<encoded body>")
//! f(@python(a, b) { ... })   ->  f(polyglot_exec_args("python", "<encoded body>", "a,b", a, b))
//! ```
//!
//! The body is hex-encoded behind [`BLOCK_PREFIX`] so no later pass can see (and rewrite) `//`, `--`,
//! `#`, quotes, braces or indentation inside it; `polyglot_exec` decodes it. The replacement keeps
//! the line count: the lines a block spanned become empty lines, so every line after a block keeps
//! its number and error messages stay right.

/// Marks a hex-encoded block body inside the generated string literal.
pub const BLOCK_PREFIX: &str = "__kpoly_hex__";

/// Decode a body produced by [`extract`]; any other string is returned unchanged.
pub fn decode(code: &str) -> std::borrow::Cow<'_, str> {
    if let Some(hex) = code.strip_prefix(BLOCK_PREFIX) {
        if hex.len() % 2 == 0 {
            let bytes: Option<Vec<u8>> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                .collect();
            if let Some(b) = bytes {
                return std::borrow::Cow::Owned(String::from_utf8_lossy(&b).into_owned());
            }
        }
    }
    std::borrow::Cow::Borrowed(code)
}

fn encode(body: &str) -> String {
    let mut s = String::with_capacity(BLOCK_PREFIX.len() + body.len() * 2);
    s.push_str(BLOCK_PREFIX);
    for b in body.bytes() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Lexical state of the Killer text that carries over line ends (only triple-quoted strings do).
#[derive(Clone, Copy, PartialEq)]
enum Carry {
    Normal,
    Triple(char),
}

/// Remove the common leading whitespace of the body lines. `first` is the text after `{` on the
/// header line; when it holds code it is kept as the first line (trimmed) and does not take part in
/// the common-indent computation.
fn dedent(raw: &str) -> String {
    let raw = raw.replace('\r', "");
    let mut lines: Vec<&str> = raw.split('\n').collect();
    if lines.len() == 1 {
        return lines[0].trim().to_string();
    }
    let first = lines.remove(0).trim();
    // the line holding the closing `}` contributes nothing when it is only indentation
    if lines.last().map_or(false, |l| l.trim().is_empty()) {
        lines.pop();
    }
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| &l[..l.len() - l.trim_start().len()])
        .fold(None::<&str>, |acc, ws| match acc {
            None => Some(ws),
            Some(a) => {
                let n = a.chars().zip(ws.chars()).take_while(|(x, y)| x == y).map(|(x, _)| x.len_utf8()).sum();
                Some(&a[..n])
            }
        })
        .unwrap_or("");
    let mut out: Vec<String> = Vec::new();
    if !first.is_empty() {
        out.push(first.to_string());
    }
    for l in lines {
        if l.trim().is_empty() {
            out.push(String::new());
        } else {
            out.push(l.strip_prefix(indent).unwrap_or(l).trim_end().to_string());
        }
    }
    while out.first().map_or(false, |l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().map_or(false, |l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// Collect a block body starting at `lines[line][col..]` (just after the opening `{`). Braces are
/// counted outside `"` strings. Returns `(raw body, index of the closing line, byte column after
/// the closing brace)`, or `None` when the block never closes.
fn collect_body(lines: &[&str], line: usize, col: usize) -> Option<(String, usize, usize)> {
    let mut depth = 1usize;
    let mut in_str = false;
    let mut escaped = false;
    let mut body = String::new();
    let mut li = line;
    let mut start = col;
    while li < lines.len() {
        let text = lines[li];
        for (off, ch) in text[start..].char_indices() {
            if in_str {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_str = false;
                }
                body.push(ch);
                continue;
            }
            match ch {
                '"' => in_str = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((body, li, start + off + 1));
                    }
                }
                _ => {}
            }
            body.push(ch);
        }
        body.push('\n');
        li += 1;
        start = 0;
    }
    None
}

struct Hit {
    /// byte index of `@` in the segment
    at: usize,
    replacement: String,
    /// lines after the current one that the block consumed
    consumed: usize,
    remainder: String,
}

/// Parse the header that starts at `seg[at] == '@'`: `@lang`, optional `(a, b)`, then `{` on the
/// same line or alone on the next line. Returns `(lang, args, body start line offset, body col)`.
fn parse_header(seg: &str, at: usize, lines: &[&str], line: usize, allow_next_line_brace: bool) -> Option<(String, Vec<String>, usize, usize, usize)> {
    let rest = &seg[at + 1..];
    let mut chars = rest.char_indices().peekable();
    match chars.peek() {
        Some(&(_, c)) if is_ident_start(c) => {}
        _ => return None,
    }
    let mut end = 0;
    while let Some(&(i, c)) = chars.peek() {
        if is_ident(c) {
            end = i + c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }
    let lang = rest[..end].to_string();
    let mut after = &rest[end..];
    let mut args = Vec::new();
    let t = after.trim_start();
    if t.starts_with('(') {
        let close = t.find(')')?;
        for a in t[1..close].split(',') {
            let a = a.trim();
            if a.is_empty() {
                continue;
            }
            if !a.chars().next().map_or(false, is_ident_start) || !a.chars().all(is_ident) {
                return None;
            }
            args.push(a.to_string());
        }
        after = &t[close + 1..];
    }
    let t = after.trim_start();
    if t.starts_with('{') {
        // byte column in `seg` just after `{`
        let col = seg.len() - t.len() + 1;
        return Some((lang, args, 0, col, 0));
    }
    if t.is_empty() && allow_next_line_brace && line + 1 < lines.len() && lines[line + 1].trim() == "{" {
        let next = lines[line + 1];
        let col = next.find('{').unwrap() + 1;
        return Some((lang, args, 1, col, 0));
    }
    None
}

/// Find the first block header in `seg` (Killer text, lexed with `carry`) and build its replacement.
fn find_block(seg: &str, prefix_out: &str, line_start: bool, carry: &mut Carry, lines: &[&str], line: usize) -> Option<Hit> {
    let b: Vec<(usize, char)> = seg.char_indices().collect();
    let mut k = 0;
    while k < b.len() {
        let (i, c) = b[k];
        if let Carry::Triple(q) = *carry {
            if seg[i..].starts_with(&q.to_string().repeat(3)) {
                *carry = Carry::Normal;
                k += 3;
            } else {
                k += 1;
            }
            continue;
        }
        let prev = if k == 0 { prefix_out.chars().last() } else { Some(b[k - 1].1) };
        // comments run to the end of the line
        if (c == '#' && prev.map_or(true, |p| p.is_whitespace())) || seg[i..].starts_with("//") || seg[i..].starts_with("--") {
            return None;
        }
        if c == '"' || c == '\'' {
            let triple = c.to_string().repeat(3);
            if seg[i..].starts_with(&triple) {
                *carry = Carry::Triple(c);
                k += 3;
                continue;
            }
            // single-line string: skip to its end (or the end of the line)
            k += 1;
            while k < b.len() {
                let ch = b[k].1;
                if ch == '\\' {
                    k += 2;
                    continue;
                }
                k += 1;
                if ch == c {
                    break;
                }
            }
            continue;
        }
        if c == '@' && !prev.map_or(false, |p| is_ident(p) || p == '@') {
            let before = format!("{}{}", prefix_out, &seg[..i]);
            let before = before.trim_end();
            let statement = line_start && before.is_empty();
            let assignment = before.ends_with('=') && !before.ends_with("==") && !before.ends_with("!=") && !before.ends_with("<=") && !before.ends_with(">=");
            if let Some((lang, args, line_off, col, _)) = parse_header(seg, i, lines, line, statement || assignment) {
                if statement || assignment || crate::polyglot::is_known_lang(&lang) {
                    // `lines[line]` is `seg`, so columns are relative to the text being scanned
                    let body_line = line + line_off;
                    if let Some((raw, close_line, close_col)) = collect_body(lines, body_line, col) {
                        let body = dedent(&raw);
                        let call = if args.is_empty() {
                            format!("polyglot_exec(\"{}\", \"{}\")", lang, encode(&body))
                        } else {
                            format!("polyglot_exec_args(\"{}\", \"{}\", \"{}\", {})", lang, encode(&body), args.join(","), args.join(", "))
                        };
                        let replacement = if statement { format!("print({})", call) } else { call };
                        return Some(Hit {
                            at: i,
                            replacement,
                            consumed: close_line - line,
                            remainder: lines[close_line][close_col..].to_string(),
                        });
                    }
                }
            }
        }
        k += 1;
    }
    None
}

/// Replace every `@lang { ... }` block in `source` (see the module docs). The output has exactly as
/// many lines as the input.
pub fn extract(source: &str) -> String {
    if !source.contains('@') {
        return source.to_string();
    }
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut carry = Carry::Normal;
    let mut li = 0;
    while li < lines.len() {
        let mut cur = String::new();
        let mut seg = lines[li].to_string();
        let mut line_start = true;
        let mut extra = 0usize;
        loop {
            let base = li + extra;
            let hit = if carry == Carry::Normal || seg.contains("'''") || seg.contains("\"\"\"") {
                find_block(&seg, &cur, line_start, &mut carry, &lines_with(&lines, base, &seg), base)
            } else {
                None
            };
            match hit {
                None => {
                    cur.push_str(&seg);
                    break;
                }
                Some(h) => {
                    cur.push_str(&seg[..h.at]);
                    cur.push_str(&h.replacement);
                    extra += h.consumed;
                    seg = h.remainder;
                    line_start = false;
                }
            }
        }
        out.push(cur);
        for _ in 0..extra {
            out.push(String::new());
        }
        li += 1 + extra;
    }
    out.join("\n")
}

/// `lines` with line `at` replaced by `seg` (the not yet scanned rest of that line).
fn lines_with<'a>(lines: &[&'a str], at: usize, seg: &'a str) -> Vec<&'a str> {
    let mut v: Vec<&str> = lines.to_vec();
    v[at] = seg;
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_of(src: &str) -> String {
        let out = extract(src);
        let start = out.find(BLOCK_PREFIX).expect("no block found");
        let hex: String = out[start..].chars().take_while(|c| *c != '"').collect();
        decode(&hex).into_owned()
    }

    #[test]
    fn keeps_line_count_and_indentation() {
        let src = "x = @python {\n    for i in range(2):\n        print(i // 1)  # c\n}\nprintln(x)\n";
        let out = extract(src);
        assert_eq!(out.matches('\n').count(), src.matches('\n').count());
        assert_eq!(body_of(src), "for i in range(2):\n    print(i // 1)  # c");
        assert!(out.ends_with("\n\n\n\nprintln(x)\n"), "{out}");
    }

    #[test]
    fn statement_assignment_and_expression_forms() {
        assert!(extract("@bash { echo 1 }").starts_with("print(polyglot_exec(\"bash\""));
        assert!(extract("y = @bash { echo 1 }").starts_with("y = polyglot_exec(\"bash\""));
        let e = extract("println(@python { print(5) })");
        assert!(e.starts_with("println(polyglot_exec(\"python\"") && e.ends_with("\"))"), "{e}");
        // an unknown name in expression position is left alone
        assert_eq!(extract("f(@zz { 1 })"), "f(@zz { 1 })");
    }

    #[test]
    fn strings_and_comments_hide_headers() {
        let s = "s = \"a@python { x }\"\n# @python { y }\n-- @python { z }";
        assert_eq!(extract(s), s);
        let t = "t = \"\"\"\n@python { q }\n\"\"\"";
        assert_eq!(extract(t), t);
    }

    #[test]
    fn args_header_and_next_line_brace() {
        let e = extract("r = @python(name, n) { print(name) }");
        assert!(e.starts_with("r = polyglot_exec_args(\"python\", \"") && e.ends_with("\"name,n\", name, n)"), "{e}");
        let src = "r = @python\n{\n    print(1)\n}\nz = 1";
        let out = extract(src);
        assert_eq!(out.lines().count(), src.lines().count());
        assert_eq!(body_of(src), "print(1)");
    }

    #[test]
    fn crlf_is_removed_and_decode_round_trips() {
        let src = "x = @python {\r\n    print(\"a\\\\b\")\r\n}\r\n";
        assert_eq!(body_of(src), "print(\"a\\\\b\")");
        assert_eq!(decode("plain"), "plain");
    }
}
