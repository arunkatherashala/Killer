# Friction log: building five real applications in Killer

Each entry: what I tried, what happened, what I expected, and the fix (commit message names the
entry). Entries are numbered in the order I hit them.

1. **`killer test` finds nothing in a freshly scaffolded project.** `killer_pkg new demo` creates
   `tests/basic.killer`; `killer test` printed "no test files found" because only `test_*.killer`
   and `*_test.killer` were discovered. Expected: the scaffolded test to run. Fix: files in a
   `tests/` folder that declare `test "name" { }` blocks are discovered whatever their name, and
   the scaffold template now contains a real `test` block.
2. **`killer run src/main.killer` could not find `import "eval.killer"` (a sibling file).** The
   docs say imports are relative to the importing file, but the `killer` driver resolved them
   against the current directory (`killer_super` did it right). Same for `killer test` with
   `import "../src/x.killer"`. Fix: `imports::set_script` used by `killer run` and `killer test`.
3. **No way to write to stderr.** A command line tool cannot report errors without polluting
   stdout. Fix: new builtins `eprintln` / `eprint` (src/stdio_builtins.rs).
4. **`f"...\n..."` printed a literal backslash-n** (escapes were not processed in f-strings /
   k-strings, only in plain strings) and `{{`/`}}` produced "unsupported expression". Fix in
   `compile_kstring`: escapes `\n \t \r \ \" \'` and doubled braces.
5. **Indexing a long string with `s[i]` in a loop is quadratic** (48k characters: 3 s in a debug
   build for ASCII text, 48 s with one accented character). Worked around by `split(s, "")` first.
   See "still rough" in the final report.
6. **`println(1e21)` printed `9223372036854775807`** (and `float(2n ** 70)`, `2.0 ** 70`, `-1e25`
   the i64 limits): the number printer cast every whole float to `i64`. Expected the exact digits.
   Fix in `Display for Value`: only whole floats below 9e18 take the integer path.
7. **`1e-7` is a parse error** ("unsupported expression `1e`"): the expression splitter cut the
   literal at the `-`. Fix: `is_exponent_sign_at` in the compiler's `+`/`-` splitter.
8. **Bigints do not work where a number is expected**: `xs[1n]` ("Cannot index"), `xs[i:2n]`,
   `"ab" * 2n`, `range(3n)`, `int(5n)`, `float(big)`, `chr(65n)`, `max(1n, 2n)`, `sum([1n, 2n])`,
   `round(2n)` all raised type errors, so an interpreter using exact integers could not even
   index a list with one. Fix: `bigint_ops::prepass` (native int/float/min/max/sum/round for
   bigints, small bigints demoted to numbers for count/size/code arguments) and bigint indexes.
9. **A deep expression crashed the interpreter process** ("thread 'main' has overflowed its
   stack", no Killer error): kx parsed `1 + 1 + ... ` (5000 terms) into a left-leaning tree of
   dicts, and freeing it recursed through `Drop` of nested `Rc<RefCell<..>>`. Reproduced with
   `x = {"l": x}` in a loop (2000 levels is enough) and `x = [x]` (100000). Fix: iterative drop
   for `SharedArray`/`SharedDict`/`SharedObject` (explicit work list), plus `==` short-circuits
   on identity for arrays. Still open: `==` between two distinct 5000-deep structures and
   `json_stringify` of one still recurse natively (listed under "still rough").
