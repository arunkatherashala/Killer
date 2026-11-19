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


## Round 5 (branch wt-fuse: fewer instructions per operation)

CPU seconds (kernel+user via GetProcessTimes, median of 9 alternating runs, quiet machine, includes about
0.03 s start-up). base = tip f352154 release build, new = this branch. "no shortcuts" is the same binary run
with `KILLER_NO_FUSION=1` (compiles without any of the changes below).

| Benchmark | base | new | no shortcuts (same binary) | Change vs base |
|-----------|-----:|----:|---------------------------:|---------------:|
| obj_methods | 0.500 | 0.422 | 0.500 | -16% (kept) |
| recursion_objects | 0.453 | 0.422 | 0.469 | -7% |
| closures | 0.438 | 0.422 | 0.453 | -4% (below the 10% bar) |
| list_comp | 0.344 | 0.344 | 0.328 | none |
| nested_data | 0.344 | 0.344 | 0.375 | none |
| dict_ops | 0.469 | 0.453 | 0.484 | noise |
| try_catch | 0.125 | 0.094 | 0.125 | noise (one 15 ms tick) |
| fib(32) | 0.062 | 0.062 | 0.062 | unchanged |
| 5M-iteration while loop | 0.438 | 0.453 | | unchanged |

Micro-benchmarks, 2M iterations of a loop body (same alternating protocol): empty `for` body 0.266 -> 0.141 (-47%),
`s = s + i` 0.422 -> 0.297 (-30%), a call `s = f(s)` 0.641 -> 0.484 (-25%), a method call 0.625 -> 0.500 (-20%).
The headline benchmarks gain much less because their time is in the call path itself, not in the loop skeleton.

What was done:
1. **Blocks that bind no names have no runtime scope.** A loop/`if` body made only of instructions that cannot create
   or inspect a name in the innermost scope (slots, loads, arithmetic, calls, field reads, `return`/`break`/`continue`,
   ...) no longer executes `EnterScope`/`ExitScope` (a scope table was popped from a pool, pushed, drained and pushed back
   on every iteration). The compiler turns the pair and the `ExitScope`s that `break`/`continue`/`return` emit for that
   block into `Nop`; the optimizer deletes `Nop` with the usual address remap. Blocks with `let`, named stores, `for`,
   `try`, closures, debugger hooks keep their scope. Bodies of `while` loops are excluded on purpose: the loop-native
   recognizers in jit_x86 depend on the exact instruction layout (without the exclusion a hot accumulator loop past 2^53
   was recognized and printed garbage; caught by tests/python_gap.rs).
2. **`for x in range(a)` / `range(a, b)` counts instead of building the array** (`RangeInit` in front of the unchanged
   builtin sequence; `ForNext` counts through two `Integer` slots). Only for integral arguments within the builtin's size
   limit and memory cap; everything else (floats, errors, huge ranges, a script-defined `range`) runs the old path and
   gives the old error. `range(2M)` alone cost 0.095 s of allocation. The function JIT treats `RangeInit` as a no-op, so
   numeric functions compile exactly as before.
3. **Look-ahead shortcuts** `LoadField`/`LoadSlotField` (`x.f` = Load, GetField, CallMethodDynamic) and
   `LoadIndexStr`/`LoadSlotIndexStr` (`d["k"]`). They replace the first instruction only and leave the originals in place, so
   jumps and error paths are unchanged. Effect (same binary, `KILLER_X_NOLOOK` on/off): obj_methods 0.453 -> 0.422 (-7%),
   nested_data within noise, so below the bar on their own; kept because they are small and obj_methods is the one target
   that passes with them (-16% in total).

Tests: tests/fusion_diff.rs runs 13 programs (break/continue/return in nested scope-less blocks, closures over loop
variables, `let`, try/catch in loops, generators, runtime error lines, range shapes incl. floats, negatives, huge, script
`range`, objects/methods) twice, normally and with `KILLER_NO_FUSION=1`, and requires identical output (the
"at instruction N" number is masked: it legitimately differs).

Not done (measured to be the next cost, not attempted): a profile (sampling RIP + line table, release build with
line tables) of `closures` shows about 25% in hash probes for `argN`/captured names, 15-20% in scope-table moves and
drains, 6% in the capture write-back at `Ret`. Removing that needs parameter and capture slots in the call frame (a
change to every place that reads `argN` by name: captures, generators, debugger, nested calls), and the list-comprehension
loop variable in a frame slot (`for` fast path duplicated for a slot target, and the comprehension variable then no longer
leaks into the enclosing scope as it does today). Neither was started.

## Round 6 (branch wt-slots: call parameters in frame slots)

CPU seconds (kernel+user via GetProcessTimes, median of 9 alternating runs, release build, loaded shared box so the
noise floor is about +-10%). base = tip 0703376, new = this branch. Design: docs/call_frames.md.

| Benchmark | base | new | Change |
|-----------|-----:|----:|-------:|
| closures | 0.406 | 0.344 | -15% (kept) |
| recursion_objects | 0.469 | 0.406 | -13% (kept) |
| obj_methods | 0.438 | 0.438 | none |
| list_comp | 0.344 | 0.328 | noise |
| calls (1M numeric calls + loop fn, JIT) | 0.203 | 0.203 | unchanged |
| fib(38) | 0.578 | 0.531 | unchanged (JIT still compiles it) |
| 5M-iteration while loop | 0.578 | 0.578 | unchanged |

