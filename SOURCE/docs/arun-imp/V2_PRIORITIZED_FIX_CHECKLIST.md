# V2 Prioritized Fix Checklist (Based on V1 Parity Validation)

Date: March 11, 2026
Source report: `docs/arun-imp/V1_V2_FEATURE_PARITY_VALIDATION.md`
Raw results: `docs/arun-imp/V1_V2_VALIDATION_REPORT.json`

## Goal

Raise V2 feature parity with V1 by fixing the 10 examples that pass in V1 but fail in V2.

Current status:

- V1 pass: 17/18
- V2 pass: 7/18
- Gap: 10 examples

## Priority Order

## P0 - Parser/Lexer blockers (highest impact)

These blockers stop many files from parsing at all.

1. Add ternary/operator token support (at least `?` path used by advanced examples)
- Symptoms: `12_advanced_features.killer` fails with lexer `unexpected character: '?'`
- Likely hotspot: `src/v2-rust/killer_vm/src/lexer.rs`
- Acceptance: `12_advanced_features.killer` parses and reaches runtime.

2. Fix function-call parsing edge cases for comma/complex args
- Symptoms: `15_phase1.killer` fails with `expected RParen, got Comma`; `13_complete_features.killer` fails with `expected RParen, got Identifier("x")`
- Likely hotspot: `src/v2-rust/killer_vm/src/parser.rs` in `parse_call`
- Acceptance: both files parse; no `RParen`/comma parser failures.

3. Support try/catch syntax variants used in examples
- Symptoms: `10_try_catch.killer` fails `expected LBrace, got LParen`
- Likely hotspot: `src/v2-rust/killer_vm/src/parser.rs` in `parse_try`
- Acceptance: `10_try_catch.killer` executes successfully.

4. Fix class/object assignment target handling
- Symptoms: `11_classes.killer` and `16_phase2_oop.killer` fail with `invalid assignment target`
- Likely hotspot: `src/v2-rust/killer_vm/src/parser.rs` in `parse_expr_statement`
- Acceptance: class examples parse and run.

5. Function declaration shape compatibility
- Symptoms: `killer_showcase_examples.killer` fails with `expected '=>', '{', or indented block after function parameters`
- Likely hotspot: `src/v2-rust/killer_vm/src/parser.rs` in `parse_function_body` and `looks_like_function`
- Acceptance: showcase file parses with current documented syntax forms.

## P1 - Runtime behavior mismatches

These parse, but behave differently at runtime.

6. Dict/object key access behavior alignment
- Symptoms: `07_dicts.killer` runtime error `Undefined variable 'name'`
- Likely hotspots:
- `src/v2-rust/killer_vm/src/parser.rs` (`Expr::MethodCall`/property handling)
- `src/v2-rust/killer_vm/src/vm.rs` (method/property dispatch for dict/object)
- Acceptance: `07_dicts.killer` runs and prints expected outputs.

7. String property/method compatibility (`.length`, method calls)
- Symptoms: `09_string_methods.killer` runtime error around `length` on string
- Likely hotspot: `src/v2-rust/killer_vm/src/vm.rs` method dispatch and builtin adaptation
- Acceptance: `09_string_methods.killer` passes without runtime type mismatch.

## P2 - Syntax broadening and consistency

These usually represent language-surface mismatches.

8. Colon and literal grammar edge cases in complex files
- Symptoms: `14_more_features.killer` parse error `expected Colon, got Number(90.0)`
- Likely hotspot: `src/v2-rust/killer_vm/src/parser.rs` in dict/object literal parsing and surrounding expression precedence
- Acceptance: `14_more_features.killer` parses and executes.

9. Cross-check parser and compiler assumptions
- Risk: parser emits AST forms that compiler/vm partially supports, causing runtime regressions after parse fixes
- Hotspots:
- `src/v2-rust/killer_vm/src/compiler.rs`
- `src/v2-rust/killer_vm/src/vm.rs`
- Acceptance: no new runtime regressions on currently passing 7 examples.

## P3 - Validation and release gate

10. Add parity test gate for V2
- Create a script to run all `examples/*.killer` on V2 and capture pass/fail counts.
- Keep V1 report as reference baseline.
- Acceptance gate for release candidate:
- Minimum target: V2 passes all 17 examples that currently pass in V1.
- Stretch target: both runtimes pass all 18 examples.

## Suggested Execution Sequence (Fastest Path)

1. Implement P0 items 1-5 in parser/lexer.
2. Run parity sweep once (expect parse errors to drop sharply).
3. Implement P1 runtime items 6-7.
4. Run parity sweep again.
5. Resolve remaining P2 item 8 and any compiler/vm assumptions (item 9).
6. Lock with P3 release gate (item 10).

## Quick Command Set for Re-validation

```powershell
# V1 reference run (standalone)
.\dist\v1-standalone\killer-v1.exe examples\01_hello.killer

# V2 run
.\src\v2-rust\killer_vm\target\release\killer-native.exe examples\01_hello.killer

# Full parity report command can be reused from previous validation session
# Output file: docs/arun-imp/V1_V2_VALIDATION_REPORT.json
```

## Definition of Done for "V2 has V1 features"

- All examples that pass in V1 also pass in V2.
- No parser errors on V1-passing corpus.
- No runtime type/property dispatch errors on string/dict/class examples.
- Validation report is regenerated and checked in with updated counts.
