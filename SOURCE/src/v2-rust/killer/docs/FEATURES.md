# Killer feature status

One table of what Killer has and what it does not, kept in step with the code. Status values:
**yes** (built and tested), **partial** (works with a stated limit), **no** (not built).
The row-by-row history is in the root `tracker.csv`; the Python comparison is in [PYTHON_COMPARISON.md](PYTHON_COMPARISON.md).

## Language

| Feature | Status | Notes |
|---|---|---|
| Variables, functions, closures, recursion | yes | f64 numbers, automatic big integers at 2^53 |
| Classes, inheritance, `this`, `new` | yes | |
| Operator overloading (`__add__`, `__eq__`, ...) | yes | reflected forms too |
| Properties (`get` / `set`) | yes | |
| `abstract`, `interface`, `implements` | yes | an interface is a parent class whose bodiless methods throw |
| `enum` | yes | |
| `const` | yes | checked at compile time |
| Private members | yes | `private fn`, `private a, b`; compile error outside the class |
| Pattern matching: values, `\|`, guards, bindings | yes | |
| Pattern matching: list patterns `[a, ...rest]` | yes | nested too |
| Pattern matching: dict patterns `{name, age: n}` | yes | nested too |
| Destructuring: lists, dicts | yes | `{a, b: c} = d` |
| Spread `...` in lists and dicts | yes | |
| Sets, dicts, lists, strings, ranges | yes | set literal `{1, 2}` |
| Comprehensions, lambdas, generators | yes | |
| `for`, `while`, `do-while`, `break`, `continue` | yes | |
| Labeled `break` / `continue` | yes | `for`, `while`, `do-while` |
| `try` / `catch` / `finally`, `throw` | yes | |
| `with` (context managers) | yes | `__exit__` cannot swallow an exception |
| `async` / `await`, threads, channels | yes | |
| Modules and `import` | yes | |
| Type hints | yes | checked; Python-style names accepted |
| Unicode names and text | yes | |
| Fractions and complex numbers | yes | library written in Killer |
| Macros | yes | `macro name(a) { ... }` and `name!(x)`, text expansion, hygienic (body-local names renamed per expansion, `$name` to capture) |
| `eval` / `exec` of text | yes | fresh VM, same capabilities as the caller (no new power), 50M-step / 30 s / 256 MB memory limit (`max_memory_mb` option, `KILLER_EVAL_MAX_MEMORY`, 1 to 2048); WARNING: never eval untrusted text, it runs with all of the caller's powers |

## Standard library

| Feature | Status | Notes |
|---|---|---|
| Math, statistics, random, big integers | yes | |
| Strings, formatting (`format`, `%`), regex | yes | |
| Files: `open`, `with`, `for line in f` | yes | text only (`r`, `w`, `a`); no binary mode |
| JSON, CSV, base64, hashing, HMAC | yes | SHA-256, HMAC-SHA256, base64 verified against FIPS 180 / RFC 4231 / RFC 4648 |
| Dates and times | yes | |
| Encryption | yes | ChaCha20-Poly1305 (RFC 8439), PBKDF2-HMAC-SHA256, OS random; checked against RFC/FIPS vectors, not audited; no AES, SHA-3, Argon2 or public-key crypto ([CRYPTO.md](CRYPTO.md)) |
| HTTP client and server | yes | |
| TCP sockets | yes | `tcp_listen`, `tcp_connect`, `sock_send`, `sock_recv` |
| UDP sockets | yes | `udp_bind`, `udp_send`, `udp_recv` |
| WebSocket client and server | yes | `ws://`, and `wss://` through TLS; text messages |
| TLS client sockets | yes | `tcp_connect_tls`, `wss://`; verified through the system `openssl` command |
| TLS servers (HTTPS, `wss://`) | yes | `tls_listen` / `tls_accept`, `serve(..., {tls: {cert, key}})`, `ws_accept` on a TLS listener; one `openssl s_server` child per connection; needs `openssl` on the PATH |
| SQL database | yes | `sql_open` / `sql_exec` / `sql_query`: SQL with joins (INNER, LEFT, CROSS), aliases, subqueries, GROUP BY / HAVING, DISTINCT; file or memory |
| Process control, environment | yes | |
| Graphical windows (GUI) | yes | native widgets with `window(...)` (label, button, input, checkbox, list, combo, `message_box`): real Win32 controls on Windows, a headless model elsewhere or with `KILLER_GUI_HEADLESS=1`; the browser window `gui_show` / `gui_open` works everywhere. Real windows were created and driven in tests but never looked at on a screen |
| WebAssembly output | partial | `killer wasm`: numbers, ASCII strings (concat, compare, index, iterate) and lists of numbers (index, assign, push, iterate) with a bump allocator and host helpers; no dicts, objects, `str()`, lists of strings, or non-ASCII text |
| Matrix library | yes | `Matrix`, det, inverse, solve; written in Killer (not fast for big sizes) |

## AI

| Feature | Status | Notes |
|---|---|---|
| Local model inference (GGUF) | yes | |
| LLM providers (OpenAI, Anthropic, Ollama) | partial | request building, headers, parsing, streaming and errors tested against a local fake server (`tests/llm_live.rs`); the opt-in live suite (`docs/LLM_TESTING.md`) is written but not yet run against a real provider (needs a key) |
| Embeddings, vector store, chunking, RAG | yes | scale measured at 33k and 332k chunks |
| Streaming responses | yes | |

## Tooling

| Feature | Status | Notes |
|---|---|---|
| Interpreter (bytecode VM) | yes | |
| x86-64 JIT | partial | falls back to the VM on other CPUs |
| Formatter, linter, test runner, profiler, REPL | yes | `killer fmt`, `lint`, `test`, `profile`, `repl` |
| Package manager | partial | local registry; no public registry |
| Language server | partial | `killer lsp` |
| Step debugger | yes | `killer debug`: step, next, finish, breakpoints, call stack; locals by slot |
| Editor support | partial | VS Code grammar |
| Installers and release workflow | partial | `install.sh` verified locally against a served archive (checksum pass and fail); the workflow has a manual dry-run mode; a real tag run and `install.ps1` are still untested; see [RELEASING.md](RELEASING.md) |
| CI on Linux, macOS, Windows | yes | |

## Not built

1. WebAssembly for dicts, sets, lambdas and lists of strings.
2. Native GUI widgets beyond the vertical stack; SQL RIGHT/FULL joins, UNION and window functions.
3. Live LLM provider tests (need an API key) and a run of the release workflow (needs a tag).
