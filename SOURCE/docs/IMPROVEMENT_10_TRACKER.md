# Tracker — 7.5–8 → 10 (language, VM, security, docs, quality)

Use this alongside `KILLER_ROADMAP_TO_10.md`. Check boxes as work lands.

**Machine-readable mirror:** update `tracker.csv` at the repo root whenever you change status here (columns: `entry_id,date,area,title,description,status,key_paths`; `status` = `done` | `pending` | `in_progress` | `n/a`).

**Score table (8.5–9.5 → 10):** see [`SCORECARD_TO_10.md`](./SCORECARD_TO_10.md) — category scores, gaps, dependency-aware priority order; rows **62–72** in `tracker.csv` under `area=scorecard`.

## Language design (~7.5 → 10)

- [x] Draft **EBNF** — `KILLER_GRAMMAR.ebnf` (includes **spec-ahead**: `match`, `async fn`, `spawn`, `import`, `switch`, `do-while`; parser/compiler catch up incrementally)
- [ ] **Merge** `parser.rs` (type lexer) with `lexer.rs` + `stmt_parser.rs` pipeline
- [ ] **Deprecate** duplicate keywords (`kfn` → `fn` migration note in guide)
- [ ] **`match`** wired from `ast::Pattern` to bytecode

## Compiler & VM (~8 → 10)

- [x] **Constant folding** — numeric/bool/compare in `optimizer::fuse_slot_patterns`
- [x] **Dead store → pop** — unused slot writes elided in optimizer (see `dead_store_unused_slot_becomes_pop`)
- [x] **Tail-call optimization** — `TailCall` opcode + compiler/VM path for self tail calls
- [ ] **NaN-boxing** — hot `Value` paths in `vm.rs` using `nanbox.rs`

## Security (~8 → 10)

- [x] **Unsafe audit** — `UNSAFE_AUDIT.md` + `#![deny(unsafe_code)]` + targeted allows
- [x] **Budget / capability types** — `ExecutionBudget`, `CapabilitySet` in `security.rs`
- [x] **Enforce** instruction + wall-clock limits in `VirtualMachine::run` (per-step budget + wall check on backward `Jump`)
- [x] **Builtin capability checks** — `require_*` on file/network/spawn/LLM/JIT-sensitive builtins; `CapabilityScopeGuard` for each `run` and spawn threads
- [ ] **Fuzz** lexer/parser — `FUZZING.md`
- [ ] **Default-deny** file access remains; document `--allow-all-paths` for power users only (replace `allow_unrestricted_file_access` default-on footguns)

## Documentation (~7.5 → 10)

- [x] **One guide** — `KILLER_LANGUAGE_GUIDE.md`
- [x] **Builtin index** — `BUILTIN_REFERENCE.md` + `scripts/gen-builtin-reference.ps1`
- [ ] **Archive** session logs under `DOCS/archive/sessions/` (see README there)

## Code quality (~7.5 → 10)

- [x] Remove **`*.bak`** from `src/` (AI provider backups)
- [ ] `cargo clippy -- -D warnings` (incremental fix per module)
- [ ] Delete or gate **stub** `phase_*` / empty enterprise modules after inventory (**note:** `phase_46_gpu_support.rs` / `phase_49_enterprise_security.rs` currently contain real code + tests — do not delete blindly)
- [x] **`#![deny(unsafe_code)]`** with explicit allows in JIT/FFI/VM/value

## AI integration (~8 → 10)

- [x] **Ollama setup script** — `SOURCE/scripts/kala-setup.ps1` (pull model + smoke + optional `killer-native` `kala_ask`)
- [x] **Tool-calling demo** — `examples/tool_calling_demo.killer` + `tool_calling_demo_data.json`; **readFile** + **parse_json** registered in `tool_calling` dispatcher (via `builtin_dispatch` to avoid `builtin`↔`tool_calling` cycle)

## Innovation / examples

- [x] **Trit + Signal sensor sketch** — `examples/trit_sensor_demo.killer`
