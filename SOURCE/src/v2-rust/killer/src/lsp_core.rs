//! Language server for Killer: JSON-RPC over stdio with `Content-Length` framing.
//!
//! Built on the real compiler: diagnostics come from [`crate::compiler::compile_killer_subset`]
//! (errors already carry the user's original line numbers, see `sourcemap`), completion offers
//! the keywords, the compiler's builtin names and everything defined in the document, hover shows
//! builtin docs and user function signatures, go-to-definition finds user functions, classes and
//! top-level variables, and document symbols list them.
//!
//! The protocol layer ([`Server::handle`]) is transport independent: it takes one parsed message
//! and returns the encoded messages to send, so tests drive it in-process. [`serve`] and
//! [`serve_stdio`] add the framing. JSON is handled by [`crate::json_lang`].

use crate::json_lang;
use crate::value::{SharedDict, Value};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};

// ---------------------------------------------------------------------------------------------
// Language data

/// Reserved words of the language.
pub const KEYWORDS: &[&str] = &[
    "fn", "kfn", "akfn", "class", "extends", "static", "new", "this", "super", "if", "else", "elif",
    "while", "for", "in", "of", "do", "match", "switch", "case", "default", "try", "catch",
    "finally", "throw", "return", "break", "continue", "yield", "import", "global", "let", "async",
    "await", "spawn", "true", "false", "null", "and", "or", "not", "instanceof", "test",
];

/// Every builtin function name the compiler routes to `CallBuiltin`, read from the compiler's own
/// `BUILTINS` table so the two cannot drift apart.
pub fn builtin_names() -> Vec<&'static str> {
    const SRC: &str = include_str!("compiler.rs");
    let start = match SRC.find("const BUILTINS: &[&str]") {
        Some(i) => i,
        None => return Vec::new(),
    };
    let body = &SRC[start..];
    let end = body.find("\n    ];").unwrap_or(body.len());
    let body = &body[..end];
    let mut names: Vec<&'static str> = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find('"') {
        let after = &rest[i + 1..];
        match after.find('"') {
            Some(j) => {
                let name = &after[..j];
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') && !names.contains(&name) {
                    names.push(name);
                }
                rest = &after[j + 1..];
            }
            None => break,
        }
    }
    names
}

