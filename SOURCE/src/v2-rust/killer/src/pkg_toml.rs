//! A small TOML subset for `killer.toml` and `killer.lock` (no external crates).
//!
//! Supported: `# comments`, `[table]` and `[a.b]` headers, `[[array.of.tables]]`, bare or quoted
//! keys, strings (basic `"..."` with escapes, literal `'...'`), integers, booleans, arrays (which
//! may span lines) and inline tables `{ k = "v", ... }`. Not supported: floats, dates, multi-line
//! strings, dotted keys on the left of `=`.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    Array(Vec<Value>),
    Table(Table),
}

/// An insertion-ordered table.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Table {
    pub entries: Vec<(String, Value)>,
}

impl Table {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Value::Str(s)) => Some(s),
            _ => None,
        }
    }
    pub fn get_table(&self, key: &str) -> Option<&Table> {
        match self.get(key) {
            Some(Value::Table(t)) => Some(t),
            _ => None,
        }
    }
    pub fn get_array(&self, key: &str) -> Option<&Vec<Value>> {
        match self.get(key) {
            Some(Value::Array(a)) => Some(a),
            _ => None,
        }
    }
    pub fn set(&mut self, key: &str, v: Value) {
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.1 = v;
        } else {
            self.entries.push((key.to_string(), v));
        }
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    _src: &'a str,
}

type R<T> = Result<T, String>;

pub fn parse(src: &str) -> R<Table> {
    let mut p = Parser { chars: src.chars().collect(), pos: 0, line: 1, _src: src };
    let mut root = Table::default();
    // path of the table currently being filled; `true` when its last segment is an array of tables
    let mut current: Vec<String> = Vec::new();
    loop {
        p.skip_ws_and_comments(true);
        if p.eof() {
            break;
        }
        if p.peek() == Some('[') {
            p.pos += 1;
            let is_arr = p.peek() == Some('[');
            if is_arr {
                p.pos += 1;
            }
            let mut path = Vec::new();
            loop {
                p.skip_inline_ws();
                path.push(p.key()?);
                p.skip_inline_ws();
                if p.peek() == Some('.') {
                    p.pos += 1;
                } else {
                    break;
                }
            }
            if p.peek() != Some(']') {
                return Err(p.err("expected ']'"));
            }
            p.pos += 1;
            if is_arr {
                if p.peek() != Some(']') {
                    return Err(p.err("expected ']]'"));
                }
                p.pos += 1;
            }
            p.end_of_line()?;
            if is_arr {
                let parent = walk(&mut root, &path[..path.len() - 1]).map_err(|e| p.err(&e))?;
                let last = path.last().unwrap();
                match parent.entries.iter_mut().find(|(k, _)| k == last) {
                    Some((_, Value::Array(a))) => a.push(Value::Table(Table::default())),
                    Some(_) => return Err(p.err(&format!("'{}' is not an array of tables", last))),
                    None => parent.entries.push((last.clone(), Value::Array(vec![Value::Table(Table::default())]))),
                }
            } else {
                walk(&mut root, &path).map_err(|e| p.err(&e))?;
            }
            current = path;
            continue;
        }
        let key = p.key()?;
        p.skip_inline_ws();
        if p.peek() != Some('=') {
            return Err(p.err(&format!("expected '=' after key '{}'", key)));
        }
        p.pos += 1;
        p.skip_inline_ws();
        let v = p.value()?;
        p.end_of_line()?;
        let t = walk(&mut root, &current).map_err(|e| p.err(&e))?;
        if t.get(&key).is_some() {
            return Err(p.err(&format!("duplicate key '{}'", key)));
        }
        t.entries.push((key, v));
    }
    Ok(root)
}

/// Descend to the table at `path`, creating it; array-of-tables segments go to their last element.
fn walk<'t>(mut t: &'t mut Table, path: &[String]) -> Result<&'t mut Table, String> {
    for seg in path {
        let idx = match t.entries.iter().position(|(k, _)| k == seg) {
            Some(i) => i,
            None => {
                t.entries.push((seg.clone(), Value::Table(Table::default())));
                t.entries.len() - 1
            }
        };
        t = match &mut t.entries[idx].1 {
            Value::Table(inner) => inner,
            Value::Array(a) => match a.last_mut() {
                Some(Value::Table(inner)) => inner,
                _ => return Err(format!("'{}' is not a table", seg)),
            },
            _ => return Err(format!("'{}' is not a table", seg)),
        };
    }
    Ok(t)
}

