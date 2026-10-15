//! `parse_csv(text[, delimiter])` and `to_csv(rows[, delimiter])` following RFC 4180.
//!
//! * Quoted fields may contain the delimiter, doubled quotes and line breaks.
//! * `\r\n`, `\n` and a final line without a newline are all accepted; a UTF-8 byte order mark is
//!   dropped; blank lines are skipped.
//! * `parse_csv` returns a list of dicts whose keys are in header order (insertion ordered).
//!   Unquoted fields are trimmed, quoted fields are kept exactly. Missing trailing fields are
//!   `""`, extra fields are ignored, an unterminated quote is an error naming the line.
//! * `to_csv` takes a list of dicts (columns in first-seen order across all rows, so a later row
//!   may add a column) or a list of lists (no header). Strings are quoted when they contain the
//!   delimiter, a quote, a line break or leading/trailing spaces; numbers, bools and bigints are
//!   written as text, null as an empty field.

use crate::error::VmError;
use crate::value::{SharedDict, Value};

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn delimiter_arg(args: &[Value], who: &str) -> Result<char, VmError> {
    match args.get(1) {
        None | Some(Value::Null) => Ok(','),
        Some(Value::Str(s)) if s.chars().count() == 1 => Ok(s.chars().next().unwrap()),
        _ => err(format!("{}(): delimiter must be a single character", who)),
    }
}

/// Split CSV text into records of fields. Quoted fields are not trimmed.
fn records(text: &str, delim: char) -> Result<Vec<Vec<String>>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false; // the current field was (or is being) quoted
    let mut in_quotes = false;
    let mut line = 1usize;
    let mut quote_line = 1usize;
    let mut chars = text.chars().peekable();
    fn finish(field: &mut String, quoted: &mut bool, row: &mut Vec<String>) {
        let v = if *quoted { std::mem::take(field) } else { std::mem::take(field).trim().to_string() };
        *quoted = false;
        row.push(v);
    }
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                if c == '\n' {
                    line += 1;
                }
                field.push(c);
            }
            continue;
        }
        if c == '"' && field.trim().is_empty() && !quoted {
            field.clear();
            quoted = true;
            in_quotes = true;
            quote_line = line;
        } else if c == delim {
            finish(&mut field, &mut quoted, &mut row);
        } else if c == '\r' || c == '\n' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            line += 1;
            finish(&mut field, &mut quoted, &mut row);
            let blank = row.len() == 1 && row[0].is_empty();
            if blank {
                row.clear();
            } else {
                out.push(std::mem::take(&mut row));
            }
        } else {
            field.push(c);
        }
    }
    if in_quotes {
        return Err(format!("unterminated quoted field starting on line {}", quote_line));
    }
    if !field.is_empty() || quoted || !row.is_empty() {
        finish(&mut field, &mut quoted, &mut row);
        let blank = row.len() == 1 && row[0].is_empty();
        if !blank {
            out.push(row);
        }
    }
    Ok(out)
}

pub fn parse_csv(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() || args.len() > 2 {
        return err("parse_csv() expects 1-2 arguments (csv_string, [delimiter])");
    }
    let text = match &args[0] {
        Value::Str(s) => s,
        _ => return err("parse_csv(): first argument must be a CSV string"),
    };
    let delim = delimiter_arg(args, "parse_csv")?;
    let recs = records(text, delim).map_err(|e| VmError::runtime_error(format!("parse_csv(): {}", e)))?;
    let mut it = recs.into_iter();
    let headers = match it.next() {
        Some(h) => h,
        None => return Ok(Value::from(Vec::<Value>::new())),
    };
    let mut rows = Vec::new();
    for rec in it {
        let mut pairs: Vec<(String, Value)> = Vec::with_capacity(headers.len());
        for (i, h) in headers.iter().enumerate() {
            pairs.push((h.clone(), Value::Str(rec.get(i).cloned().unwrap_or_default())));
        }
        rows.push(Value::Dict(SharedDict::from_pairs(pairs)));
    }
    Ok(Value::from(rows))
}

fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => format!("{}", other),
    }
}

fn quote(field: &str, delim: char) -> String {
    let needs = field.contains(delim)
        || field.contains('"')
        || field.contains('\n')
        || field.contains('\r')
        || field.starts_with(' ')
        || field.ends_with(' ');
    if needs {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn to_csv(args: &[Value]) -> Result<Value, VmError> {
    if args.is_empty() || args.len() > 2 {
        return err("to_csv() expects 1-2 arguments (rows, [delimiter])");
    }
    let delim = delimiter_arg(args, "to_csv")?;
    let rows = match &args[0] {
        Value::Array(a) => a.to_vec(),
        _ => return err("to_csv(): first argument must be a list of dicts or a list of lists"),
    };
    let sep = delim.to_string();
    let mut out = String::new();
    if !rows.is_empty() && rows.iter().all(|r| matches!(r, Value::Array(_))) {
        for r in &rows {
            if let Value::Array(a) = r {
                let line: Vec<String> = a.to_vec().iter().map(|v| quote(&cell(v), delim)).collect();
                out.push_str(&line.join(&sep));
                out.push('\n');
            }
        }
        return Ok(Value::Str(out));
    }
    let mut headers: Vec<String> = Vec::new();
    for r in &rows {
        match r {
            Value::Dict(d) => {
                for (k, _) in d.iter() {
                    if !headers.contains(&k) {
                        headers.push(k);
                    }
                }
            }
            other => return err(format!("to_csv(): every row must be a dict (or every row a list), got {}", other.type_name())),
        }
    }
    if headers.is_empty() {
        return Ok(Value::Str(out));
    }
    out.push_str(&headers.iter().map(|h| quote(h, delim)).collect::<Vec<_>>().join(&sep));
    out.push('\n');
    for r in &rows {
        if let Value::Dict(d) = r {
            let line: Vec<String> = headers.iter().map(|h| quote(&d.get(h).map(|v| cell(&v)).unwrap_or_default(), delim)).collect();
            out.push_str(&line.join(&sep));
            out.push('\n');
        }
    }
    Ok(Value::Str(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_quoted_fields_and_crlf() {
        let r = records("a,b\r\n\"x\ny\",2\r\n3,4", ',').unwrap();
        assert_eq!(r, vec![vec!["a", "b"], vec!["x\ny", "2"], vec!["3", "4"]]);
    }

    #[test]
    fn quoted_keep_spaces_unquoted_trim() {
        let r = records("a,b\n\" x \", y \n", ',').unwrap();
        assert_eq!(r[1], vec![" x ", "y"]);
    }

    #[test]
    fn unterminated_quote_is_an_error() {
        assert!(records("a\n\"oops\n", ',').unwrap_err().contains("line 2"));
    }

    #[test]
    fn blank_lines_and_bom() {
        let r = records("\u{feff}a,b\n\n1,2\n\n", ',').unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0][0], "a");
    }
}