/// One-line documentation for the commonly used builtins.
pub fn builtin_doc(name: &str) -> Option<&'static str> {
    Some(match name {
        "print" => "print(value, ...) - write the values to standard output, followed by a newline",
        "println" => "println(value, ...) - write the values to standard output, followed by a newline",
        "len" | "length" => "len(x) - number of elements of an array, characters of a string, or keys of a dict",
        "push" => "push(array, value) - append a value to the end of an array",
        "pop" => "pop(array) - remove and return the last element of an array",
        "contains" => "contains(collection, item) - true if the array, string or set contains the item",
        "index_of" => "index_of(collection, item) - position of the item, or -1",
        "get" => "get(dict, key[, default]) - value for a key, or the default when missing",
        "keys" => "keys(dict) - array of the keys of a dict",
        "values" => "values(dict) - array of the values of a dict",
        "range" => "range(n) / range(start, end[, step]) - sequence of numbers",
        "type_of" => "type_of(x) - name of the type of a value",
        "to_string" | "str" => "str(x) - convert a value to its string form",
        "to_int" | "int" => "int(x) - convert a number or string to an integer",
        "to_float" | "float" => "float(x) - convert a number or string to a float",
        "sort" => "sort(array) - sort an array in place (ascending)",
        "reverse" => "reverse(array) - reverse an array in place",
        "reversed" => "reversed(x) - a reversed copy of an array or string",
        "join" => "join(array, separator) - join the elements into a string",
        "split" => "split(text, separator) - split a string into an array",
        "trim" => "trim(text) - remove leading and trailing whitespace",
        "replace" => "replace(text, from, to) - replace every occurrence of a substring",
        "upper" => "upper(text) - uppercase copy of a string",
        "lower" => "lower(text) - lowercase copy of a string",
        "starts_with" => "starts_with(text, prefix) - true if the text begins with the prefix",
        "ends_with" => "ends_with(text, suffix) - true if the text ends with the suffix",
        "substr" => "substr(text, start[, length]) - part of a string",
        "format" => "format(template, args...) - format a string",
        "parse_int" => "parse_int(text) - parse an integer from a string",
        "parse_float" => "parse_float(text) - parse a float from a string",
        "abs" => "abs(x) - absolute value",
        "floor" => "floor(x) - round down to an integer (use floor(a / b) for integer division)",
        "ceil" => "ceil(x) - round up to an integer",
        "round" => "round(x) - round to the nearest integer",
        "sqrt" => "sqrt(x) - square root",
        "pow" => "pow(base, exponent) - power",
        "min" => "min(a, b, ...) / min(array) - smallest value",
        "max" => "max(a, b, ...) / max(array) - largest value",
        "sum" => "sum(array) - total of the numbers in an array",
        "map" => "map(function, array) - apply a function to every element",
        "filter" => "filter(function, array) - keep the elements for which the function is true",
        "reduce" => "reduce(function, array[, initial]) - fold an array to one value",
        "assert" => "assert(condition[, message]) - fail when the condition is false",
        "assert_eq" => "assert_eq(actual, expected) - fail with a readable diff when the values differ",
        "assert_throws" => "assert_throws(function) - fail unless calling the function throws",
        "json_parse" => "json_parse(text) - parse JSON text into dicts, arrays and values",
        "json_stringify" => "json_stringify(value[, indent]) - convert a value to JSON text",
        "sleep" => "sleep(seconds) - pause the program",
        "exit" => "exit([code]) - end the program",
        "input" => "input([prompt]) - read a line from standard input",
        "regex_match" => "regex_match(text, pattern) - true if the pattern matches the text",
        "async_spawn" => "async_spawn(function, args...) - run a function on another thread; returns a future",
        "chan_new" => "chan_new() - create a channel",
        "chan_send" => "chan_send(channel, value) - send a value on a channel",
        "chan_recv" => "chan_recv(channel) - receive a value from a channel (blocks)",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------------------------
// Document analysis

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymKind {
    Function,
    Method,
    Class,
    Variable,
}

impl SymKind {
    fn completion_kind(self) -> i64 {
        match self {
            SymKind::Function => 3,
            SymKind::Method => 2,
            SymKind::Class => 7,
            SymKind::Variable => 6,
        }
    }
    fn symbol_kind(self) -> i64 {
        match self {
            SymKind::Function => 12,
            SymKind::Method => 6,
            SymKind::Class => 5,
            SymKind::Variable => 13,
        }
    }
}

/// A function, class, method or variable defined in a document.
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymKind,
    /// 0-based line of the definition.
    pub line: usize,
    /// 0-based character range of the name on that line.
    pub col: usize,
    /// Signature line, e.g. `fn add(a, b)` or `class Dog extends Animal`.
    pub detail: String,
    /// Enclosing class for methods.
    pub container: Option<String>,
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Strip strings and `#` / `//` / `--` comments from a line (replacing them by spaces so that
/// columns are preserved). Good enough for brace counting and symbol scanning.
fn mask_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '#' || (c == '/' && chars.get(i + 1) == Some(&'/')) || (c == '-' && chars.get(i + 1) == Some(&'-')) {
            while i < chars.len() {
                out.push(' ');
                i += 1;
            }
            break;
        }
        if c == '"' || c == '\'' {
            let q = c;
            out.push(' ');
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                out.push(' ');
                i += 1;
                if d == '\\' && i < chars.len() {
                    out.push(' ');
                    i += 1;
                } else if d == q {
                    break;
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Find the symbols defined in `text`.
pub fn scan_symbols(text: &str) -> Vec<Symbol> {
    let mut syms = Vec::new();
    let mut depth: i32 = 0;
    // (class name, brace depth inside the class body)
    let mut class_stack: Vec<(String, i32)> = Vec::new();
    for (ln, raw) in text.lines().enumerate() {
        let masked = mask_line(raw);
        let trimmed = masked.trim_start();
        let indent_cols = masked.chars().count() - trimmed.chars().count();
        let words: Vec<&str> = trimmed.split(|c: char| !is_ident_char(c)).filter(|w| !w.is_empty()).collect();
        let first = words.first().copied().unwrap_or("");
        let mut pending_class: Option<String> = None;
        let sig_end = raw.find('{').unwrap_or(raw.len());
        let signature = raw[..sig_end].trim().to_string();
        let find_name = |name: &str, after: usize| -> usize {
            // column of `name` on the masked line after the keyword at `after`
            masked.chars().skip(after).collect::<String>().find(name).map(|b| {
                masked.chars().skip(after).collect::<String>()[..b].chars().count() + after
            }).unwrap_or(indent_cols)
        };
        match first {
            "fn" | "kfn" | "akfn" => {
                if let Some(name) = words.get(1) {
                    let in_class = class_stack.last().filter(|(_, d)| *d == depth).map(|(n, _)| n.clone());
                    syms.push(Symbol {
                        name: name.to_string(),
                        kind: if in_class.is_some() { SymKind::Method } else { SymKind::Function },
                        line: ln,
                        col: find_name(name, indent_cols + first.chars().count()),
                        detail: signature.clone(),
                        container: in_class,
                    });
                }
            }
            "async" if words.get(1).map_or(false, |w| *w == "fn" || *w == "kfn") => {
                if let Some(name) = words.get(2) {
                    syms.push(Symbol {
                        name: name.to_string(),
                        kind: SymKind::Function,
                        line: ln,
                        col: find_name(name, indent_cols + 5),
                        detail: signature.clone(),
                        container: None,
                    });
                }
            }
            "class" => {
                if let Some(name) = words.get(1) {
                    syms.push(Symbol {
                        name: name.to_string(),
                        kind: SymKind::Class,
                        line: ln,
                        col: find_name(name, indent_cols + 5),
                        detail: signature.clone(),
                        container: None,
                    });
                    pending_class = Some(name.to_string());
                }
            }
            _ => {
                // variable assignment at brace depth 0: `x = ...` / `let x = ...` / `global x = ...`
                if depth == 0 {
                    let (rest, off) = match first {
                        "let" | "global" => {
                            let o = indent_cols + first.chars().count();
                            (masked.chars().skip(o).collect::<String>(), o)
                        }
                        _ => (trimmed.to_string(), indent_cols),
                    };
                    let rest_trim = rest.trim_start();
                    let lead = rest.chars().count() - rest_trim.chars().count();
                    let name: String = rest_trim.chars().take_while(|c| is_ident_char(*c)).collect();
                    if !name.is_empty() && name.chars().next().map_or(false, is_ident_start) && !KEYWORDS.contains(&name.as_str()) {
                        let after: String = rest_trim.chars().skip(name.chars().count()).collect();
                        let after = after.trim_start();
                        let assigns = after.starts_with('=') && !after.starts_with("==")
                            || ["+=", "-=", "*=", "/="].iter().any(|p| after.starts_with(p));
                        if assigns && !syms.iter().any(|s| s.kind == SymKind::Variable && s.name == name) {
                            syms.push(Symbol {
                                name: name.clone(),
                                kind: SymKind::Variable,
                                line: ln,
                                col: off + lead,
                                detail: raw.trim().to_string(),
                                container: None,
                            });
                        }
                    }
                }
            }
        }
        for c in masked.chars() {
            match c {
                '{' => {
                    depth += 1;
                    if let Some(n) = pending_class.take() {
                        class_stack.push((n, depth));
                    }
                }
                '}' => {
                    if let Some((_, d)) = class_stack.last() {
                        if *d == depth {
                            class_stack.pop();
                        }
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
    }
    syms
}

/// The identifier under (line, character) of `text`.
pub fn word_at(text: &str, line: usize, character: usize) -> Option<String> {
    let l: Vec<char> = text.lines().nth(line)?.chars().collect();
    let mut i = character.min(l.len());
    if i == l.len() || !is_ident_char(l[i]) {
        if i > 0 && is_ident_char(l[i - 1]) {
            i -= 1;
        } else {
            return None;
        }
    }
    let mut s = i;
    while s > 0 && is_ident_char(l[s - 1]) {
        s -= 1;
    }
    let mut e = i;
    while e < l.len() && is_ident_char(l[e]) {
        e += 1;
    }
    Some(l[s..e].iter().collect())
}

/// A diagnostic: 0-based line plus message.
#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub line: usize,
    pub message: String,
    /// 1 error, 2 warning.
    pub severity: i64,
}

/// Compile `text` and report the error (if any) with the user's line number.
pub fn diagnose(text: &str) -> Vec<Diag> {
    let text_owned = text.to_string();
    let result = std::panic::catch_unwind(|| crate::compiler::compile_killer_subset(&text_owned).map(|_| ()));
    match result {
        Ok(Ok(())) => Vec::new(),
        Ok(Err(e)) => vec![error_to_diag(&e.to_string(), text)],
        Err(_) => vec![Diag { line: 0, message: "internal compiler error".to_string(), severity: 1 }],
    }
}

/// Turn a compile error string such as `Parse error: Line 2: Parse error: Line 3: msg` into a
/// diagnostic at the innermost reported line.
fn error_to_diag(raw: &str, text: &str) -> Diag {
    let nlines = text.lines().count().max(1);
    let mut msg = raw.trim().to_string();
    let mut line = 0usize;
    let mut other_file = false;
    // The innermost `Line N` wins; its message is what follows.
    let mut search_from = 0;
    let mut last: Option<(usize, usize, usize, bool)> = None; // (start, digits_end, n, of_file)
    while let Some(i) = msg[search_from..].find("Line ") {
        let start = search_from + i;
        let digits: String = msg[start + 5..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let dend = start + 5 + digits.len();
            let of_file = msg[dend..].starts_with(" of ");
            last = Some((start, dend, digits.parse().unwrap_or(1), of_file));
        }
        search_from = start + 5;
    }
    if let Some((start, dend, n, of_file)) = last {
        other_file = of_file;
        line = n.saturating_sub(1).min(nlines - 1);
        let mut tail = msg[dend..].to_string();
        if of_file {
            // keep "of <file>" in the message, it is not this document
            tail = tail.trim_start_matches(':').to_string();
            msg = format!("{}Line {}{}", &msg[..0], n, tail);
        } else {
            msg = tail.trim_start_matches(':').trim().to_string();
        }
        let _ = start;
    }
    // drop wrapper prefixes
    for p in ["Parse error: ", "Runtime error: ", "Error: "] {
        while msg.starts_with(p) {
            msg = msg[p.len()..].to_string();
        }
    }
    if other_file {
        line = 0;
    }
    Diag { line, message: msg.trim().to_string(), severity: 1 }
}

// ---------------------------------------------------------------------------------------------
// JSON helpers (on top of json_lang)

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Dict(SharedDict::from_pairs(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()))
}
fn arr(items: Vec<Value>) -> Value {
    Value::from(items)
}
fn s(text: &str) -> Value {
    Value::Str(text.to_string())
}
fn n(x: i64) -> Value {
    Value::Number(x as f64)
}
fn field(v: &Value, key: &str) -> Value {
    match v {
        Value::Dict(d) => d.get(key).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
fn path(v: &Value, keys: &[&str]) -> Value {
    let mut cur = v.clone();
    for k in keys {
        cur = field(&cur, k);
    }
    cur
}
fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::Str(t) => Some(t.clone()),
        _ => None,
    }
}
fn as_usize(v: &Value) -> Option<usize> {
    match v {
        Value::Number(x) if *x >= 0.0 => Some(*x as usize),
        Value::Integer(i) if *i >= 0 => Some(*i as usize),
        _ => None,
    }
}
fn encode(v: &Value) -> String {
    json_lang::stringify(v, 0).unwrap_or_else(|_| "null".to_string())
}
fn range_value(line: usize, c0: usize, c1: usize) -> Value {
    obj(vec![
        ("start", obj(vec![("line", n(line as i64)), ("character", n(c0 as i64))])),
        ("end", obj(vec![("line", n(line as i64)), ("character", n(c1 as i64))])),
    ])
}

// ---------------------------------------------------------------------------------------------
// The server

pub struct Server {
    docs: HashMap<String, String>,
    builtins: Vec<&'static str>,
    pub shutdown_requested: bool,
    pub exited: bool,
}

impl Default for Server {
    fn default() -> Self {
        Server::new()
    }
}

impl Server {
    pub fn new() -> Server {
        Server { docs: HashMap::new(), builtins: builtin_names(), shutdown_requested: false, exited: false }
    }

    fn response(id: &Value, result: Value) -> String {
        encode(&obj(vec![("jsonrpc", s("2.0")), ("id", id.clone()), ("result", result)]))
    }

    fn error_response(id: &Value, code: i64, message: &str) -> String {
        encode(&obj(vec![
            ("jsonrpc", s("2.0")),
            ("id", id.clone()),
            ("error", obj(vec![("code", n(code)), ("message", s(message))])),
        ]))
    }

    fn publish(&self, uri: &str) -> String {
        let diags = match self.docs.get(uri) {
            Some(text) => diagnose(text),
            None => Vec::new(),
        };
        self.publish_list(uri, &diags)
    }

    fn publish_list(&self, uri: &str, diags: &[Diag]) -> String {
        let items: Vec<Value> = diags
            .iter()
            .map(|d| {
                let width = self
                    .docs
                    .get(uri)
                    .and_then(|t| t.lines().nth(d.line).map(|l| l.chars().count()))
                    .unwrap_or(0);
                obj(vec![
                    ("range", range_value(d.line, 0, width)),
                    ("severity", n(d.severity)),
                    ("source", s("killer")),
                    ("message", s(&d.message)),
                ])
            })
            .collect();
        encode(&obj(vec![
            ("jsonrpc", s("2.0")),
            ("method", s("textDocument/publishDiagnostics")),
            ("params", obj(vec![("uri", s(uri)), ("diagnostics", arr(items))])),
        ]))
    }

    fn completions(&self, uri: &str) -> Value {
        let mut items: Vec<Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut add = |label: &str, kind: i64, detail: &str, items: &mut Vec<Value>| {
            if seen.insert(label.to_string()) {
                items.push(obj(vec![("label", s(label)), ("kind", n(kind)), ("detail", s(detail))]));
            }
        };
        if let Some(text) = self.docs.get(uri) {
            for sym in scan_symbols(text) {
                add(&sym.name, sym.kind.completion_kind(), &sym.detail, &mut items);
            }
        }
        for k in KEYWORDS {
            add(k, 14, "keyword", &mut items);
        }
        for b in &self.builtins {
            add(b, 3, builtin_doc(b).unwrap_or("builtin function"), &mut items);
        }
        obj(vec![("isIncomplete", Value::Bool(false)), ("items", arr(items))])
    }

    fn find_word(&self, params: &Value) -> Option<(String, String, usize, usize)> {
        let uri = as_str(&path(params, &["textDocument", "uri"]))?;
        let line = as_usize(&path(params, &["position", "line"]))?;
        let ch = as_usize(&path(params, &["position", "character"]))?;
        let text = self.docs.get(&uri)?;
        let w = word_at(text, line, ch)?;
        Some((uri, w, line, ch))
    }

    fn hover(&self, params: &Value) -> Value {
        let (uri, word, _, _) = match self.find_word(params) {
            Some(x) => x,
            None => return Value::Null,
        };
        let text = &self.docs[&uri];
        let syms = scan_symbols(text);
        let body = if let Some(sym) = syms.iter().find(|x| x.name == word && x.kind != SymKind::Variable) {
            format!("```killer\n{}\n```", sym.detail)
        } else if let Some(sym) = syms.iter().find(|x| x.name == word) {
            format!("```killer\n{}\n```\nvariable `{}`", sym.detail, sym.name)
        } else if let Some(d) = builtin_doc(&word) {
            format!("**{}** (builtin)\n\n{}", word, d)
        } else if self.builtins.contains(&word.as_str()) {
            format!("**{}** (builtin function)", word)
        } else if KEYWORDS.contains(&word.as_str()) {
            format!("`{}` (keyword)", word)
        } else {
            return Value::Null;
        };
        obj(vec![("contents", obj(vec![("kind", s("markdown")), ("value", s(&body))]))])
    }

    fn definition(&self, params: &Value) -> Value {
        let (uri, word, _, _) = match self.find_word(params) {
            Some(x) => x,
            None => return Value::Null,
        };
        let syms = scan_symbols(&self.docs[&uri]);
        let pick = syms
            .iter()
            .find(|x| x.name == word && x.kind != SymKind::Variable)
            .or_else(|| syms.iter().find(|x| x.name == word));
        match pick {
            Some(sym) => obj(vec![
                ("uri", s(&uri)),
                ("range", range_value(sym.line, sym.col, sym.col + sym.name.chars().count())),
            ]),
            None => Value::Null,
        }
    }

    fn document_symbols(&self, params: &Value) -> Value {
        let uri = match as_str(&path(params, &["textDocument", "uri"])) {
            Some(u) => u,
            None => return arr(vec![]),
        };
        let text = match self.docs.get(&uri) {
            Some(t) => t,
            None => return arr(vec![]),
        };
        let items = scan_symbols(text)
            .into_iter()
            .map(|sym| {
                let mut pairs = vec![
                    ("name", s(&sym.name)),
                    ("kind", n(sym.kind.symbol_kind())),
                    (
                        "location",
                        obj(vec![
                            ("uri", s(&uri)),
                            ("range", range_value(sym.line, sym.col, sym.col + sym.name.chars().count())),
                        ]),
                    ),
                ];
                if let Some(c) = &sym.container {
                    pairs.push(("containerName", s(c)));
                }
                obj(pairs)
            })
            .collect();
        arr(items)
    }

    /// Handle one decoded message; returns the encoded messages to send back.
    pub fn handle(&mut self, msg: &Value) -> Vec<String> {
        let method = as_str(&field(msg, "method")).unwrap_or_default();
        let id = field(msg, "id");
        let is_request = !matches!(id, Value::Null);
        let params = field(msg, "params");
        let mut out = Vec::new();
        match method.as_str() {
            "initialize" => {
                let caps = obj(vec![
                    ("textDocumentSync", obj(vec![("openClose", Value::Bool(true)), ("change", n(1))])),
                    ("completionProvider", obj(vec![("triggerCharacters", arr(vec![s(".")]))])),
                    ("hoverProvider", Value::Bool(true)),
                    ("definitionProvider", Value::Bool(true)),
                    ("documentSymbolProvider", Value::Bool(true)),
                ]);
                out.push(Self::response(
                    &id,
                    obj(vec![
                        ("capabilities", caps),
                        ("serverInfo", obj(vec![("name", s("killer-lsp")), ("version", s(env!("CARGO_PKG_VERSION")))])),
                    ]),
                ));
            }
            "initialized" | "$/cancelRequest" | "workspace/didChangeConfiguration" => {}
            "shutdown" => {
                self.shutdown_requested = true;
                out.push(Self::response(&id, Value::Null));
            }
            "exit" => self.exited = true,
            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    as_str(&path(&params, &["textDocument", "uri"])),
                    as_str(&path(&params, &["textDocument", "text"])),
                ) {
                    self.docs.insert(uri.clone(), text);
                    out.push(self.publish(&uri));
                }
            }
            "textDocument/didChange" => {
                if let Some(uri) = as_str(&path(&params, &["textDocument", "uri"])) {
                    if let Value::Array(changes) = field(&params, "contentChanges") {
                        // full-text sync: the last change holds the whole document
                        if let Some(text) = changes.to_vec().last().and_then(|c| as_str(&field(c, "text"))) {
                            self.docs.insert(uri.clone(), text);
                            out.push(self.publish(&uri));
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = as_str(&path(&params, &["textDocument", "uri"])) {
                    self.docs.remove(&uri);
                    out.push(self.publish_list(&uri, &[]));
                }
            }
            "textDocument/completion" => {
                let uri = as_str(&path(&params, &["textDocument", "uri"])).unwrap_or_default();
                out.push(Self::response(&id, self.completions(&uri)));
            }
            "textDocument/hover" => out.push(Self::response(&id, self.hover(&params))),
            "textDocument/definition" => out.push(Self::response(&id, self.definition(&params))),
            "textDocument/documentSymbol" => out.push(Self::response(&id, self.document_symbols(&params))),
            _ => {
                if is_request {
                    out.push(Self::error_response(&id, -32601, &format!("method not found: {method}")));
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Framing

/// Read one `Content-Length` framed message body; `None` at end of input.
pub fn read_message<R: BufRead>(r: &mut R) -> Option<String> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let t = line.trim();
        if t.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some(v) = t.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().ok();
        }
    }
    let mut buf = vec![0u8; len?];
    r.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// Write one framed message.
pub fn write_message<W: Write>(w: &mut W, body: &str) -> std::io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    w.flush()
}

/// Serve until `exit` or end of input. Returns the process exit code (0 after a clean shutdown).
pub fn serve<R: BufRead, W: Write>(mut input: R, mut output: W) -> i32 {
    let mut server = Server::new();
    while let Some(body) = read_message(&mut input) {
        let msg = match json_lang::parse(&body) {
            Ok(m) => m,
            Err(_) => {
                let _ = write_message(&mut output, &Server::error_response(&Value::Null, -32700, "parse error"));
                continue;
            }
        };
        for reply in server.handle(&msg) {
            if write_message(&mut output, &reply).is_err() {
                return 1;
            }
        }
        if server.exited {
            return if server.shutdown_requested { 0 } else { 1 };
        }
    }
    if server.shutdown_requested { 0 } else { 1 }
}

/// Serve over the process's stdin/stdout.
pub fn serve_stdio() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(stdin.lock(), stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(id: i64, method: &str, params: Value) -> Value {
        obj(vec![("jsonrpc", s("2.0")), ("id", n(id)), ("method", s(method)), ("params", params)])
    }
    fn note(method: &str, params: Value) -> Value {
        obj(vec![("jsonrpc", s("2.0")), ("method", s(method)), ("params", params)])
    }
    fn open(uri: &str, text: &str) -> Value {
        note(
            "textDocument/didOpen",
            obj(vec![("textDocument", obj(vec![("uri", s(uri)), ("languageId", s("killer")), ("version", n(1)), ("text", s(text))]))]),
        )
    }
    fn pos(uri: &str, line: i64, ch: i64) -> Value {
        obj(vec![("textDocument", obj(vec![("uri", s(uri))])), ("position", obj(vec![("line", n(line)), ("character", n(ch))]))])
    }
    fn parse(m: &str) -> Value {
        json_lang::parse(m).unwrap()
    }

    #[test]
    fn builtin_names_come_from_compiler() {
        let b = builtin_names();
        assert!(b.contains(&"print") && b.contains(&"json_parse") && b.len() > 200, "{}", b.len());
    }

    #[test]
    fn diagnostics_have_original_line() {
        let mut sv = Server::new();
        let out = sv.handle(&open("file:///a.killer", "x = 1\nfn f(a) {\n  return a +\n}\n"));
        let m = parse(&out[0]);
        assert_eq!(as_str(&field(&m, "method")).unwrap(), "textDocument/publishDiagnostics");
        let d = match path(&m, &["params", "diagnostics"]) {
            Value::Array(a) => a.to_vec(),
            _ => panic!(),
        };
        assert_eq!(d.len(), 1);
        assert_eq!(as_usize(&path(&d[0], &["range", "start", "line"])), Some(2));
    }

    #[test]
    fn clean_file_has_no_diagnostics() {
        let mut sv = Server::new();
        let out = sv.handle(&open("file:///a.killer", "print(1)\n"));
        let m = parse(&out[0]);
        match path(&m, &["params", "diagnostics"]) {
            Value::Array(a) => assert!(a.to_vec().is_empty()),
            _ => panic!(),
        }
    }

    #[test]
    fn symbols_and_definition() {
        let src = "class Animal {\n  fn speak() {\n    return 1\n  }\n}\nfn add(a, b) {\n  return a + b\n}\ntotal = add(1, 2)\n";
        let syms = scan_symbols(src);
        let names: Vec<_> = syms.iter().map(|x| (x.name.as_str(), x.kind)).collect();
        assert!(names.contains(&("Animal", SymKind::Class)));
        assert!(names.contains(&("speak", SymKind::Method)));
        assert!(names.contains(&("add", SymKind::Function)));
        assert!(names.contains(&("total", SymKind::Variable)));
        let mut sv = Server::new();
        sv.handle(&open("file:///a.killer", src));
        let r = parse(&sv.handle(&req(2, "textDocument/definition", pos("file:///a.killer", 8, 10)))[0]);
        assert_eq!(as_usize(&path(&r, &["result", "range", "start", "line"])), Some(5));
        let h = parse(&sv.handle(&req(3, "textDocument/hover", pos("file:///a.killer", 8, 10)))[0]);
        assert!(as_str(&path(&h, &["result", "contents", "value"])).unwrap().contains("fn add(a, b)"));
    }
}
