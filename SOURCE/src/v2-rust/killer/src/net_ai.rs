//! Network and AI building blocks: HTTP server (`http_serve`), HTTP client extras, dense vector
//! store, chunking/tokens, embeddings helpers and logging. This file only dispatches; the
//! implementations live in their own modules. Hooked like `lang_builtins`: `call` returns `None`
//! for names it does not own, and `is_builtin` lets the compiler emit `CallBuiltin` for them.

use crate::error::VmError;
use crate::value::Value;

pub fn is_builtin(name: &str) -> bool {
    crate::http_serve::NAMES.contains(&name) || name == "http_request"
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if let Some(r) = crate::http_serve::call(name, args) {
        return Some(r);
    }
    match name {
        "http_request" => Some(crate::http_req::builtin_http_request(args)),
        _ => None,
    }
}
