# Platform support

| Target | Status |
|---|---|
| x86_64-pc-windows-msvc | Built and tested locally and in CI (windows-latest): all tests, parity probes and doc examples pass |
| x86_64-unknown-linux-gnu | Built and tested in CI (ubuntu-latest): all tests, parity probes and doc examples pass |
| aarch64-apple-darwin | Built and tested natively in CI (macos-latest is an Apple Silicon runner); interpreter only, the x86-64 JIT is inactive there |
| aarch64-unknown-linux-gnu | Type-checked only (`cargo check --lib --bins --tests`, in CI) |
| x86_64-unknown-linux-musl | Type-checked only (in CI) |
| x86_64-apple-darwin | Type-checked only (in CI); not run |

Last verified on GitHub Actions on 2026-10-18 at commit `b3de537` (workflows `killer-native` and `Killer CI`, all jobs green).

"Type-checked" means the code compiles for that target; it was not linked or run there.

## Notes

- The x86-64 JIT (`jit_x86.rs`, `jit_fn.rs`) is only active on x86-64; other architectures use the interpreter.
- Anonymous executable memory uses `libc` constants (`MAP_ANON` differs between Linux and macOS). On macOS the JIT retries with `MAP_JIT` if a plain RWX mapping is refused.
- Helper scripts (`parity/run_parity.py`, `docs/check_examples.py`, `bench/run.py`) pick `killer_super.exe` on Windows and `killer_super` elsewhere; set `KILLER_BIN` to override.
- To re-check a target locally: `rustup target add <target>` then `cargo check --lib --bins --tests --target <target>`. Use a separate `CARGO_TARGET_DIR` if disk space is tight.
