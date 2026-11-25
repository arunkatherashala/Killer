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

// --- Body scanner: finds the `}` that closes a block -------------------------------------------
//
// Only `{` and `}` in the foreign code count. To know which ones are code, the scanner knows the
// strings and comments of each supported language (`syntax_for`), so a quote or a brace inside a
// string or a comment never decides where the block ends. When no closing brace is found the
// block is a compile error naming its line; it is never handed to another pass.

/// Backslash, single quote and double quote bytes.
const BS: u8 = 92;
const SQ: u8 = 39;
const DQ: u8 = 34;

/// How `#` starts a comment.
#[derive(Clone, Copy, PartialEq)]
enum Hash {
    No,
    /// anywhere outside a string (Python, Ruby, R, Elixir, PHP)
    Any,
    /// only at the start of a word (shells, PowerShell: `a#b` is one word)
    WordStart,
    /// anywhere except right after `$` (Perl: `$#array`)
    NotAfterDollar,
}

/// What a single quote starts.
#[derive(Clone, Copy, PartialEq)]
enum Single {
    /// nothing (Swift)
    No,
    /// a string that ends at the end of the line (Python, JS, Lua)
    Line,
    /// a string that may span lines (shells, Ruby, Perl, PHP, R, Elixir, PowerShell)
    Multi,
    /// a character literal of one character or one escape; anything else (Rust lifetimes,
    /// Haskell primes, C++ digit separators) is no string
    Char,
}

/// The lexical rules of one language, as far as the scanner needs them.
#[derive(Clone, Copy)]
struct Syntax {
    hash: Hash,
    /// `//` line comments
    slash: bool,
    /// `/* */` comments; `nest`: they nest (Rust, Swift, Kotlin, Scala)
    c_block: bool,
    nest: bool,
    /// `--` line comments (Lua, Haskell)
    dash: bool,
    single: Single,
    /// the escape character works inside single-quoted strings (not in the shells or PowerShell)
    single_escape: bool,
    /// double-quoted strings may span lines
    double_multi: bool,
    /// escape character in strings (backtick in PowerShell, backslash elsewhere)
    esc: u8,
    /// the escape character also escapes the next character in code (shells, PowerShell)
    code_escape: bool,
    /// triple-quoted strings (double, and single where single quotes start strings)
    triple: bool,
    /// backtick: 0 = nothing, 1 = raw string (Go, R), 2 = JS template literal with `${ }`
    backtick: u8,
    /// JS regex literals `/.../`
    regex: bool,
    /// heredocs: 0 = none, 2 = shell `<<WORD`, 3 = PHP `<<<WORD`
    heredoc: u8,
    /// PowerShell here-strings (`@` + quote at the end of a line) and `<# #>` comments
    powershell: bool,
    /// Lua `[[ ]]` long strings and `--[[ ]]` comments
    lua_long: bool,
    /// Haskell `{- -}` comments
    haskell: bool,
    /// Rust raw strings (`r` or `r#` before the quote)
    rust_raw: bool,
    /// Zig line strings (two backslashes to the end of the line)
    zig: bool,
    /// a line starting with `=` + letter opens a doc block that ends at a line starting with this
    /// (Ruby `=begin` .. `=end`, Perl POD .. `=cut`)
    doc_end: Option<&'static str>,
}

const BASE: Syntax = Syntax {
    hash: Hash::No,
    slash: false,
    c_block: false,
    nest: false,
    dash: false,
    single: Single::Line,
    single_escape: true,
    double_multi: false,
    esc: BS,
    code_escape: false,
    triple: false,
    backtick: 0,
    regex: false,
    heredoc: 0,
    powershell: false,
    lua_long: false,
    haskell: false,
    rust_raw: false,
    zig: false,
    doc_end: None,
};
const C_LIKE: Syntax = Syntax { slash: true, c_block: true, single: Single::Char, ..BASE };

