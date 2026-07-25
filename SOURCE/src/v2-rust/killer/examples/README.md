# Killer examples — canonical `.killer` programs

**Killer is the language you ship; these files are the source.** The Rust crate in this directory is the runtime and tools. New demos, trinary logic, stdlib samples, and “language ready” proof belong here as **`*.killer`**.

## Run this first

| File | How |
|------|-----|
| [`killer_language_ready.killer`](./killer_language_ready.killer) | Manifest: print this via `run_killer_source` / `compile_killer_default` (VM) to see the map of examples. |
| [`hello.killer`](./hello.killer) | `cargo run --bin killer_super -- examples/hello.killer --run` — baseline `kfn` + `println`. |
| [`trinary_awake.killer`](./trinary_awake.killer) | `cargo run --bin killer_super -- examples/trinary_awake.killer --run` — trinary + native emission path. |

## Trinary / Kleene (all Killer)

| File | Notes |
|------|--------|
| [`trinary_in_killer.killer`](./trinary_in_killer.killer) | VM-oriented: use **`#`** for top-level comments. |
| [`trinary_kleene_table.killer`](./trinary_kleene_table.killer) | Full 3×3 AND/OR + NOT tables. |

## killer_ui (sketch; engine is Rust today)

| File | Notes |
|------|--------|
| [`ui_builtin_demo.killer`](./ui_builtin_demo.killer) | Runnable VM demo: `ui_core_version`, `ui_headless_tick`, `ui_headless_snapshot_json`, `ui_native_window`. `cargo run --bin killer-native -- examples/ui_builtin_demo.killer` |
| HTTP JSON (Tier 3) | **`killer_ui_serve`** / **`killer_ui serve`** — `GET /health`, headless + version JSON; optional `[HOST] PORT`. |
| [`ui_tier4_sugar.killer`](./ui_tier4_sugar.killer) | Tier 4: line sugar (`ui version`, `v = ui snapshot`) + `ui_help()`. `cargo run --bin killer-native -- examples/ui_tier4_sugar.killer` |
| [`ui_parallel_sketch.killer`](./ui_parallel_sketch.killer) | Comment-only sketch of future `ui.*` / `graph.*` blocks; run `cargo run --release --bin killer_ui_demo` for the Rust scaffold. |

## Python-style list helpers

| File | Notes |
|------|--------|
| [`stdlib_pythonic.killer`](./stdlib_pythonic.killer) | Python-like: `sorted`, `sum`, `enumerate`, `all`, `any`, `zip`, `reversed`, `copy`, `get`, `setdefault`. |

## More samples in this folder

Fibonacci variants, stdlib (`stdlib_*.killer`), arrays, dicts, subsets — all **`.killer`**. Build/test commands: [`../QUICKSTART.md`](../QUICKSTART.md). Positioning: [`../../../docs/POSITIONING.md`](../../../docs/POSITIONING.md).
