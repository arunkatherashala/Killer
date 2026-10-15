//! `list(x)` and generator materialisation.
//!
//! `list(x)` returns a new list: a copy of a list, the characters of a string, the keys of a
//! dict, the members of a set, or every value a generator yields. Builtins that consume a whole
//! sequence (`sum`, `sorted`, `min`, `max`, `join`, `reverse`, `zip`, `enumerate`, `any`, `all`,
//! `set`, `map`, `filter`, `reduce`) also accept a generator: the VM drains it into a list first
//! (see [`MATERIALIZE`]); `len` deliberately does not, because that would silently consume it.

use crate::error::VmError;
use crate::value::Value;

pub const NAMES: &[&str] = &["list"];

/// Builtins whose generator arguments are replaced by the list of everything they yield.
pub const MATERIALIZE: &[&str] = &[
    "list", "sum", "sorted", "min", "max", "join", "reverse", "reversed", "zip", "enumerate", "any", "all", "set", "map", "filter",
    "reduce",
];

/// Most items `list(generator)` will collect before giving up (an endless generator).
pub const MAX_ITEMS: usize = 10_000_000;

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if name != "list" {
        return None;
    }
    Some(match args {
        [Value::Array(a)] => Ok(Value::from(a.to_vec())),
        [Value::Str(s)] => Ok(Value::from(s.chars().map(|c| Value::Str(c.to_string())).collect::<Vec<_>>())),
        [Value::Dict(d)] => Ok(Value::from(d.iter().map(|(k, _)| Value::Str(k)).collect::<Vec<_>>())),
        [set @ Value::Set(_)] => crate::builtin::BuiltinFunctions::call("set_to_array", std::slice::from_ref(set)),
        [Value::Generator(_)] => Err(VmError::runtime_error("list(): generator was not materialised".to_string())),
        [other] => Err(VmError::runtime_error(format!("list() cannot make a list from a {}", other.type_name()))),
        _ => Err(VmError::runtime_error("list(x) expects exactly one argument".to_string())),
    })
}
