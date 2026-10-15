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
10. **CSV handling was lossy** (found by the CSV tool app): `parse_csv` split on lines before
    looking at quotes, so a quoted field containing a newline became two broken rows; the result
    dicts were built from a `HashMap` so columns came back in alphabetical order (a report could
    not keep the file's column order); `to_csv` sorted its header alphabetically, silently
    returned an empty string for a list of lists, and dropped bigints. Fix: new
    `src/csv_builtins.rs` (RFC 4180 state machine, insertion-ordered dicts, BOM, blank lines,
    unterminated-quote error with the line number, list-of-lists rows, first-seen column order).
11. **`print` is not what the stdlib index says.** The index says "without a newline" but
    `print` is `println`; and `print("a", 1)` was a parse error while `println("a", 1)` worked.
    There was no way to write a prompt or progress line without a newline. Fix: statement
    `print` with several arguments goes through the builtin; new `stdout_write` /
    `stdout_flush`; the index text now says what `print` really does.
12. **`sorted(rows, key)` with a list key failed** ("keys must be all numbers or all strings"):
    ordering groups by sum descending then by name needs a tuple key like Python's
    `key=lambda e: (-e.sum, e.name)`. Fix: keys may be lists (compared element by element),
    booleans and bigints.
13. **`json_pretty(value)` failed** ("first argument must be a JSON string") although the
    stdlib index describes it as "encode a value as indented JSON text". Fix: non-string values
    are encoded first; a string is still re-indented as JSON text.
14. **A CSV with a header and no data rows has no columns** (`parse_csv` returns `[]`), so the
    tool could not tell "no rows" from "wrong column". Fix: new builtin `csv_columns(text)`.
    (Also noted: `for k, v in {"a": 1} {` with a dict literal directly after `in` fails with
    "Undefined variable k"; with a variable it works. Left as rough.)
15. **Operator overloading was half there** (found writing the game's vector and stat classes):
    `3 * v` ("right operand must be a number, got object"), `-v` (same message, because unary
    minus is `0 - v`), `v % k`, `a > b` / `a <= b` / `a >= b` with only `__lt__` defined ("does not
    define __gt__"), and `obj == 5` inside a function (internal "EqSlotConst: slot is not a
    number" error). Fix: `object_binop` in the VM: reflected `__radd__ __rsub__ __rmul__ __rdiv__
    __rmod__`, `__neg__`, `__mod__`, comparisons derived from `__lt__`; comparing an object with a
    number constant is simply unequal. Not done: `__len__`, `__getitem__`, `__contains__`,
    `__iter__`, `__pow__`.
