# Killer: current status

Status: **beta**. This page says, in plain words, what works, how we know, what has not been
verified, and what is still open. It is the short version; the evidence is in
[`parity/PARITY_REPORT.md`](../parity/PARITY_REPORT.md) and [`bench/RESULTS.md`](../bench/RESULTS.md).

## Does it work?

Yes. The checks below were run locally on Windows x86-64, and the test suite, parity probes and
documentation examples also pass on GitHub's Linux (x86-64), macOS (Apple Silicon) and Windows runners
(commit `b3de537`, see [`PLATFORMS.md`](PLATFORMS.md)):

| Check | Result |
|-------|--------|
| Automated tests (`cargo test --tests --no-fail-fast`) | 3,402 passed, 0 failed |
| Language parity probes (`python parity/run_parity.py`, 166 programs written like a Python/JS/Java programmer would) | 166 of 166 pass (it was 76 of 166, 46%, at the first audit) |
| Documentation examples (`python docs/check_examples.py`) | 58 of 58 run and print what the docs say |
| Documentation links (`python docs/check_links.py`) | 67 links, 0 broken |
| Real programs | 18 programs in `parity/realworld/` and 5 applications in `examples/apps/` (REST service, RAG question answering, CSV tool, text adventure, expression interpreter), with Python mirrors for the deterministic parts |
| Fuzzing | seeded runs of 20,000 cases over several seeds, no panic, hang or stack overflow |
| Speed | numeric code is native-compiled (the README table); `fib(32)` and a 5M-iteration loop did not get slower during the work |

## What Killer has

- **Language:** functions, lambdas, closures (shared captured variables), default, variadic and keyword
  arguments, classes with inheritance, `super`, `static`, operator overloading and dunder methods
  (`__len__`, `__getitem__`, `__iter__`, `__call__`, ...), `try` / `catch` / `finally` / `throw`,
  `match` / `switch`, all loop forms, comprehensions (list, dict, set), lazy generators, imports,
  big integers (`123n`), uncertainty-aware numbers (`72 ± 3`) and three-valued logic.
- **Numbers:** one rule for `%` (floor, like Python) with `rem()` for the C-style remainder.
- **Standard library:** strings, lists, dicts, sets, JSON, CSV, files, regex, dates, hashing, base64,
  logging, async tasks, channels, mutexes.
- **AI and network:** a real HTTP server with Killer handlers (including streaming), a hardened HTTP
  client, streaming LLM calls, embeddings, a vector store, chunking, an offline RAG demo
  (`examples/rag_demo.killer`).
- **Tools:** `killer` driver (`run`, `fmt`, `lint`, `test`, `profile`, `repl`, `lsp`), package manager
  (`killer_pkg`, `killer.toml`, lockfile with SHA-256, registry), language server, VS Code extension,
  debugger (DAP), formatter, linter, test framework (`test "name" { }`), profiler with flame graph.
- **Safety:** resource limits (recursion depth, allocation sizes, nesting), capability checks on file,
  network, process and native-code builtins, tasks get deep copies (no shared references between
  threads).

## What has not been verified

- **Other platforms.** Linux ARM, Linux musl and Intel macOS are type-checked in CI but not run;
  Linux and macOS are only covered by what the CI runners exercise (no manual testing there).
- **Release workflow, install scripts, release packaging, VS Code client script:** the release
  workflow (it runs on version tags) and `install.sh` / `install.ps1` have never been executed, and
  the VS Code client script has never run in an editor. The test, parity, documentation and
  cross-check workflows do run on GitHub and pass.
- **Real providers:** calls to OpenAI, Anthropic, Ollama and others were only tested against local
  fake servers.
- **Quiet-machine timings:** the benchmarks were measured on a loaded machine; use CPU-time numbers in
  `bench/RESULTS.md` as indicative.

## Known limits (honest list)

- Indexing a very long ASCII string (`s[i]` plus `len(s)` in a loop over 200k characters) is linear per
  call, about 1.6 s for the whole loop. Fixing it needs a different string representation.
- Plain numbers do not promote to big integers automatically; write `123n` or `big(x)`.
- The HTTP server handles one request at a time, has no TLS, and HTTPS client calls need `curl`.
- Token counts from `count_tokens` are an approximation (the real tokenizer needs a model file).
- A variable not assigned on the path taken reads as `null` (no "used before assignment" error).
- `__setitem__` applies to non-string keys only (`obj["x"] = v` is a field write).
- The debugger shows one stack frame; the language server does not follow imports across files.
- 192 older source files in the repository are not part of the build (never compiled); they are
  untouched and listed in `parity/FILE_INVENTORY.csv`.
- Tracked build artifacts (`*.so`, `*.pdb`, `*.dll.exp`) get rewritten by test runs and are left
  uncommitted.

## How to check it yourself

```
cd SOURCE/src/v2-rust/killer
cargo build --bins
cargo test --tests --no-fail-fast
python parity/run_parity.py          # set KILLER_BIN to the built killer_super
python docs/check_examples.py
```