fn syntax_for(lang: &str) -> Syntax {
    match crate::polyglot::find_runtime(lang).map(|r| r.name).unwrap_or("") {
        "python" => Syntax { hash: Hash::Any, triple: true, ..BASE },
        "node" | "ts" => Syntax { slash: true, c_block: true, backtick: 2, regex: true, ..BASE },
        "ruby" => Syntax { hash: Hash::Any, single: Single::Multi, double_multi: true, doc_end: Some("=end"), ..BASE },
        "perl" => Syntax { hash: Hash::NotAfterDollar, single: Single::Multi, double_multi: true, doc_end: Some("=cut"), ..BASE },
        "php" => Syntax { hash: Hash::Any, slash: true, c_block: true, single: Single::Multi, double_multi: true, heredoc: 3, ..BASE },
        "lua" => Syntax { dash: true, lua_long: true, ..BASE },
        "r" => Syntax { hash: Hash::Any, single: Single::Multi, double_multi: true, backtick: 1, ..BASE },
        "go" => Syntax { backtick: 1, ..C_LIKE },
        "bash" | "sh" => Syntax { hash: Hash::WordStart, single: Single::Multi, single_escape: false, double_multi: true, code_escape: true, heredoc: 2, ..BASE },
        "powershell" => Syntax { hash: Hash::WordStart, single: Single::Multi, single_escape: false, double_multi: true, esc: b'`', code_escape: true, powershell: true, ..BASE },
        "swift" => Syntax { nest: true, single: Single::No, triple: true, ..C_LIKE },
        "kotlin" | "scala" => Syntax { nest: true, triple: true, ..C_LIKE },
        "elixir" => Syntax { hash: Hash::Any, single: Single::Multi, double_multi: true, triple: true, ..BASE },
        "haskell" => Syntax { dash: true, haskell: true, single: Single::Char, ..BASE },
        "zig" => Syntax { c_block: false, zig: true, ..C_LIKE },
        "java" => Syntax { triple: true, ..C_LIKE },
        "rust" => Syntax { nest: true, double_multi: true, rust_raw: true, ..C_LIKE },
        "c" | "cpp" => C_LIKE,
        // an unknown name fails when the block runs; its strings end at the line end, so a stray
        // quote cannot hide the closing brace
        _ => BASE,
    }
}

#[derive(Clone, PartialEq)]
enum Mode {
    Code,
    /// a string closed by `q`; `multi`: it may span lines; `esc`: escape character (0 = none)
    Str { q: u8, multi: bool, esc: u8 },
    Triple(u8),
    Block { open: &'static [u8], close: &'static [u8], nest: bool, depth: usize },
    /// Lua long bracket of this level (`]==]` = 2)
    Long(usize),
    /// Rust raw string closed by a double quote and this many `#`
    RustRaw(usize),
    /// heredoc body: ends at a line equal to the word
    HereDoc(String),
    /// PowerShell here-string: ends at a line starting with this quote and `@`
    PsHere(u8),
    /// Ruby / Perl doc block
    Doc(&'static str),
    /// JS template literal
    Template,
}

impl Mode {
    fn what(&self) -> &'static str {
        match self {
            Mode::Code => "",
            Mode::Str { .. } => "string",
            Mode::Triple(_) => "triple-quoted string",
            Mode::Block { .. } => "block comment",
            Mode::Long(_) => "long string",
            Mode::RustRaw(_) => "raw string",
            Mode::HereDoc(_) => "heredoc",
            Mode::PsHere(_) => "here-string",
            Mode::Doc(_) => "doc block",
            Mode::Template => "template literal",
        }
    }
}

fn starts(b: &[u8], i: usize, pat: &[u8]) -> bool {
    b.len() >= i + pat.len() && &b[i..i + pat.len()] == pat
}

fn ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

