//! Surface-syntax sugar that is rewritten into plain Killer before the real compiler runs.
//!
//! Everything here is a purely textual, string-aware transformation, so the compiler proper only
//! ever sees the small core syntax:
//!
//! * `'single'` and `"""triple"""` strings become ordinary `"double"` strings
//! * `f"...{x}..."` becomes the existing `k"..."` interpolated string
//! * `0xFF`, `0b101`, `0o17` and `1_000` become plain decimal numbers
//! * `and` / `or` become `&&` / `||`, `elif` becomes `else if`
//! * `x += e` (also `-= *= /= %=`) becomes `x = x + (e)`, `x++` becomes `x = x + 1`
//! * `a, b = b, a` and `a, b = pair()` become temporaries plus single assignments
//!
//! Comments (`#`, `//`, `--`) and embedded-language blocks (`@python { ... }`) are copied verbatim.

/// Rewrite `source`. `is_polyglot_header` tells whether a trimmed line opens an `@lang { ... }`
/// block, whose contents must not be touched.
pub fn preprocess(source: &str, is_polyglot_header: &dyn Fn(&str) -> bool) -> String {
    preprocess_mapped(source, is_polyglot_header).0
}

/// Like [`preprocess`], also returning for every output line the index of the input line it came
/// from (a collapsed multi-line string shifts the lines after it; a statement that expands into
/// several lines maps all of them to its own line).
pub fn preprocess_mapped(source: &str, is_polyglot_header: &dyn Fn(&str) -> bool) -> (String, Vec<usize>) {
    let mut skips: Vec<(usize, usize)> = Vec::new();
    let lexed = lex_pass(source, is_polyglot_header, &mut skips);
    // lexed line j came from source line j + (lines swallowed on earlier lines)
    let lexed_lines = crate::sourcemap::line_count(&lexed);
    let mut lex_map: Vec<usize> = Vec::with_capacity(lexed_lines);
    {
        // (output line holding the string, lines it swallowed); lines after it shift by that much
        let events: Vec<(usize, usize)> = skips
            .iter()
            .map(|&(off, k)| (lexed.as_bytes()[..off.min(lexed.len())].iter().filter(|&&b| b == b'\n').count(), k))
            .collect();
        let mut shift = 0usize;
        let mut next_event = 0usize;
        for line in 0..lexed_lines {
            lex_map.push(line + shift);
            while next_event < events.len() && events[next_event].0 == line {
                shift += events[next_event].1;
                next_event += 1;
            }
        }
    }
    let (text, line_map) = line_pass_mapped(&lexed);
    let map = line_map.iter().map(|&i| lex_map[i.min(lex_map.len() - 1)]).collect();
    (text, map)
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// ------------------------------------------------------------------------------------------
// Pass 1: strings, numbers, word operators
// ------------------------------------------------------------------------------------------

fn lex_pass(source: &str, is_polyglot_header: &dyn Fn(&str) -> bool, skips: &mut Vec<(usize, usize)>) -> String {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(source.len() + 16);
    let mut i = 0;

    let line_end = |from: usize| -> usize {
        let mut j = from;
        while j < n && chars[j] != '\n' {
            j += 1;
        }
        j
    };

    while i < n {
        // start of a line: embedded-language blocks are copied untouched
        if i == 0 || chars[i - 1] == '\n' {
            let end = line_end(i);
            let line: String = chars[i..end].iter().collect();
            let trimmed = line.trim();
            if trimmed.starts_with('@') && is_polyglot_header(trimmed) {
                let (copied, next) = copy_polyglot_block(&chars, i);
                if next > i {
                    out.push_str(&copied);
                    i = next;
                    continue;
                }
            }
        }

        let c = chars[i];

        // comments run to the end of the line
        let prev_is_space = i == 0 || chars[i - 1].is_whitespace();
        let two = |a: char, b: char| c == a && i + 1 < n && chars[i + 1] == b;
        if (c == '#' && prev_is_space) || two('/', '/') || two('-', '-') {
            let end = line_end(i);
            out.extend(chars[i..end].iter());
            i = end;
            continue;
        }

        // strings
        if c == '"' || c == '\'' {
            let triple = i + 2 < n && chars[i + 1] == c && chars[i + 2] == c;
            // f"..." / f'...' -> k"..."
            if out.ends_with('f') || out.ends_with('F') {
                let before = out.chars().rev().nth(1);
                if before.map_or(true, |b| !is_ident(b)) {
                    out.pop();
                    out.push('k');
                }
            }
            if triple {
                let mut j = i + 3;
                let mut body = String::new();
                let mut closed = false;
                while j < n {
                    if chars[j] == '\u{5c}' && j + 1 < n {
                        body.push(chars[j]);
                        body.push(chars[j + 1]);
                        j += 2;
                        continue;
                    }
                    if j + 3 <= n && chars[j] == c && chars[j + 1] == c && chars[j + 2] == c {
                        closed = true;
                        break;
                    }
                    body.push(chars[j]);
                    j += 1;
                }
                out.push('"');
                let escaped = escape_for_double(&body);
                // a multi-line string collapses onto one line: remember how many lines vanished
                let swallowed = body.matches('\n').count().saturating_sub(escaped.matches('\n').count());
                if swallowed > 0 {
                    skips.push((out.len(), swallowed));
                }
                out.push_str(&escaped);
                out.push('"');
                i = if closed { j + 3 } else { j };
                continue;
            }
            // single- or double-quoted string on one line
            let mut j = i + 1;
            let mut body = String::new();
            let mut closed = false;
            while j < n {
                let d = chars[j];
                if d == '\\' && j + 1 < n {
                    body.push(d);
                    body.push(chars[j + 1]);
                    j += 2;
                    continue;
                }
                if d == c {
                    closed = true;
                    break;
                }
                if d == '\n' {
                    break;
                }
                body.push(d);
                j += 1;
            }
            if c == '"' {
                // keep double-quoted strings exactly as written
                out.push('"');
                out.push_str(&body);
                if closed {
                    out.push('"');
                    i = j + 1;
                } else {
                    i = j;
                }
            } else {
                out.push('"');
                out.push_str(&single_to_double(&body));
                out.push('"');
                i = if closed { j + 1 } else { j };
            }
            continue;
        }

        // words and numbers
        if is_ident(c) {
            let start = i;
            let mut j = i;
            while j < n && is_ident(chars[j]) {
                j += 1;
            }
            let word: String = chars[start..j].iter().collect();
            let preceded_by_dot = out.trim_end().ends_with('.');
            if word.chars().next().map_or(false, |d| d.is_ascii_digit()) {
                // number: 0xFF 0b101 0o17 1_000 1_000.5
                let mut text = word.clone();
                if j + 1 < n && chars[j] == '.' && chars[j + 1].is_ascii_digit() && !text.contains(|ch: char| ch.is_ascii_alphabetic()) {
                    text.push('.');
                    j += 1;
                    while j < n && (chars[j].is_ascii_digit() || chars[j] == '_') {
                        text.push(chars[j]);
                        j += 1;
                    }
                }
                out.push_str(&number_to_decimal(&text));
                i = j;
                continue;
            }
            let replaced = if preceded_by_dot {
                None
            } else {
                match word.as_str() {
                    "and" => Some("&&"),
                    "or" => Some("||"),
                    "elif" => Some("else if"),
                    _ => None,
                }
            };
            match replaced {
                Some(r) => out.push_str(r),
                None => out.push_str(&word),
            }
            i = j;
            continue;
        }

        out.push(c);
        i += 1;
    }
    out
}

/// Copy an `@lang { ... }` block verbatim. Returns (text, index after it); `next == start` when the
/// header turns out not to open a block.
fn copy_polyglot_block(chars: &[char], start: usize) -> (String, usize) {
    let n = chars.len();
    let mut depth = 0i32;
    let mut opened = false;
    let mut j = start;
    let mut lines_without_brace = 0;
    while j < n {
        let end = {
            let mut e = j;
            while e < n && chars[e] != '\n' {
                e += 1;
            }
            e
        };
        let line: String = chars[j..end].iter().collect();
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                opened = true;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if !opened {
            lines_without_brace += 1;
            // the brace may be on the very next line; otherwise this is not a block
            if lines_without_brace > 2 {
                return (String::new(), start);
            }
            if line.trim().is_empty() {
                return (String::new(), start);
            }
        }
        j = (end + 1).min(n);
        if opened && depth <= 0 {
            break;
        }
    }
    if !opened {
        return (String::new(), start);
    }
    (chars[start..j].iter().collect(), j)
}

/// Body of a triple-quoted string -> body of a normal double-quoted string.
fn escape_for_double(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                out.push('\\');
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Body of a single-quoted string -> body of a double-quoted string.
fn single_to_double(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\'') => out.push('\''),
                Some(next) => {
                    out.push('\\');
                    out.push(next);
                }
                None => out.push('\\'),
            },
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

fn number_to_decimal(text: &str) -> String {
    let t: String = text.chars().filter(|c| *c != '_').collect();
    let lower = t.to_ascii_lowercase();
    let parse = |digits: &str, radix: u32| u128::from_str_radix(digits, radix).ok().map(|v| v.to_string());
    if let Some(d) = lower.strip_prefix("0x") {
        if let Some(v) = parse(d, 16) {
            return v;
        }
    } else if let Some(d) = lower.strip_prefix("0b") {
        if let Some(v) = parse(d, 2) {
            return v;
        }
    } else if let Some(d) = lower.strip_prefix("0o") {
        if let Some(v) = parse(d, 8) {
            return v;
        }
    }
    t
}

// ------------------------------------------------------------------------------------------
// Pass 2: statement rewrites (augmented assignment, ++, multiple assignment)
// ------------------------------------------------------------------------------------------

/// Replace every character inside a string literal with placeholder bytes so structure can be
/// examined without being fooled by quoted text. The result has exactly the same *byte* length as
/// the input (a multi-byte character becomes that many placeholder bytes), so byte offsets found in
/// the masked line are valid in the original line.
fn mask_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let mut escaped = false;
    let fill = |out: &mut String, c: char| {
        for _ in 0..c.len_utf8() {
            out.push('\u{1}');
        }
    };
    for c in line.chars() {
        if in_str {
            if escaped {
                escaped = false;
                fill(&mut out, c);
            } else if c == '\\' {
                escaped = true;
                fill(&mut out, c);
            } else if c == '"' {
                in_str = false;
                out.push('"');
            } else {
                fill(&mut out, c);
            }
        } else {
            if c == '"' {
                in_str = true;
            }
            out.push(c);
        }
    }
    out
}

