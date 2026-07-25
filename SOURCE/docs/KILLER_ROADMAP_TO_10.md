# Killer — roadmap to strong 10/10 across categories

This refines the original improvement plan with **spelling fixes**, **measurable exit criteria**, and **dependency order**. Treat it as a working backlog, not a promise of delivery dates.

## Executive snapshot

| Area | Rough today | North star | Single biggest unlock |
|------|-------------|------------|------------------------|
| Language | ~7.5 | Formal grammar + one parser pipeline | Single grammar + one AST path |
| Compiler / VM | ~8 | IR + real folds + dead-code | Constant fold + dead stores |
| Performance | ~8.5 | NaN-boxing + better JIT regions | NaN-boxing in hot `Value` paths |
| Nova | ~9 | Correct parallel encode + spec | Fix parallel encoding contention |
| Security | ~8 | Default-deny + caps + limits | Capability struct on VM |
| Innovation | ~9 | Demos that *use* trits / signals | One flagship trit demo |
| AI / Kala | ~6.5 | One provider, end-to-end, honest UX | Ollama “2-minute setup” + real tools |
| Tooling | ~7 | LSP + fmt + test runner | Diagnostics + go-to-def |
| Docs | ~7 | One guided guide + changelog | `KILLER_LANGUAGE_GUIDE.md` |
| Testing | ~7 | CI + unit tests on core | `lexer`/`compiler`/`vm` `#[test]` |

**Kala UI (2026-03):** Voice Studio is a **single large panel** with a **Three.js AI point-cloud head** (cyan vertices + wireframe + soft glow), inspired by classic “neural mesh” visuals. **Live camera is hidden** but still drives subtle head motion and wave hints; needs network once for the Three.js module.

---

## Wave 1 — Foundations (credibility)

Priorities: automated benchmarks + CI, `clippy -D warnings` triage, one user guide, root `README`, delete obvious dead trees (`.bak`, empty phase stubs).

**Exit criteria:** CI runs `cargo test` + smoke scripts green on `main`; new contributor can build and run hello-world from README only.

---

## Wave 2 — Core runtime (speed + correctness)

Priorities: merge parser story toward one pipeline, NaN-boxing for numeric/bool/null, constant folding + dead-variable elimination, fix Nova parallel encode, `unsafe` audit with documented exceptions.

**Exit criteria:** Published micro-bench CSV shows no regression vs baseline; Nova parallel encoder beats sequential on multi-core fixtures.

---

## Wave 3 — Ecosystem (adoption)

Priorities: LSP diagnostics + go-to-definition + hover, `killer fmt` / `killer test` / `killer init`, VS Code extension polish, **Kala + Ollama** path documented and scripted.

**Exit criteria:** Fresh machine: `kala_setup` → working `kala_ask` against local model in under 10 minutes.

---

## Wave 4 — Polish (specs and trust)

Priorities: EBNF/PEG grammar in-repo, Nova on-disk spec + checksums, fuzz targets for parser/VM, short whitepaper or blog on trit/signal design.

**Exit criteria:** External contributor can implement a third-party Nova reader from the spec alone.

---

## Category notes (abbreviated)

- **Language:** Pick canonical keywords (`fn` vs `kfn`); wire `match` / destructuring for existing `Pattern` AST nodes.
- **Compiler:** TCO for `return f(args)` tails; optional TAC IR before bytecode.
- **Performance:** Expand JIT beyond two patterns; string interning; consider threaded dispatch after profiling proves win.
- **Nova:** Streaming encoder; shared string dictionary across columns; FSE/Huffman trade study documented.
- **Security:** Heap cap + wall-clock timeout + fuzz; WASM story where it removes whole classes of bugs.
- **AI:** Remove or relabel placeholder “ML” modules; ship one tool-calling demo (read → execute → reply loop).
- **Tooling:** REPL history + completion; package install via git URL.

---

## How to use this doc

1. Pick **one wave**; finish exit criteria before declaring “10/10” in that slice.  
2. For Kala, **measure** answer routing (code vs prose vs web) with fixed prompt suites — separate from Smoke (`KALA_SMOKE.md`).  
3. Keep session diaries out of `docs/`; use `archive/` or git history.

**Checklist with checkboxes:** `IMPROVEMENT_10_TRACKER.md` (language, VM, security, docs, quality).
