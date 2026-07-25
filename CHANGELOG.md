# Changelog

All notable changes to this repository are summarized here. The crate **version** for the main runtime is defined in `SOURCE/src/v2-rust/killer/Cargo.toml` (currently **1.0.0**).

## Unreleased

### Runtime (`killer-native`)

- **Bytecode**: Added `TailCall { target, arg_count }` for self tail-calls (constant stack depth for direct recursion).
- **Compiler**: When compiling a named `fn`, tail positions that `return f(...)` or end with `f(...)` where `f` is the current function emit `TailCall` instead of `Call` + `Ret` (subset pipeline / `compile_fn_definition` and `compile_statements` tail path).
- **Optimizer**: Dead-store elimination for `StoreSlot(s)` when slot `s` is never read in the fragment (replaced with `Pop` to preserve stack depth).
- **VM**: Optional `ExecutionBudget` — instruction step limit per `run`, wall-clock limit checked on **backward** `Jump` edges (loop back-edges).
- **Security**: `SecurityConfig` adds `max_execution_ms` (default `0` = unused) for policy mapping to VM budgets.
- **Release profile**: `lto = "fat"`, `panic = "abort"`.

### Docs

- Root `README.md` and this `CHANGELOG.md` added for discoverability.
