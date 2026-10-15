# Killer

Killer is a from-scratch scripting language with a bytecode VM, written in Rust with zero external crates.
It has Python-like syntax with braces, closures that share captured variables, classes, lazy generators,
exceptions with file and line numbers, arbitrary-precision integers, a large builtin library, an optional
x86-64 JIT for numeric code (other architectures use the interpreter) and a few unusual features (trits,
uncertain numbers, offline AI building blocks).

**Status: beta / pre-release.** The version is the one in
[`SOURCE/src/v2-rust/killer/Cargo.toml`](SOURCE/src/v2-rust/killer/Cargo.toml) (currently 1.0.0).
Performance numbers are only quoted from [`bench/RESULTS.md`](SOURCE/src/v2-rust/killer/bench/RESULTS.md);
anything else is "not measured". What does not work yet is listed in the
[parity report](SOURCE/src/v2-rust/killer/parity/PARITY_REPORT.md). Many documents under `DOCS/`,
`AI_FEATURES/` and `DOCS/archive/` are historical session reports and are not authoritative.

## What is in the box

- The language: functions, lambdas, closures, classes, generators, `try`/`catch`/`finally`, `match`,
  comprehensions, big integers (`123n`), `async`/channels/mutexes.
- The `killer` driver: `run`, `fmt`, `lint`, `test`, `repl`, `lsp`; a test framework (`test "name" { }`).
- Packages: `killer.toml`, `killer.lock`, `killer_pkg new/add/install/publish/check`, compile-time `import`.
- Network and AI: an HTTP server (`serve`, `route`), a hardened HTTP client, a vector store, chunking,
  offline embeddings, RAG prompts, logging.
- Editor support: a language server, a VS Code extension, a Debug Adapter Protocol server.
- Safety limits (recursion depth, allocation size, nesting) and capability checks on file, network,
  process and native-code builtins.

## Install

Prebuilt release binaries (Linux x64, Windows x64, macOS x64 and arm64), checksum-verified, installed to
`~/.killer/bin`:

```bash
sh install.sh            # Linux / macOS; see --help and --dry-run
```
```powershell
./install.ps1            # Windows PowerShell
```

Or build from source (Rust stable; `curl` is needed at run time for HTTPS):

```bash
cd SOURCE/src/v2-rust/killer
cargo build --release --bin killer --bin killer_super --bin killer_pkg --bin killer_lsp --bin killer_dap
```

## Quickstart

```bash
echo 'println("hello, killer")' > hello.killer
killer run hello.killer         # or: killer_super hello.killer --run
killer repl
killer test                     # run test blocks and fn test_*() functions
killer fmt --check . && killer lint .
killer_pkg new myapp            # a project with killer.toml
```

## Test

```bash
cd SOURCE/src/v2-rust/killer
cargo test --tests
cargo build --bins
KILLER_BIN=$PWD/target/debug/killer_super python parity/run_parity.py      # 166 language probes
KILLER_BIN=$PWD/target/debug/killer_super python docs/check_examples.py    # every documentation example
python docs/check_links.py                                                 # relative links in the docs
```

## Where to look

- Crate README: [`SOURCE/src/v2-rust/killer/README.md`](SOURCE/src/v2-rust/killer/README.md)
- Tutorial (14 lessons): [`docs/TUTORIAL.md`](SOURCE/src/v2-rust/killer/docs/TUTORIAL.md)
- Language reference: [`docs/LANGUAGE.md`](SOURCE/src/v2-rust/killer/docs/LANGUAGE.md)
- Standard library index: [`docs/STDLIB_INDEX.md`](SOURCE/src/v2-rust/killer/docs/STDLIB_INDEX.md)
- Contributor architecture guide: [`docs/ARCHITECTURE.md`](SOURCE/src/v2-rust/killer/docs/ARCHITECTURE.md)
- Parity audit: [`parity/PARITY_REPORT.md`](SOURCE/src/v2-rust/killer/parity/PARITY_REPORT.md)
- Benchmarks (Killer vs CPython): [`bench/RESULTS.md`](SOURCE/src/v2-rust/killer/bench/RESULTS.md), run with `python bench/run.py`
- VS Code extension: [`editors/vscode`](SOURCE/src/v2-rust/killer/editors/vscode/README.md)
- Changes: [`CHANGELOG.md`](CHANGELOG.md)
- Older design and status documents: [`DOCS/`](DOCS)

License: see [`LICENSE`](LICENSE).
