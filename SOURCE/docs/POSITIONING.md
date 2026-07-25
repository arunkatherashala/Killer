# Killer native — positioning (what we claim vs what we prove)

## Killer language first — `.killer` is what you ship (super language, ready now)

**The product-facing “super language” is Killer: one syntax, one extension — `*.killer`.** Programs, demos, curriculum, and integrations should be authored as **`.killer`** from now on. The Rust codebase (`killer-native`) is the **runtime and toolchain** that loads, compiles, and executes that source—it is **not** where new product or teaching logic should live.

- **Ready now:** run and extend `SOURCE/src/v2-rust/killer/examples/*.killer` — start with `killer_language_ready.killer`, then `hello.killer`, trinary demos, fibonacci, stdlib suites.
- **Single story:** VM bytecode path + optional `killer_super` native emission—no second user-facing language.

## Primary story (one sentence)

**Killer** is the **super language** you author as **`*.killer`**, executed by **Killer native**: an embeddable **bytecode VM** (plus a growing **JIT / native** path), **optional AI and tooling**, and **first-class trits** (−1 / 0 / +1) for **three-valued logic** when that fits the problem—not a claim to be “#1 in the world” on every metric at once.

## Pillars (all of them, honestly)

| Pillar | Claim | Proof we use |
|--------|--------|----------------|
| **Fast** | Competitive on *our* hot paths | `cargo bench --bench vm_runtime` + before/after in perf PRs (`PERFORMANCE_ROADMAP.md`) |
| **Small** | Default crate avoids crates.io deps; slim embed path is a goal | `cargo tree`, release binary size snapshots (occasionally) |
| **Strong** | Correctness + security knobs | `cargo test --lib`, `pipeline_conformance`, `security` tests |
| **Heavy load** | Queues, batches, spill paths exist | Stress tests behind `#[ignore]` / features; bounded APIs where it matters |
| **Simple** | One documented happy path | `QUICKSTART.md`, `LANGUAGE_PIPELINE.md` |
| **Trits / “ternary”** | **Balanced trits** in `Value`, VM opcodes, and builtins—not just `?:` | `tests/trit_three_valued.rs` + this section |

## Trits: advantage, not magic

- **Differentiator:** Few language VMs ship **native trit values and opcodes** (`TritAnd` / `TritOr` / …) alongside normal scalars.
- **Not automatic win:** Most everyday code is fine with **bools**; trits help when you want **explicit unknown** or Kleene-style **min/max** composition without encoding three states as ad-hoc integers.
- **Syntax note:** The **ternary operator** `a ? b : c` is ordinary sugar; the **interesting** part is **`Value::Trit`** and **`trit_*` builtins**.

## What we do *not* claim here

- Universal “fastest language on Earth” without named competitors and reproducible benches.
- Simultaneous minimality and maximal feature count without tradeoffs (we use **features** and optional crates for that).

## Trinary + `.killer` (new track, same VM)

Show trits in ordinary **`.killer`** scripts — no parallel file format. Start here: [`TRIT_KILLER_FILES.md`](./TRIT_KILLER_FILES.md), `examples/killer_language_ready.killer` (manifest), and `examples/trinary_awake.killer` in the killer-native crate.

## Related docs

- `LANGUAGE_PIPELINE.md` — how source becomes bytecode.
- `PERFORMANCE_ROADMAP.md` — speed, size, load, measurement habits.
- `TRIT_KILLER_FILES.md` — running trit-first `.killer` examples.
- `SOURCE/src/v2-rust/killer/QUICKSTART.md` — build, test, bench commands.
