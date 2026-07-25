//! **Pipes** — Angular-style pipe transforms for template expressions.
//!
//! Built-in pipes: uppercase, lowercase, titlecase, date, number, currency,
//! percent, json, slice, async, keyvalue, truncate. Custom pipes via PipeRegistry.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Pipe Value
// ══════════════════════════════════════════════════════════════════════════════

/// Value flowing through a pipe.
#[derive(Debug, Clone, PartialEq)]
pub enum PipeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<PipeValue>),
    Map(Vec<(String, PipeValue)>),
}

impl PipeValue {
    pub fn as_str(&self) -> String {
        match self {
            PipeValue::Null => "".into(),
            PipeValue::Bool(b) => b.to_string(),
            PipeValue::Int(i) => i.to_string(),
            PipeValue::Float(f) => format!("{f}"),
            PipeValue::Str(s) => s.clone(),
            PipeValue::List(l) => format!("[{}]", l.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", ")),
            PipeValue::Map(m) => format!("{{{}}}", m.iter().map(|(k, v)| format!("{k}: {}", v.as_str())).collect::<Vec<_>>().join(", ")),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Built-in Pipes
// ══════════════════════════════════════════════════════════════════════════════

pub fn pipe_uppercase(val: &PipeValue, _args: &[PipeValue]) -> PipeValue {
    PipeValue::Str(val.as_str().to_uppercase())
}

pub fn pipe_lowercase(val: &PipeValue, _args: &[PipeValue]) -> PipeValue {
    PipeValue::Str(val.as_str().to_lowercase())
}

pub fn pipe_titlecase(val: &PipeValue, _args: &[PipeValue]) -> PipeValue {
    let s = val.as_str();
    let result: String = s.split_whitespace().map(|word| {
        let mut chars = word.chars();
        match chars.next() {
            Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str().to_lowercase()),
            None => String::new(),
        }
    }).collect::<Vec<_>>().join(" ");
    PipeValue::Str(result)
}

pub fn pipe_json(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let indent = args.first().and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None }).unwrap_or(0);
    let json = to_json(val, indent, 0);
    PipeValue::Str(json)
}

fn to_json(val: &PipeValue, indent: usize, depth: usize) -> String {
    let prefix = if indent > 0 { " ".repeat(indent * depth) } else { String::new() };
    let inner_prefix = if indent > 0 { " ".repeat(indent * (depth + 1)) } else { String::new() };
    let nl = if indent > 0 { "\n" } else { "" };
    match val {
        PipeValue::Null => "null".into(),
        PipeValue::Bool(b) => b.to_string(),
        PipeValue::Int(i) => i.to_string(),
        PipeValue::Float(f) => format!("{f}"),
        PipeValue::Str(s) => format!("\"{s}\""),
        PipeValue::List(items) => {
            if items.is_empty() { return "[]".into(); }
            let inner: Vec<String> = items.iter().map(|v| format!("{inner_prefix}{}", to_json(v, indent, depth + 1))).collect();
            format!("[{nl}{}{nl}{prefix}]", inner.join(&format!(",{nl}")))
        }
        PipeValue::Map(pairs) => {
            if pairs.is_empty() { return "{}".into(); }
            let inner: Vec<String> = pairs.iter().map(|(k, v)| format!("{inner_prefix}\"{k}\": {}", to_json(v, indent, depth + 1))).collect();
            format!("{{{nl}{}{nl}{prefix}}}", inner.join(&format!(",{nl}")))
        }
    }
}

/// Number formatting: `{{ val | number:'1.2-2' }}`.
pub fn pipe_number(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let num = match val {
        PipeValue::Int(i) => *i as f64,
        PipeValue::Float(f) => *f,
        _ => return val.clone(),
    };
    let decimals = args.first().and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None }).unwrap_or(0);
    PipeValue::Str(format!("{:.prec$}", num, prec = decimals))
}

/// Currency pipe: `{{ val | currency:'USD' }}`.
pub fn pipe_currency(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let num = match val {
        PipeValue::Int(i) => *i as f64,
        PipeValue::Float(f) => *f,
        _ => return val.clone(),
    };
    let symbol = args.first().map(|a| a.as_str()).unwrap_or_else(|| "$".into());
    PipeValue::Str(format!("{symbol}{num:.2}"))
}

/// Percent pipe: `{{ 0.25 | percent }}` → `25%`.
pub fn pipe_percent(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let num = match val {
        PipeValue::Int(i) => *i as f64,
        PipeValue::Float(f) => *f,
        _ => return val.clone(),
    };
    let decimals = args.first().and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None }).unwrap_or(0);
    PipeValue::Str(format!("{:.prec$}%", num * 100.0, prec = decimals))
}

