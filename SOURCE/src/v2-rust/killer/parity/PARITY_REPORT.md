# Killer parity report

This is an evidence-based audit of Killer: what works, what is promised but not wired up, and what
mainstream languages offer that Killer does not yet. Everything here was measured by *running* the
real binary or analysing the real source; nothing is estimated. All of it is reproducible:

| Check | Command | Output |
|-------|---------|--------|
| Language parity (166 probe programs) | `python parity/run_parity.py --md` | `LANGUAGE_PARITY.md` |
| Every source file: compiled, tested, stubbed? | `python parity/inventory.py` | `FILE_INVENTORY.csv` |
| README function table vs reality | `python parity/claims.py` | printed |

## 1. What is genuinely strong (verified)

These are real, compiled, and tested, and several go beyond what mainstream languages offer:

- **A complete from-scratch toolchain with zero crates**: lexer, parser, compiler, bytecode VM,
  garbage collector, security capabilities, standard library of **583 callable builtins**.
- **A native x86-64 JIT** for numeric code (functions, loops, arrays, `range`, math builtins) that
  beats CPython by about 1.3x to 6.8x on the benchmarks in the README, with safe fallback to the
  interpreter. Code: `src/jit_fn.rs`, `src/jit_x86.rs`.
- **Uncertainty-aware numbers**: `believe x = v ± m` (guaranteed intervals) and `gauss(mean, sigma)`
  (statistical), with three-valued comparisons and Kleene logic. No mainstream language has this.
- **Three-valued (trit) logic**, async/await/spawn, classes with inheritance, optional type
  annotations with a gradual checker, a minimal C FFI, `for` loops, comprehensions, modules.
- **Real subsystems**: the Ghost VM (signed, resumable, fuel-bounded capsules, 63 tests), the Kore
  columnar format (own LZ77/Huffman/range coder), and a local **quantized GGUF transformer
  inference engine** written in plain Rust (`src/inference/`).
- **1,906 tests inside compiled code** plus 50 integration test files, all passing.

## 2. Language parity: Killer vs what programmers expect

166 probe programs, each written the natural way a Python/JS/Java programmer would write it, run
for real. **76 pass (46%).** Per area:

| Area | Pass / Total | Biggest gaps |
|------|:-----------:|---------------|
| Basics | 8 / 21 | `+=` `++`, ternary, `and`/`or`/`not`, tuple assign, bitwise ops, `//`, hex, string `*` |
| Control flow | 8 / 14 | `elif`, `match`, `switch`, do-while, C-style `for`, `for i, x in enumerate` |
| Functions | 4 / 22 | functions as values, lambdas, closures, default args (silently `null`), varargs, globals |
| OOP | 5 / 15 | inherited `init` args lost (silently `null`), `super`, `static`, `instanceof`, operator overloading |
| Collections | 13 / 24 | slicing, negative index (wrong), `in`, list `+`, `set`, tuples, dict comprehension |
| Strings | 9 / 19 | indexing, slicing, iteration, f-strings, single quotes, triple quotes, unicode length |
| Math | 6 / 10 | `log`, `gcd`, `float()`, `PI` |
| Errors | 2 / 9 | **no `try/catch/finally/throw` in scripts at all** |
| Iterators | 4 / 5 | generators (`yield`) |
| Modules & I/O | 3 / 9 | `import` of another script, `env`, `args`, `sleep`, regex |
| Concurrency | 3 / 5 | `mutex_new`; `async_spawn` needs first-class functions |
| Types | 5 / 6 | `bool()` |
| Killer-specific | 6 / 7 | `mean` |

**Two of these fail silently instead of loudly, which is the most dangerous kind of gap:**

- A default argument (`fn greet(name, greeting = "Hi")`) parses but the default is `null`.
- A subclass without its own `init` does not receive the parent's `init` arguments
  (`new Dog("Rex")` gives `this.name == null`).

The single biggest root cause is that **functions are not first-class values in scripts**: a function
cannot be stored in a variable, passed to `map`/`filter`/`reduce`/`sorted`, returned from another
function, or called through a variable. That alone accounts for most of the Functions area plus the
closure, decorator, callback and higher-order cases elsewhere. The second is that **functions cannot
read or write top-level variables** (they are stored in frame slots, not visible to callees).

## 3. Promised but not wired: builtin parity

- **209 builtins are implemented in `builtin.rs` but were not callable from scripts** because the
  compiler's name registry omitted them (`map`, `filter`, `reduce`, `json_parse`, `fileExists`,
  `listDir`, `mkdir`, `async_spawn`, `chan_new`, `date_*`, `http_put`, `set_*` and more).
  **63 have now been registered** (the genuine standard-library ones). The other 145 were left out on
  purpose: generic English words that could hijack user functions (`no`, `next`, `debug`, `days`),
  Android/phone/mic simulations, model-provider aliases, and the unverified Kore/UI sets.
- **55 names are accepted by the compiler but fail at runtime** with "unknown function"
  (`REGISTERED_BUT_DEAD.txt`): the snake_case file API (`file_read`, `file_write`, `file_exists`,
  `file_append`, `file_delete`), `sleep`, `parse_int`, `parse_float`, `to_int`, `to_float`,
  `to_string`, `type_of`, `substr`, `time_now`, `format`, plus the `mic_*`, `phone_*`, `service_*`
  stubs. Several are one-line aliases to functions that already exist.
