# Killer performance roadmap — “all dimensions”

You asked to optimize **everything at once**: fastest, smallest, strongest, heaviest load, simplest UX, and benchmarks that **you** own and improve over time. Those goals **overlap but also trade off**. This file maps each dimension to **what we measure** and **what we change in code**, so progress stays honest and reproducible.

## 1. Fastest (latency & throughput)

| What | How we measure | Implementation levers |
|------|----------------|----------------------|
| Compile small programs | `cargo bench --bench vm_runtime` — prints µs/iter for trivial compile + AST path | Bytecode builder hot paths, fewer allocations in `compiler.rs` / lexer |
| VM steady state | Same bench: `vm_run_counting_loop_500` | Nanboxed values, instruction dispatch, JIT hot paths (`jit_x86.rs`), opcode fusion |
| Startup | Time to first useful bytecode (add a bench if needed) | Lazy init of heavy modules, `#[cfg(feature)]` for optional subsystems |

**Rule:** “World-class” only counts with **pinned toolchain + machine notes** (or fixed CI runner) and **before/after diffs** in commit messages.

## 2. Smallest (binary / deps / RAM)

| What | How we measure | Implementation levers |
|------|----------------|----------------------|
| Release binary | `cargo build --release` + `llvm-size` / `cargo bloat --release` | `strip`, LTO (already in `Cargo.toml`), fewer `pub` modules if embedding |
| Dependency graph | `cargo tree` — killer-native stays **zero default crates.io deps** | Keep `killer_rcore` optional; no accidental `serde` on hot path |
| Idle RSS | Process RSS after `VirtualMachine::new()` | Don’t initialize global caches until first use |

**Tradeoff:** Smallest artifact often means **fewer features compiled in** — use Cargo **features** for “slim” vs “full”.

## 3. Strongest (correctness, safety, integrity)

| What | How we measure | Implementation levers |
|------|----------------|----------------------|
| Regressions | `cargo test --lib` + `pipeline_conformance` + targeted integration | Expand conformance tests per opcode / pipeline |
| Security | `security` module tests + path / recursion policy | Fuzz `validate_file_path`, VM stack bounds |
| Replay / audit | Time-machine event log tests | Tie critical VM transitions to append-only history where product needs it |

## 4. Heaviest load (throughput under stress)

| What | How we measure | Implementation levers |
|------|----------------|----------------------|
| Queue / batch paths | SuperProcessor + `distributed_queues` benches (add Criterion groups later) | Bounded queues, **backpressure**, batch sizing |
| Soak | Long-running job + memory curve | Spill-to-disk policies, cap in-flight work |

**Tradeoff:** Infinite “simple” APIs without limits **fail** under load; expose **defaults with safe bounds** and document tuning.

## 5. Simplicity (for users and maintainers)

| What | How we measure | Implementation levers |
|------|----------------|----------------------|
| One happy path | `QUICKSTART.md` + single `run_killer_source` story | Fewer public entrypoints; docs match code |
| Pipeline clarity | `LANGUAGE_PIPELINE.md` | Parser vs default compiler spelled out |

## 6. Benchmarks “broken only by Killer”

Meaning: **Killer owns the official harness** and improves **its own baselines** every release.

- **Canonical benches:** `benches/vm_runtime.rs`, `benches/ai_benchmark.rs` — **no extra crates.io deps** (works offline); stdout timings you can paste into release notes.
- **Optional Criterion:** add `criterion` as a `dev-dependency` and swap harnesses if you want HTML regression graphs on machines that can reach crates.io.
- **Process:** each perf PR cites before/after lines from `cargo bench --bench vm_runtime`.
- **CI:** `.github/workflows/killer-bench.yml` — **workflow_dispatch** only; default PR CI stays fast (`.github/workflows/killer-native.yml`).

## Commands

```bash
cd SOURCE/src/v2-rust/killer
cargo bench --bench vm_runtime
cargo bench --bench ai_benchmark
```

## Native GGUF inference (`killer-native --model`)

**Tokens/sec** is not printed automatically today — measure manually so numbers stay honest:

1. Build release: `cd SOURCE/src/v2-rust/killer && cargo build --release --bin killer-native`
2. Run twice (cold vs warm cache) and use a wall clock:
   - `killer-native --model qwen2.5 "Write a short paragraph about trits."`
3. Note model path, prompt length, output length, and **elapsed seconds** in this file or release notes.
4. Rough tok/s ≈ `(output_chars / 4) / seconds` unless you count tokens from the inference engine.

Optional: after `scripts/kala-setup.ps1`, compare the same prompt through **Ollama** vs **native GGUF** on the same machine.

## Honest summary

You can push **all** axes over time, but not with one magic switch. Use this doc to pick **which number moves** each sprint (e.g. “10% faster VM loop” or “−200KB binary”) and prove it with the benches above.

**Habit:** Occasionally record **release binary size** (`cargo build --release` + your preferred size tool) in release notes when you care about the “small” pillar—same honesty as wall-clock benches.

## Alignment with positioning

See [`POSITIONING.md`](./POSITIONING.md) for how fast / small / strong / load / simple / trits fit one narrative without overclaiming.