/// Parse a leading assignment target `name`, `a.b.c`, `a[i]`, `a[i][j]`, `this.x[i]` from `s`.
/// Returns the byte length of the target, or `None`.
fn target_len(s: &str) -> Option<usize> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    if i >= b.len() || !(b[i].is_alphabetic() || b[i] == '_') {
        return None;
    }
    while i < b.len() && is_ident(b[i]) {
        i += 1;
    }
    loop {
        if i < b.len() && b[i] == '.' && i + 1 < b.len() && (b[i + 1].is_alphabetic() || b[i + 1] == '_') {
            i += 1;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
        } else if i < b.len() && b[i] == '[' {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == '[' {
                    depth += 1;
                } else if b[i] == ']' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            if depth != 0 {
                return None;
            }
        } else {
            break;
        }
    }
    Some(b[..i].iter().map(|c| c.len_utf8()).sum())
}

fn split_top_level_commas(masked: &str, original: &str) -> Vec<String> {
    // `masked` and `original` have identical byte layouts (see `mask_strings`), and commas are ASCII,
    // so every split position is a valid char boundary in both.
    let m = masked.as_bytes();
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in m.iter().enumerate() {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(original[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(original[start..].trim().to_string());
    parts
}

fn line_pass_mapped(source: &str) -> (String, Vec<usize>) {
    let mut counter = 0usize;
    let mut out = String::with_capacity(source.len() + 16);
    let mut map = Vec::new();
    let mut first = true;
    for (idx, line) in source.split('\n').enumerate() {
        if !first {
            out.push('\n');
        }
        first = false;
        let rewritten = rewrite_line(line, &mut counter);
        // a statement may expand into several lines; all of them belong to this one
        for _ in 0..rewritten.matches('\n').count() + 1 {
            map.push(idx);
        }
        out.push_str(&rewritten);
    }
    (out, map)
}

fn rewrite_line(line: &str, counter: &mut usize) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.starts_with("--") {
        return line.to_string();
    }
    let indent = &line[..line.len() - trimmed.len()];
    let masked_full = mask_strings(line);
    let masked = &masked_full[masked_full.len() - trimmed.len()..];
    // a trailing comment must not leak into the rewritten expression
    let (code, comment) = match find_comment(masked, trimmed) {
        Some(at) => (trimmed[..at].trim_end(), &trimmed[at..]),
        None => (trimmed, ""),
    };
    let masked_code = &masked[..code.len()];
    let comment_part = if comment.is_empty() { String::new() } else { format!("  {}", comment) };
    let code = code.trim_end_matches(';').trim_end();
    let masked_code = &masked_code[..code.len().min(masked_code.len())];

    // 1) x++  /  a[i]++  /  this.n++
    if let Some(tl) = target_len(code) {
        let rest = code[tl..].trim();
        if rest == "++" {
            let target = &code[..tl];
            return format!("{}{} = {} + 1{}", indent, target, target, comment_part);
        }
        // 2) x += e   x -= e   x *= e   x /= e   x %= e
        let rest_masked = masked_code[tl..].trim_start();
        let rest_orig = code[tl..].trim_start();
        let mut it = rest_masked.chars();
        if let (Some(op), Some('=')) = (it.next(), it.next()) {
            if matches!(op, '+' | '-' | '*' | '/' | '%') && !rest_masked[2..].starts_with('=') {
                let target = &code[..tl];
                let rhs = rest_orig[2..].trim();
                if !rhs.is_empty() {
                    return format!("{}{} = {} {} ({}){}", indent, target, target, op, rhs, comment_part);
                }
            }
        }
    }

    // 3) a, b = x, y   /   a, b = pair()
    if let Some(eq) = find_assign_eq(masked_code) {
        let lhs_m = masked_code[..eq].trim();
        let lhs_o = code[..eq].trim();
        let rhs_m = masked_code[eq + 1..].trim();
        let rhs_o = code[eq + 1..].trim();
        if lhs_m.contains(',')
            && lhs_m.split(',').all(|p| {
                let p = p.trim();
                !p.is_empty() && p.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && p.chars().all(is_ident)
            })
            && !rhs_m.starts_with('=')
        {
            let names: Vec<&str> = lhs_o.split(',').map(|p| p.trim()).collect();
            let id = *counter;
            *counter += 1;
            let parts = split_top_level_commas(rhs_m, rhs_o);
            let mut stmts: Vec<String> = Vec::new();
            if parts.len() == names.len() {
                for (k, p) in parts.iter().enumerate() {
                    stmts.push(format!("__mt{}_{} = {}", id, k, p));
                }
                for (k, name) in names.iter().enumerate() {
                    stmts.push(format!("{} = __mt{}_{}", name, id, k));
                }
            } else if parts.len() == 1 {
                stmts.push(format!("__mu{} = {}", id, rhs_o));
                for (k, name) in names.iter().enumerate() {
                    stmts.push(format!("{} = __mu{}[{}]", name, id, k));
                }
            } else {
                return line.to_string();
            }
            return format!("{}{}{}", indent, stmts.join("; "), comment_part);
        }
    }
    line.to_string()
}

/// Byte index of the first comment start (`#` after whitespace, `//`, `--`) outside strings.
fn find_comment(masked: &str, _original: &str) -> Option<usize> {
    let b = masked.as_bytes();
    for i in 0..b.len() {
        let c = b[i];
        let prev_space = i == 0 || b[i - 1].is_ascii_whitespace();
        if (c == b'#' && prev_space) || (c == b'/' && i + 1 < b.len() && b[i + 1] == b'/') || (c == b'-' && i + 1 < b.len() && b[i + 1] == b'-') {
            return Some(i);
        }
    }
    None
}

/// Byte index of the first plain `=` that is an assignment (not `==`, `<=`, `>=`, `!=`, `=>`).
fn find_assign_eq(masked: &str) -> Option<usize> {
    let b = masked.as_bytes();
    let mut depth = 0i32;
    for i in 0..b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 => {
                let prev = if i > 0 { b[i - 1] } else { 0 };
                let next = if i + 1 < b.len() { b[i + 1] } else { 0 };
                if !matches!(prev, b'=' | b'<' | b'>' | b'!') && next != b'=' && next != b'>' {
                    return Some(i);
                }
                return None;
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String {
        preprocess(s, &|l| l.starts_with("@python") || l.starts_with("@go"))
    }

    #[test]
    fn line_map_survives_collapsed_strings_and_expanded_statements() {
        let (out, map) = preprocess_mapped("s = \"\"\"a\nb\nc\"\"\"\nx = 1\ny = 2\n", &|_| false);
        assert_eq!(out, "s = \"a\\nb\\nc\"\nx = 1\ny = 2\n");
        assert_eq!(map, vec![0, 3, 4, 5]);
        let (out, map) = preprocess_mapped("a = 1\nb, c = c, b\nz = 3\n", &|_| false);
        assert_eq!(map.len(), crate::sourcemap::line_count(&out), "{out}");
        let line_of = |needle: &str| out.split('\n').position(|l| l.contains(needle)).unwrap();
        assert_eq!(map[line_of("z = 3")], 2, "{out}");
        assert_eq!(map[line_of("a = 1")], 0, "{out}");
        assert!(map[..line_of("z = 3")].iter().skip(1).all(|&m| m == 1), "{map:?}");
    }

    #[test]
    fn single_quoted_strings_become_double_quoted() {
        assert_eq!(p("x = 'hi'"), "x = \"hi\"");
        assert_eq!(p("x = 'say \"hi\"'"), "x = \"say \\\"hi\\\"\"");
        assert_eq!(p("x = 'it\\'s'"), "x = \"it's\"");
        assert_eq!(p("x = \"it's\""), "x = \"it's\"");
    }

    #[test]
    fn triple_quoted_strings_become_one_line() {
        assert_eq!(p("s = \"\"\"a\nb\"\"\"\nprintln(s)"), "s = \"a\\nb\"\nprintln(s)");
        assert_eq!(p("s = '''x\"y'''"), "s = \"x\\\"y\"");
    }

    #[test]
    fn f_strings_become_k_strings() {
        assert_eq!(p("println(f\"n is {n}\")"), "println(k\"n is {n}\")");
        assert_eq!(p("x = f'a{b}'"), "x = k\"a{b}\"");
        // an identifier that merely ends in f is untouched
        assert_eq!(p("shelf\"x\""), "shelf\"x\"");
    }

    #[test]
    fn number_literals_are_normalised() {
        assert_eq!(p("x = 0xFF + 0b101 + 0o17"), "x = 255 + 5 + 15");
        assert_eq!(p("x = 1_000_000"), "x = 1000000");
        assert_eq!(p("x = 1_000.5"), "x = 1000.5");
        assert_eq!(p("x = 3.14"), "x = 3.14");
        assert_eq!(p("a.b2 = 7"), "a.b2 = 7");
    }

    #[test]
    fn word_operators() {
        assert_eq!(p("if a and b or c {"), "if a && b || c {");
        assert_eq!(p("elif x {"), "else if x {");
        assert_eq!(p("} elif x {"), "} else if x {");
        assert_eq!(p("x = android + origin"), "x = android + origin");
        assert_eq!(p("x = o.and"), "x = o.and");
        assert_eq!(p("s = \"this and that\""), "s = \"this and that\"");
    }

    #[test]
    fn comments_are_left_alone() {
        assert_eq!(p("x = 1 # don't 'touch' this"), "x = 1 # don't 'touch' this");
        assert_eq!(p("// it's a comment and more"), "// it's a comment and more");
        assert_eq!(p("x = 'a#b'"), "x = \"a#b\"");
    }

    #[test]
    fn polyglot_blocks_are_untouched() {
        let src = "@python {\nprint('hi' and 0xFF)\n}\nx = 'a'\n";
        assert_eq!(p(src), "@python {\nprint('hi' and 0xFF)\n}\nx = \"a\"\n");
    }

    #[test]
    fn augmented_assignment_and_increment() {
        assert_eq!(p("x += 5"), "x = x + (5)");
        assert_eq!(p("  total -= a * 2"), "  total = total - (a * 2)");
        assert_eq!(p("a[i] *= 3"), "a[i] = a[i] * (3)");
        assert_eq!(p("this.n += 1"), "this.n = this.n + (1)");
        assert_eq!(p("x++"), "x = x + 1");
        assert_eq!(p("this.count++"), "this.count = this.count + 1");
        assert_eq!(p("x /= 2  # halve"), "x = x / (2)  # halve");
        // comparisons and plain assignment are not touched
        assert_eq!(p("x == 5"), "x == 5");
        assert_eq!(p("x = y + 1"), "x = y + 1");
        assert_eq!(p("if x <= 3 {"), "if x <= 3 {");
        assert_eq!(p("s += \"a+=b\""), "s = s + (\"a+=b\")");
    }

    #[test]
    fn multiple_assignment() {
        assert_eq!(p("a, b = 1, 2"), "__mt0_0 = 1; __mt0_1 = 2; a = __mt0_0; b = __mt0_1");
        assert_eq!(p("a, b = b, a"), "__mt0_0 = b; __mt0_1 = a; a = __mt0_0; b = __mt0_1");
        assert_eq!(p("a, b = pair()"), "__mu0 = pair(); a = __mu0[0]; b = __mu0[1]");
        assert_eq!(p("a, b = f(1, 2)"), "__mu0 = f(1, 2); a = __mu0[0]; b = __mu0[1]");
        assert_eq!(p("a, b = [1, 2], \"x, y\""), "__mt0_0 = [1, 2]; __mt0_1 = \"x, y\"; a = __mt0_0; b = __mt0_1");
        // function headers and ordinary lines are untouched
        assert_eq!(p("fn f(a, b = 5) {"), "fn f(a, b = 5) {");
        assert_eq!(p("x = [1, 2]"), "x = [1, 2]");
        assert_eq!(p("for i, x in pairs {"), "for i, x in pairs {");
    }

    #[test]
    fn unique_temporaries_per_statement() {
        let out = p("a, b = 1, 2\nc, d = 3, 4");
        assert!(out.contains("__mt0_0") && out.contains("__mt1_0"), "{}", out);
    }

    #[test]
    fn plain_programs_are_unchanged() {
        let src = "fn add(a, b) {\n  return a + b\n}\nprintln(add(1, 2))\n";
        assert_eq!(p(src), src);
    }
}

#[cfg(test)]
mod unicode_tests {
    use super::*;

    fn p(s: &str) -> String {
        preprocess(s, &|_| false)
    }

    #[test]
    fn non_ascii_text_in_strings_does_not_break_offsets() {
        assert_eq!(p("x += \"→ café —\""), "x = x + (\"→ café —\")");
        assert_eq!(p("a, b = \"é,ü\", 'ß'"), "__mt0_0 = \"é,ü\"; __mt0_1 = \"ß\"; a = __mt0_0; b = __mt0_1");
        assert_eq!(p("println(\"⚠ warning — it's\")  # ünïcode comment"), "println(\"⚠ warning — it's\")  # ünïcode comment");
        assert_eq!(p("s = '→'\ns += \"é\""), "s = \"→\"\ns = s + (\"é\")");
    }

    #[test]
    fn mask_has_the_same_byte_length() {
        for s in ["", "abc", "x = \"é→—\"", "a \"b\\\"c\" d", "\"unterminated é"] {
            assert_eq!(mask_strings(s).len(), s.len(), "{:?}", s);
        }
    }
}