impl<'a> Parser<'a> {
    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn err(&self, msg: &str) -> String {
        format!("line {}: {}", self.line, msg)
    }
    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }
    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.pos += 1;
            }
        }
    }
    fn skip_ws_and_comments(&mut self, newlines: bool) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => self.pos += 1,
                Some('\n') if newlines => {
                    self.pos += 1;
                    self.line += 1;
                }
                Some('#') => self.skip_comment(),
                _ => break,
            }
        }
    }
    fn end_of_line(&mut self) -> R<()> {
        self.skip_inline_ws();
        self.skip_comment();
        match self.peek() {
            None => Ok(()),
            Some('\r') => {
                self.pos += 1;
                self.end_of_line()
            }
            Some('\n') => {
                self.pos += 1;
                self.line += 1;
                Ok(())
            }
            Some(c) => Err(self.err(&format!("unexpected '{}'", c))),
        }
    }
    fn key(&mut self) -> R<String> {
        match self.peek() {
            Some('"') | Some('\'') => self.string(),
            _ => {
                let start = self.pos;
                while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_' || c == '-') {
                    self.pos += 1;
                }
                if start == self.pos {
                    return Err(self.err("expected a key"));
                }
                Ok(self.chars[start..self.pos].iter().collect())
            }
        }
    }
    fn string(&mut self) -> R<String> {
        let q = self.peek().unwrap();
        self.pos += 1;
        let mut s = String::new();
        loop {
            let Some(c) = self.peek() else { return Err(self.err("unterminated string")) };
            self.pos += 1;
            if c == q {
                return Ok(s);
            }
            if c == '\n' {
                return Err(self.err("unterminated string"));
            }
            if c == '\\' && q == '"' {
                let Some(e) = self.peek() else { return Err(self.err("unterminated string")) };
                self.pos += 1;
                match e {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    'u' => {
                        let hex: String = self.chars[self.pos..(self.pos + 4).min(self.chars.len())].iter().collect();
                        self.pos += 4;
                        let ch = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).ok_or_else(|| self.err("bad \\u escape"))?;
                        s.push(ch);
                    }
                    other => return Err(self.err(&format!("unknown escape '\\{}'", other))),
                }
            } else {
                s.push(c);
            }
        }
    }
    fn value(&mut self) -> R<Value> {
        match self.peek() {
            Some('"') | Some('\'') => Ok(Value::Str(self.string()?)),
            Some('[') => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws_and_comments(true);
                    if self.peek() == Some(']') {
                        self.pos += 1;
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value()?);
                    self.skip_ws_and_comments(true);
                    match self.peek() {
                        Some(',') => self.pos += 1,
                        Some(']') => {}
                        _ => return Err(self.err("expected ',' or ']' in array")),
                    }
                }
            }
            Some('{') => {
                self.pos += 1;
                let mut t = Table::default();
                loop {
                    self.skip_inline_ws();
                    if self.peek() == Some('}') {
                        self.pos += 1;
                        return Ok(Value::Table(t));
                    }
                    let k = self.key()?;
                    self.skip_inline_ws();
                    if self.peek() != Some('=') {
                        return Err(self.err("expected '=' in inline table"));
                    }
                    self.pos += 1;
                    self.skip_inline_ws();
                    let v = self.value()?;
                    if t.get(&k).is_some() {
                        return Err(self.err(&format!("duplicate key '{}'", k)));
                    }
                    t.entries.push((k, v));
                    self.skip_inline_ws();
                    match self.peek() {
                        Some(',') => self.pos += 1,
                        Some('}') => {}
                        _ => return Err(self.err("expected ',' or '}' in inline table")),
                    }
                }
            }
            Some(_) => {
                let start = self.pos;
                while matches!(self.peek(), Some(c) if !c.is_whitespace() && c != ',' && c != ']' && c != '}' && c != '#') {
                    self.pos += 1;
                }
                let word: String = self.chars[start..self.pos].iter().collect();
                match word.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    w => w.replace('_', "").parse::<i64>().map(Value::Int).map_err(|_| self.err(&format!("unsupported value '{}' (strings must be quoted)", w))),
                }
            }
            None => Err(self.err("expected a value")),
        }
    }
}

pub fn quote(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn key_text(k: &str) -> String {
    if !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        k.to_string()
    } else {
        quote(k)
    }
}

fn inline(v: &Value) -> String {
    match v {
        Value::Str(s) => quote(s),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => format!("[{}]", a.iter().map(inline).collect::<Vec<_>>().join(", ")),
        Value::Table(t) => {
            if t.entries.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {} }}", t.entries.iter().map(|(k, v)| format!("{} = {}", key_text(k), inline(v))).collect::<Vec<_>>().join(", "))
            }
        }
    }
}

/// Serialize a table: scalars first, then sub-tables as `[headers]`, arrays of tables as `[[..]]`.
pub fn to_string(t: &Table) -> String {
    let mut out = String::new();
    write_table(&mut out, t, &mut Vec::new());
    out
}

fn is_array_of_tables(v: &Value) -> bool {
    matches!(v, Value::Array(a) if !a.is_empty() && a.iter().all(|x| matches!(x, Value::Table(_))))
}

