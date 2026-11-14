# Changelog

All notable changes to `killer-native` are documented here.

## [Unreleased]

### Security
- **Warning: the old `encryption` module of the crate (XOR "AES-256-GCM"/"ChaCha20-Poly1305", unchecked tag, fake
  SHA-256/PBKDF2) did not encrypt.** Treat anything it protected as unprotected and rotate those secrets; the new
  real implementation cannot decrypt its output. Full details in `docs/RELEASE_NOTES.md`.
- `eval`/`exec` inherit the caller's capabilities and have a step and time budget; `curl -k` removed from the
  web-search fetchers and image download (certificates are verified); `openssl`/`curl` resolved to absolute paths;
  API keys no longer on command lines.

### Changed
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
