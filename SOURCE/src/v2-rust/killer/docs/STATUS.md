# Killer: current status

Status: **beta**. This page says, in plain words, what works, how we know, what has not been
verified, and what is still open. It is the short version; the evidence is in
[`parity/PARITY_REPORT.md`](../parity/PARITY_REPORT.md) and [`bench/RESULTS.md`](../bench/RESULTS.md).

## Does it work?

Yes. The checks below were run locally on Windows x86-64. The test suite, parity probes and
documentation examples last ran on GitHub's Linux (x86-64), macOS (Apple Silicon) and Windows runners
at commit `b3de537` (see [`PLATFORMS.md`](PLATFORMS.md)); later changes were checked locally only:

| Check | Result |
|-------|--------|
| Automated tests (`cargo test --lib --tests --no-fail-fast`) | 3,904 passed, 0 failed (local run of 2026-11-25, root `tracker.csv` row 209) |
| Language parity probes (`python parity/run_parity.py`, 166 programs written like a Python/JS/Java programmer would) | 166 of 166 pass (it was 76 of 166, 46%, at the first audit) |
| Documentation examples (`python docs/check_examples.py`) | 71 of 71 run and print what the docs say |
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
- **AI and network:** a real HTTP server with Killer handlers (including streaming), TLS serving
  (HTTPS and `wss://` through the system `openssl`, `tests/tls_server.rs`), TCP, UDP and WebSocket
  sockets, a hardened HTTP client with an SSRF guard (loopback and private addresses refused unless
  `allow_private`), streaming LLM calls, embeddings, a vector store, chunking, an offline RAG demo
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

## Archived code (not features)

Dead code was moved out of `src/` with `git mv` (history kept, nothing deleted):

- `archive/uncompiled/src/`: 12 files that were never declared as modules and so never compiled:
  `contract_programming`, `dependent_types` (+ `_ast`, `_check`, `_integration`), `effect_handlers`,
  `effect_system`, `message_passing`, `trait_system`, `type_checking_runtime`, `vm_integration`,
  `vm_v2_components`. **Dependent types, an effect system, and the trait system described in older design docs are
  not implemented.** `ghost_vm_sha.rs` and
  `mcp_minjson.rs` stayed because `ghost_vm.rs` and `bin/killer_mcp.rs` include them with `#[path]`.
- `archive/stubs/`: 33 declared-but-empty modules of 7-21 lines that nothing referenced (abac, concurrency_solver,
  database_mongodb, database_postgresql, database_query, distributed_tracing, file_upload, game_theory, graphql,
  hlc, io_solver, load_balancer, locks, math_impl, medical_biomedical, message_queue, middleware,
  millennium_prize, network_science, oauth2, paxos, raft, rbac, request_response, service_discovery, sessions,
  signal_processing, state_machines, statistics_solver, template_engine, time_solver, token_introspection,
  type_solver). They exposed no working functions, so listing them as features was misleading. Their `pub mod`
  lines were removed from `src/lib.rs`; to restore one, `git mv` it back and re-add the line.

## Known limits (honest list)

- Indexing a very long ASCII string (`s[i]` plus `len(s)` in a loop over 200k characters) is linear per
  call, about 1.6 s for the whole loop. Fixing it needs a different string representation.
- Whole-number `+ - * **` results beyond 2^53 become exact big integers automatically, but an integer
  literal above 2^53 is read as a float: write `123n` for those.
- The HTTP server handles one request at a time. TLS serving (`serve(..., {tls: {cert, key}})`,
  `tls_listen`) works but needs the system `openssl` on the PATH (one `openssl s_server` child per
  connection; there is no TLS stack inside Killer), and HTTPS client calls need `curl`.
- Token counts from `count_tokens` are an approximation (the real tokenizer needs a model file).
- `__setitem__` applies to non-string keys only (`obj["x"] = v` is a field write).
- The DAP debugger adapter reports one stack frame (`stackTrace`); the language server does not follow imports across files.
- 5 source files under `src/` are not part of the build (never declared as modules: `python/mod.rs`,
  `codegen/tests.rs`, `custom_optimization/`); the older dead files were archived (above). The per-file
  list is `parity/FILE_INVENTORY.csv`, regenerated with `python parity/inventory.py`.
- Tracked build artifacts (`*.so`, `*.pdb`, `*.dll.exp`) get rewritten by test runs and are left
  uncommitted.

## How to check it yourself

```
cd SOURCE/src/v2-rust/killer
cargo build --bins
cargo test --lib --tests --no-fail-fast
python parity/run_parity.py          # set KILLER_BIN to the built killer_super
python docs/check_examples.py
```

## Archived in the inference/stub pass

Moved with `git mv` into `archive/uncompiled/` and removed from the build; none of these are features:

- `kafka/` (the Kafka client was a mock), `spark/`, `cluster_coordinator.rs` and its test
  (`cluster_demo_test.rs`; the `cluster-demo-tests` Cargo feature now gates nothing).
- `llm_client.rs` (returned canned text; use `crate::llm` for real calls), `agent_framework.rs` (fake
  act/observe), `websocket.rs` (simulated; the `websocket_new` / `websocket_server_new` builtins were removed;
  the real WebSocket client/server is `ws_connect` / `ws_accept` in `net_sock`), `auth.rs` (always returned no user).
- `ai/providers/local.rs` stays compiled because `ProviderManager` registers it, but its simulated embedding,
  classification, extraction and ONNX paths now return errors.
