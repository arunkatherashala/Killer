# Scorecard — path to 10.0 (living)

Single-page view of **where we are**, **what’s missing**, and **what to do next**.  
Machine-readable tasks: root `tracker.csv` (filter `area` or search `title`). Markdown checklist: `IMPROVEMENT_10_TRACKER.md`.

---

## Category scores

| Category | Score | Target | Primary gap | Unlocks ~9.5+ when… |
|----------|-------|--------|-------------|---------------------|
| **Language** | **8.5** | 10 | **`match` is not in `Stmt` / stmt_parser** — `Switch` exists; Rust-style `match` + `compile_match()` still absent. Second pipeline: `parser.rs` vs `lexer`+`stmt_parser`. | `Stmt::Match` (or `Expr::Match`) parsed, then `compile_match()` emitting compare + branch bytecode; parsers merged. |
| **Compiler / VM** | **9.0** | 10 | **NaN-boxing** wired through hot `Value` dispatch. **Dead stores**: slot liveness is conservative (no full CFG). | `nanbox` on stack/slots; optional CFG liveness for safer dead-store removal. |
| **Performance** | **9.5** | 10 | Interpreter still **threaded match**; JIT limited (counter-style loops). **Computed goto** (where supported) or **wider JIT** (non-counter loops, branches). | Benchmarked gain on representative `.killer` loops + if/else. |
| **Nova** | **9.5** | 10 | **Parallel encode** without allocator/cache line blowups; **streaming decode** (chunked / async-friendly). | Load test + correctness on wide tables. |
| **Innovation** | **9.5** | 10 | **Published** ternary + `Signal` + qubit story (paper or formal spec), not only demos. | Citable PDF or spec repo + examples linked from guide. |
| **AI** | **8.5** | 10 | **End-to-end scripted demo**: Ollama (or Tier-2) → model emits `TOOL_CALL` → engine runs tool → **structured result** parsed (JSON schema / typed slots), not only free text. **Structured output** from LLM layer. | One `scripts/` or CI-friendly flow that exits 0 with assertions on tool results. |
| **Tooling** | **9.5** | 10 | **`killer test`** runner (discover `test_*.killer`, run, count assert_* failures). **VS Code**: syntax highlighting + (later) LSP diagnostics. | `killer-native --test <dir>` (or equivalent) documented in QUICKSTART. |
| **Code quality** | **9.0** | 10 | **`cargo build --lib`** still reports **~27 warnings** (mix of dead_code, unused imports, etc.). **`cargo clippy -- -D warnings`** not clean. | Warning count → 0 on default `killer-native` lib build; clippy green. |
| **Documentation** | **9.5** | 10 | User guide **depth**: target **300+ lines**, one tutorial per major feature (control flow, data, AI, Nova, security). | New/expanded `KILLER_LANGUAGE_GUIDE.md` + `tutorials/` tree. |
| **Testing** | **9.5** | 10 | **cargo-fuzz** run in CI or documented repeatable command. **Coverage** (tarpaulin / `llvm-cov`). **Property tests** on lexer/parser invariants. | Fuzz job + coverage badge or threshold in workflow. |

---

## Already in tree (don’t re-plan from zero)

- **Security / capabilities**: `CapabilitySet`, `require_*`, VM `run` + spawn scopes (see `security.rs`, `vm.rs`).
- **Tool-calling plumbing**: `readFile` + `parse_json` in tool registry, `builtin_dispatch`, `examples/tool_calling_demo.killer`, `SOURCE/scripts/kala-setup.ps1`.
- **Demos**: `examples/trit_sensor_demo.killer` (trits + `Signal`).
- **Roadmap files**: `KILLER_GRAMMAR.ebnf`, `PERFORMANCE_ROADMAP.md`, `IMPROVEMENT_10_TRACKER.md`, `tracker.csv`.

---

## Highest-impact next moves (dependency-aware order)

1. **Language: `match` pipeline** — Add **`Match` token** + **`Stmt::Match`** { scrutinee, arms: (pattern | literal, guard?, body) } in **stmt_parser** (and lexer), *then* **`compile_match()`** in `compiler.rs`. Skipping straight to `compile_match()` without AST/parser leaves nothing to compile.
2. **Code quality** — Drive **lib warnings → 0**, then **`clippy -D warnings`** in small PRs (module-by-module). Fast credibility win for CI and contributors.
3. **Documentation** — Grow **user guide + tutorials** in lockstep with features (especially after `match` lands).
4. **AI** — **One scripted e2e**: e.g. PowerShell or CI job that starts Ollama (or skips if absent), runs a tiny `.killer` that calls `khlm_with_tools`, asserts stderr/stdout contains expected **tool result substring** or parses **JSON** tool payload. Add **structured output** helper in `tool_calling` or KhLM (parse model JSON into `Value`).
5. **Compiler** — NaN-boxing + CFG liveness (after match/if patterns stable).
6. **Performance** — Computed-goto or extended JIT after VM value representation stabilizes.
7. **Testing** — Fuzz + coverage as gates once clippy is under control.

---

## Revision notes (why this differs from a flat priority list)

- **`compile_match` first** is correct **only after** parser + AST exist; the scorecard is ordered that way here.
- **AI 8.5 → 9.0** is mostly **verification + structure** (script + parsing), not only more builtins.
- **“27 dead_code warnings”** matches **`cargo build --lib`** reporting **27 warnings** total for `killer-native` (not all are `dead_code`; triage per warning kind).

Update this file when scores move; mirror new line items into **`tracker.csv`** (`area=scorecard` or the specific `area`).
