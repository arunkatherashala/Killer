# Killer feature status

One table of what Killer has and what it does not, kept in step with the code. Status values:
**yes** (built, and a test or checked example named in the Proof column exercises it), **partial** (works with a stated limit, or only partly tested), **untested** (code exists but no test or run proves it), **no** (not built). The Proof column names test files found by grep; they were not re-run for this page, so "yes" means "a named test exists", not "passes today". Stability of each area: [STABILITY.md](STABILITY.md); security: `SOURCE/docs/project/THREAT_MODEL.md`.
The row-by-row history is in the root `tracker.csv`; the Python comparison is in [PYTHON_COMPARISON.md](PYTHON_COMPARISON.md).

## Language

| Feature | Status | Notes | Proof |
|---|---|---|---|
| Variables, functions, closures, recursion | yes | f64 numbers, automatic big integers at 2^53 | tests/language_basics.rs, tests/bigint.rs, parity/probes.py (166 probes) |
| Classes, inheritance, `this`, `new` | yes | | tests/language_basics.rs, tests/python_gap.rs |
| Operator overloading (`__add__`, `__eq__`, ...) | yes | reflected forms too | tests/language_basics.rs (`__add__`), tests/python_gap.rs |
| Properties (`get` / `set`) | yes | | tests/python_gap.rs (`getters_and_setters_work_like_python_properties`) |
| `abstract`, `interface`, `implements` | yes | an interface is a parent class whose bodiless methods throw | tests/language_additions.rs |
| `enum` | yes | | tests/language_additions.rs |
| `const` | yes | checked at compile time | tests/language_additions.rs, tests/language_basics.rs, tests/language_semantics.rs |
| Private members | yes | `private fn`, `private a, b`; compile error outside the class | tests/language_additions.rs |
| Pattern matching: values, `\|`, guards, bindings | yes | | tests/language_basics.rs, parity/probes.py (166 probes) |
| Pattern matching: list patterns `[a, ...rest]` | yes | nested too | tests/language_basics.rs, tests/slots_diff.rs |
| Pattern matching: dict patterns `{name, age: n}` | yes | nested too | tests/language_additions.rs |
| Destructuring: lists, dicts | yes | `{a, b: c} = d` | tests/language_basics.rs, tests/language_additions.rs |
| Spread `...` in lists and dicts | yes | | tests/language_basics.rs, tests/language_additions.rs |
| Sets, dicts, lists, strings, ranges | yes | set literal `{1, 2}` | tests/language_basics.rs, tests/builtin_pythonic.rs |
| Comprehensions, lambdas, generators | yes | | tests/generators.rs, tests/language_basics.rs |
| `for`, `while`, `do-while`, `break`, `continue` | yes | | tests/for_loops.rs, tests/language_additions.rs, parity/probes.py (166 probes) |
| Labeled `break` / `continue` | yes | `for`, `while`, `do-while` | tests/language_additions.rs (`break <label>`) |
| `try` / `catch` / `finally`, `throw` | yes | | tests/language_basics.rs, tests/error_lines.rs |
| `with` (context managers) | yes | `__exit__` cannot swallow an exception | tests/python_gap.rs (`__exit__`) |
| `async` / `await`, threads, channels | yes | | tests/concurrency_stress.rs, tests/thread_safety.rs, tests/language_basics.rs |
| Modules and `import` | yes | | tests/language_basics.rs, tests/packages.rs |
| Type hints | partial | checked; Python-style names accepted | tests/compiler_consistency.rs (annotated functions); no dedicated type-error test found by name |
| Unicode names and text | yes | | tests/string_index.rs, tests/perf_regress.rs |
| Fractions and complex numbers | yes | library written in Killer | tests/language_additions.rs, tests/bigint.rs |
| Macros | yes | `macro name(a) { ... }` and `name!(x)`, text expansion, hygienic (body-local names renamed per expansion, `$name` to capture) | tests/language_additions.rs |
| Polyglot `@lang { }` blocks | yes | runs an installed interpreter or compiler as a subprocess and returns its stdout (no transpiling); body taken verbatim; `@lang(a, b) { }` passes values as JSON (`KILLER_ARGS`); timeout, output cap, scrubbed env, private temp dir, absolute runtime paths, `POLYGLOT_E_*` errors, policy (`KILLER_POLYGLOT_ALLOW`, `KILLER_PROFILE=strict`), audit log; not a sandbox. Runtimes tested here: python, bash, sh, ruby, perl, java, PowerShell (Windows); node, go, lua, gcc only if installed | tests/polyglot.rs, tests/capabilities.rs, unit tests in src/polyglot.rs and src/polyglot_front.rs |
| `eval` / `exec` of text | yes | fresh VM, same capabilities as the caller (no new power), 50M-step / 30 s / 256 MB memory limit (`max_memory_mb` option, `KILLER_EVAL_MAX_MEMORY`, 1 to 2048); live-memory check only in the CLI binaries or with the `mem-cap` feature / `CountingAlloc` installed (without it the live-memory check does not exist, only the pre-checks, steps and time apply); WARNING: never eval untrusted text, it runs with all of the caller's powers | tests/eval_safety.rs, tests/eval_memory.rs, tests/eval_memory_cli.rs, tests/eval_memory_embedded.rs, tests/memory_soak.rs |

