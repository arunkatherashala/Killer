//! Python/JS-style text helpers (`capitalize`, `title`, `find`, `count`, `zfill`, `isdigit`, ...)
//! and "method call falls back to the builtin of the same name" dispatch.
//!
//! `recv.name(args)` on a string, array or dict that the VM has no dedicated method for becomes
//! `name(recv, args)` (after mapping Python spellings such as `startswith` to the builtin
//! `starts_with`), so every builtin that takes the value as its first argument is also available
//! in method form.

use crate::builtin::BuiltinFunctions;
use crate::error::VmError;
use crate::value::Value;

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn arity<'a>(who: &str, args: &'a [Value], min: usize, max: usize) -> Result<&'a [Value], VmError> {
    if args.len() < min || args.len() > max {
        return err(format!("{}() expects {} to {} arguments, got {}", who, min, max, args.len()));
    }
    Ok(args)
}

fn string<'a>(v: &'a Value, who: &str) -> Result<&'a str, VmError> {
    match v {
        Value::Str(s) => Ok(s.as_str()),
        other => err(format!("{}() expects a string, got {}", who, other.type_name())),
    }
}

fn width_arg(v: &Value, who: &str) -> Result<usize, VmError> {
    match v {
        Value::Number(n) if n.is_finite() => Ok(n.max(0.0) as usize),
        other => err(format!("{}() width must be a number, got {}", who, other.type_name())),
    }
}

/// Character index of byte offset `byte` in `s`.
fn char_index(s: &str, byte: usize) -> usize {
    s[..byte].chars().count()
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_letter = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_letter {
                out.extend(c.to_lowercase());
            } else {
                out.extend(c.to_uppercase());
            }
            prev_letter = true;
        } else {
            out.push(c);
            prev_letter = false;
        }
    }
    out
}

/// Builtins implemented here. `None` means "not mine".
pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        "capitalize" => (|| {
            let a = arity(name, args, 1, 1)?;
            let s = string(&a[0], name)?;
            let mut chars = s.chars();
            Ok(Value::Str(match chars.next() {
                Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
                None => String::new(),
            }))
        })(),
        "title" => (|| {
            let a = arity(name, args, 1, 1)?;
            Ok(Value::Str(title_case(string(&a[0], name)?)))
        })(),
        "swapcase" => (|| {
            let a = arity(name, args, 1, 1)?;
            let s = string(&a[0], name)?;
            Ok(Value::Str(
                s.chars()
                    .flat_map(|c| -> Vec<char> {
                        if c.is_uppercase() {
                            c.to_lowercase().collect()
                        } else {
                            c.to_uppercase().collect()
                        }
                    })
                    .collect(),
            ))
        })(),
        "center" => (|| {
            let a = arity(name, args, 2, 3)?;
            let s = string(&a[0], name)?;
            let width = width_arg(&a[1], name)?;
            let fill = match a.get(2) {
                Some(Value::Str(f)) if f.chars().count() == 1 => f.chars().next().unwrap(),
                None => ' ',
                _ => return err("center() fill must be a single character"),
            };
            let len = s.chars().count();
            if width <= len {
                return Ok(Value::Str(s.to_string()));
            }
            // same split as CPython: the odd padding character goes on the left when width is odd
            let pad = width - len;
            let left = pad / 2 + (pad & width & 1);
            let mut out: String = std::iter::repeat(fill).take(left).collect();
            out.push_str(s);
            out.extend(std::iter::repeat(fill).take(pad - left));
            Ok(Value::Str(out))
        })(),
        "zfill" => (|| {
            let a = arity(name, args, 2, 2)?;
            let s = string(&a[0], name)?;
            let width = width_arg(&a[1], name)?;
            let len = s.chars().count();
            if width <= len {
                return Ok(Value::Str(s.to_string()));
            }
            let zeros: String = std::iter::repeat('0').take(width - len).collect();
            Ok(Value::Str(match s.chars().next() {
                Some(sign @ ('+' | '-')) => format!("{}{}{}", sign, zeros, &s[1..]),
                _ => format!("{}{}", zeros, s),
            }))
        })(),
        "isdigit" | "isalpha" | "isalnum" | "isspace" | "isupper" | "islower" => (|| {
            let a = arity(name, args, 1, 1)?;
            let s = string(&a[0], name)?;
            let nonempty = !s.is_empty();
            Ok(Value::Bool(match name {
                "isdigit" => nonempty && s.chars().all(|c| c.is_numeric()),
                "isalpha" => nonempty && s.chars().all(|c| c.is_alphabetic()),
                "isalnum" => nonempty && s.chars().all(|c| c.is_alphanumeric()),
                "isspace" => nonempty && s.chars().all(|c| c.is_whitespace()),
                // at least one cased character and none of the opposite case
                "isupper" => s.chars().any(|c| c.is_uppercase()) && !s.chars().any(|c| c.is_lowercase()),
                _ => s.chars().any(|c| c.is_lowercase()) && !s.chars().any(|c| c.is_uppercase()),
            }))
        })(),
        "splitlines" => (|| {
            let a = arity(name, args, 1, 1)?;
            let s = string(&a[0], name)?;
            Ok(Value::from(s.lines().map(|l| Value::Str(l.to_string())).collect::<Vec<_>>()))
        })(),
        "find" | "rfind" => (|| {
            let a = arity(name, args, 2, 2)?;
            let (s, sub) = (string(&a[0], name)?, string(&a[1], name)?);
            let found = if name == "find" { s.find(sub) } else { s.rfind(sub) };
            Ok(Value::Number(found.map_or(-1.0, |b| char_index(s, b) as f64)))
        })(),
        "count" => (|| {
            let a = arity(name, args, 2, 2)?;
            match (&a[0], &a[1]) {
                (Value::Str(s), Value::Str(sub)) => Ok(Value::Number(if sub.is_empty() {
                    (s.chars().count() + 1) as f64
                } else {
                    s.matches(sub.as_str()).count() as f64
                })),
                (Value::Array(arr), item) => Ok(Value::Number(arr.iter_cloned().filter(|x| x == item).count() as f64)),
                _ => err("count() expects (string, substring) or (array, item)"),
            }
        })(),
        _ => return None,
    })
}

