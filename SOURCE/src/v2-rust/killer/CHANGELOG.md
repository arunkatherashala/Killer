# Changelog

All notable changes to `killer-native` are documented here.

## [Unreleased]

### Documentation
- New: `docs/STABILITY.md` (stability and deprecation policy, past behaviour changes), `docs/OPERATIONS.md` (every `KILLER_*` variable, limits, what is missing), `docs/PERFORMANCE.md` (honest index of `bench/RESULTS.md`), `SOURCE/docs/project/THREAT_MODEL.md` (threat, mitigation, proof test, residual risk).
- `docs/FEATURES.md` now has a Proof column naming a test file per row; unproven rows were downgraded (`untested`/`partial`) and the Not built list was extended. README claims aligned (HTTP server TLS, release signing, test count).

### Added
- Tooling: `killer new|init|add|remove|install|update|list|publish|check` in the `killer` driver (same code as `killer_pkg`); tests for `killer lsp` (hover, completion, definition of classes and variables), the VS Code extension files (`tests/vscode_extension.rs`), lockfile checksum verification when a registry and its checksum are both rewritten, `fmt` convergence over `examples/` with comments kept, and every lint suppression form.

### Security
- **Warning: the old `encryption` module of the crate (XOR "AES-256-GCM"/"ChaCha20-Poly1305", unchecked tag, fake
  SHA-256/PBKDF2) did not encrypt.** Treat anything it protected as unprotected and rotate those secrets; the new
  real implementation cannot decrypt its output. Full details in `docs/RELEASE_NOTES.md`.
- `eval`/`exec` inherit the caller's capabilities and have a step and time budget; `curl -k` removed from the
  web-search fetchers and image download (certificates are verified); `openssl`/`curl` resolved to absolute paths;
  API keys no longer on command lines.

### Changed
- **Behaviour change: `serve` runs handlers on a bounded worker pool, and handlers no longer share global state.**
  The accept loop hands each connection to a bounded queue (`queue`, default 1024) served by `workers` threads (default
  min(CPUs, 8), at most 256); a full queue answers `503` with `Retry-After: 1` at once, so one slow client or SSE stream
  no longer blocks everyone. Requests are read by up to 64 reader threads (no VM) before a worker takes them, so
  clients that send slowly do not hold the workers. Each worker runs its own VM on a private deep copy of the program data taken when `serve`
  starts (globals, the handler, the routes and what their closures captured, copied in one walk, so closures sharing a
  store still share it inside a worker). A handler's change to a global is therefore not seen by the other workers nor by
  the script after `serve` returns: share state with `mutex_*` / `atomic_*` cells, channels or a database, or pass
  `workers: 1`, which runs the handler on the script's own VM and data as before (one request at a time, no copy:
  the script sees the handler's changes after `serve` returns; `serve_once` too; `examples/apps/todo` does this for its
  in-memory store).
  `max_requests` counts requests admitted to routing across all workers (exactly that many reach a handler; a later one
  that was already accepted gets `503`; `serve` returns once they are answered). `server_stop()` stops accepting at
  once, answers queued connections `503` and waits up to the new `drain_timeout` (default 30 s) for running handlers.
  `serve` returns the number of requests answered. `request_timeout`, `read_timeout`, `stream_timeout`, `idle_timeout`
  and `tls` keep their meaning. Bad `workers` / `queue` values are errors. Handlers on workers run in the interpreter
  (the JIT is not used on worker threads, as for `async_spawn` tasks). Tests: `tests/http_pool.rs`, new HTTPS pool test
  in `tests/tls_server.rs`. The accept loop blocks in `accept` (a watchdog thread
  wakes it for `server_stop`, `max_requests` and `idle_timeout`) instead of polling with a 2 ms sleep. Load (release
  builds, Windows, 8 CPUs, shared machine, Python client, 7 alternating runs, medians; base = the one-at-a-time server):
  trivial handler, 5 bursts of 200 concurrent `GET`s: 317 req/s -> 2264 req/s, no request refused, server CPU about
  470 vs 520 us per request; 2000 sequential `GET`s: 2.84 ms -> 2.40 ms per request (a loopback connect+close alone
  costs about 2.5 ms on this machine); 20 clients trickling their request over 5 s plus one fast client: the fast one
  waited 5.6 s -> 0.036 s (5 runs). `fib(32)` and a 5M-iteration loop are unchanged (CPU median of 7).