fn write_table(out: &mut String, t: &Table, path: &mut Vec<String>) {
    for (k, v) in &t.entries {
        if !matches!(v, Value::Table(_)) && !is_array_of_tables(v) {
            let _ = writeln!(out, "{} = {}", key_text(k), inline(v));
        }
    }
    for (k, v) in &t.entries {
        match v {
            Value::Table(sub) => {
                path.push(key_text(k));
                if !out.is_empty() {
                    out.push('\n');
                }
                let _ = writeln!(out, "[{}]", path.join("."));
                write_table(out, sub, path);
                path.pop();
            }
            Value::Array(a) if is_array_of_tables(v) => {
                path.push(key_text(k));
                for item in a {
                    if let Value::Table(sub) = item {
                        if !out.is_empty() {
                            out.push('\n');
                        }
                        let _ = writeln!(out, "[[{}]]", path.join("."));
                        write_table(out, sub, path);
                    }
                }
                path.pop();
            }
            _ => {}
        }
    }
}

/// Line-based edit that keeps comments: set `key = value_text` inside `[section]`, creating the
/// section when it is missing. `value_text` must already be valid TOML.
pub fn set_in_section(text: &str, section: &str, key: &str, value_text: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let header = format!("[{}]", section);
    let new_line = format!("{} = {}", key_text(key), value_text);
    let start = lines.iter().position(|l| l.trim() == header);
    match start {
        None => {
            if lines.last().map_or(false, |l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(header);
            lines.push(new_line);
        }
        Some(s) => {
            let mut end = lines.len();
            for (i, l) in lines.iter().enumerate().skip(s + 1) {
                if l.trim_start().starts_with('[') {
                    end = i;
                    break;
                }
            }
            let mut found = None;
            for i in s + 1..end {
                if line_key(&lines[i]).as_deref() == Some(key) {
                    found = Some(i);
                    break;
                }
            }
            match found {
                Some(i) => lines[i] = new_line,
                None => {
                    let mut at = end;
                    while at > s + 1 && lines[at - 1].trim().is_empty() {
                        at -= 1;
                    }
                    lines.insert(at, new_line);
                }
            }
        }
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Remove `key` from `[section]`; returns the new text and whether anything was removed.
pub fn remove_from_section(text: &str, section: &str, key: &str) -> (String, bool) {
    let lines: Vec<&str> = text.lines().collect();
    let header = format!("[{}]", section);
    let mut out = Vec::new();
    let mut in_section = false;
    let mut removed = false;
    for l in lines {
        if l.trim_start().starts_with('[') {
            in_section = l.trim() == header;
        } else if in_section && line_key(l).as_deref() == Some(key) {
            removed = true;
            continue;
        }
        out.push(l);
    }
    let mut s = out.join("\n");
    s.push('\n');
    (s, removed)
}

fn line_key(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    Some(k.trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_shapes() {
        let t = parse("# top\n[package]\nname = \"app\" # c\nversion = '1.0.0'\n\n[dependencies]\nlibA = \"^1.2\"\nlibB = { path = \"../b\" }\nlibC = { git = \"https://x/y.git\", rev = \"abc\" }\n").unwrap();
        assert_eq!(t.get_table("package").unwrap().get_str("name"), Some("app"));
        let d = t.get_table("dependencies").unwrap();
        assert_eq!(d.get_str("libA"), Some("^1.2"));
        assert_eq!(d.get_table("libB").unwrap().get_str("path"), Some("../b"));
        assert_eq!(d.get_table("libC").unwrap().get_str("rev"), Some("abc"));
    }

    #[test]
    fn arrays_arrays_of_tables_and_round_trip() {
        let src = "version = 1\n\n[[package]]\nname = \"a\"\ndependencies = [\n  \"b 1.0.0\",\n  \"c 2.0.0\",\n]\n\n[[package]]\nname = \"b\"\nok = true\n";
        let t = parse(src).unwrap();
        let pk = t.get_array("package").unwrap();
        assert_eq!(pk.len(), 2);
        let again = parse(&to_string(&t)).unwrap();
        assert_eq!(t, again);
    }

    #[test]
    fn errors_name_the_line() {
        assert!(parse("[package\nname = 1").unwrap_err().contains("line 1"));
        assert!(parse("a = 1\nb = nope\n").unwrap_err().contains("line 2"));
        assert!(parse("a = 1\na = 2\n").unwrap_err().contains("duplicate"));
    }

    #[test]
    fn edit_helpers_keep_comments() {
        let src = "[package]\nname = \"x\"\n\n[dependencies]\n# deps\na = \"1.0.0\"\n";
        let s = set_in_section(src, "dependencies", "b", "\"^2\"");
        assert!(s.contains("# deps") && s.contains("b = \"^2\""));
        let s = set_in_section(&s, "dependencies", "a", "\"1.1.0\"");
        assert!(s.contains("a = \"1.1.0\"") && !s.contains("\"1.0.0\""));
        let (s, r) = remove_from_section(&s, "dependencies", "a");
        assert!(r && !s.contains("a = "));
        let s = set_in_section(&s, "dev-dependencies", "t", "\"*\"");
        assert!(parse(&s).unwrap().get_table("dev-dependencies").is_some());
    }
}