/// Slice pipe: `{{ list | slice:1:3 }}`.
pub fn pipe_slice(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let start = args.first().and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None }).unwrap_or(0);
    let end = args.get(1).and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None });
    match val {
        PipeValue::Str(s) => {
            let e = end.unwrap_or(s.len()).min(s.len());
            let s_start = start.min(s.len());
            PipeValue::Str(s[s_start..e].into())
        }
        PipeValue::List(list) => {
            let e = end.unwrap_or(list.len()).min(list.len());
            let s_start = start.min(list.len());
            PipeValue::List(list[s_start..e].to_vec())
        }
        _ => val.clone(),
    }
}

/// Truncate pipe: `{{ text | truncate:50 }}`.
pub fn pipe_truncate(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let max_len = args.first().and_then(|a| if let PipeValue::Int(n) = a { Some(*n as usize) } else { None }).unwrap_or(100);
    let suffix = args.get(1).map(|a| a.as_str()).unwrap_or_else(|| "...".into());
    let s = val.as_str();
    if s.len() <= max_len {
        PipeValue::Str(s)
    } else {
        PipeValue::Str(format!("{}{suffix}", &s[..max_len]))
    }
}

/// KeyValue pipe: converts Map to List of {key, value} pairs.
pub fn pipe_keyvalue(val: &PipeValue, _args: &[PipeValue]) -> PipeValue {
    if let PipeValue::Map(pairs) = val {
        PipeValue::List(pairs.iter().map(|(k, v)| {
            PipeValue::Map(vec![
                ("key".into(), PipeValue::Str(k.clone())),
                ("value".into(), v.clone()),
            ])
        }).collect())
    } else {
        val.clone()
    }
}