/// Lua long bracket `[==[` at `b[i]`: `Some((level, length))`.
fn long_open(b: &[u8], i: usize) -> Option<(usize, usize)> {
    if b.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while b.get(j) == Some(&b'=') {
        j += 1;
    }
    (b.get(j) == Some(&b'[')).then(|| (j - i - 1, j - i + 1))
}

/// Length of a character literal starting at the single quote `b[i]`, or `None` when this quote
/// starts no literal.
fn char_literal(text: &str, i: usize) -> Option<usize> {
    let b = text.as_bytes();
    if b.get(i + 1) == Some(&BS) {
        let mut j = i + 3;
        while j < b.len() && j < i + 14 {
            if b[j] == SQ {
                return Some(j - i + 1);
            }
            j += 1;
        }
        return None;
    }
    let c = text.get(i + 1..)?.chars().next()?;
    (c as u32 != SQ as u32 && b.get(i + 1 + c.len_utf8()) == Some(&SQ)).then(|| c.len_utf8() + 2)
}

/// A heredoc word after `<<` / `<<<` at `b[j..]` (optional `-`, blanks and quotes):
/// `Some((word, index after it))`.
fn heredoc_word(b: &[u8], mut j: usize, dash: bool) -> Option<(String, usize)> {
    if dash && b.get(j) == Some(&b'-') {
        j += 1;
    }
    while matches!(b.get(j), Some(b' ') | Some(b'\t')) {
        j += 1;
    }
    let quote = match b.get(j) {
        Some(&q) if q == SQ || q == DQ => {
            j += 1;
            Some(q)
        }
        _ => None,
    };
    let start = j;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
        j += 1;
    }
    if j == start || b[start].is_ascii_digit() {
        return None;
    }
    let word = String::from_utf8_lossy(&b[start..j]).into_owned();
    if let Some(q) = quote {
        if b.get(j) != Some(&q) {
            return None;
        }
        j += 1;
    }
    Some((word, j))
}

/// Can a `/` after `prev` (the last non-blank byte before it) start a JS regex literal?
fn regex_may_start(prev: Option<u8>) -> bool {
    prev.map_or(true, |p| b"(,=:[!&|?{};+-*%<>~^".contains(&p))
}

