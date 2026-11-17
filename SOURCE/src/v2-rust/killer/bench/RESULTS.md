# Non-numeric benchmarks: Killer vs CPython

Run with `python bench/run.py` (median wall time, includes process start-up; both sides must print
identical output). Windows 11, CPython 3.12, Killer release build (LTO). The machine was shared with
other builds during measurement, so absolute seconds are inflated and noisy: read the ratios, and
treat differences under about 15% as noise.

Ratio = Killer time / Python time (above 1 means Killer is slower). The ratio table below was measured
before "Round 2"; Round 2 (further down) only has CPU-time before/after numbers, because the machine was
too loaded for wall-clock ratios. No new ratio table has been measured since, so none is quoted elsewhere.

| Benchmark | Before (ratio) | After (ratio) | What it does |
|-----------|---------------:|--------------:|--------------|
| closures | 20.3 | 4.4 | 1.3M closure calls (captured variables) |
| recursion_objects | 9.4 | 2.5 | build and walk a 131k-node object tree recursively |
| list_comp | 8.1 | 6.6 | four list comprehensions over 200k-500k items (top level) |
| obj_methods | 4.7 | 4.0 | 1.1M method calls on objects, 100k object creations |
| str_index | 3.8 | 4.8 (noise; see below) | `s[i]`, `s[i:i+3]`, `for ch in s` over a 20k string |
| nested_data | 3.6 | 3.8 | 100k list-of-dict rows: build, traverse, group |
| split_join | 2.1 | 1.4 | split / join / replace / upper on a 2 MB string |
| dict_ops | 2.1 | 2.0 | 200k string-key and int-key inserts, lookups, deletes |
| try_catch | 1.9 | 1.4 | 300k try/catch iterations, 10% throwing |
| json_bench | 1.3 | 1.6 | stringify + parse + stringify of 30k records |
| word_freq | 1.2 | 1.4 | 400k dict counts |
| list_ops | 1.0 | 1.0 | 500k append / index / pop / sorted |
| sorted_key | 0.9 | 1.2 | sorted() with a key lambda, 150k items |
| str_concat | 0.04 | 0.07 | `s = s + x` in loops (Killer much faster: CPython's `t = t + a + b` is quadratic) |
| **geometric mean** | **2.14** | **1.81** | |

CPU-time (not wall) comparison of the same two binaries, which is less sensitive to load
(seconds, before -> after): closures 4.9 -> 1.4, recursion_objects 3.9 -> 0.8, obj_methods 1.5 -> 1.0,
list_comp 2.5 -> 2.0, try_catch 0.36 -> 0.20, str_index 0.75 -> 0.69, others within noise.
The ratio columns above come from separate wall-clock runs and so moved by more than the CPU numbers
for the benchmarks that did not change (json_bench, word_freq, sorted_key): that spread is the noise
floor, not a regression.

Numeric JIT is unaffected: `fib(32)` 0.109 s -> 0.094 s, 5M-iteration top-level loop 0.59 s -> 0.55 s
(CPU time, before -> after).

## Where Killer is faster
String building by repeated concatenation, large `sorted`/list operations (level), word counting.

## Where Killer is still slower (largest first)
1. **List comprehensions (about 6x when measured; partly addressed in Round 2: -19% CPU).** Before Round 2 comprehensions compiled to an inline loop whose temporaries are
   named variables (`Load`/`Store` by name through the scope chain), not frame slots, about 17
   instructions per element. The fix is in the compiler (allocate slots for the temporaries, or emit a
   dedicated iteration instruction); `compile_list_comprehension` only has `&CompileContext`.
2. **String indexing (about 4x).** `s[i]` no longer copies the string, but the loop still pays a
   byte scan to confirm an ASCII prefix and `len(s)` scans the whole string (`is_ascii`) each time,
   so very long strings are still O(n) per index/len. A cached ASCII flag or a rope/byte-string
   representation would fix it. Non-ASCII strings use the O(n) `chars().nth` path.
3. **Objects and methods (about 4x when measured; partly addressed in Round 2: -18% CPU).** Before Round 2 each call created a scope `HashMap` with `String`
   keys (`this`, `arg0`) and a locals `Vec`; field writes `this.x = v` push a `ConstStr` clone.
   Next steps: pool scope maps, use `Cow<'static, str>` keys for synthetic names, and give
   `ObjectInstance.fields` the FNV hasher (it uses SipHash).
4. **Nested data / dicts (2-4x).** `r["k"]` clones the value out of the dict; row construction
   allocates one `String` per key literal.
5. **JSON (1.6x), try/catch (1.4x), split/join (1.4x).** Close to parity; no hot spot found.

## Optimizations applied (commit f2bfada)
| Change | Measured effect |
|--------|-----------------|
| Calls bind args off the stack and build the implicit `args` array only if the program mentions `args` | closures, recursion_objects (with the next item), plain call about 10% |
| Closure captures iterated and written back in place (no `Vec`/`String` allocation per call) | closures 3.5x less CPU |
| `Store(name)`: history only when `LoadHistory` exists, no reactive scan without live vars, in-place update, no telemetry | recursion_objects, list_comp ~20% |
| `IndexWrite` without redundant `store_var`; method inline cache; cached `init` lookup | recursion_objects 4.8x less CPU (3.9 -> 0.8 s), obj_methods ~30% |
| `s[i]`/`len(s)` on a string slot without copying the string | str_index ~8% at 20k chars; asymptotic win for long strings |

## Round 2 (non-numeric gaps, commits 7372b95, cfcc2c1, 121fc7f)

The machine was heavily loaded (a release build took 115 minutes), so wall-clock ratios were
meaningless. These numbers are **CPU seconds (kernel+user, best of 3) of the same benchmark with the
previous binary and the new one**, which is the only comparison that held up under load.

| Benchmark | CPU before (s) | CPU after (s) | Change | What did it |
|-----------|---------------:|--------------:|-------:|-------------|
| list_comp | 1.50 | 1.22 | -19% | comprehension iterator/index/result in frame slots, `ForNext` (7372b95) |
| nested_data | 0.67 | 0.52 | -22% | literal-key dict read without cloning the key (`ConstStr`+`IndexRead` peephole) (121fc7f) |
| obj_methods | 0.50 | 0.41 | -18% | pooled scope tables per call, FNV hasher for object fields (121fc7f) |
| recursion_objects | 0.69 | 0.44 | -36% | same two changes |
| str_index | 0.34 | 0.30 | -12% | `for ch in s` splits into characters once (121fc7f) |
| closures | 1.19 | 1.09 | -8% | scope-table pool |
| json_bench | 0.36 | 0.38 | noise | parse/stringify fast paths (cfcc2c1): no measurable win at this size |
| dict_ops | 0.42 | 0.42 | noise | number keys without `String` (121fc7f): no measurable win |
| word_freq | 0.28 | 0.33 | noise | untouched |

Numeric JIT unchanged: `fib(32)` 0.062 s -> 0.047 s; 5M-iteration `while` loop 0.23-0.28 s before and
0.27-0.30 s after (the spread between repeated runs of the same binary is as large as the difference).
`cargo test --tests` and the 166 parity probes pass.

Still slower than CPython (see the table at the top for ratios under a quiet machine): list
comprehensions (the loop body still goes through `CallBuiltin("push")` and a named loop variable
`StoreLocal`), method calls (one `HashMap` insert with a `String` key per `this`/`argN` binding),
string indexing (not changed: caching an ASCII flag needs a change to `Value::Str`, a plain `String`
used in thousands of places; a cache keyed by pointer/length would be unsound when a string is freed
and another with the same length lands at the same address), dict reads that clone string values,
JSON, and try/catch.

## Bug found while writing the benchmarks (since fixed)
Status: both items below were re-checked on the current tree and work (`5 in d` with a numeric key,
`if !(k in d) { ... }`). The original report follows.

`k in some_dict` with a numeric `k` raises `contains() expects string, array, dict or set`
(string keys work), and `if !(x in d) { ... }` is a parse error ("condition must be followed by `{`").
`nested_data` therefore uses string keys and `if k in groups { ... } else { ... }`.

The optimizations were kept in a single commit because they share VM state fields; they are
independent in effect.

## Round 3: long strings and global reads in deep recursion (release, in-script `system_time_ms`, ms)

Wall time of a process start-up on this loaded machine is about 5 s, so these use the in-script
clock; base = tip bd2ec31, after = this branch; two runs each.

| Benchmark | Base | After | What |
|-----------|-----:|------:|------|
| recursion_global | 1556 / 1563 | 18 / 14 | depth 20000, reads a global at each level (was quadratic; scope lookup now stops at the call frame, then the global scope) |
| str_index_unicode | 383 / 397 | 200 / 263 | `s[i]` and `len(s)` per iteration over 53249 chars, one accented char first |
| str_index_long | 1847 / 1654 | 1659 / 1594 | same over 212992 ASCII chars (not improved, see below) |
| fib(32) | 51-129 | 46-56 | unchanged (noise) |
| 5M-iteration loop | 641-822 | 672-827 | unchanged (noise) |

Remaining limit: `len(s)` is one memory-speed pass over the string (about 4.5 us for 212 KB) and `s[i]`
scans the prefix `s[..=i]` for non-ASCII bytes, so a loop that does both per iteration over a 200k
ASCII string is still O(n^2) with a small constant (about 1.6 s). Making it O(1) needs a cached
ASCII flag / char-offset table inside the value, i.e. changing `Value::Str(String)` to a newtype:
about 1500 `Value::Str` construction/match sites across src/ and tests/, not done here. Strings are
no longer copied on load for `len(s)` / `s[i]` (named, parameter, global and slot strings), and
non-ASCII `s[i]`, `len`, `slice` and `charAt` use a vectorised lead-byte scan instead of decoding.

## Round 3 (branch perf-round3: closures, list_comp, nested_data, objects)

CPU seconds (kernel+user via GetProcessTimes, best of 3, same machine, includes about 0.03 s
process start-up), baseline = 514fab5 release build, after = this branch.

| Benchmark | CPU before (s) | CPU after (s) | Change |
|-----------|---------------:|--------------:|-------:|
| closures | 0.656 | 0.375 | -43% |
| list_comp | 0.938 | 0.266 | -72% |
| nested_data | 0.641 | 0.344 | -46% |
| obj_methods | 0.453 | 0.375 | -17% |
| recursion_objects | 0.438 | 0.375 | -14% |
| dict_ops | 0.453 | 0.391 | noise |
| list_ops | 0.234 | 0.188 | noise |
| str_index / try_catch / sorted_key / split_join | 0.078 / 0.109 / 0.141 / 0.203 | 0.062 / 0.094 / 0.109 / 0.156 | noise |
| fib(32) / 5M-iteration loop | 0.047 / 0.484 | 0.031 / 0.516 | unchanged |

Wall ratio Killer/Python on the same machine after the first three changes (n=3): closures 3.5,
obj_methods 3.1, nested_data 2.4, dict_ops 2.3, recursion_objects 2.2, list_comp 1.9, split_join 1.8.

What changed:
1. **`BuildArray` allocated ~18 KB per array literal** (biggest win: list_comp, nested_data). It took a buffer
   from the VM's `ValueBufferPool` (256 `Value` slots), moved it into the new array and never returned it.
   It now splits the elements off the value stack with exact capacity. Building 200k `[i, i*i]` pairs:
   664 ms -> 83 ms.
2. **GC trigger scales with live arrays.** `gc_collect` walks every live array, and it ran every 50,000
   allocations, so building N nested lists was quadratic. The next collection now waits for
   `max(50_000, live arrays)` further allocations (amortised O(1)); cycles are still collected, only later.
3. **`push(list, v)` fast path** in `CallBuiltinId(4, 2)`: the value is moved in, and the result is not built
   when a `Pop` follows (comprehension bodies).
4. **Closures/calls:** `Value::Function.params` is `Rc<[String]>` (loading a function value used to clone a
   `Vec<String>`: two allocations), and scope tables recycle their key strings (`key_pool`) instead of
   allocating a `String` for every `arg0` / captured name / `this` per call.

Checked and not hot: the `format!("{other:?}").contains("args")` scan and the optimizer structures built in
`run()` execute once per program run (about 0.03 s total start-up), not per instruction.

Not done / rejected: field writes still push a `ConstStr` key (an allocation per `this.x = v`); removing it
needs a new instruction in bytecode.rs (outside the allowed files). Comprehension variables are still named
(`Load("i")`), turning them into slots needs the compiler to rewrite every body reference. The remaining
per-call cost in closures/methods is hashing and dispatch (about 15 ns per instruction), which needs frame
slots for call parameters to improve.

## Round 4 (branch wt-speed4: non-numeric VM paths, honest account)

CPU seconds (kernel+user via GetProcessTimes, median of 5-9 alternating runs, includes about 0.03 s start-up).
The machine was shared with other cargo builds, so the noise floor was about +-10%; a change was kept only
when it showed more than 10% and all tests passed. Before = tip 2b0926e, after = this branch.

| Benchmark | CPU before (s) | CPU after (s) | Verdict |
|-----------|---------------:|--------------:|---------|
| nested_data | 0.375 / 0.438 | 0.328 / 0.359 | -12% / -18% (two runs), kept |
| 300k `{"a": i, "b": 2}` literals | 0.156 / 0.203 | 0.125 / 0.172 | -15..-20%, kept |
| closures | 0.438 | 0.406 | noise |
| list_comp | 0.312 | 0.312 | unchanged |
| obj_methods | 0.453 | 0.469 | noise |
| recursion_objects | 0.703 | 0.703 | unchanged |
| dict_ops | 0.422 | 0.453 | noise (large dicts: same hash-index path as before) |
| fib(32) / 5M-iteration loop | 0.062 / 0.609 | 0.062 / 0.594 | unchanged (JIT untouched) |

Kept: **small dictionaries have no hash index** (`ordered_map.rs`). Up to 8 entries the map is a vector scanned
linearly; the index (and the second copy of every key it held) is built when the ninth key arrives. A 4-key
dictionary literal used to allocate an index table plus four extra key strings.

Tried and reverted (no gain above the noise floor, so not kept):
- A word-at-a-time (Fx-style) hasher in place of FNV-1a for scope/object/dict-index maps: closures, obj_methods
  and recursion_objects unchanged.
- One hash probe instead of two in `lookup_mut` (needed an `unsafe` raw pointer): unchanged.
- `NewObjectN` skipping the `classes` probe when `init_cache` hits: unchanged.
- Scopes as a linear-scan vector for up to 8 names (`VarMap`, no hashing for call frames): 4 closure/method
  micro-benchmarks and closures/obj_methods all within +-5%.

Finding: a sampling profile of a method-call loop put under 2% of the time in malloc/memcmp/memcpy/clone and
spread the rest evenly over the roughly 400 KB `run_loop_inner` function, so the remaining cost of closures,
list comprehensions and method calls is per-instruction dispatch (about 70 ns for the loop skeleton plus a
store, 5-15 ns per instruction), not allocation or name hashing. Further gains need fewer instructions per
operation (fused `Load; GetField` / `Load; ConstStr; IndexRead`, frame slots for call parameters), which
needs new instructions in bytecode.rs; not done here.

## Strings (branch wt-str)

CPU time (kernel+user via GetProcessTimes, median of 5 alternating runs, release build, noisy Windows box).
Benchmarks are scratch scripts over an 852,000-character string (`"a..z"` doubled 15 times); idx_* do 200,000 / 20,000 indexes.

| Case | before (s) | after (s) | verdict |
|---|---|---|---|
| `s[i]` non-ASCII string (1 leading `é`), `i < 20000`, 20k iterations | 0.781 | 0.125 | kept (6.2x): non-negative index no longer runs `char_len` (a full scan) before `char_at` |
| `s[i]` ASCII string, `i < 200000` | 0.703 | 0.672 | unchanged (already a prefix `is_ascii` scan at ~30 GB/s) |
| `s[i*40]` on non-ASCII string (expression index, not fused) | 2.875 | 2.234 | side effect of the same change; dominated by the clone below |

Measured but not changed (baseline CPU, same string): `s[i*4]` ASCII, 200k iterations 15.8 s; `len(s)` 200k times 6.9 s;
slicing 300 x 50k chars 0.27 s; find 0.14 s; split 0.25 s; join 0.30 s; upper+lower 0.16 s; replace 0.17 s; `in` 0.17 s;
`s < u` 1000 times 0.70 s; 100k `x = x + "ab"` 0.13 s; 100k f-strings 0.23 s. Process start-up and building the string are about 0.1 s of each.

What the profile says:

1. `s[i]`/`len(s)` on a plain variable are fused (`fused_str_op`) and borrow the string: the ASCII case is O(i)
   per index at memory speed (3.5 us at i = 100k), `len` is O(n) (about 30 us at 852k chars). That is still O(n^2)
   over a 1M-character loop.
2. Any other shape (`s[i*4]`, `s < u`, passing `s` to a function, `s == t`) executes `Load`, which **clones the whole
   string**: 79 us per index at 852k chars. This, not the character scan, is the dominant cost for long strings.
3. The builtin string functions (find, split, join, upper, replace, `in`, concat, format) show no O(n^2) behaviour and
   nothing measuring more than 10% to gain at this size.

Not done, with reason: caching an ASCII flag / char count / offset index needs the metadata stored with the string.
`Value::Str(String)` is constructed or matched at 1481 sites in 90 files (vm.rs, builtin.rs, nova.rs, ...), so a
`Value::Str(KStr)` (or `Rc<str>` plus cached flag, which would also remove the Load clone of point 2) is a mechanical but
repo-wide change that would conflict with the concurrent VM/bytecode work. A cache keyed by (pointer, length) outside the
value was rejected: a freed string and a new one with different content can share both, so it is unsound.