/// Date pipe: `{{ timestamp | date:'YYYY-MM-DD' }}`.
pub fn pipe_date(val: &PipeValue, args: &[PipeValue]) -> PipeValue {
    let ts = match val {
        PipeValue::Int(i) => *i,
        _ => return val.clone(),
    };
    let _fmt = args.first().map(|a| a.as_str()).unwrap_or_else(|| "YYYY-MM-DD".into());
    // Simple timestamp to date (seconds since epoch, approximate)
    let days = ts / 86400;
    let mut y = 1970i64;
    let mut remaining = days;
    while remaining >= 365 {
        let leap = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining < leap { break; }
        remaining -= leap;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0;
    for md in &mdays {
        if remaining < *md { break; }
        remaining -= md;
        m += 1;
    }
    PipeValue::Str(format!("{y:04}-{:02}-{:02}", m + 1, remaining + 1))
}

// ══════════════════════════════════════════════════════════════════════════════
// Pipe Registry
// ══════════════════════════════════════════════════════════════════════════════

/// Pipe function signature.
pub type PipeFn = fn(&PipeValue, &[PipeValue]) -> PipeValue;

/// Registry of named pipes.
pub struct PipeRegistry {
    pipes: HashMap<String, PipeFn>,
}

impl PipeRegistry {
    pub fn new() -> Self {
        let mut reg = PipeRegistry { pipes: HashMap::new() };
        reg.register_builtins();
        reg
    }

    fn register_builtins(&mut self) {
        self.pipes.insert("uppercase".into(), pipe_uppercase);
        self.pipes.insert("lowercase".into(), pipe_lowercase);
        self.pipes.insert("titlecase".into(), pipe_titlecase);
        self.pipes.insert("json".into(), pipe_json);
        self.pipes.insert("number".into(), pipe_number);
        self.pipes.insert("currency".into(), pipe_currency);
        self.pipes.insert("percent".into(), pipe_percent);
        self.pipes.insert("slice".into(), pipe_slice);
        self.pipes.insert("truncate".into(), pipe_truncate);
        self.pipes.insert("keyvalue".into(), pipe_keyvalue);
        self.pipes.insert("date".into(), pipe_date);
    }

    /// Register a custom pipe.
    pub fn register(&mut self, name: &str, f: PipeFn) {
        self.pipes.insert(name.into(), f);
    }

    /// Apply pipe by name.
    pub fn apply(&self, name: &str, val: &PipeValue, args: &[PipeValue]) -> Option<PipeValue> {
        self.pipes.get(name).map(|f| f(val, args))
    }

    /// Chain multiple pipes: `val | pipe1 | pipe2:arg`.
    pub fn chain(&self, val: &PipeValue, pipes: &[(&str, Vec<PipeValue>)]) -> PipeValue {
        let mut current = val.clone();
        for (name, args) in pipes {
            if let Some(result) = self.apply(name, &current, args) {
                current = result;
            }
        }
        current
    }

    pub fn pipe_count(&self) -> usize { self.pipes.len() }
    pub fn has_pipe(&self, name: &str) -> bool { self.pipes.contains_key(name) }
}

impl Default for PipeRegistry {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase() {
        let r = pipe_uppercase(&PipeValue::Str("hello".into()), &[]);
        assert_eq!(r, PipeValue::Str("HELLO".into()));
    }

    #[test]
    fn lowercase() {
        let r = pipe_lowercase(&PipeValue::Str("HELLO".into()), &[]);
        assert_eq!(r, PipeValue::Str("hello".into()));
    }

    #[test]
    fn titlecase() {
        let r = pipe_titlecase(&PipeValue::Str("hello world".into()), &[]);
        assert_eq!(r, PipeValue::Str("Hello World".into()));
    }

    #[test]
    fn number_format() {
        let r = pipe_number(&PipeValue::Float(3.14159), &[PipeValue::Int(2)]);
        assert_eq!(r, PipeValue::Str("3.14".into()));
    }

    #[test]
    fn currency_format() {
        let r = pipe_currency(&PipeValue::Float(42.5), &[PipeValue::Str("€".into())]);
        assert_eq!(r, PipeValue::Str("€42.50".into()));
    }

    #[test]
    fn percent_format() {
        let r = pipe_percent(&PipeValue::Float(0.256), &[PipeValue::Int(1)]);
        assert_eq!(r, PipeValue::Str("25.6%".into()));
    }

    #[test]
    fn slice_string() {
        let r = pipe_slice(&PipeValue::Str("abcdef".into()), &[PipeValue::Int(1), PipeValue::Int(4)]);
        assert_eq!(r, PipeValue::Str("bcd".into()));
    }

    #[test]
    fn slice_list() {
        let list = PipeValue::List(vec![PipeValue::Int(10), PipeValue::Int(20), PipeValue::Int(30), PipeValue::Int(40)]);
        let r = pipe_slice(&list, &[PipeValue::Int(1), PipeValue::Int(3)]);
        assert_eq!(r, PipeValue::List(vec![PipeValue::Int(20), PipeValue::Int(30)]));
    }

    #[test]
    fn truncate() {
        let r = pipe_truncate(&PipeValue::Str("Hello World!".into()), &[PipeValue::Int(5)]);
        assert_eq!(r, PipeValue::Str("Hello...".into()));
    }

    #[test]
    fn json_pipe() {
        let val = PipeValue::Map(vec![("name".into(), PipeValue::Str("Alice".into()))]);
        let r = pipe_json(&val, &[]);
        assert_eq!(r, PipeValue::Str(r#"{"name": "Alice"}"#.into()));
    }

    #[test]
    fn keyvalue_pipe() {
        let val = PipeValue::Map(vec![("a".into(), PipeValue::Int(1)), ("b".into(), PipeValue::Int(2))]);
        let r = pipe_keyvalue(&val, &[]);
        if let PipeValue::List(items) = r {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn date_pipe() {
        // 2024-01-01 = 19723 days since epoch
        let r = pipe_date(&PipeValue::Int(1704067200), &[]);
        assert_eq!(r, PipeValue::Str("2024-01-01".into()));
    }

    #[test]
    fn registry_chain() {
        let reg = PipeRegistry::new();
        let val = PipeValue::Str("hello world".into());
        let result = reg.chain(&val, &[("titlecase", vec![]), ("uppercase", vec![])]);
        assert_eq!(result, PipeValue::Str("HELLO WORLD".into()));
    }

    #[test]
    fn custom_pipe() {
        let mut reg = PipeRegistry::new();
        fn reverse(val: &PipeValue, _: &[PipeValue]) -> PipeValue {
            PipeValue::Str(val.as_str().chars().rev().collect())
        }
        reg.register("reverse", reverse);
        assert!(reg.has_pipe("reverse"));
        let r = reg.apply("reverse", &PipeValue::Str("abc".into()), &[]);
        assert_eq!(r, Some(PipeValue::Str("cba".into())));
    }

    #[test]
    fn builtin_count() {
        let reg = PipeRegistry::new();
        assert_eq!(reg.pipe_count(), 11);
    }
}
