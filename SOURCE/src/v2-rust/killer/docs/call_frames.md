# Call frames: how a call binds its parameters

This note describes how a call to a Killer function gets its arguments, how closures see the
variables they captured, and who else depends on that. It is the design behind "Round 6" in
`bench/RESULTS.md`.

## Before (the named scheme, still available)

A call pushed a runtime scope (a hash map `name -> value`, marked `frame`) and a locals frame (a
`Vec<Value>` addressed by slot number). The caller's arguments were bound into the *scope* under the
synthetic names `arg0`, `arg1`, ...; the compiler turned every use of a parameter `x` into
`Load("arg0")` / `Store("arg0")`, i.e. a hash probe through the scope chain. A closure's captured
variables were copied into the same scope by name on entry (`Captures::each` + `store_local`) and
written back by name at `Ret` (`Captures::refresh_with`).

## Now (`Program::param_slots`)

When the program is compiled by the text compiler (`compile_killer_subset`) the parameters of a
function with `n` parameters are the frame slots `0..n`:

* The compiler reserves them (`first_free_slot`: local slots start at `n`) and emits `LoadSlot(i)` /
  `StoreSlot(i)` / `IndexWriteSlot(i)` for parameter `i` (`param_ld` / `param_st` in `compiler.rs`).
  Everything that already exists for slots (fused `LoadSlotField`, `AddSlotConst`, string fast paths,
  the function JIT) now applies to parameters too.
* Callers bind in the VM through two helpers: `bind_call_args` (arguments on the value stack: `Call`,
  `TailCall`, `CallDynamic`, `new C(..)`, method inline cache) and `bind_args_owned` (arguments in a
  `Vec`: method calls, `super` calls, static methods, callbacks and tasks run from Rust through
  `call_value_nested` / `call_method_nested`). Both fill slots `0..arity` of the fresh frame, pad
  missing arguments with `null` and drop extra ones (they stay visible through `args` when the program
  mentions it). Frames come from `frame_pool` and go back to it at `Ret` / `TailCall`.
* `this`, `args`, captured variables, shared (`Value::Cell`) variables and every name that is not a
  parameter or a plain local stay in the named scope exactly as before.
* The flag lives on the `Program` (`param_slots`), so every binder reads `program.param_slots`; there
  is no VM-wide mode that could go stale. Programs built by the AST compiler (`compile_statements`),
  the assembler and tests keep `param_slots = false` and the old scheme, which still works
  end to end. `KILLER_NO_PARAM_SLOTS=1` selects it for the text compiler too, which is how
  `tests/slots_diff.rs` compares both ways.
* `call_function_sync` (the legacy mini-interpreter used by a few array methods) has no frame of its
  own, so for slot programs it delegates to `call_value_nested_inner`.

### Who needs the names

| Consumer | How it gets parameter names now |
|----------|--------------------------------|
| `killer debug` `locals` | `debug_locals(nparams)` labels the first `nparams` slots `argN`; `nparams` comes from the declaration around the current line (`dap::params_at_line`) |
| DAP `variables` | the `__dl` checkpoint also reports the frame's slots as `__slotN`; the adapter maps them to the names in the declaration |
| closures capturing a parameter | the compiler emits `LoadSlot(i)` then a named `Store` before `ConstFunc`, as it did with `Load("argN")` |
| generators | `MakeGenerator` already moves the whole locals frame (slots included) into the generator and back on every resume |
| error messages | unchanged: they name the source variable from the line table, not `argN` |
| `eval` / `exec` | compile their own `Program` and run in a fresh VM |
| spawned tasks | run `call_value_nested` with the shared `Arc<Program>`, so they read the same flag |

### Function JIT

`jit_fn.rs` receives `param_slots` (`FnJit::set_param_slots`, set in `run`). With it set,
`LoadSlot(i)` for `i < arity` reads the pushed argument (`arg_disp`), `IndexWriteSlot(i)` on an array
argument is supported, and any store to a parameter slot (`StoreSlot`, `AddSlotConst`, ...) makes the
function ineligible (the old code rejected `Store("argN")` too), so it falls back to the interpreter.
