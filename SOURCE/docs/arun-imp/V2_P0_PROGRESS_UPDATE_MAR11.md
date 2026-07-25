# V2 P0 Progress Update (Mar 11, 2026)

## Scope Completed

Implemented first parser/VM patch wave for V2 based on parity failures.

Files changed:

- `src/v2-rust/killer_vm/src/parser.rs`
- `src/v2-rust/killer_vm/src/vm.rs`

## What Was Fixed

1. Parser flexibility improvements
- Optional parentheses in `if`/`while` conditions.
- Single-statement body fallback for control-flow blocks.
- `catch(error)` and `catch error` parsing support.
- Property assignment support: `obj.field = value` and `this.field = value`.

2. Dict/object parsing/runtime improvements
- Dict literals now support unquoted keys: `{name: "Alice"}`.
- VM supports object field index read/write via string keys.
- Dict dynamic access supports `length`, `keys()`, `values()`, `entries()`.

3. Dynamic method support improvements
- String property/method support added: `.length`, `.upper()`, `.lower()`, `.charAt()`, `.substring()`, `.replace()`, `.split()`.
- Basic array dynamic support added: `.length`, `.push(...)`, `.pop()`, `.join(...)`.

4. Try/catch runtime handling
- Division/modulo by zero now route into active `try/catch` frames instead of always hard-failing.

## Targeted Re-test Results (Previously Failing 10 Files)

Pass:

- `07_dicts.killer`
- `10_try_catch.killer`

Still failing:

- `09_string_methods.killer`
- `11_classes.killer`
- `12_advanced_features.killer`
- `13_complete_features.killer`
- `14_more_features.killer`
- `15_phase1.killer`
- `16_phase2_oop.killer`
- `killer_showcase_examples.killer`

Current targeted improvement: `2 / 10` fixed in this patch wave.

## Remaining High-Priority Blockers

1. Advanced lexer/parser syntax
- `? :` ternary operator
- Arrow function assignment forms (example 15)
- Additional class syntax variants (example 16)
- Mixed indentation-style function bodies (showcase file)

2. Class runtime semantics
- Constructor/init behavior and field lifecycle still incomplete.
- Property access on null/missing fields still causing failures in class examples.

3. Method/property consistency
- Remaining mismatch in `09_string_methods.killer` (dict/array/property flow around `length`).

## Next Patch Wave Plan

1. Add lexer/parser support for ternary (`? :`) and arrow assignment expressions.
2. Add constructor invocation semantics for `new Class(...)` (`init` bridging).
3. Normalize property access behavior for null/non-object edge cases.
4. Re-run full parity report and refresh `docs/arun-imp/V1_V2_FEATURE_PARITY_VALIDATION.md`.