- **The README's own function table: 18 of 70 functions do not exist** (`log`, `gcd`, `lcm`, `env`,
  `exit`, `args`, `sleep`, `base64_encode`, `base64_decode`, `hmac_sha256`, `aes_encrypt`,
  `gc_stats`, `jit_stats`, `mutex_new`, `vector_store`, `vector_search`, `khlm_route`, `startsWith`).

## 4. File parity: what is actually compiled

555 Rust files, 273,183 lines (`FILE_INVENTORY.csv`):

| Status | Files | Lines | Inline tests |
|--------|------:|------:|-------------:|
| Compiled into the crate | 317 | 172,943 | 1,906 |
| Binaries (`src/bin`) | 45 | 11,328 | 0 |
| **Never compiled** | **193** | **88,912** | **1,894 (never run)** |

- The never-compiled files are the `phase_37` to `phase_49` modules (office formats, templates,
  reporting, collaboration, GPU, WASM v2, enterprise security, generics), older duplicates of compiled
  code, and the directories `kafka/`, `ecosystem/`, `spark/`, `codegen/`, `llvm_backend/`, `server/`.
  A sample read found them coherent but mostly **in-memory simulations** (for example the GPU module
  hardcodes a device name; the "AES-256" module says "simulation"; the DOCX writer notes that real
  DOCX would be zipped). They are not skeletons, but they do not provide the capability their names
  suggest, and their 1,894 tests do not run.
- 43 compiled files of 200+ lines have no inline tests (53,640 lines), including `nova.rs` (4,455
  lines) and `kore.rs` (1,181 lines). The core files `vm.rs`, `compiler.rs` and `builtin.rs` (24,000
  lines) also have no inline tests; they are covered only by the integration tests in `tests/`.
  (`killer_ui/` is well tested: 63 of its 69 compiled files have tests.)
- Of the 45 binaries, about 10 are real tools (`killer_super`, REPL, playground, MCP server, package
  registry, Ghost VM and hive). About 35 are benchmarks, demos or experiments.
- The root contains ~4,257 Markdown files (many inside copies/build output) and dozens of loose
  experiment artifacts; two `ghost_*_output.txt` files only contain "exe is not recognized".

## 5. AI parity: honest read

- **Real**: local GGUF inference (`src/inference/`), cloud LLM calls (`src/llm.rs`, via `curl`/TCP),
  `ghost_ask` which tries exact-math shortcuts first.
- **Not what the name suggests**: Kala's offline "code generation" and "knowledge" answers are
  keyword-matched canned templates (`khlm_polyglot.rs`); its tests assert that a code fence is
  returned, not that the code is correct. `ai/providers/*` return literal "simulated" text.
- **Unverified**: nothing in `tests/` loads a real `.gguf`, so the transformer has not been proven
  end to end. The Kore header claim of "beats Parquet" has no comparison behind it.

## 6. Prioritized roadmap to close the gaps

**Tier 1: correctness that silently misleads (fix first)**
1. Default arguments, and inherited `init` arguments (stop returning `null` silently).
2. Functions as first-class values: variables, arguments, return values, callbacks, lambdas
   (`fn(x) { ... }` and `(x) => ...`), closures capturing variables.
3. Functions reading and writing top-level variables (and `global`).
4. Negative indexing (`a[-1]` currently returns `a[0]`).

**Tier 2: the features every language is expected to have**
5. `try` / `catch` / `finally` / `throw` in scripts (the AST path has it; the script compiler does not).
6. Operators: `+=` `-=` `*=` `/=`, `++`, `?:`, `and`/`or`/`not`, `elif`, chained comparison, `in`,
   `//`, bitwise `& | ^ << >> ~`, hex literals, string `*`, `==`/`<` on strings.
7. Strings: indexing, slicing, iteration, f-strings, single and triple quotes, `ord`/`chr`, unicode length.
8. Collections: slicing, list `+`, `delete`, `set`, tuples, dict comprehension, `insert`, `sorted` with a key.
9. `super`, `static`, `instanceof`, operator overloading, `toString`; multiple assignment and
   destructuring; varargs; keyword arguments.

**Tier 3: completeness**
10. `match`, `switch`, do-while, C-style `for`, generators (`yield`), `enumerate` with unpacking.
11. Make every README function real (the 18 missing, plus the 55 dead registry names) or remove it.
12. Decide the fate of the 193 never-compiled files: wire in, archive, or delete.
13. Add a real end-to-end test for GGUF inference, and a real Parquet comparison for Kore.

## 7. Method and limits

- Language probes use the *natural* spelling; where Killer has its own name (for example `starts_with`
  instead of `startsWith`), the probe uses the common name and counts it as a gap, because parity
  means a newcomer can do the obvious thing. The probe file is `parity/probes.py`; expectations are
  what the equivalent Python/JS program prints. Adjust a probe if you disagree with it.
- File status comes from following `mod` declarations from `lib.rs` and each binary; a file is
  "never compiled" only if no compiled file declares it.
- "Stub markers" in `FILE_INVENTORY.csv` are a text heuristic and have false positives (a function
  named `rel32_placeholder` counts). They are a hint, not a verdict.
- Reading all 273,000 lines line by line was not attempted; the subsystem verdicts for AI, Ghost,
  Kore, UI and platform code come from structured sampling and test inspection, not from running every
  path. The language, builtin and README results are measured.
