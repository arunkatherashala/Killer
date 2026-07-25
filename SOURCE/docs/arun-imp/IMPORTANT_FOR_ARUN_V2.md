# Important Notes for Arun (V2 Native Runtime)

Date: March 11, 2026
Project: Killer

## V2 Snapshot

Rust V2 is the native runtime path (`killer-native.exe`) and is intended to be fully standalone (no Python dependency for end users).

## Build Steps (Windows)

```powershell
cd src\v2-rust\killer_vm
cargo build --release
```

Expected output binary:

- `src/v2-rust/killer_vm/target/release/killer-native.exe`

## Run Command

```powershell
.\src\v2-rust\killer_vm\target\release\killer-native.exe program.killer
```

## Current Verified Status (From Latest V2 Reports)

### Working
- Lexer newline issue is fixed.
- Single-argument function calls are working in basic tests.
- Basic variables, assignment, and arithmetic execute.

### Known Parser Gaps
- Multi-argument calls can fail (example: `print("x =", 10)`).
- Parser compatibility is partial for broader example coverage.
- Some advanced syntax paths still need parser enhancements.

## Practical Recommendation

- Use V1 standalone (`dist/v1-standalone/killer-v1.exe`) for broad compatibility now.
- Continue V2 parser work for full feature parity and long-term default runtime.

## Suggested Next Engineering Tasks (V2)

1. Add parser support for comma-separated call arguments.
2. Re-run full `examples/*.killer` compatibility matrix.
3. Document pass/fail list in a dedicated V2 test report.
4. Tag release once V2 parser reaches parity target.

## Quick Sanity Test (V2)

```powershell
.\src\v2-rust\killer_vm\target\release\killer-native.exe examples\01_hello.killer
```

If this fails on comma-separated `print` calls, use V1 standalone until parser patch lands.
