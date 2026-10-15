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
