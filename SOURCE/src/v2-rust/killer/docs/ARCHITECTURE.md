# Architecture guide for contributors

Where things live in the `killer-native` crate (`SOURCE/src/v2-rust/killer`) and how a program gets from
text to output. Every statement here was checked against the source at the time of writing; file and
function names are given so you can check again. For the user-facing language see
[LANGUAGE.md](LANGUAGE.md). Zero external crates (apart from `libc` on Unix) is a project rule.

## Overview

```
.killer text
   |  compile_killer_subset(source)                       src/compiler.rs
   |    limits -> imports -> `//` hint -> testfw -> typecheck -> controlflow -> lambda
   |    -> sugar -> indentation -> polyglot -> ui sugar -> normalize_lines
   |    -> compile_normalized / compile_block  (text lines -> Instruction list)
   |    -> optimizer (peephole, old->new index map)
   v
Program { instructions, function_arities, function_names, method_bytecode, classes, live_vars, line_table }
   |  VirtualMachine::run(&program)                       src/vm.rs
   v
interpreter loop  --(hot numeric loops / pure numeric functions, x86-64 only)-->  native code
```

The real compiler is a **line-based text compiler**: the source is rewritten by a series of
text-to-text passes into plain brace-delimited statements, one per line, and `compile_block` turns those
lines into bytecode. (`lexer.rs`, `parser.rs` and `ast.rs` also exist and feed `compile_killer_ast`,
used by `run_killer_ast`; the `killer`/`killer_super` binaries do not use that path.)
`compile_killer_default` is just `compile_killer_subset`.

## The compile pipeline, in order

All of this is in `compile_killer_subset` (`src/compiler.rs`). The order matters.

| # | Step | File | What it does |
|---|------|------|--------------|
| 0 | `limits::check_source` | `limits.rs` | refuses nesting over 200, more than 1,000 operators in one expression, `!`/`~` runs over 200 (before anything is scanned) |
| 1 | `imports::resolve_mapped` | `imports.rs` | inlines `import "file.killer"` and package imports (`pkg_paths.rs`), once per file, depth limit 32, 8 MB total; returns the first line map (file and line of every line) |
| 2 | `hints::silent_floor_division` | `hints.rs` | `x // 2` becomes a compile error instead of silently being a comment |
| 3 | `testfw::lower` | `testfw.rs` | rewrites `test "name" {` into `fn __ktest_..._hash() {` **on the same line** (no map needed) |
| 4 | `typecheck::process` | `typecheck.rs` | gradual type check of optional annotations (definite errors only) |
| 5 | `controlflow::lower_mapped` | `controlflow.rs` | `match`, `switch`, `do-while`, C-style `for`, destructuring `for` become core statements |
| 6 | `lambda::lift_mapped` | `lambda.rs` | lifts `fn(x) { }`, `(a, b) => expr` and `x => expr` into named functions defined on the line before the statement that uses them |
| 7 | `sugar::preprocess_mapped` | `sugar.rs` | surface sugar to core syntax: `'single'`/`"""triple"""` and `f".."` strings, `0xFF`/`0b101`/`1_000` literals, `and`/`or`/`elif`, `+=` and `++`, multiple assignment |
| 8 | `preprocess_indentation_mapped` | `compiler.rs` | indentation (offside) blocks to braces; strips `--` and `//` comments |
| 9 | `preprocess_polyglot_mapped` | `compiler.rs` | extracts `@python {}`, `@c {}`... blocks before braces are split |
| 10 | `preprocess_ui_sugar` | `compiler.rs` | UI sugar, one output line per input line |
| 11 | `normalize_lines` | `compiler.rs` | splits braces and statements: one statement per `(line number, text)` entry |
| 12 | `compile_normalized` -> `compile_block` | `compiler.rs` | scans globals and function signatures, emits instructions (closure sharing decided by `closure_scan.rs`), patches calls, appends `Halt`, calls `main()` automatically if defined and not called |
| 13 | `optimizer::optimize_bytecode_with_map` | `optimizer.rs` | peephole and dead-store passes; the old-to-new index map re-keys `function_arities`, `function_names`, `method_bytecode` and the line table |