/// The builtin that implements method `method` of `recv` (Python/JS spellings included).
fn builtin_for_method<'a>(recv: &Value, method: &'a str) -> &'a str {
    if matches!(recv, Value::Set(_)) {
        match method {
            "add" => return "set_add",
            "remove" | "discard" => return "set_remove",
            "contains" | "has" => return "set_has",
            "union" => return "set_union",
            "intersection" => return "set_intersection",
            "difference" => return "set_difference",
            "clear" => return "set_clear",
            "size" => return "set_size",
            "to_array" | "toArray" => return "set_to_array",
            _ => {}
        }
    }
    match method {
        "startswith" | "startsWith" => "starts_with",
        "endswith" | "endsWith" => "ends_with",
        "ljust" => "pad_right",
        "rjust" => "pad_left",
        "index" => "index_of",
        "has" | "has_key" | "contains_key" => "contains",
        "items" => "entries",
        "append" => "push",
        other => other,
    }
}

/// Run `recv.method(args)` as a builtin call with `recv` as the first argument. `None` when no
/// such builtin exists (the caller then reports the usual "cannot call method" error).
pub fn call_as_method(recv: &Value, method: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    // `sep.join(items)`: the receiver is the separator, the builtin takes the items first
    if method == "join" {
        if let (Value::Str(_), [Value::Array(_)]) = (recv, args) {
            return Some(BuiltinFunctions::call("join", &[args[0].clone(), recv.clone()]));
        }
    }
    // `s.split()` splits on runs of whitespace
    if method == "split" && args.is_empty() {
        if let Value::Str(s) = recv {
            return Some(Ok(Value::from(
                s.split_whitespace().map(|w| Value::Str(w.to_string())).collect::<Vec<_>>(),
            )));
        }
    }
    let name = builtin_for_method(recv, method);
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push(recv.clone());
    full.extend_from_slice(args);
    match BuiltinFunctions::call(name, &full) {
        Err(e) if e.to_string().contains("unknown function") => None,
        other => Some(other),
    }
}
