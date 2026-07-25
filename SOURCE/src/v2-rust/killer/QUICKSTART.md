# Killer Native — quick start

**Super language = Killer; programs = `*.killer` — ready from now.** One public surface for logic; Rust in this crate is the **engine** (VM, bytecode, `killer_super`, tests). Positioning: [`SOURCE/docs/POSITIONING.md`](../../../docs/POSITIONING.md).

**Author only in `.killer` for what you ship.** Demos, trinary, stdlib samples: `examples/*.killer` ([`examples/README.md`](./examples/README.md)). Rust changes are for the runtime—not a second language for showcases.

Paths below use the crate root: `SOURCE\src\v2-rust\killer` (from repo root: `killer\SOURCE\src\v2-rust\killer`).

## “Language ready” manifest (pure Killer)

Index: [`examples/README.md`](./examples/README.md). Entry manifest (print when run on VM):

- `examples/killer_language_ready.killer`

| File | Role |
|------|------|
| `examples/hello.killer` | Baseline syntax and `println` / `kfn` (killer_super-friendly) |
| `examples/trinary_awake.killer` | Trinary + `killer_super` native path |
| `examples/trinary_in_killer.killer` | Trinary helpers + VM-oriented (`#` comments at top level) |
| `examples/trinary_kleene_table.killer` | Full Kleene AND/OR/NOT tables in Killer |

## Trit-first `.killer` demo

From this directory:

```powershell
cargo run --bin killer_super -- examples/trinary_awake.killer
```

Details: [`SOURCE/docs/TRIT_KILLER_FILES.md`](../../../docs/TRIT_KILLER_FILES.md).

## Windows (PowerShell)

```powershell
cd c:\path\to\killer\SOURCE\src\v2-rust\killer
cargo build
cargo test --lib
cargo test --test pipeline_conformance
cargo test --test trit_three_valued
cargo test --test ai_integration_tests --test ai_annotations_tests
cargo test --test knowledge_base_tests
```

Default `cargo test` runs **library + integration** tests. **Doc tests are disabled** in `Cargo.toml` (`[lib] doctest = false`) because many `///` examples are illustrative only. Legacy suites that depend on `killer_rcore` are **gated by features** (see below). Some stress benchmarks are **`#[ignore]`**; run them with `cargo test -- --ignored`.

### Revalidate anytime (local CI parity)

After compiler, VM, or builtin changes, run the same checks as `.github/workflows/killer-native.yml`, plus builtin/parser smoke:

```powershell
cd SOURCE\src\v2-rust\killer
pwsh -File scripts\revalidate_killer.ps1
```

Full integration suite (slow): `pwsh -File scripts\revalidate_killer.ps1 -Full`. Release build: add `-Release`.

Linux / macOS / Git Bash:

```bash
cd SOURCE/src/v2-rust/killer
chmod +x scripts/revalidate_killer.sh   # once
./scripts/revalidate_killer.sh            # or: ./scripts/revalidate_killer.sh --full
```

## MCP binary

Tools: `killer_version`, `killer_compile`, `killer_run` (subprocess: runs `.killer`, captures stdout/stderr), `killer_ollama_status`. Place `killer-native` next to `killer-mcp`, or set env `KILLER_NATIVE` to the executable path.

```powershell
cargo run --bin killer-mcp -- --help
```

## Optional / heavy suites

| Feature | Command |
|--------|---------|
| Legacy `killer_rcore` integration copies | Add optional path dep + `legacy-killer-rcore-tests = ["dep:killer_rcore"]` in `Cargo.toml` (see comment in that file), then `cargo test --features legacy-killer-rcore-tests`, **or** run those tests from the `killer_rcore` workspace/crate instead |
| CLI E2E (spawns nested `cargo run`) | `cargo test --features cli-e2e-tests --test integration_cli_e2e` |
| Cluster demo (needs `cluster_coordinator` in tree) | `cargo test --features cluster-demo-tests --test cluster_demo` |
| SuperProcessor 100k stress | `cargo test --test superprocessor_real_world_tests -- --ignored --nocapture` |

## Benchmarks (std-only timings, no extra deps)

```powershell
cargo bench --bench vm_runtime
cargo bench --bench ai_benchmark
```

Numbers print to stdout (µs/ms per iteration). Full strategy (fast / small / strong / load / simple): `SOURCE/docs/PERFORMANCE_ROADMAP.md`. Optional Criterion + HTML reports are described there if you add a `dev-dependency`.

## CSV size check (NOVA / KPAR / NOVZ / NOVT)

Generates a ~10 MiB CSV in `%TEMP%`, then prints sizes for **CSV→NOVZ**, **CSV→KORE** (`nova_write`), **CSV→KPAR** (`nova_write` + `nova_to_parquet`; Nova’s layout, not Apache Parquet), **KORE→NOVZ** (compress the `.kore` file), plus **NOVT** / **NOVD** demos and **NOVD→NOVZ** / **NOVT→NOVZ**.

```powershell
cargo run --release --bin csv_format_compare
```

## KORE / Nova speed + size benchmark (for sharing / recommendations)

Prints **bytes**, **% of CSV**, **encode ms**, **decode ms** for **CSV → NOVZ** (generic bytes, same class as gzip), **CSV → KORE**, **CSV → KPAR**, and **KORE → NOVZ** on the same ~10 MiB synthetic CSV. No extra crates — compare **gzip** / **zstd** on the same `.csv` externally and line up against the NOVZ row.

```powershell
cargo run --release --bin kore_nova_format_bench
```

## killer_ui (A–D parallel scaffold)

Rust module `killer_ui`: UI patch model (A), native window **stub** (B until eframe added), operator DAG + cook (C), multi-panel workspace (D).

```powershell
cargo run --release --bin killer_ui_demo
```

Headless UI as JSON over HTTP (Tier 3 web lane; default port `8787`):

```powershell
cargo run --bin killer_ui_serve -- 8787
# or: cargo run --bin killer_ui -- serve 8787
# bind all interfaces: cargo run --bin killer_ui -- serve 0.0.0.0 8787
# curl http://127.0.0.1:8787/health
# curl http://127.0.0.1:8787/killer-ui/headless.json
# curl http://127.0.0.1:8787/killer-ui/version.json
```

Tier 5 CLI (same HTTP + text demo + window stub): `cargo run --bin killer_ui -- help`

Tier 4 line sugar + builtins: `examples/ui_tier4_sugar.killer`. Sketch for full `ui { }` blocks: `examples/ui_parallel_sketch.killer`. **Web parallels** (Three.js / React / Angular / Node patterns): `killer_ui::web_stack`, full table in `SOURCE/docs/KILLER_UI_ENGINE.md`.

## Kala UI smoke (HTTP + browser checklist)

From this directory, start the chat UI, then run the script in a second terminal (or follow the manual checklist):

```powershell
cargo run --bin kala_smoke_server
# elsewhere:
.\scripts\kala-smoke.ps1
```

Details, bash script, and a short evaluation checklist: [`KALA_SMOKE.md`](./KALA_SMOKE.md).

## CI

GitHub Actions workflow: `.github/workflows/killer-native.yml` (same commands as the short list above). Optional: `.github/workflows/killer-bench.yml` (manual dispatch) to run benches on a clean runner.

## Language pipelines

See `SOURCE/docs/LANGUAGE_PIPELINE.md` for `run_killer_source` vs `run_killer_parsed` vs `compile_killer_ast`.
