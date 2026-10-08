# Changelog

All notable changes to this repository are summarized here. The crate **version** for the main runtime is defined in `SOURCE/src/v2-rust/killer/Cargo.toml` (currently **1.0.0**).

## Unreleased

### Language development line (branch `feature/assistant-ki`)

Status: beta / pre-release. Summarised from `git log --oneline`; the measured numbers are in
`SOURCE/src/v2-rust/killer/parity/PARITY_REPORT.md` and `bench/RESULTS.md`.

**Parity audit.** Added a reproducible parity audit (`parity/run_parity.py`, 166 probe programs
written the way a Python/JS/Java programmer would write them). Before the fixes 76 of 166 passed
(46%); now 166 of 166 pass. 63 builtins that were registered but unreachable were wired up
(`f6e5f58`); about fifty more remain registered but not implemented (`parity/REGISTERED_BUT_DEAD.txt`).

**Language features added**
- First-class functions, globals, reliable `return`; default arguments and inherited constructors
- Lambdas, arrow functions, closures, working decorators
- `try` / `catch` / `finally` / `throw`; `finally` runs when `return`, `break` or `continue` leaves the try
- `match`, `switch`, `do-while`, C-style `for`, destructuring (`a, b = ...` and `[a, b] = ...`)
- Classes: `super`, `instanceof`, `static` methods, method chaining, operator overloads (`__add__`), `toString`
- Variadic parameters, keyword arguments, dict and set comprehensions, mutex builtins
- Lazy generators (`yield`) implemented as coroutines
- Expression precedence fixes; ternary, `in`, chained comparisons, bitwise operators, short-circuit `&&`, `||`, `??`
- String indexing and slicing, negative indices, f-strings (including single-quoted strings inside braces), Python/JS-style string, array and dict methods
- A real JSON implementation and compile-time file imports; `readFile` / `writeFile` use the real builtins
- Dictionaries keep insertion order and `for k in dict` walks the keys; sets are shared mutable values with set operators
- `async_spawn` / `spawn` with globals, classes, arguments and errors; `chan_recv` no longer blocks other channel operations
- Uncertainty-aware arithmetic (`believe x = 10 ± 1`, `gauss`), three-valued comparisons, Kleene logic for `&&`/`||`/`!` on trits, `T_POS`/`T_ZERO`/`T_NEG` as plain names
- Assistant runtime and type-annotation checking

**Diagnostics.** Runtime and parse errors report the original file and line through every rewriting
pass (`3a35b6c`), covered by end-to-end tests (`0c6c87b`).

**Performance.** VM fast paths for calls, closures, method dispatch, stores and string indexing,
plus a non-numeric Killer-vs-CPython suite (`python bench/run.py`). Measured geometric-mean
time ratio vs CPython on that suite went from 2.14 to 1.81 (above 1 means slower than CPython; see
`bench/RESULTS.md` for per-benchmark ratios and caveats).

**Tests and fixes.** Ten real-world programs with reviewed expected output; regression tests for
`range`, for-loops, string append and shared state; GC data-loss and dict aliasing fixes;
Unix JIT build fixed by adding `libc`; subprocess capture test made portable.

**Release engineering and documentation (this change set)**
- Removed unsupported "production ready" and unmeasured performance claims from documentation; one version (Cargo.toml, 1.0.0)
- CI runs `cargo test --tests` on Linux, Windows and macOS, plus the parity audit and documentation example checks; benchmark smoke job (non-gating)
- Tag-triggered release workflow (linux-x64, windows-x64, macos-x64, macos-arm64, SHA256 sums) and `install.sh` / `install.ps1`
- The x86-64 JIT no longer allocates executable pages on other architectures (aarch64 falls back to the interpreter)
- New `docs/LANGUAGE.md`, `docs/TUTORIAL.md`, generated `docs/STDLIB_INDEX.md`; every example is executed in CI

**Not present:** arbitrary-precision integers (64-bit integers saturate on overflow).

### Earlier unreleased notes

### Runtime (`killer-native`)

- **Bytecode**: Added `TailCall { target, arg_count }` for self tail-calls (constant stack depth for direct recursion).
- **Compiler**: When compiling a named `fn`, tail positions that `return f(...)` or end with `f(...)` where `f` is the current function emit `TailCall` instead of `Call` + `Ret` (subset pipeline / `compile_fn_definition` and `compile_statements` tail path).
- **Optimizer**: Dead-store elimination for `StoreSlot(s)` when slot `s` is never read in the fragment (replaced with `Pop` to preserve stack depth).
- **VM**: Optional `ExecutionBudget` — instruction step limit per `run`, wall-clock limit checked on **backward** `Jump` edges (loop back-edges).
- **Security**: `SecurityConfig` adds `max_execution_ms` (default `0` = unused) for policy mapping to VM budgets.
- **Release profile**: `lto = "fat"`, `panic = "abort"`.

### Docs

- Root `README.md` and this `CHANGELOG.md` added for discoverability.
