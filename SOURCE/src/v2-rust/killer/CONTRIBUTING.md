# Contributing to Killer

Thank you for your interest in contributing! This document covers how to get started, the project conventions, and how to submit changes.

## Getting started

```powershell
git clone https://github.com/arunkatherashala/Killer
cd Killer/SOURCE/src/v2-rust/killer
cargo build
cargo test
```

Linux / macOS:
```bash
git clone https://github.com/arunkatherashala/Killer
cd Killer/SOURCE/src/v2-rust/killer
cargo build
cargo test
```

## Running the full test suite

```powershell
# Short suite (same as CI)
pwsh -File scripts\revalidate_killer.ps1

# Full integration suite (slow)
pwsh -File scripts\revalidate_killer.ps1 -Full
```

```bash
./scripts/revalidate_killer.sh
./scripts/revalidate_killer.sh --full
```

## Project layout

```
src/
  lib.rs              — crate root, all pub mod declarations
  vm.rs               — core virtual machine
  compiler.rs         — source → bytecode compiler
  builtin.rs          — built-in functions and Kala dispatch
  llm.rs              — LLM client and knowledge base
  khlm_polyglot.rs    — KhLM AI router + offline code generation
  jit_x86.rs          — x86-64 JIT compiler
  kore*.rs            — KORE binary columnar format
  nova*.rs            — Nova format and audio/video/image engines
  stdlib*.rs          — standard library implementations
  bin/                — standalone binaries
tests/                — integration test suites
benches/              — benchmarks (std-only, no extra crates)
examples/             — .killer source programs (the language you ship)
docs/                 — AI integration docs
```

## Writing `.killer` programs

New features, demos, and examples should be written in `.killer` files under `examples/`. The Rust code in `src/` is the runtime engine — not a second language for showcases.

```powershell
cargo run --bin killer_super -- examples/hello.killer --run
```

## Adding a new built-in function

1. Add the implementation in `src/builtin.rs` inside `BuiltinFunctions`
2. Register it in the dispatch table (search for `"fn_name"` patterns in `builtin_dispatch.rs`)
3. Add a test in `tests/` or extend an existing suite
4. Expose it in a `.killer` example under `examples/`

## Code style

- Rust edition 2021, `#![deny(unsafe_code)]` crate-wide — modules that need `unsafe` use a local `#![allow(unsafe_code)]`
- No clippy warnings in new code (run `cargo clippy`)
- Keep doc comments to one line — longer explanations belong in `docs/`
- No `unimplemented!()` or `todo!()` in code that will be called at runtime

## Pull requests

- One logical change per PR
- All tests must pass: `cargo test`
- Title format: `feat(scope): short description` or `fix(scope): short description`
- Reference the issue number if one exists

## Reporting bugs

Open an issue on GitHub with:
1. What you ran (command or `.killer` source)
2. What you expected
3. What actually happened (full error output)
4. OS and `cargo --version` / `rustc --version`

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
