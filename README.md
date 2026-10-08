# Killer

Killer is a from-scratch scripting language with a bytecode VM, written in Rust with zero external crates.
It has Python-like syntax, closures, classes, generators, exceptions with line numbers, a large builtin
library, an optional x86-64 JIT for numeric code (other architectures use the interpreter) and a few
unusual features (trits, signals, qubits, AI helpers).

**Status: beta / pre-release.** The version is the one in `SOURCE/src/v2-rust/killer/Cargo.toml`
(currently 1.0.0). Performance numbers are only quoted from
[`bench/RESULTS.md`](SOURCE/src/v2-rust/killer/bench/RESULTS.md); anything else is "not measured".
Many documents under `DOCS/`, `AI_FEATURES/` and `DOCS/archive/` are historical session reports
and are not authoritative.

## Build and run

```bash
cd SOURCE/src/v2-rust/killer
cargo build --release --bin killer_super
echo 'print("hello, killer")' > hello.killer
./target/release/killer_super hello.killer       # killer_super.exe on Windows
```

Or install a prebuilt release binary (see `install.sh` / `install.ps1` in this repo root).

## Test

```bash
cd SOURCE/src/v2-rust/killer
cargo test --tests
KILLER_BIN=target/debug/killer_super python parity/run_parity.py
KILLER_BIN=target/debug/killer_super python docs/check_examples.py
```

## Where to look

- Crate README: [`SOURCE/src/v2-rust/killer/README.md`](SOURCE/src/v2-rust/killer/README.md)
- Language reference: [`docs/LANGUAGE.md`](SOURCE/src/v2-rust/killer/docs/LANGUAGE.md)
- Tutorial: [`docs/TUTORIAL.md`](SOURCE/src/v2-rust/killer/docs/TUTORIAL.md)
- Parity audit: [`parity/PARITY_REPORT.md`](SOURCE/src/v2-rust/killer/parity/PARITY_REPORT.md)
- Benchmarks (Killer vs CPython): [`bench/RESULTS.md`](SOURCE/src/v2-rust/killer/bench/RESULTS.md), run with `python bench/run.py`
- Changes: [`CHANGELOG.md`](CHANGELOG.md)
- Older design and status documents: [`DOCS/`](DOCS)

License: see [`LICENSE`](LICENSE).