What was done: the text compiler gives parameter i of a function the frame slot i (`LoadSlot`/`StoreSlot`/
`IndexWriteSlot`), local slots start after the parameters; callers fill those slots (`bind_call_args`,
`bind_args_owned`) instead of inserting `argN` into a hash map; locals frames are recycled (`frame_pool`). The function
JIT reads `LoadSlot(i < arity)` as argument i and rejects stores to parameter slots (as it rejected `Store("argN")`).
`killer debug locals` and the DAP adapter recover the names from the declaration. `KILLER_NO_PARAM_SLOTS=1` switches the
compiler back to the named scheme; tests/slots_diff.rs runs 13 programs both ways (recursion, mutual recursion,
closures sharing state, closures in loops, defaults/rest, methods/this/super, generators, spawn/await, errors and
finally, arity mismatch, higher-order calls, deep recursion, eval, hot accumulator loops).

Not done (analysis only, no measurement): captured variables are still copied into the call scope by name and written
back at `Ret`. Making read-only captures skip the write-back, or giving captured by-value variables slots after the
parameters (needs an ordered `Captures`), is the remaining cost in `closures` (about 2.9x Python on a quiet run of
`bench/run.py -n 1`). The list-comprehension loop variable was left alone (it would change leaking behaviour).
Each release build took about an hour on the loaded machine, so only this one step was measured.

## Round 7 (branch wt-cap: closure captures)

CPU seconds (kernel+user via GetProcessTimes, median of 9-15 alternating runs, release build, 4-minute builds on a
quiet box, noise floor about +-8%). base = f7c1b4f (= Round 6 code), "ro" = commit 1 only, "slots" = commit 1+2.
Design notes: docs/call_frames.md ("Captured variables").

| Benchmark | base | ro (step 1) | slots (step 1+2) | Change vs base |
|-----------|-----:|------------:|-----------------:|---------------:|
| closures | 0.359 | 0.297 / 0.281 | 0.266 / 0.250 | -26% to -30% (kept) |
| list_comp | 0.297 | 0.312 | 0.312 | noise (code path untouched) |
| obj_methods | 0.391 | 0.406 | 0.391 | noise |
| recursion_objects | 0.406 | | 0.328 | -19% in one 9-run set (not reproduced separately; treat as noise) |
| nested_data | 0.344 | | 0.359 | noise |
| fib(32) | 0.062 | | 0.047 | unchanged (JIT untouched) |
| 5M-iteration while loop | 0.531 | | 0.516 | unchanged |

Same-binary switches (15 alternating runs of `closures`): all on 0.250, `KILLER_NO_CAP_SLOTS=1` 0.297, `KILLER_NO_RO_CAPTURES=1`
0.344. Wall ratio from `bench/run.py -n 3` on the new binary (loaded box, includes start-up): closures 2.13x
Python, list_comp 2.11x, nested_data 1.80x, obj_methods 2.63x, geometric mean 1.52; no output mismatches.

Step 1 (commit 9bc9dd1): a closure whose body provably never assigns a captured name skips the write-back at `Ret`
and the `closure_frames` bookkeeping. The proof is a conservative text scan of the body
(`closure_scan::captures_readonly`: any assignment whose left side mentions the name, `for` targets, `catch`/`with`/`del`/
`let`/`fn`/`class` lines that mention it, `eval`/`exec`/`debug`/`locals`/`globals`/`vars`/`yield`, also inside nested
closures and one-line blocks). The compiler marks the `ConstFunc` with `@ro`; the flag lives in `Captures`.
Step 2 (commit 9e1ce95): `Captures` became an ordered list, and the by-value captures of a read-only closure live in frame
slots `arity..arity+n` (compiled to `LoadSlot`, copied in after `bind_call_args`), so a call no longer inserts them into
the scope hash map nor drains them at return. Only 3 call sites bind captures (dynamic call, field call, `call_value_nested`).

Tests: tests/capture_diff.rs runs 21 programs three ways (default, `KILLER_NO_CAP_SLOTS=1`, `KILLER_NO_RO_CAPTURES=1`) and
requires identical output: shared counters, closures in loops/dicts/lists, 3-level nesting, recursion through a captured
function, decorators, generators, spawned tasks, try/finally and error line numbers, `eval`, comprehension forms and
comprehension-variable visibility (it leaks when the name is new and does not clobber an existing slot variable: kept exactly),
`+=`/`++`/destructuring/catch rebinding of captures (these stay on the old write-back path), captured collections, defaults,
methods, `map`/`sorted` callbacks, hot loops. Full suite: 3808 passed, 0 failed (was about 3787 + 21 new);
parity 166/166; thread_safety and concurrency_stress rerun 3 times, all passing.

Not done, with reason:
* List comprehension loop variable in a frame slot (item B). Today every element stores the variable by name and the
  name stays in the scope afterwards (the leak), also when the element expression throws part-way (the catch handler can
  see it). A slot variable would need an extra "leak at the end" instruction that cannot reproduce the throw-midway case
  exactly, and lambdas inside the comprehension capture the variable by name. Observable behaviour would change, so it was
  left alone. Gain would be at most the two hash probes per element (about 2 of 4 per iteration).
* Item C (method path, dict iteration, string building, `sorted` key, `map/filter`): no sampling profile is possible on this
  box (everything is inside one 400 KB `run_loop_inner`), and `obj_methods` already goes through the method inline cache;
  nothing was attempted without a measured target.

Residual risks: `killer debug locals` and the DAP adapter no longer list the slot-bound captures of a read-only closure by
name (they show as unnamed slots after the parameters; the names still resolve in `eval` of the closure only when they are
not slot-bound, and `eval`/`debug` in a body disables the optimisation anyway); the text scan is conservative but textual:
a future statement form that writes a variable without matching one of the patterns would be a miscompile (the
`KILLER_NO_RO_CAPTURES` switch exists to bisect such a report).
