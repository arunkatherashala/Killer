# Changelog

All notable changes to this repository are summarized here. The crate **version** for the main runtime is defined in `SOURCE/src/v2-rust/killer/Cargo.toml` (currently **1.0.0**).

## Unreleased

### Language development line (branch `feature/assistant-ki`)

Status: beta / pre-release. Summarised from `git log --oneline`; the measured numbers are in
`SOURCE/src/v2-rust/killer/parity/PARITY_REPORT.md` and `bench/RESULTS.md`.

**Parity audit.** Added a reproducible parity audit (`parity/run_parity.py`, 166 probe programs
written the way a Python/JS/Java programmer would write them). Before the fixes 76 of 166 passed
(46%); now 166 of 166 pass. 63 builtins that were registered but unreachable were wired up
(`f6e5f58`); the registered-but-not-implemented names were later implemented or removed (see "Stdlib" below; the dead list is empty).

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

**Superseded:** this paragraph used to say arbitrary-precision integers were missing; big integers (`123n`) now exist, see below. Plain 64-bit numbers still saturate on overflow.

### Seven merged workstreams (language semantics, tooling, packages, network and AI, VM performance, robustness, docs)

Everything below is from `git log 40336af..HEAD` (about 50 commits); measured numbers are in
`parity/PARITY_REPORT.md` and `bench/RESULTS.md`.

**Language**
- Closures share the variables they capture (one cell per call, decided before the body is compiled by `closure_scan.rs`); loop variables are captured per iteration, recursive closures work.
- `for x in ...` rebinds a loop variable that already exists (slot, parameter, catch variable or C-style counter).
- `finally` is covered by a test matrix on every exit path (return, break, continue, nested, generators, closures, overriding return/throw).
- `x // 2` is a compile error ("`//` starts a comment in Killer; use floor(a / b) for integer division"); four repo files that relied on the silent behaviour now use `floor`.
- Big integers: hand-written `bigint.rs` (Karatsuba multiplication, Knuth division, floor semantics), `123n` / `0xFFn` literals, `big`, `factorial`, `divmod`, `idiv`, `powmod`, `isqrt`, `bit_length`, mixed arithmetic rules, JSON and `sorted` support; cross-checked against Python on thousands of random cases.
- One-line blocks (`if c { stmt }`, `for`, `while`, `else`, `try`/`catch`), `for k, v in dict`, builtins as first-class values (`f = print`, `map(xs, str)`).
- `reversed(xs)` returns a copy instead of reversing in place; a function stored in an object field is callable like a method (`this.callback(x)`); `delete(d, 5)` works for number keys.

**Standard library**
- Implemented the registered-but-dead builtins that are useful: `format("{} is {:.2}", ...)`, `substr`, `time_now`, `to_float`, `to_int`, `to_string`, `type_of`, `parse_int`, `parse_float` and the snake_case file names; the unreachable `phone_*`, `mic_*`, `service_*` names were dropped from the registry.

**AI and network**
- Real HTTP server with Killer handlers: `serve`, `serve_once`, `route`, `routes_clear`, `server_stop`, `free_port`.
- Hardened HTTP client: URL, headers and body go to curl on stdin (secrets never on a command line), TLS verification on by default, `http://` only for loopback, header-injection checks, `http_request` with timeouts and retries with exponential backoff; LLM provider calls use the same path.
- Vector store (`vec_new/add/search/get/delete/len/clear/free/save/load`), `cosine`, `dot`, `norm`, `top_k`, `chunk_text`, `chunk_by_sentence`, `count_tokens` (documented approximation), `build_rag_prompt`, `mock_embed`, `llm_embed_batch`, `llm_embed(opts, text)`, `log_debug/info/warn/error` (`KILLER_LOG`, `KILLER_LOG_FORMAT=json`), `load_env`, and `examples/rag_demo.killer`.

**Tooling**
- `killer` driver: `run`, `fmt`, `lint`, `test`, `repl`, `lsp`, `--version`.
- Formatter (behaviour-preserving, idempotent), linter with real rules (`undefined-function`, `unused-variable`, `unreachable-code`, `shadowed-builtin`, `floor-division-comment`, `# lint:ignore`), test framework (`test "name" { }`, `fn test_*`, `assert_eq` diffs, `assert_throws`, JUnit XML).
- Language server `killer_lsp`, VS Code extension (`editors/vscode`), Debug Adapter Protocol server `killer_dap` (breakpoints, stepping, variables; one stack frame).

**Packages**
- `killer.toml` manifest, `killer.lock` with SHA-256, semver requirements, backtracking resolver; `killer_pkg new/init/add/remove/install/update/list/publish/check`; file-backed registry (`kpm-registry`, `file://` registries); compile-time `import name` through packages; a missing import is a compile error.

**Safety**
- Call depth limited to 100,000 frames (catchable error), 400 levels of Rust re-entry, 256 MB strings, 16M-element lists, 32M `range`, nesting limit 200, 1,000 operators per expression, bounded JSON output; deterministic fuzz harness (`tests/robustness.rs`).
- No `Rc` is shared between OS threads: every value crossing a task boundary is deep-copied; pooled spawn VMs are reset; task threads get a 128 MB stack.
- Capability audit: every builtin touching files, network, processes, native code or models is gated (`tests/capabilities.rs`); `io_port_*`/`interrupts_*` no longer execute privileged instructions; temp files for `@python`/`@c`/... blocks use private per-run directories.

**Performance** (CPU seconds, same benchmark before/after, `bench/RESULTS.md` round 2)
- List comprehensions in frame slots (-19%), literal-key dict reads without cloning (-22% nested_data), pooled scope tables and FNV object fields (obj_methods -18%, recursion_objects -36%), `for ch in s` splits once, JSON fast paths (no measurable change at benchmark size).

**Documentation and CI**
- Language reference, tutorial (14 lessons), generated stdlib index, contributor architecture guide; `docs/check_examples.py` runs every example and `docs/check_links.py` checks relative links, both in CI.
- Eight more real-world programs in `parity/realworld/` (matrix, mini language, text wrapping, LRU cache, events, heap, dates, test runner).

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
