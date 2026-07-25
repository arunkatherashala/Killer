# Track A to Track B Migration Plan

This note defines how Killer moves from the current Python-hosted runtime (Track A) to the native Rust runtime (Track B) without breaking existing users.

## Current State

- **Track A (Current/Stable)**
  - Python-hosted runtime and compiler path
  - Entry path: `killer` launcher using `main.py`
  - Full feature coverage today
- **Track B (In Progress)**
  - Native runtime under `native/killer_vm`
  - Entry path: `run-native.bat` / `killer-native`
  - Growing subset coverage with native execution

## Migration Principle

Track B becomes default only after parity gates are met. Until then:

- Keep Track A unchanged for all production usage.
- Build Track B in parallel with explicit compatibility checks.
- Promote Track B by feature gates, not by date.

## Safe Cutover Stages

1. **Stage 0: Parallel Development (now)**
   - Track A remains default.
   - Track B advances with subset + bytecode runtime.

2. **Stage 1: Feature Parity Matrix**
   - Build a matrix of core features (syntax, OOP, exceptions, modules, stdlib).
   - Mark each feature as: `A-only`, `A+B`, or `B-only`.

3. **Stage 2: Dual-Run Verification**
   - For selected programs, run both A and B.
   - Compare outputs and error behavior.
   - Fix B discrepancies until stable.

4. **Stage 3: Opt-In Native Mode**
   - Add CLI flag to current launcher for native runtime (for example `--native`).
   - Keep default as Track A.

5. **Stage 4: Native Default + Python Fallback**
   - Switch default to Track B once parity threshold is met.
   - Keep fallback mode to Track A for unsupported edge cases.

6. **Stage 5: Track A Decommission (optional, late)**
   - Only after long stabilization and compatibility confidence.

## Cutover Gates (Required)

Before switching default to Track B:

- Core language parity achieved for agreed feature set.
- Regression suite passes in native mode for target scope.
- Installer/CLI supports native runtime across Windows/macOS/Linux.
- Performance and stability baseline documented.
- Fallback path to Track A remains available during transition period.

## Immediate Next Actions

- Add explicit parity matrix doc (`core features -> A/B status`).
- Maintain Track B phase split doc (`docs/project/TRACK_B_PHASE1_PHASE2_FEATURES.md`).
- Add shared sample suite run in both tracks.
- Add native mode toggle in launcher once subset reaches agreed threshold.

## Outcome

This plan ensures no disruption: existing Killer users continue on Track A while Track B matures safely into the default runtime.
