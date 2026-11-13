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
| Macros | yes | `macro name(a) { ... }` and `name!(x)`, text expansion, not hygienic |
| `eval` / `exec` of text | yes | fresh VM, same capabilities as the caller (no new power) |

## Standard library

| Feature | Status | Notes |
|---|---|---|
| Math, statistics, random, big integers | yes | |
| Strings, formatting (`format`, `%`), regex | yes | |
| Files: `open`, `with`, `for line in f` | yes | text only (`r`, `w`, `a`); no binary mode |
| JSON, CSV, base64, hashing, HMAC | yes | |
| Dates and times | yes | |
| Encryption | yes | ChaCha20-Poly1305, PBKDF2, OS random |
| HTTP client and server | yes | |
| TCP sockets | yes | `tcp_listen`, `tcp_connect`, `sock_send`, `sock_recv` |
| UDP sockets | yes | `udp_bind`, `udp_send`, `udp_recv` |
| WebSocket client and server | yes | `ws://`, and `wss://` through TLS; text messages |
| TLS client sockets | yes | `tcp_connect_tls`, `wss://`; verified through the system `openssl` command |
| SQL database | yes | `sql_open` / `sql_exec` / `sql_query`: single-table SQL, no joins; file or memory |
| Process control, environment | yes | |
| Graphical windows (GUI) | yes | the browser is the window: `gui_show`, `gui_open`; no native widgets |
| WebAssembly output | partial | `killer wasm`: numbers, ASCII strings (concat, compare, index, iterate) and lists of numbers (index, assign, push, iterate) with a bump allocator and host helpers; no dicts, objects, `str()`, lists of strings, or non-ASCII text |
| Matrix library | yes | `Matrix`, det, inverse, solve; written in Killer (not fast for big sizes) |

## AI

| Feature | Status | Notes |
|---|---|---|
| Local model inference (GGUF) | yes | |
| LLM providers (OpenAI, Anthropic, Ollama) | partial | code and mocks tested; no live-provider test (needs a key) |
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
| Installers and release workflow | partial | written, never run (a release tag is needed to test) |
| CI on Linux, macOS, Windows | yes | |

## Not built

1. TLS listeners (HTTPS servers); WebAssembly for strings, lists and objects.
2. SQL joins and subqueries; native GUI widgets; hygienic macros.
3. Live LLM provider tests (need an API key) and a run of the release workflow (needs a tag).
