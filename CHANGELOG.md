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
- `killer run` and `killer test` resolve `import` relative to the script's folder and name the script in error messages (they used the current directory and `<script>`).
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

### Waves 2 and 3 (streaming, profiler, scoping, real applications, one remainder rule, dunder methods, hardening, platforms)

Everything below was merged after the section above; counts and timings come from the work that
produced each item (`parity/PARITY_REPORT.md`, `bench/RESULTS.md`, `docs/PLATFORMS.md`).

**Language**
- Python-style scope: a plain assignment is function-scoped (file-scoped at top level), so a variable first assigned inside an `if`/`else`/loop/`try` body is visible after the block; `let` stays block-scoped. A name never assigned on the path taken reads as `null`.
- `%` is **floor modulo everywhere** (`-7 % 3` is `2` for numbers and big integers). `rem(a, b)` gives the C-style truncated remainder and `mod(a, b)` is the floor version. Constant folding, the interpreter and the native function JIT agree (579 Python-generated cases in `tests/modulo.rs`).
- One truthiness rule for the whole language (`bool(0n)` is false like `0`); `5 in [5n]` and `index_of` compare numbers and big integers exactly; `int("123...")` and `json_parse` of integers above 2^53 give big integers.
- One-line definitions (`fn f() { return 1 }`, `class A { fn m() { return 7 } }`), a dict literal directly after `in` (`for k, v in {"a": 1} {`).
- Every registered builtin is a first-class value (`f = vec_new`, `map(xs, mock_embed)`): non-core builtins get a wrapper that forwards the arguments actually passed.
- Dunder methods: `__len__`, `__getitem__`, `__setitem__` (non-string keys), `__contains__`, `__iter__` / `__next__`, `__pow__`, `__neg__`, `__mod__`, `__call__`, `__bool__`, `__str__`, with clear errors when a class lacks the method.
- Operator overloading gaps closed (`3 * v`, `-v`, `v % k`, comparisons from `__lt__` alone); `return` inside `try` no longer leaves a stale handler; a bare `return` pushes `null`; `1e-7` parses; `print(a, b)` works; `eprintln`, `stdout_write`, `isDir`, `isFile`, `list()`, lossless CSV (`csv_columns`, multi-line quoted fields) and generators accepted by `sum`, `sorted`, `min`, `max`, `join`.

**AI and network**
- Streaming LLM output: `llm_stream(opts, prompt)` returns a lazy generator of text chunks (OpenAI-compatible SSE, Anthropic SSE, Ollama NDJSON, plus an offline `mock` provider), `llm_stream_cb`, `gen_close`; a server handler may return a generator or array and the server sends `text/event-stream`. Incremental SSE parser in `src/sse.rs`.
- HTTP server limits: read and per-request deadlines (408 / 503), `server_stop()` works from any thread; a handler error returns a generic JSON 500 (the real error goes to the log; `opts.debug: true` includes it); routes registered before `serve` are visible to a server started in a spawned task.
- Profiler: `killer profile script.killer [--html out.html]`, `killer_super --profile`, builtins `profile_start/stop/report`; text table by self time plus a self-contained HTML flame graph.

**Performance**
- Variable lookup inside a function searches only that call's scopes and then the globals (O(1)); reading a global at recursion depth 20,000 went from about 1,560 ms to about 15 ms. A function no longer sees its caller's local variables.
- `len(s)` and `s[i]` no longer copy the string on load; character scans use a vectorised lead-byte count (non-ASCII indexing about 1.5 to 2x faster). Indexing a very long ASCII string is still linear per call (see the open items in `parity/PARITY_REPORT.md`).
- Garbage collector marks dicts, objects and closure captures once (a 30,000-term expression program went from over 300 s to 9 s in debug); list comprehensions, scope tables, FNV object fields and literal dict keys: see `bench/RESULTS.md` rounds 2 and 3.

**Safety and robustness**
- Comparison, copy, GC and display of deeply nested or cyclic values are iterative or depth-limited; a 100k-deep structure no longer overflows the native stack.
- Linear-time compiler passes for else-if chains, multi-line literals, lambda lifting and class bodies; operator limits per statement; a fuzz-found infinite loop on an unterminated string in a chained comparison is a compile error; long fuzz runs (20,000 cases, several seeds) are clean.

**Real-application trial.** Five applications under `examples/apps/` (todo REST service, document Q&A with RAG, CSV tool, text-adventure engine, expression interpreter), each with `killer.toml`, modules and `test "..." { }` files, run by `tests/apps.rs`; 22 friction points found while writing them are logged in `examples/apps/FRICTION.md` and fixed.

**Platforms.** The library, binaries and tests type-check for x86_64 and aarch64 Linux (gnu), x86_64 Linux (musl), x86_64 and aarch64 macOS and Windows; the unix `mmap` flags use `libc` constants and macOS retries the JIT mapping with `MAP_JIT`. Only Windows x86-64 has actually been run; CI has a cross-check job. Test fixtures under `examples/apps`, `tests/data` and `parity/realworld` are pinned to LF in `.gitattributes`. See `docs/PLATFORMS.md`.

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