Note the position of testfw (3) before typecheck (4): test blocks are already functions when the type
checker sees them. Errors raised after step 1 are translated back with `remap_error`.

### Source maps and the line table

Passes 5-9 insert and delete lines, so a line number in the compiler's text is meaningless to the user.
Each `*_mapped` pass returns `(String, Vec<usize>)`: for the `j`-th line of the output text
(`text.split('\n')`), entry `j` is the **0-based index of the input line it came from**. `SourceMap`
(`sourcemap.rs`) starts from the import pass (which knows the real `(file, line)` of every line) and
`compose`s each pass's vector onto it, so after the last pass `map.loc(n)` is the original file and line of
line `n` of the compiler's text. `compile_normalized` records `(instruction index, file, line)` entries
in `Program::line_table` (a sorted `LineTable`, translated through the optimizer's index map). It is
only read when an error is reported: the VM sets the location from `line_table.lookup(ip)` on the
innermost uncaught error (`src/vm.rs`, `run`), so the hot loop never touches it. `tests/error_lines.rs`
checks end to end that errors name the right file and line through lambdas, `match`, loops and imports.

## The VM

`src/vm.rs`, `struct VirtualMachine`: a **stack machine** (`stack: Vec<Value>`) with per-call local
frames (`locals_stack: Vec<Vec<Value>>`, slots addressed by index; frame 0 is global code), name-keyed scope
tables for variables that are not slot-allocated (`scopes`, recycled through a pool), a call stack, class
registry, try/catch handler stack (`handlers`, `pending_throw`), a generator manager, and closure frames.
Instructions are the enum `bytecode::Instruction` (`src/bytecode.rs`): slot loads/stores and fused
slot-and-constant arithmetic, `CallBuiltin`, `Call`, `LoadOrBuiltin` (a bare builtin name used as a value),
`BoxVar` (create or load a shared closure cell), `ForNext` (iteration used by `for` and comprehensions),
`ConstBigInt`, trit opcodes, and so on. `VirtualMachine::run(&Program)` executes until `Halt`.

- **Limits**: `MAX_CALL_DEPTH = 100_000` call frames (catchable `stack overflow: recursion too deep`);
  re-entry of the interpreter from Rust (callbacks, operator methods, `toString`, generators) is capped at
  400 levels; allocation limits live next to the operations (strings 256 MB, lists 16M elements, `range` 32M).
- **Re-entry**: `call_value_nested` runs a Killer function from Rust (used by `serve` handlers and by
  spawned tasks).
- **Threads**: `async_spawn`/`spawn` run on pooled VMs created with `VirtualMachine::new_for_spawn()` (JIT
  disabled, fully reset between tasks, 128 MB reserved stack). Rule: no `Rc` is shared between OS
  threads; everything that crosses a thread boundary is copied with `Value::detached()`
  (`src/value.rs`). `tests/thread_safety.rs` covers it.
- **VM-level builtins**: names that must call back into bytecode (`serve`, `serve_once`) are handled in the
  `CallBuiltin` arm of `run` via `http_serve::vm_call` before the ordinary dispatcher.

## Values

`src/value.rs`, `enum Value`: `Number(f64)`, `Integer(i64)` (OS-level integer), `Bool`, `Str(String)`,
`Array`/`Dict`/`Object`/`Set` (shared `Rc<RefCell<..>>` handles: assignment aliases), `Function { params,
bytecode_start, captured }`, `Class`, `Generator`, `Trit`, `Tryte`, `Signal`, `Qubit`, `Uncertain`,
`Gauss`, `Future`, `Bytes`, `Pointer`, `TaskError`, **`Cell(Rc<RefCell<Value>>)`** (a shared closure
variable; only ever stored in scopes and captures, loads and stores see through it) and
**`BigInt(Rc<BigInt>)`** (`bigint.rs`, operator rules in `bigint_ops.rs`; plain Number arithmetic and the
slot fast paths are untouched, BigInt appears only in the fallback branches). `fast_hash.rs` holds the
FNV hasher used for scopes and object fields.

## JIT

Two x86-64-only native tiers, both optional and both falling back to the interpreter:

- `jit_x86.rs` (`JitEngine`): recognises hot loop shapes in bytecode (counter, accumulator, countdown,
  sum-range...) and emits machine code into an executable page.
- `jit_fn.rs` (`FnJit`): compiles **pure numeric functions** (numbers, arrays of numbers, loops,
  calls to other such functions). Any condition native code cannot handle sets a bail flag and the
  interpreter re-runs the call; after a bail-out in deep recursion the JIT backs off.

Executable pages come from `ExecPage::alloc`, which returns `None` unless `target_arch = "x86_64"`, so
other CPUs always use the interpreter. Spawned task VMs have the JIT disabled.

## Builtins

A call to a name known to the compiler becomes `CallBuiltin(name, argc)`; the VM calls
`BuiltinFunctions::call(name, args)` (`src/builtin.rs`), a large `match` on the name. Unmatched names fall
through (the `_` arm) to `lang_builtins::call`, then `net_ai::call` (which dispatches to `http_serve`,
`vec_builtins`, `rag_builtins`, `embed_builtins`, `log_builtins`), then `text_methods::call`. A name is
known to the compiler if it is in `const BUILTINS` (`src/compiler.rs`) or `net_ai::is_builtin(name)` is
true; a script function of the same name wins only for the bigint names (`big`, `factorial`, ...) and
otherwise the builtin takes the call.

### How to add a builtin

1. Put the code in a new module `src/my_builtins.rs` with
   `pub const NAMES: &[&str] = &[...]` and `pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>>`
   (return `None` for names you do not own). Declare it in `src/lib.rs`.
2. Hook the fallback: add `.or_else(|| crate::my_builtins::call(name, args))` in `net_ai::call` and
   `|| crate::my_builtins::NAMES.contains(&name)` in `net_ai::is_builtin` (or, for a generic everyday
   builtin, add it to `lang_builtins::call` and the `BUILTINS` list in `compiler.rs`). If the builtin must
   call Killer code, add a `vm_call` hook like `http_serve::vm_call`.
3. If it touches files, the network, processes, native code or a model, call the matching
   `security::require_file_read` / `require_file_write` / `require_network` / `require_process_spawn` /
   `require_llm` / `require_ffi` first. `tests/capabilities.rs` sweeps every `"name" =>` arm of
   `builtin.rs` under a restricted capability set; add your name to a gated-family test if it is outside
   that file.
4. Add a one-line description to `DESC` (or `NEWDESC`) in `docs/gen_stdlib_index.py` and a group in
   `NAMES_BY_GROUP`, then run `python docs/gen_stdlib_index.py` to refresh `docs/STDLIB_INDEX.md`
   (the generator reads `BUILTINS` and every module's `NAMES` table).
5. Test it: unit tests in the module (`#[cfg(test)]`) and an end-to-end program in `tests/`. Add a
   documented example to `docs/LANGUAGE.md` if users should know about it; `check_examples.py` runs it.

Note: a bare builtin name used as a value (`f = print`) only works for names in `BUILTINS`
(`LoadOrBuiltin` consults that table), not for names that only `net_ai::is_builtin` knows.

### How to add a rewriting pass with a line map

1. Write `pub fn my_pass_mapped(src: &str) -> (String, Vec<usize>)` in its own module. Return the
   rewritten text and, for each line of the output (`split('\n')`), the 0-based index of the input line
   it came from. If you only rewrite within lines, return `(0..n).collect()` (or use the same-line trick
   `testfw::lower` uses and skip the map).
2. Insert it in `compile_killer_subset` at the right place, composing the map right after the call:
   `let (out, pass) = crate::my_pass::my_pass_mapped(&text); let map = map.compose(&pass);`.
   Passes that run before step 11 must keep producing text the later passes understand (braces, one
   statement per line after normalisation).
3. Keep the pass cheap and total: it runs on hostile input too (`tests/robustness.rs` fuzzes the
   compiler; it must not panic or loop).
4. Add a line-number test to `tests/error_lines.rs` (an error after your construct must name the
   original line) and a behaviour test.

## Tooling modules

`bin/killer.rs` is the driver: a table `COMMANDS: (name, help, handler)`; add a subcommand with one row.
Handlers live in `cli_tools.rs` (`run`, `fmt`, `lint`, `repl`), `testfw.rs` (`test`) and `lsp_core.rs`
(`lsp`). `formatter.rs`, `linter.rs` (`LINT_RULES`, `lint:ignore`), `lsp_core.rs` (language server core;
`bin/killer_lsp.rs` is the thin binary), `dap.rs` (`killer_dap`, debuggee instrumented by
`compile_killer_debug`), `pkg_*.rs` + `package_manager.rs` (manifest, lockfile, resolver, registry, CLI;
`bin/killer_pkg.rs` calls `pkg_cli::run`), `limits.rs`, `closure_scan.rs`, `hints.rs`.

## Test layout

- Unit tests: `#[cfg(test)] mod tests` inside the module they test (about 2,000 in compiled code).
- Integration tests: `tests/*.rs`, each its own test binary. Most spawn the real binaries through
  `env!("CARGO_BIN_EXE_killer_super")` / `CARGO_BIN_EXE_killer`, so run `cargo test --tests` rather than
  `--lib`. Notable ones: `language_basics`, `language_semantics`, `shared_semantics` (closures),
  `error_lines`, `bigint` (with `tests/data/gen_bigint_cases.py`), `realworld` (programs in
  `parity/realworld/`), `examples_run` (every `examples/**.killer` against its `.expected`), `robustness`
  (deterministic fuzz; `KILLER_FUZZ_ITERS` and `KILLER_FUZZ_SEED` for long runs), `thread_safety`,
  `capabilities`, `packages`, `killer_cli`, `lsp_roundtrip`, `dap_session`, `formatter_corpus`, `net_ai`.
- `parity/run_parity.py` runs 166 probe programs (`parity/probes.py`) and must print 166/166;
  `parity/inventory.py` regenerates `FILE_INVENTORY.csv`.
- `docs/check_examples.py` executes every fenced `killer` block in `docs/LANGUAGE.md` and
  `docs/TUTORIAL.md` (`# expect:` lines are the expected output; `killer skip` blocks are not run);
  `docs/check_links.py` checks relative Markdown links.
- `bench/run.py` compares Killer with CPython on `bench/*.killer` / `*.py`; results in `bench/RESULTS.md`.

## How CI runs

`.github/workflows/ci.yml`:

- **check**: `cargo check --all-targets`; clippy and `cargo fmt --check` are report-only (they never fail).
- **test** (Ubuntu, Windows, macOS): `cargo test --tests`, then a debug build of `killer_super`, then
  `python parity/run_parity.py`, `python docs/check_examples.py` and `python docs/check_links.py` with
  `KILLER_BIN` pointing at that binary.
- **bench-smoke**: runs three benchmarks once; no gating (debug timings are meaningless).
- **benchmark** and **build-windows**: release builds uploaded as artifacts.

`.github/workflows/release.yml` builds `killer_super` for four targets on a `v*` tag and publishes archives
with SHA-256 files; `install.sh` and `install.ps1` download them. `.github/workflows/killer-native.yml` is a
second, narrower test workflow.

## Debug builds, timing and machine limits

Full builds take many minutes; use debug builds (`cargo build --bins`) while iterating and
`cargo test --test <name>` for one suite. Release builds use `lto = "fat"` and `panic = "abort"`, which
makes them very slow to compile.