/// Find the `}` that closes a block whose body starts at `v.get(line)[col..]`. Returns the closing
/// line and the byte column after the brace, or `Err((line, what))`: the unclosed string or comment
/// that ran to the end of the file and the line it opened on (`what == ""`: the braces themselves
/// never balance).
fn scan_body(v: &View, line: usize, col: usize, syn: &Syntax) -> Result<(usize, usize), (usize, &'static str)> {
    let mut mode = Mode::Code;
    let mut opened = line;
    // brace depth of the block, then one entry per open JS `${` interpolation
    let mut depth: Vec<usize> = vec![1];
    let mut pending: Vec<String> = Vec::new();
    let mut li = line;
    while li < v.len() {
        let text = v.get(li).strip_suffix('\r').unwrap_or(v.get(li));
        let b = text.as_bytes();
        let mut i = if li == line { col } else { 0 };
        // line-oriented modes
        match mode.clone() {
            Mode::HereDoc(word) => {
                if text.trim() == word {
                    mode = if pending.is_empty() { Mode::Code } else { Mode::HereDoc(pending.remove(0)) };
                }
                li += 1;
                continue;
            }
            Mode::PsHere(q) => {
                let t = text.trim_start();
                if !starts(t.as_bytes(), 0, &[q, b'@']) {
                    li += 1;
                    continue;
                }
                mode = Mode::Code;
                i = text.len() - t.len() + 2;
            }
            Mode::Doc(end) => {
                if text.starts_with(end) {
                    mode = Mode::Code;
                }
                li += 1;
                continue;
            }
            _ => {}
        }
        if let (Mode::Code, Some(end), 0) = (&mode, syn.doc_end, i) {
            if b.first() == Some(&b'=') && b.get(1).map_or(false, |c| c.is_ascii_alphabetic()) && !text.starts_with(end) {
                mode = Mode::Doc(end);
                opened = li;
                li += 1;
                continue;
            }
        }
        while i < b.len() {
            let c = b[i];
            match mode {
                Mode::Code => {
                    let prev = if i == 0 { None } else { Some(b[i - 1]) };
                    if (syn.slash && starts(b, i, b"//")) || (syn.zig && starts(b, i, &[BS, BS])) {
                        break;
                    }
                    let block = if syn.c_block && starts(b, i, b"/*") {
                        Some(Mode::Block { open: b"/*", close: b"*/", nest: syn.nest, depth: 1 })
                    } else if syn.haskell && starts(b, i, b"{-") {
                        Some(Mode::Block { open: b"{-", close: b"-}", nest: true, depth: 1 })
                    } else if syn.powershell && starts(b, i, b"<#") {
                        Some(Mode::Block { open: b"<#", close: b"#>", nest: false, depth: 1 })
                    } else {
                        None
                    };
                    if let Some(m) = block {
                        mode = m;
                        opened = li;
                        i += 2;
                        continue;
                    }
                    if syn.dash && starts(b, i, b"--") {
                        match long_open(b, i + 2).filter(|_| syn.lua_long) {
                            Some((level, len)) => {
                                mode = Mode::Long(level);
                                opened = li;
                                i += 2 + len;
                                continue;
                            }
                            None => break,
                        }
                    }
                    if c == b'#' {
                        let comment = match syn.hash {
                            Hash::No => false,
                            Hash::Any => true,
                            Hash::WordStart => prev.map_or(true, |p| p.is_ascii_whitespace() || b";|&()".contains(&p)),
                            Hash::NotAfterDollar => prev != Some(b'$'),
                        };
                        if comment {
                            break;
                        }
                    }
                    if syn.lua_long {
                        if let Some((level, len)) = long_open(b, i) {
                            mode = Mode::Long(level);
                            opened = li;
                            i += len;
                            continue;
                        }
                    }
                    if syn.rust_raw && c == b'r' && prev.map_or(true, |p| !ident_byte(p) || (p == b'b' && (i < 2 || !ident_byte(b[i - 2])))) {
                        let mut j = i + 1;
                        while b.get(j) == Some(&b'#') {
                            j += 1;
                        }
                        if b.get(j) == Some(&DQ) {
                            mode = Mode::RustRaw(j - i - 1);
                            opened = li;
                            i = j + 1;
                            continue;
                        }
                    }
                    if syn.powershell && c == b'@' && matches!(b.get(i + 1), Some(&q) if q == DQ || q == SQ) && text[i + 2..].trim().is_empty() {
                        mode = Mode::PsHere(b[i + 1]);
                        opened = li;
                        break;
                    }
                    if syn.heredoc == 2 && starts(b, i, b"<<") && !starts(b, i, b"<<<") {
                        let before = &text[..i];
                        let arith = before.rfind("((").map_or(false, |o| before.rfind("))").map_or(true, |e| e < o));
                        if !arith {
                            if let Some((word, j)) = heredoc_word(b, i + 2, true) {
                                pending.push(word);
                                opened = li;
                                i = j;
                                continue;
                            }
                        }
                    }
                    if syn.heredoc == 3 && starts(b, i, b"<<<") {
                        if let Some((word, j)) = heredoc_word(b, i + 3, false) {
                            pending.push(word);
                            opened = li;
                            i = j;
                            continue;
                        }
                    }
                    if c == DQ {
                        opened = li;
                        if syn.triple && starts(b, i, &[DQ, DQ, DQ]) {
                            mode = Mode::Triple(DQ);
                            i += 3;
                        } else {
                            mode = Mode::Str { q: DQ, multi: syn.double_multi, esc: syn.esc };
                            i += 1;
                        }
                        continue;
                    }
                    if c == SQ {
                        match syn.single {
                            Single::No => i += 1,
                            Single::Char => i += char_literal(text, i).unwrap_or(1),
                            Single::Line | Single::Multi => {
                                opened = li;
                                if syn.triple && starts(b, i, &[SQ, SQ, SQ]) {
                                    mode = Mode::Triple(SQ);
                                    i += 3;
                                } else {
                                    // `$'...'` is the shell string that has escapes
                                    let esc = if syn.single_escape || (syn.heredoc == 2 && prev == Some(b'$')) { syn.esc } else { 0 };
                                    mode = Mode::Str { q: SQ, multi: syn.single == Single::Multi, esc };
                                    i += 1;
                                }
                            }
                        }
                        continue;
                    }
                    if c == b'`' && syn.backtick > 0 {
                        mode = if syn.backtick == 2 { Mode::Template } else { Mode::Str { q: b'`', multi: true, esc: 0 } };
                        opened = li;
                        i += 1;
                        continue;
                    }
                    if syn.regex && c == b'/' && regex_may_start(text[..i].trim_end().bytes().last()) {
                        let mut j = i + 1;
                        let mut class = false;
                        while j < b.len() {
                            match b[j] {
                                BS => j += 1,
                                b'[' => class = true,
                                b']' => class = false,
                                b'/' if !class => break,
                                _ => {}
                            }
                            j += 1;
                        }
                        i = if j < b.len() { j + 1 } else { i + 1 };
                        continue;
                    }
                    if syn.code_escape && c == syn.esc {
                        i += 2;
                        continue;
                    }
                    if c == b'{' {
                        *depth.last_mut().unwrap() += 1;
                    } else if c == b'}' {
                        let d = depth.last_mut().unwrap();
                        *d -= 1;
                        if *d == 0 {
                            if depth.len() == 1 {
                                return Ok((li, i + 1));
                            }
                            depth.pop();
                            mode = Mode::Template;
                        }
                    }
                    i += 1;
                }
                Mode::Str { q, esc, .. } => {
                    if esc != 0 && c == esc {
                        i += 2;
                    } else {
                        if c == q {
                            mode = Mode::Code;
                        }
                        i += 1;
                    }
                }
                Mode::Triple(q) => {
                    if c == BS {
                        i += 2;
                    } else if starts(b, i, &[q, q, q]) {
                        mode = Mode::Code;
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                Mode::Block { open, close, nest, depth: d } => {
                    if nest && starts(b, i, open) {
                        mode = Mode::Block { open, close, nest, depth: d + 1 };
                        i += open.len();
                    } else if starts(b, i, close) {
                        mode = if d == 1 { Mode::Code } else { Mode::Block { open, close, nest, depth: d - 1 } };
                        i += close.len();
                    } else {
                        i += 1;
                    }
                }
                Mode::Long(level) => {
                    if c == b']' && b[i + 1..].iter().take(level).filter(|&&x| x == b'=').count() == level && b.get(i + 1 + level) == Some(&b']') {
                        mode = Mode::Code;
                        i += level + 2;
                    } else {
                        i += 1;
                    }
                }
                Mode::RustRaw(hashes) => {
                    if c == DQ && b[i + 1..].iter().take(hashes).filter(|&&x| x == b'#').count() == hashes {
                        mode = Mode::Code;
                        i += 1 + hashes;
                    } else {
                        i += 1;
                    }
                }
                Mode::Template => {
                    if c == BS {
                        i += 2;
                    } else if c == b'`' {
                        mode = Mode::Code;
                        i += 1;
                    } else if starts(b, i, b"${") {
                        depth.push(1);
                        mode = Mode::Code;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Mode::HereDoc(_) | Mode::PsHere(_) | Mode::Doc(_) => break,
            }
        }
        // a one-line string ends with its line unless the line ends in an escape (continuation)
        if let Mode::Str { multi: false, .. } = mode {
            if i <= b.len() {
                mode = Mode::Code;
            }
        }
        if mode == Mode::Code && !pending.is_empty() {
            mode = Mode::HereDoc(pending.remove(0));
        }
        li += 1;
    }
    match mode {
        Mode::Code => Err((line, "")),
        m => Err((opened, m.what())),
    }
}

/// Collect a block body starting at `v.get(line)[col..]` (just after the opening `{`): `(raw body,
/// index of the closing line, byte column after the closing brace)`, or the compile error.
fn collect_body(v: &View, header: usize, lang: &str, line: usize, col: usize) -> Result<(String, usize, usize), String> {
    let (close_line, close_col) = scan_body(v, line, col, &syntax_for(lang)).map_err(|(at, what)| {
        if what.is_empty() {
            format!("Line {}: @{} block is never closed: no closing brace matches its opening brace", header + 1, lang)
        } else {
            format!("Line {}: @{} block is never closed: the {} opened on line {} runs to the end of the file", header + 1, lang, what, at + 1)
        }
    })?;
    let mut body = String::new();
    for li in line..=close_line {
        let text = v.get(li);
        let from = if li == line { col } else { 0 };
        if li == close_line {
            body.push_str(&text[from..close_col - 1]);
        } else {
            body.push_str(&text[from..]);
            body.push('\n');
        }
    }
    Ok((body, close_line, close_col))
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
fn parse_header(seg: &str, at: usize, lines: &View, line: usize, allow_next_line_brace: bool) -> Option<(String, Vec<String>, usize, usize, usize)> {
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
    if t.is_empty() && allow_next_line_brace && line + 1 < lines.len() && lines.get(line + 1).trim() == "{" {
        let next = lines.get(line + 1);
        let col = next.find('{').unwrap() + 1;
        return Some((lang, args, 1, col, 0));
    }
    None
}

/// Find the first block header in `seg` (Killer text, lexed with `carry`) and build its replacement.
fn find_block(seg: &str, prefix_out: &str, line_start: bool, carry: &mut Carry, lines: &View, line: usize) -> Result<Option<Hit>, String> {
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
            return Ok(None);
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
                    {
                        let (raw, close_line, close_col) = collect_body(lines, line, &lang, body_line, col)?;
                        let body = dedent(&raw);
                        let call = if args.is_empty() {
                            format!("polyglot_exec(\"{}\", \"{}\")", lang, encode(&body))
                        } else {
                            format!("polyglot_exec_args(\"{}\", \"{}\", \"{}\", {})", lang, encode(&body), args.join(","), args.join(", "))
                        };
                        let replacement = if statement { format!("print({})", call) } else { call };
                        return Ok(Some(Hit {
                            at: i,
                            replacement,
                            consumed: close_line - line,
                            remainder: lines.get(close_line)[close_col..].to_string(),
                        }));
                    }
                }
            }
        }
        k += 1;
    }
    Ok(None)
}

/// Replace every `@lang { ... }` block in `source` (see the module docs). The output has exactly as
/// many lines as the input. A block that never closes is an error (`Line N: ...`, N 1-based).
pub fn extract(source: &str) -> Result<String, String> {
    if !has_header_candidate(source) {
        return Ok(source.to_string());
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
                find_block(&seg, &cur, line_start, &mut carry, &View { lines: &lines, at: base, seg: &seg }, base)?
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
    Ok(out.join("\n"))
}

/// The source lines with line `at` read as `seg` (the not yet scanned rest of that line), without
/// copying the line vector (one copy per scanned line made the pass quadratic).
struct View<'a> {
    lines: &'a [&'a str],
    at: usize,
    seg: &'a str,
}

