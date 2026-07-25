# V1 vs V2 Feature Parity Validation

Date: March 11, 2026
Scope: Validate whether V2 currently supports all features covered by V1 examples.

## Validation Method

Ran all `.killer` files under `examples/` on both runtimes:

- V1 standalone: `dist/v1-standalone/killer-v1.exe`
- V2 native: `src/v2-rust/killer_vm/target/release/killer-native.exe`

Machine-readable raw reports:

- Initial baseline: `docs/arun-imp/V1_V2_VALIDATION_REPORT.json`
- After wave 2: `docs/arun-imp/V1_V2_VALIDATION_REPORT_AFTER_WAVE2.json`

## Summary (Final - After Wave 3)

- Total examples tested: 18
- V1 pass count: 17
- V2 pass count: 17
- Both pass (parity): 17
- V1 pass but V2 fail: 0
- Both fail: 1

## Conclusion

V2 now matches all V1-passing examples in `examples/`.

Current parity is `17/18` on the full example set and `17/17` against the V1-passing corpus.

## Parity Achieved (All V1-Passing Files)

- `01_hello.killer`
- `02_conditionals.killer`
- `03_loops.killer`
- `04_calculator.killer`
- `05_functions.killer`
- `06_arrays.killer`
- `07_dicts.killer`
- `08_for_loops.killer`
- `09_string_methods.killer`
- `10_try_catch.killer`
- `11_classes.killer`
- `12_advanced_features.killer`
- `13_complete_features.killer`
- `14_more_features.killer`
- `15_phase1.killer`
- `16_phase2_oop.killer`
- `killer_showcase_examples.killer`

## Remaining Non-Parity Case

- `killer-v2-showcase.killer` fails on both V1 and V2 (not a parity blocker).

## Raw Reports

- Initial baseline: `docs/arun-imp/V1_V2_VALIDATION_REPORT.json`
- After wave 2: `docs/arun-imp/V1_V2_VALIDATION_REPORT_AFTER_WAVE2.json`
- Final: `docs/arun-imp/V1_V2_VALIDATION_REPORT_FINAL.json`