- The eval memory cap's counting allocator is now declared by the Killer CLI binaries (`killer`, `killer_super`,
  `killer-native`, `killer-repl`) instead of the library, so the library no longer conflicts with an embedder's own
  `#[global_allocator]`; `unsafe` is allowed only in `memcap.rs` for it. Without the allocator (embedder, no
  `mem-cap`) the live-memory check does not exist and only the pre-checks, step and time budgets apply;
  `memcap::is_installed()` reports it.
- Cryptography now uses the RustCrypto crates `chacha20poly1305`, `sha2`, `hmac`, `pbkdf2` and `subtle`
  instead of hand-written code. These are the first external dependencies of `killer-native` apart from `libc` on Unix, which the JIT uses (the "zero
  external crates" claim no longer applies). The five duplicate SHA-256 implementations, the SHA-512,
  HMAC, PBKDF2, ChaCha20 and Poly1305 code were removed and replaced by thin wrappers. Builtins, error
  messages and the `KSE2` blob format are unchanged, and all RFC and FIPS test vectors still pass.
  `chacha20poly1305` had an independent NCC Group audit; `sha2`, `hmac` and `pbkdf2` are widely used and
  reviewed. SHA-1 (WebSocket handshake accept key only, no builtin exposes it) and base64 stay hand-written.

### Changed
- `%` is now **floor modulo** (Python) for plain numbers too: the result has the sign of the divisor, so
  `-7 % 3` is `2` (was `-1`), `7 % -3` is `-2`, `-5.5 % 2` is `0.5`. BigInt `%` already behaved this way.
  Applies to the VM, constant folding, the native function JIT, `%=`, and `divmod`'s remainder. Division by
  zero errors are unchanged; `x % inf` and `inf % x` follow Python.

### Added
- Optional Cargo feature `mem-cap` (off by default): the library installs `memcap::CountingAlloc` as the global
  allocator for embedders that want the eval live-memory cap.
- `rem(a, b)`: the C-style truncated remainder (sign of the dividend, like Rust `%` / C `fmod`) for numbers
  and bigints; `mod(a, b)`: function form of `%`. A script-defined function of the same name wins.
- `LICENSE` file (MIT)
- `README.md` — user-facing documentation with quickstart, features, and binary reference
- `.gitignore` to exclude build artifacts from version control
- Crate-level `exclude` list in `Cargo.toml` to keep artifacts out of published package
- `CHANGELOG.md` and `CONTRIBUTING.md`

### Fixed
- `Cargo.toml` `readme` field now correctly points to `README.md` at the crate root
- Duplicate entries removed from `module_names_in_order.json`

---

## [1.0.0] — 2026

### Core VM & JIT
- NaN-boxing (`nanbox`) — 8-byte inline scalars, zero heap allocation for f64/bool/trit
- x86-64 JIT compiler (`jit_x86`) — hot loop detection + native machine code generation
- Bytecode compiler, AST, and multi-pass optimizer
- Trinary (3-valued) logic — native `trit` type (True / False / Unknown)

### Standard Library
- 600+ functions across math, I/O, networking, JSON, HTTP, crypto, concurrency, collections
- KORE / Nova binary columnar formats with SIMD-accelerated encode/decode

### AI Integration
- Built-in LLM client with provider routing (OpenAI-compatible, local models)
- KhLM polyglot AI router — 5-tier intelligence: CAG → LLM → RLM → Ghost-108
- Vector memory — TF-IDF embeddings + cosine similarity, auto-recall
- Affect engine — 6-dimensional emotional state for AI responses
- Imagination engine — counterfactual reasoning and conceptual bridges
- Agent framework — composable AI agents with tool use

### Production Features
- Telemetry — request/latency/resource metrics
- Circuit breaker — automatic error recovery
- Structured logging with correlation IDs
- Retry policies with exponential backoff
- Health check system (liveness / readiness / startup probes)
- Audit logging — compliance trails for financial/security use cases
- Encryption — AES-256-GCM, password hashing, key management

### Tooling
- `killer-mcp` — MCP server bridge (`killer_compile`, `killer_run`, `killer_version`)
- `killer_ui_serve` — Kala chat UI over HTTP
- `ghost_hive` — Ghost VM hive coordinator
- LSP server — IDE integration (autocomplete, diagnostics)
- Package manager (`killer_10x`)

### Multimedia
- Image/audio/video generation in pure Rust (zero external crates)
- Nova Audio Engine — WAV synthesis (ambient, nature, space, beat, ocean)
- Nova Video Engine — animated GIF generator (GIF89a + LZW encoder)

### Android
- AAudio NDK microphone recording engine
- Phone state monitoring — call detection, VoIP, auto-record
- Foreground service, permissions, notifications
- Encrypted evidence storage with secure wipe
