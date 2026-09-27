# Changelog

All notable changes to `killer-native` are documented here.

## [Unreleased]

### Added
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
