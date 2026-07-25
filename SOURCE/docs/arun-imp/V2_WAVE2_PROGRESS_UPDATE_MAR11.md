# V2 Wave 2 Progress Update (Mar 11, 2026)

## Summary

Second patch wave completed and validated.

Post-wave2 parity numbers:

- Total examples: 18
- V1 pass: 17
- V2 pass: 13
- V1/V2 parity: 13
- Remaining V1-pass but V2-fail: 4

Raw report:

- `docs/arun-imp/V1_V2_VALIDATION_REPORT_AFTER_WAVE2.json`

## What Improved in Wave 2

### Parser/Lexer
- Added `?` token in lexer.
- Added ternary expression parsing (`cond ? a : b`).
- Added arrow assignment sugar parsing:
  - `f = (x, y) => x + y`
  - `f = x => x * x`
- Updated function arrow syntax to return expression by default.
- Added class method modifiers support for parsing `static/get/set` forms.
- Added template placeholder expression parsing in backticks, not only identifiers.
- Added fallback single-statement function body parsing for indentation-style function forms.

### Compiler
- Added bytecode generation for ternary expressions.
- Added implicit return of final expression in function/method bodies.

### VM Runtime
- Added static class method dispatch (`ClassName.method()` calls on `Value::Class`).
- Added lightweight constructor/init field assignment on `new Class(...)` for common `this.field = param` patterns.
- Fixed dynamic property access control flow bug that re-ran the same instruction and caused cascading method errors.

## Newly Passing Examples

The following high-impact examples now pass in V2:

- `09_string_methods.killer`
- `11_classes.killer`
- `15_phase1.killer`
- `16_phase2_oop.killer`

(And previous wave already fixed `07_dicts.killer` and `10_try_catch.killer`.)

## Remaining Failing Examples (V1 passes, V2 fails)

1. `12_advanced_features.killer`
- Current failure: parse issue around increment/decrement and compound operators (`unexpected token: Plus`).

2. `13_complete_features.killer`
- Current failure: parse mismatch in advanced call/expression form (`expected RParen, got Identifier("x")`).

3. `14_more_features.killer`
- Current failure: parse mismatch around switch/case/object syntax (`expected Colon, got Number(90.0)`).

4. `killer_showcase_examples.killer`
- Current failure: parse issue around arrow expression in showcase script (`unexpected token: Arrow`).

## Next Patch Wave Focus

1. Add parser support for `++`, `--`, and compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`).
2. Add parser/runtime support for `switch/case/default` and `do-while` surface syntax.
3. Fix remaining arrow-expression edge case in showcase file.
4. Re-run full parity and target `17/17` against V1-passing corpus.
