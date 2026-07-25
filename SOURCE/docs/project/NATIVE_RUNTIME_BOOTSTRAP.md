# Killer Native Runtime Bootstrap

This document starts the independent (non-Python-hosted) Killer runtime track.

For safe production transition strategy, see: `docs/project/TRACK_A_TO_B_MIGRATION.md`.

## What is added

- Native Rust runtime scaffold at `src/v2-rust/killer_vm`
- Minimal bytecode parser + VM execution loop
- CLI binary target: `killer-native`
- Example bytecode programs in `src/v2-rust/killer_vm/examples`
- Prerequisite validator script: `src/v2-rust/killer_vm/scripts/check-prereqs.ps1`

## Build and run

Prerequisite: install Rust toolchain (`rustc` + `cargo`).

Windows (recommended):

```powershell
winget install Rustlang.Rustup
```

Then open a new terminal and verify:

```powershell
cargo --version
```

Optional prerequisite checker:

```powershell
powershell -ExecutionPolicy Bypass -File native\killer_vm\scripts\check-prereqs.ps1
```

If build fails with `LNK1181: cannot open input file 'dbghelp.lib'`:

- Open Visual Studio Build Tools installer
- Ensure these are installed:
	- Desktop development with C++
	- Windows 10 SDK or Windows 11 SDK (must include x64 libraries)

From project root:

```powershell
cd native\killer_vm
cargo run -- --help
cargo run -- --version
cargo run -- examples\hello.kbc
cargo run -- examples\math.kbc
cargo run -- examples\variables.kbc
cargo run -- examples\branching.kbc
cargo run -- examples\scope.kbc
cargo run -- examples\function_call.kbc
cargo run -- examples\function_named.kbc
cargo run -- examples\function_args.kbc
cargo run -- --killer examples\subset_demo.killer
cargo run -- --killer examples\subset_control_flow.killer
cargo run -- --killer examples\subset_conditionals.killer
cargo run -- --killer examples\subset_logical.killer
cargo run -- --killer examples\subset_break_continue.killer
cargo run -- --killer examples\subset_functions.killer
```

Windows reliable helper (uses VS Dev Command + cargo path):

```powershell
cd native\killer_vm
.\run-native.bat
.\run-native.bat examples\hello.kbc
.\run-native.bat examples\function_call.kbc
.\run-native.bat --killer examples\subset_demo.killer
.\run-native.bat --killer examples\subset_control_flow.killer
.\run-native.bat --killer examples\subset_conditionals.killer
.\run-native.bat --killer examples\subset_logical.killer
.\run-native.bat --killer examples\subset_break_continue.killer
.\run-native.bat --killer examples\subset_functions.killer
```

## Native Killer subset mode

The native runtime now supports a first direct Killer-source execution path:

- CLI mode: `--killer <file.killer>`
- Supported subset:
	- `let name = expression;`
	- `name = expression;` (reassignment)
	- `print(expression);`
	- `if (condition) { ... }`
	- `if (...) { ... } else { ... }`
	- `while (condition) { ... }`
	- `break;` and `continue;` inside `while` loops
	- `fn name(params) { ... }` function declarations
	- `return expression;` inside functions
	- function calls in expressions/statements: `add(5, 3)`
	- expression types: numbers, strings, booleans, variable names, and basic arithmetic (`+ - * /`)
	- comparison operators for conditions: `==`, `!=`, `>`, `<`, `>=`, `<=`
	- logical operators: `&&`, `||`

## Current instruction set

- `CONST_STR "text"`
- `CONST_NUM <number>`
- `CONST_BOOL <true|false>`
- `LABEL <name>`
- `FUNC <name> [arity]`
- `ENTER_SCOPE`
- `EXIT_SCOPE`
- `STORE <name>`
- `LOAD <name>`
- `ADD`
- `SUB`
- `MUL`
- `DIV`
- `EQ`, `NE`, `GT`, `LT`
- `JUMP <instruction_index|label>`
- `JUMP_IF_FALSE <instruction_index|label>`
- `CALL <instruction_index|function_label> [arg_count]`
- `RET`
- `POP`
- `PRINT`
- `HALT`

## Example programs

- `examples/hello.kbc`
- `examples/math.kbc`
- `examples/variables.kbc`
- `examples/branching.kbc`
- `examples/scope.kbc`
- `examples/function_call.kbc`
- `examples/function_named.kbc`
- `examples/function_args.kbc`

## Why this matters

- Removes dependency on Python for this execution path.
- Establishes the VM architecture base for full Killer runtime migration.
- Enables staged replacement of current Python-hosted interpreter.
- Adds function arguments via `arg0`, `arg1`, ... with arity validation.
- Adds first direct `.killer` execution path on native VM via `--killer` subset compiler.
- Adds loop control statements (`break`, `continue`) aligned with advanced examples.
- Adds native subset functions (`fn`, parameters, calls, `return`) aligned with core function examples.

## Next milestones

1. Add named parameter metadata (beyond `arg0`, `arg1`) in bytecode.
2. Add object/class model parity.
3. Add direct compile pipeline from Killer AST to native bytecode.
4. Add test harness for bytecode VM regression suites.
