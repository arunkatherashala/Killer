//! Network and AI building blocks: HTTP server (`http_serve`), HTTP client extras, dense vector
//! store, chunking/tokens, embeddings helpers and logging. This file only dispatches; the
//! implementations live in their own modules. Hooked like `lang_builtins`: `call` returns `None`
//! for names it does not own, and `is_builtin` lets the compiler emit `CallBuiltin` for them.

use crate::error::VmError;
use crate::value::Value;

pub fn is_builtin(name: &str) -> bool {
    crate::http_serve::NAMES.contains(&name)
        || crate::llm_stream::NAMES.contains(&name)
        || crate::vm_profile::NAMES.contains(&name)
        || name == "http_request"
        || crate::vec_builtins::NAMES.contains(&name)
        || crate::rag_builtins::NAMES.contains(&name)
        || crate::embed_builtins::NAMES.contains(&name)
        || crate::log_builtins::NAMES.contains(&name)
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if name == "http_request" {
        return Some(crate::http_req::builtin_http_request(args));
    }
    crate::http_serve::call(name, args)
        .or_else(|| crate::vec_builtins::call(name, args))
        .or_else(|| crate::rag_builtins::call(name, args))
        .or_else(|| crate::embed_builtins::call(name, args))
        .or_else(|| crate::log_builtins::call(name, args))
}
