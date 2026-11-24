# Panic audit

Measured with `python scripts/count_panics.py` (non-test code only: comments, string literals,
`#[cfg(test)]`/`#[test]` items, `tests/`, `benches/`, `archive/` ignored; parser-style `self.expect(..)` not counted).
Counted: `.unwrap()`, `.expect(`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`.

| | sites |
|---|---|
| before (commit 3108ba1) | 433 |
| after (this track) | 235 |

(An earlier hand count reported 440 non-test sites; the script counts `src/bin/*` and all modules, hence the different baseline.)

## What changed

* Every `.lock().unwrap()` / `.read().unwrap()` / `.write().unwrap()` on a Mutex/RwLock in non-test code now uses
  `unwrap_or_else(|e| e.into_inner())`, so a panic in one thread no longer cascades through a poisoned lock.
* Reachable panics fixed with regression tests: see `tests/panic_regress2.rs`, `tests/no_panics_decoders.rs`.

## Files whose count dropped

| file | before | after |
|---|---|---|
| ../../../../../wt_scratch2/base/src/killer_async.rs | 30 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/observable.rs | 27 | 0 |
| ../../../../../wt_scratch2/base/src/kore_v2.rs | 18 | 0 |
| ../../../../../wt_scratch2/base/src/circuit_breaker.rs | 15 | 0 |
| ../../../../../wt_scratch2/base/src/khlm_polyglot.rs | 15 | 0 |
| ../../../../../wt_scratch2/base/src/builtin.rs | 14 | 0 |
| ../../../../../wt_scratch2/base/src/bin/kore_profile.rs | 13 | 0 |
| ../../../../../wt_scratch2/base/src/vm.rs | 13 | 0 |
| ../../../../../wt_scratch2/base/src/bin/time_machine_api.rs | 12 | 0 |
| ../../../../../wt_scratch2/base/src/spark/memory.rs | 12 | 0 |
| ../../../../../wt_scratch2/base/src/bigint.rs | 11 | 0 |
| ../../../../../wt_scratch2/base/src/ml_module.rs | 11 | 0 |
| ../../../../../wt_scratch2/base/src/production.rs | 11 | 0 |
| ../../../../../wt_scratch2/base/src/compiler.rs | 10 | 0 |
| ../../../../../wt_scratch2/base/src/assassin_assist.rs | 9 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/data_fetch.rs | 9 | 0 |
| ../../../../../wt_scratch2/base/src/kore_engine.rs | 9 | 0 |
| ../../../../../wt_scratch2/base/src/bin/kore_bench.rs | 8 | 0 |
| ../../../../../wt_scratch2/base/src/cffi.rs | 8 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/di.rs | 8 | 0 |
| ../../../../../wt_scratch2/base/src/optimization/genetic_algorithm.rs | 8 | 0 |
| ../../../../../wt_scratch2/base/src/error.rs | 7 | 0 |
| ../../../../../wt_scratch2/base/src/wasm_compile.rs | 7 | 0 |
| ../../../../../wt_scratch2/base/src/http_server.rs | 6 | 0 |
| ../../../../../wt_scratch2/base/src/time_machine/event_log.rs | 6 | 0 |
| ../../../../../wt_scratch2/base/src/ai_workflow_engine.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/guardian.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/lang_builtins.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/net_sock.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/nova.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/spark/io_parallel.rs | 5 | 0 |
| ../../../../../wt_scratch2/base/src/bin/kore_nova_bench.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/bin/time_machine_cli.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/concurrency.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/reactive.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/test_bed.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/optimization/scaling.rs | 4 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/animation.rs | 3 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/csg3d.rs | 3 | 0 |
| ../../../../../wt_scratch2/base/src/llm.rs | 3 | 0 |
| ../../../../../wt_scratch2/base/src/net_tls_server.rs | 3 | 0 |
| ../../../../../wt_scratch2/base/src/vec_builtins.rs | 3 | 0 |
| ../../../../../wt_scratch2/base/src/android_audio.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/bin/ghost_vm.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/bin/killer_full_metrics.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/bin/killer_super.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/dap.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/formatter.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/ghost_hive.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/ghost_lang.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/ghost_world.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/hints.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/killer_serialization.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/animation3d.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/component.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/lazy_evaluation.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/nova_trit_codec.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/pkg_toml.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/time_machine/reconstructor.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/version.rs | 2 | 0 |
| ../../../../../wt_scratch2/base/src/aead.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/bin/nova_csv_bench.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/bin/use_cases.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/controlflow.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/csv_builtins.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/encryption.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/eval_builtins.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/ghost_vm.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/http_serve.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/image_gen.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/kala_ui.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/killer_10x.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/killer_cli.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/killer_super/perf_profiling.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/layout.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/killer_ui/renderer3d.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/log_builtins.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/mcp_minjson.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/optimization/incremental.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/optimization/production.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/optimization/rust_compiler.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/pkg_cli.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/polyglot.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/python/mod.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/spill_to_disk.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/sql_engine.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/stmt_parser.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/text_methods.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/time_machine/timeseries.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/vision.rs | 1 | 0 |
| ../../../../../wt_scratch2/base/src/vm_profile.rs | 1 | 0 |
| src/aead.rs | 0 | 1 |
| src/bin/nova_csv_bench.rs | 0 | 1 |
| src/bin/use_cases.rs | 0 | 1 |
| src/cffi.rs | 0 | 1 |
| src/concurrency.rs | 0 | 1 |
| src/controlflow.rs | 0 | 1 |
| src/csv_builtins.rs | 0 | 1 |
| src/encryption.rs | 0 | 1 |
| src/eval_builtins.rs | 0 | 1 |
| src/ghost_vm.rs | 0 | 1 |
| src/guardian.rs | 0 | 1 |
| src/http_serve.rs | 0 | 1 |
| src/killer_10x.rs | 0 | 1 |
| src/killer_cli.rs | 0 | 1 |
| src/killer_super/perf_profiling.rs | 0 | 1 |
| src/killer_ui/layout.rs | 0 | 1 |
| src/killer_ui/renderer3d.rs | 0 | 1 |
| src/llm.rs | 0 | 1 |
| src/log_builtins.rs | 0 | 1 |
| src/mcp_minjson.rs | 0 | 1 |
| src/optimization/incremental.rs | 0 | 1 |
| src/optimization/production.rs | 0 | 1 |
| src/optimization/rust_compiler.rs | 0 | 1 |
| src/pkg_cli.rs | 0 | 1 |
| src/polyglot.rs | 0 | 1 |
| src/python/mod.rs | 0 | 1 |
| src/spill_to_disk.rs | 0 | 1 |
| src/sql_engine.rs | 0 | 1 |
| src/stmt_parser.rs | 0 | 1 |
| src/text_methods.rs | 0 | 1 |
| src/time_machine/timeseries.rs | 0 | 1 |
| src/vm_profile.rs | 0 | 1 |
| src/bin/ghost_vm.rs | 0 | 2 |
| src/bin/killer_super.rs | 0 | 2 |
| src/dap.rs | 0 | 2 |
| src/formatter.rs | 0 | 2 |
| src/ghost_hive.rs | 0 | 2 |
| src/ghost_lang.rs | 0 | 2 |
| src/ghost_world.rs | 0 | 2 |
| src/hints.rs | 0 | 2 |
| src/killer_serialization.rs | 0 | 2 |
| src/killer_ui/animation3d.rs | 0 | 2 |
| src/killer_ui/component.rs | 0 | 2 |
| src/nova_trit_codec.rs | 0 | 2 |
| src/pkg_toml.rs | 0 | 2 |
| src/production.rs | 0 | 2 |
| src/time_machine/reconstructor.rs | 0 | 2 |
| src/version.rs | 0 | 2 |
| src/bin/time_machine_api.rs | 0 | 3 |
| src/killer_ui/animation.rs | 0 | 3 |
| src/killer_ui/csg3d.rs | 0 | 3 |
| src/net_tls_server.rs | 0 | 3 |
| src/vec_builtins.rs | 0 | 3 |
| src/bin/kore_nova_bench.rs | 0 | 4 |
| src/bin/time_machine_cli.rs | 0 | 4 |
| src/killer_ui/reactive.rs | 0 | 4 |
| src/killer_ui/test_bed.rs | 0 | 4 |
| src/optimization/scaling.rs | 0 | 4 |
| src/spark/io_parallel.rs | 0 | 4 |
| src/ai_workflow_engine.rs | 0 | 5 |
| src/killer_async.rs | 0 | 5 |
| src/lang_builtins.rs | 0 | 5 |
| src/net_sock.rs | 0 | 5 |
| src/nova.rs | 0 | 5 |
| src/builtin.rs | 0 | 6 |
| src/error.rs | 0 | 7 |
| src/wasm_compile.rs | 0 | 7 |
| src/bin/kore_bench.rs | 0 | 8 |
| src/optimization/genetic_algorithm.rs | 0 | 8 |
| src/compiler.rs | 0 | 10 |
| src/vm.rs | 0 | 10 |
| src/bigint.rs | 0 | 11 |
| src/ml_module.rs | 0 | 11 |
| src/bin/kore_profile.rs | 0 | 13 |
| src/kore_v2.rs | 0 | 16 |

## Budget

`tests/panic_budget.rs` fails if any file (or the total) has MORE sites than recorded in `tests/panic_budget.txt`,
and if any non-test `.lock().unwrap()` reappears. Lower the numbers when you remove sites.

## Remaining sites (what is left)

* Mostly invariant-guarded (`checked non-empty`, `non-zero modulus`, fixed-size `try_into` after a length check).
* kore_v2 encoder paths (`heap.pop().unwrap()` after explicit size checks) run on data the writer built, not on file input.
* Not covered: the JIT and hot VM loops (out of scope for this track), `src/bin/*` benchmark tools.
