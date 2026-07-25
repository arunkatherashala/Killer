# `.killer` files and trinary (trit) logic

Killer source uses the **`.killer`** extension. **Trinary** here means **balanced trits** built into the runtime: `int_to_trit`, `trit_and`, `trit_or`, `trit_not`, `trit_word`, tryte helpers, and VM opcodes — not a separate language fork.

Full **`examples/`** index (all `.killer`): [`../src/v2-rust/killer/examples/README.md`](../src/v2-rust/killer/examples/README.md). Language-wide positioning: [`POSITIONING.md`](./POSITIONING.md).

## Flagship examples (all `.killer`)

- [`trinary_awake.killer`](../src/v2-rust/killer/examples/trinary_awake.killer) — **`kfn` + `println`**, Kleene words (`yes` / `maybe` / `no`); best with **`killer_super`**.
- [`trinary_in_killer.killer`](../src/v2-rust/killer/examples/trinary_in_killer.killer) — **`print` + `kfn` policy-style** trits; use **`#`** for top-level line comments so the **default VM** parser accepts the file.
- [`trinary_kleene_table.killer`](../src/v2-rust/killer/examples/trinary_kleene_table.killer) — full **3×3 AND/OR** and **NOT** tables, only builtins—no Rust.
- [`killer_language_ready.killer`](../src/v2-rust/killer/examples/killer_language_ready.killer) — **manifest**: reminds that **Killer language = `.killer`** and lists entry demos.

Run **`trinary_awake.killer`** from the **killer-native** crate root:

```bash
cargo run --bin killer_super -- examples/trinary_awake.killer
```

(On Windows PowerShell, same command from `SOURCE\src\v2-rust\killer`.)

VM-oriented scripts (`trinary_in_killer.killer`, `trinary_kleene_table.killer`, `killer_language_ready.killer`) run through **`compile_killer_default`** / `run_killer_source` (see `LANGUAGE_PIPELINE.md` and integration tests).

## Principles (aligned with “new track, don’t break old”)

- Existing `.killer` examples and pipelines stay valid.
- Trit-heavy programs are **just more .killer** using `trit_*` / `T_POS` / `T_ZERO` / `T_NEG` and optional tryte APIs.
- Proof in tree: `tests/trit_three_valued.rs` + [`POSITIONING.md`](./POSITIONING.md).

## Persistence note

RAM and disk are still **bytes**; “store in trinary” means **encode** trits (JSON, packed trytes, or text). See the earlier guidance in chat — implement save/load as a **layer**, not a VM break.
