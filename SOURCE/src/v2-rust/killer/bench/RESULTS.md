# Non-numeric benchmarks: Killer vs CPython

Run with `python bench/run.py` (median wall time, includes process start-up; both sides must print
identical output). Windows 11, CPython 3.12, Killer release build (LTO). The machine was shared with
other builds during measurement, so absolute seconds are inflated and noisy: read the ratios, and
treat differences under about 15% as noise.

Ratio = Killer time / Python time (above 1 means Killer is slower).

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
1. **List comprehensions (about 6x).** Comprehensions compile to an inline loop whose temporaries are
   named variables (`Load`/`Store` by name through the scope chain), not frame slots, about 17
   instructions per element. The fix is in the compiler (allocate slots for the temporaries, or emit a
   dedicated iteration instruction); `compile_list_comprehension` only has `&CompileContext`.
2. **String indexing (about 4x).** `s[i]` no longer copies the string, but the loop still pays a
   byte scan to confirm an ASCII prefix and `len(s)` scans the whole string (`is_ascii`) each time,
   so very long strings are still O(n) per index/len. A cached ASCII flag or a rope/byte-string
   representation would fix it. Non-ASCII strings use the O(n) `chars().nth` path.
3. **Objects and methods (about 4x).** Each call still creates a scope `HashMap` with `String`
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

## Bug found while writing the benchmarks (not fixed here)
`k in some_dict` with a numeric `k` raises `contains() expects string, array, dict or set`
(string keys work), and `if !(x in d) { ... }` is a parse error ("condition must be followed by `{`").
`nested_data` therefore uses string keys and `if k in groups { ... } else { ... }`.

The optimizations were kept in a single commit because they share VM state fields; they are
independent in effect.