impl<'a> View<'a> {
    fn get(&self, i: usize) -> &'a str {
        if i == self.at { self.seg } else { self.lines[i] }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

/// Can `source` hold a block header at all: an `@` not preceded by an identifier character (so an
/// e-mail address does not count) and followed by one?
fn has_header_candidate(source: &str) -> bool {
    let b = source.as_bytes();
    b.iter().enumerate().any(|(i, &c)| {
        c == b'@'
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'@'))
            && b.get(i + 1).map_or(false, |&n| n.is_ascii_alphabetic() || n == b'_' || n >= 0x80)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_of(src: &str) -> String {
        let out = extract(src).unwrap();
        let start = out.find(BLOCK_PREFIX).expect("no block found");
        let hex: String = out[start..].chars().take_while(|c| *c != '"').collect();
        decode(&hex).into_owned()
    }

    #[test]
    fn keeps_line_count_and_indentation() {
        let src = "x = @python {\n    for i in range(2):\n        print(i // 1)  # c\n}\nprintln(x)\n";
        let out = extract(src).unwrap();
        assert_eq!(out.matches('\n').count(), src.matches('\n').count());
        assert_eq!(body_of(src), "for i in range(2):\n    print(i // 1)  # c");
        assert!(out.ends_with("\n\n\n\nprintln(x)\n"), "{out}");
    }

    #[test]
    fn statement_assignment_and_expression_forms() {
        assert!(extract("@bash { echo 1 }").unwrap().starts_with("print(polyglot_exec(\"bash\""));
        assert!(extract("y = @bash { echo 1 }").unwrap().starts_with("y = polyglot_exec(\"bash\""));
        let e = extract("println(@python { print(5) })").unwrap();
        assert!(e.starts_with("println(polyglot_exec(\"python\"") && e.ends_with("\"))"), "{e}");
        // an unknown name in expression position is left alone
        assert_eq!(extract("f(@zz { 1 })").unwrap(), "f(@zz { 1 })");
    }

    #[test]
    fn strings_and_comments_hide_headers() {
        let s = "s = \"a@python { x }\"\n# @python { y }\n-- @python { z }";
        assert_eq!(extract(s).unwrap(), s);
        let t = "t = \"\"\"\n@python { q }\n\"\"\"";
        assert_eq!(extract(t).unwrap(), t);
    }

    #[test]
    fn args_header_and_next_line_brace() {
        let e = extract("r = @python(name, n) { print(name) }").unwrap();
        assert!(e.starts_with("r = polyglot_exec_args(\"python\", \"") && e.ends_with("\"name,n\", name, n)"), "{e}");
        let src = "r = @python\n{\n    print(1)\n}\nz = 1";
        let out = extract(src).unwrap();
        assert_eq!(out.lines().count(), src.lines().count());
        assert_eq!(body_of(src), "print(1)");
    }

    #[test]
    fn crlf_is_removed_and_decode_round_trips() {
        let src = "x = @python {\r\n    print(\"a\\\\b\")\r\n}\r\n";
        assert_eq!(body_of(src), "print(\"a\\\\b\")");
        assert_eq!(decode("plain"), "plain");
    }

    /// The decoded body of the only block in `src` and the Killer text after it.
    fn split(src: &str) -> (String, String) {
        let out = extract(src).unwrap();
        assert_eq!(out.matches('\n').count(), src.matches('\n').count(), "{out}");
        let start = out.find(BLOCK_PREFIX).unwrap_or_else(|| panic!("no block found in {out}"));
        let hex: String = out[start..].chars().take_while(|c| *c != '"').collect();
        let rest = out[start + hex.len()..].trim_start_matches(|c| c == '"' || c == ')').trim().to_string();
        (decode(&hex).into_owned(), rest)
    }

    fn check(src: &str, body: &str) {
        let (b, rest) = split(src);
        assert_eq!(b, body, "body of {src}");
        assert_eq!(rest, "println(x)", "text after the block in {src}");
    }

    #[test]
    fn a_stray_quote_in_a_string_or_comment_does_not_hide_the_closing_brace() {
        check(r#"x = @python {
    print('a "quote')
}
println(x)"#, r#"print('a "quote')"#);
        check(r#"x = @bash {
    echo 'say "hi'
}
println(x)"#, r#"echo 'say "hi'"#);
        check(r#"x = @python {
    # the "x
    print(1)
}
println(x)"#, "# the \"x\nprint(1)");
        check(r#"x = @python {
    # don't } close
    print("it's", 'q"}', """a "b
    } c""")
}
println(x)"#, "# don't } close\nprint(\"it's\", 'q\"}', \"\"\"a \"b\n} c\"\"\")");
        check(r#"x = @bash {
    # don't "panic }
    echo "it's ${HOME}" '}' $'a\'"}'
}
println(x)"#, "# don't \"panic }\necho \"it's ${HOME}\" '}' $'a\\'\"}'");
        check(r#"x = @ruby {
    puts 'it''s "x }' # don't
}
println(x)"#, "puts 'it''s \"x }' # don't");
    }

    #[test]
    fn js_template_literals_comments_and_regexes() {
        let body = "const o = {a: 1};\nconsole.log(`v=${ {b: o.a}.b } \"q } ${'}'}`);\n// don't \"worry }\n/* } \" */\nconst r = /\"}/;";
        let src = format!("x = @node {{\n    {}\n}}\nprintln(x)", body.replace('\n', "\n    "));
        check(&src, body);
    }

    #[test]
    fn heredocs_here_strings_and_long_strings() {
        check("x = @bash {\n    cat <<EOF\nit's a \"test\" }\nEOF\n    echo done\n}\nprintln(x)", "    cat <<EOF\nit's a \"test\" }\nEOF\n    echo done");
        check("x = @powershell {\n    $s = @\"\nit's \"quoted\" }\n\"@\n    Write-Output $s\n}\nprintln(x)", "    $s = @\"\nit's \"quoted\" }\n\"@\n    Write-Output $s");
        check("x = @lua {\n    -- don't \"\n    print([[ } \" ]])\n}\nprintln(x)", "-- don't \"\nprint([[ } \" ]])");
        check("x = @go {\n    s := `\"}`\n    r := '\"'\n}\nprintln(x)", "s := `\"}`\nr := '\"'");
    }

    #[test]
    fn char_literals_lifetimes_and_raw_strings() {
        check("x = @rust {\n    fn f<'a>(s: &'a str) -> char { '}' }\n    let q = '\"'; let r = r#\"}\"#;\n}\nprintln(x)", "fn f<'a>(s: &'a str) -> char { '}' }\nlet q = '\"'; let r = r#\"}\"#;");
        check("x = @c {\n    char c = '{'; puts(\"}\"); /* } */\n}\nprintln(x)", "char c = '{'; puts(\"}\"); /* } */");
        check("x = @cpp {\n    int n = 1'000; char c = '\"';\n}\nprintln(x)", "int n = 1'000; char c = '\"';");
    }

    #[test]
    fn an_unknown_language_still_closes_after_a_stray_quote() {
        check("x = @zzlang {\n    say 'a \"b\n}\nprintln(x)", "say 'a \"b");
    }

    #[test]
    fn an_unclosed_block_is_an_error_naming_its_line() {
        let e = extract("x = @python {\n    print(1)\n").unwrap_err();
        assert!(e.starts_with("Line 1: @python block is never closed"), "{e}");
        let e = extract("println(1)\ny = @python {\n    s = \"\"\"never\n}\n").unwrap_err();
        assert!(e.starts_with("Line 2: ") && e.contains("triple-quoted string opened on line 3"), "{e}");
        let e = extract("a = 1\nprintln(@bash { echo 'x }\n").unwrap_err();
        assert!(e.starts_with("Line 2: @bash block is never closed") && e.contains("string opened on line 2"), "{e}");
    }
}
