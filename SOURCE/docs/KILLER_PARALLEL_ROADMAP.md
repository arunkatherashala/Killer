# Killer parallel workstreams

Single place to track improvements that can advance together without blocking each other.

| Track | Goal | Notes |
|--------|------|--------|
| **Docs** | Keep `KILLER_LANGUAGE_GUIDE.md`, `BUILTIN_REFERENCE.md`, and positioning docs aligned with shipped behavior. | Prefer small, factual deltas per release. |
| **Examples** | `SOURCE/src/v2-rust/killer/sample_programs/` — tiny `.killer` files for onboarding. | Run with `killer-native <path>`. |
| **CLI** | Stable flags: `--repl`, `--watch`, `--format`, `--test`, model registry, inference. | `--watch` polls file content; Ctrl+C exits. |
| **Kala / AI** | Online-first when an LLM is configured; cache clear + routing visibility. | See `kala_ui`, `builtin` expert path, `llm` KB. |
| **Tests** | `cargo test -p killer-native --lib` for fast feedback; integration tests under `tests/`. | Full binary tests may need smoke server stopped (no lock on `kala_smoke_server.exe`). |
| **Nova / Trit** | Codec and VM experiments — see `tests/nova_*`, `nova_trit_codec_integration`. | Larger scope; document before expanding surface area. |

## Quick commands

- Run a sample: `killer-native SOURCE/src/v2-rust/killer/sample_programs/hello.killer`
- Watch edits: `killer-native --watch SOURCE/src/v2-rust/killer/sample_programs/hello.killer`
- Library tests: `cargo test -p killer-native --lib`