## Standard library

| Feature | Status | Notes | Proof |
|---|---|---|---|
| Math, statistics, random, big integers | yes | | tests/bigint.rs, tests/builtin_pythonic.rs, tests/modulo.rs |
| Strings, formatting (`format`, `%`), regex | yes | | tests/builtin_pythonic.rs, tests/string_index.rs, tests/string_append.rs |
| Files: `open`, `with`, `for line in f` | yes | text only (`r`, `w`, `a`); no binary mode | tests/capabilities.rs (gating); open/with in tests/python_gap.rs; not every mode tested by name |
| JSON, CSV, base64, hashing, HMAC | yes | SHA-256, HMAC-SHA256, base64 verified against FIPS 180 / RFC 4231 / RFC 4648 | hashing, HMAC, base64: tests/crypto_vectors.rs; JSON and CSV: tests/realworld.rs and parity realworld programs only |
| Dates and times | partial | | no dedicated test found by name; parity probes only |
| Encryption | yes | ChaCha20-Poly1305 (RFC 8439), SHA-2, HMAC, PBKDF2-HMAC-SHA256 from the RustCrypto crates (`chacha20poly1305` had an [NCC Group audit](https://github.com/RustCrypto/AEADs/tree/master/chacha20poly1305#readme); `sha2`, `hmac`, `pbkdf2` are widely used and reviewed), OS random; checked against RFC/FIPS vectors, Killer itself not audited; no AES, SHA-3, Argon2 or public-key crypto ([CRYPTO.md](CRYPTO.md)) | tests/crypto_vectors.rs (FIPS 180-4, RFC 4231, RFC 7914, RFC 8439, RFC 4648; tamper tests) |
| HTTP client and server | yes | | tests/net_ai.rs, tests/ssrf_guard.rs, tests/http_server_tests.rs (request parsing only) |
| TCP sockets | yes | `tcp_listen`, `tcp_connect`, `sock_send`, `sock_recv` | tests/sockets.rs |
| UDP sockets | yes | `udp_bind`, `udp_send`, `udp_recv` | tests/sockets.rs |
| WebSocket client and server | yes | `ws://`, and `wss://` through TLS; text messages | tests/sockets.rs, tests/tls_server.rs |
| TLS client sockets | yes | `tcp_connect_tls`, `wss://`; verified through the system `openssl` command | tests/sockets.rs, tests/tool_resolution.rs; needs system `openssl` |
| TLS servers (HTTPS, `wss://`) | yes | `tls_listen` / `tls_accept`, `serve(..., {tls: {cert, key}})`, `ws_accept` on a TLS listener; one `openssl s_server` child per connection; needs `openssl` on the PATH | tests/tls_server.rs; needs system `openssl` |
| SQL database | yes | `sql_open` / `sql_exec` / `sql_query`: SQL with joins (INNER, LEFT, CROSS), aliases, subqueries, GROUP BY / HAVING, DISTINCT; file or memory | tests/sql_joins.rs, tests/language_additions.rs |
| Process control, environment | partial | | tests/capabilities.rs (gating only); behaviour not separately tested by name |
| Graphical windows (GUI) | yes | native widgets with `window(...)` (label, button, input, checkbox, list, combo, `message_box`): real Win32 controls on Windows, a headless model elsewhere or with `KILLER_GUI_HEADLESS=1`; the browser window `gui_show` / `gui_open` works everywhere. Real windows were created and driven in tests but never looked at on a screen | tests/gui_native.rs (programmatic only) |
| WebAssembly output | partial | `killer wasm`: numbers, ASCII strings (concat, compare, index, iterate) and lists of numbers (index, assign, push, iterate) with a bump allocator and host helpers; no dicts, objects, `str()`, lists of strings, or non-ASCII text | tests/wasm.rs |
| Matrix library | yes | `Matrix`, det, inverse, solve; written in Killer (not fast for big sizes) | tests/integration_week6_phase2_realworld.rs, tests/integration_week8_phase_b.rs (mention Matrix; not re-verified) |

## AI

| Feature | Status | Notes | Proof |
|---|---|---|---|
| Local model inference (GGUF) | untested | | NO test loads a model: no GGUF model on the dev machine (tracker row 122); code is in `src/inference/` |
| LLM providers (OpenAI, Anthropic, Ollama) | partial | request building, headers, parsing, streaming and errors tested against a local fake server (`tests/llm_live.rs`); the opt-in live suite (`docs/LLM_TESTING.md`) is written but **NOT TESTED** against any real provider (waiting for an Ollama install or a key) | tests/llm_live.rs (offline fake-server tests); live suite never run |
| Embeddings, vector store, chunking, RAG | yes | scale measured at 33k and 332k chunks | tests/apps.rs (RAG app), tests/net_ai.rs |
| Streaming responses | yes | | tests/streaming_profile.rs, tests/llm_live.rs (fake server only) |

## Tooling

| Feature | Status | Notes | Proof |
|---|---|---|---|
| Interpreter (bytecode VM) | yes | | tests/vm_tests.rs, tests/language_basics.rs, tests/examples_run.rs |
| x86-64 JIT | partial | falls back to the VM on other CPUs | tests/modulo.rs (`native_function_jit_matches_python`), tests/fusion_diff.rs, tests/perf_regress.rs; no dedicated JIT-versus-VM suite found |
| Formatter, linter, test runner, profiler, REPL | yes | `killer fmt`, `lint`, `test`, `profile`, `repl`; `fmt` is idempotent over `examples/` and keeps comments, `lint` has rule ids and `lint:ignore` suppression (`tests/killer_cli.rs`, `tests/formatter_corpus.rs`) | tests/killer_cli.rs, tests/formatter_corpus.rs |
| Package manager | yes | `killer new/add/install/update/remove/list/check`; `killer.toml` plus `killer.lock` with sha256 checksums, verified on install against the lockfile (`tests/pkg_driver.rs`, `tests/packages.rs`); file, path and git dependencies; a missing import is a hard error; no public registry (a self-hosted one works) | tests/pkg_driver.rs, tests/packages.rs |
| Language server | yes | `killer lsp` and `killer_lsp`: diagnostics from the real compiler, completion (keywords, builtins, symbols in the file), hover, go-to-definition for functions, classes and variables in the same file, document symbols; single-file only, no cross-file index (`tests/lsp_roundtrip.rs`) | tests/lsp_roundtrip.rs |
| Step debugger | yes | `killer debug`: step, next, finish, breakpoints, call stack; locals by slot | tests/dap_session.rs (DAP adapter); `killer debug` CLI: tests/new_features.rs (not re-verified) |
| Editor support | partial | VS Code extension in `editors/vscode` (grammar, snippets, language configuration, client that launches the server); the files are validated by `tests/vscode_extension.rs`, but the extension has not been packaged or run inside VS Code in CI | tests/vscode_extension.rs (file validation only) |
| Installers and release workflow | partial | `install.sh` verified locally against a served archive (checksum pass and fail); the workflow has a manual dry-run mode; a real tag run and `install.ps1` are still untested; see [RELEASING.md](RELEASING.md) | tests/installer_integrity.rs (install.sh against a file:// release) |
| CI on Linux, macOS, Windows | yes | | workflows outside this crate; last verified 2026-10-18 at b3de537 (PLATFORMS.md); not re-run for this pass |

## Not built

1. WebAssembly for dicts, sets, lambdas and lists of strings.
2. Native GUI widgets beyond the vertical stack; SQL RIGHT/FULL joins, UNION and window functions.
3. Live LLM provider tests (need an API key) and a run of the release workflow (needs a tag).
4. A TLS stack inside Killer (rustls or similar): TLS needs the system `openssl` and HTTPS needs `curl`.
5. Database drivers for PostgreSQL, MongoDB and others; Kafka, Spark; a message queue (archived 7-21 line stubs, see STATUS.md).
6. Observability: Prometheus, OpenTelemetry, distributed tracing exporters; no metrics endpoint.
7. Authentication standards: JWT, OAuth2, OIDC, RBAC/ABAC (archived stubs only).
8. AES, SHA-3, Argon2, public-key crypto, code or release signing.
9. A published container image, a signed release, a packaged VS Code extension, a tested `install.ps1`.
10. Dependent types, effect system, trait system (archived, never compiled).
11. Cluster/distributed execution as a supported feature (`cluster_coordinator` is not wired; its test is feature-gated and does not compile).
