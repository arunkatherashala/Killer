# Killer parity report

This is an evidence-based audit of Killer: what works, what is promised but not wired up, and what
mainstream languages offer that Killer does not yet. Everything here was measured by *running* the
real binary or analysing the real source; nothing is estimated. All of it is reproducible:

| Check | Command | Output |
|-------|---------|--------|
| Language parity (166 probe programs) | `python parity/run_parity.py --md` | `LANGUAGE_PARITY.md` |
| Every source file: compiled, tested, stubbed? | `python parity/inventory.py` | `FILE_INVENTORY.csv` |
| README function table vs reality | `python parity/claims.py` | printed |
| All Rust tests (lib, binaries, integration) | `cargo test --tests` | 3,253 passed, 0 failed, 9 ignored |
| Documentation examples and links | `python docs/check_examples.py`, `python docs/check_links.py` | printed |
| Benchmarks vs CPython | `python bench/run.py` | `bench/RESULTS.md` (the only source of performance claims) |

Last refreshed at the merge of the seven workstreams (`5486e55` plus the documentation commits):
parity **166/166**, **3,253** tests passed in 118 test binaries (`cargo test --tests`, debug build,
9 `#[ignore]`d long-running tests not included).

## 1. What is genuinely strong (verified)

These are real, compiled, and tested, and several go beyond what mainstream languages offer:

- **A complete from-scratch toolchain with zero crates**: lexer, parser, compiler, bytecode VM,
  garbage collector, security capabilities, standard library of **711 registered builtin names** (`docs/STDLIB_INDEX.md`; every one is callable).
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
- **2,083 tests inside compiled code** plus 65 integration test files, all passing (3,253 in total, 9 ignored).

## 2. Language parity: Killer vs what programmers expect

166 probe programs, each written the natural way a Python/JS/Java programmer would write it, run
for real against the release binary. **Before the fixes: 76 passed (46%). Now: 166 pass (100%).**

| Area | Before | Now |
|------|:------:|:---:|
| Basics | 8 / 21 | 21 / 21 |
| Control flow | 8 / 14 | 14 / 14 |
| Functions | 4 / 22 | 22 / 22 |
| OOP | 5 / 15 | 15 / 15 |
| Collections | 13 / 24 | 24 / 24 |
| Strings | 9 / 19 | 19 / 19 |
| Math | 6 / 10 | 10 / 10 |
| Errors | 2 / 9 | 9 / 9 |
| Iterators | 4 / 5 | 5 / 5 |
| Modules & I/O | 3 / 9 | 9 / 9 |
| Concurrency | 3 / 5 | 5 / 5 |
| Types | 5 / 6 | 6 / 6 |
| Killer-specific | 6 / 7 | 7 / 7 |

Both silent-failure bugs found by the audit (default arguments and inherited `init` arguments turning
into `null`) are fixed, as is the root cause behind most of the Functions gaps: functions are now
first-class values (variables, arguments, return values, callbacks, lambdas, closures, decorators).

**Deliberate design decisions, now documented by the probes rather than hidden as failures:**

- `//` starts a comment (C/JS style), so integer division is spelled `floor(a / b)`. The mistake `x // 2` is a
  compile error with that hint: a `//` after an expression-ending token and followed by an operand (a number or `(`,
  a name glued to it, or a name already used earlier) is rejected; `// prose`, `//TODO` and full-line comments are
  comments. For exact integer division of big values use `idiv(a, b)`.
- Assignment inside a function is local unless declared `global` (the Python rule).
- `regex_match(text, pattern)` takes the text first.
- Closures share the variables they capture: a local that a nested function assigns (or that the enclosing
  function reassigns after the nested definition) lives in one shared cell per call, so the counter
  `inc`/`get`/`reset` trio works. Loop variables and `let` bindings are fresh per iteration (like JS `let`); a
  plain assigned variable is one variable per call (Python rule).
- Generators are lazy coroutines: calling a function that contains `yield` returns a generator
  without running the body, and each `next(g)` / `for` iteration resumes it to the next `yield`
  (infinite generators, on-demand side effects, `has_next(g)`). A generator that is abandoned
  part-way (for example after `break`) keeps its suspended frame until the program ends.
- `finally` runs on every exit of its `try`: normal end, caught or uncaught error, `return`, `break`, `continue`; a
  `return`/`throw` inside `finally` overrides the pending one.
- Rewriting passes (lambdas, `match`/`switch`, `do-while`, C-style `for`, `import`, test blocks) carry a
  line map, so errors report the original file and line (`tests/error_lines.rs`).

## 3. Promised but not wired: builtin parity

- **209 builtins are implemented in `builtin.rs` but were not callable from scripts** because the
  compiler's name registry omitted them (`map`, `filter`, `reduce`, `json_parse`, `fileExists`,
  `listDir`, `mkdir`, `async_spawn`, `chan_new`, `date_*`, `http_put`, `set_*` and more).
  **63 have now been registered** (the genuine standard-library ones). The other 145 were left out on
  purpose: generic English words that could hijack user functions (`no`, `next`, `debug`, `days`),
  Android/phone/mic simulations, model-provider aliases, and the unverified Kore/UI sets.
- **55 names were accepted by the compiler but failed at runtime** ("unknown function"): the
  snake_case file API, `sleep`, `parse_int`, `parse_float`, `to_int`, `to_float`, `to_string`,
  `type_of`, `substr`, `time_now`, `format`, plus `mic_*`, `phone_*`, `service_*` stubs. **Now zero**:
  the useful ones were implemented (`format` with `{}`/`{N}`/`{:.N}`, `substr`, `time_now`, the aliases),
  the unreachable device names were dropped from the registry, and `REGISTERED_BUT_DEAD.txt` is empty.
- **The README's own function table: 18 of 70 functions did not exist; 14 have since been added**
  (`log`, `gcd`, `lcm`, `env`, `exit`, `args`, `sleep`, `base64_*`, `hmac_sha256`, `gc_stats`,
  `jit_stats`, `mutex_new`, `startsWith`). The other 4 (`aes_encrypt`, `vector_store`,
  `vector_search`, `khlm_route`) were removed from the table because nothing implements them.

## 4. File parity: what is actually compiled

592 Rust files, 291,400 lines (`FILE_INVENTORY.csv`, regenerated with `python parity/inventory.py`):

| Status | Files | Lines | Inline tests |
|--------|------:|------:|-------------:|
| Compiled into the crate | 351 | 191,883 | 2,083 |
| Binaries (`src/bin`) | 49 | 11,228 | 0 |
| **Never compiled** | **192** | **88,289** | **1,874 (never run)** |

- The never-compiled files are the `phase_37` to `phase_49` modules (office formats, templates,
  reporting, collaboration, GPU, WASM v2, enterprise security, generics), older duplicates of compiled
  code, and the directories `kafka/`, `ecosystem/`, `spark/`, `codegen/`, `llvm_backend/`, `server/`.
  A sample read found them coherent but mostly **in-memory simulations** (for example the GPU module
  hardcodes a device name; the "AES-256" module says "simulation"; the DOCX writer notes that real
  DOCX would be zipped). They are not skeletons, but they do not provide the capability their names
  suggest, and their 1,874 tests do not run.
- 46 compiled files of 200+ lines have no inline tests (57,981 lines), including `nova.rs` (4,455
  lines) and `kore.rs` (1,181 lines). The core files `vm.rs`, `compiler.rs` and `builtin.rs` (24,000
  lines) also have no inline tests; they are covered only by the integration tests in `tests/`.
  (`killer_ui/` is well tested: 63 of its 69 compiled files have tests.)
- Of the 49 binaries, about 14 are real tools (`killer_super`, the `killer` driver, `killer_lsp`,
  `killer_dap`, `killer_pkg`, REPL, playground, MCP server, package registry, Ghost VM and hive). The
  rest are benchmarks, demos or experiments.
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

## 6. Roadmap

**Done (Tier 1 and 2, and most of Tier 3):** default arguments, inherited constructors, first-class
functions, lambdas, closures, globals, negative indexing, `try/catch/finally/throw`, operators
(`+=`, `++`, ternary, `and/or/not`, chained comparison, `in`, bitwise, string and list `*` / `+`),
string indexing/slicing/unicode length, slicing, sets, tuples, comprehensions (list, dict, set),
`super`, `static`, `instanceof`, operator overloading, `toString`, varargs, keyword arguments,
`match`, `switch`, do-while, C-style `for`, destructuring `for`, lazy generators, JSON, file imports,
mutexes, `async_spawn` with function values.

**Done since the first audit:** `finally` on early `return`/`break`/`continue`, line-number mapping
through the rewriting passes, closures that share captured variables, the 55 dead registry names, big
integers, one-line blocks, `for k, v in dict`, builtins as values, the `//` compile error, the package
manager, the tooling suite, HTTP server, vector store and RAG helpers, resource limits.

**Still open:**
1. **Block scoping.** A variable first assigned inside an `if`/loop body is not visible after the block
   (`Undefined variable`); another change is fixing this (in progress, not yet merged here).
2. `len(s)` and `s[i]` are O(n) on very long strings (ASCII scan per call; non-ASCII uses `chars().nth`).
3. The Killer HTTP server has no TLS and serves one request at a time.
4. The debugger (`killer_dap`) shows one stack frame (innermost scope plus globals).
5. Closures created before the enclosing function first assigns a captured variable keep the old
   copy semantics (the sharing decision is textual, see `closure_scan.rs`).
6. Number `%` keeps C semantics (`-7 % 3 == -1`) while BigInt uses floor semantics (`-7n % 3n == 2`);
   `bool(0n)` is `true` although `if 0n` is falsy; floats of 2^63 or more print saturated.
7. `count_tokens` is an approximation (about 10-20 percent off GPT-style tokenizers); the real tokenizer
   needs a GGUF model.
8. 192 source files (88,289 lines) are never compiled: wire in, archive, or delete.
9. HTTPS needs `curl` on the machine (the client shells out to it); there is no built-in TLS.
10. A real end-to-end test for GGUF inference, and a real Parquet comparison for Kore.
11. Builtins owned by the net/AI modules (`vec_*`, `serve`, `llm_embed_batch`, ...) can be called but not
    passed as first-class values (only names in the compiler's `BUILTINS` table qualify).

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

## 8. Robustness, thread safety and capabilities (audit W6)

**Contract.** Whatever a script contains, the compiler and VM must not panic, hang or overflow the
native stack; they report a clear Killer error instead. `tests/robustness.rs` enforces this with a
deterministic fuzz harness (seeded xorshift, no crates): thousands of mutated variants of the real
`.killer` files in `examples/`, `parity/realworld/` and `tests/`, token soup, and hostile shapes
(deep nesting, 100k-element literals, huge operator chains, `"x" * 1e9`, `range(1e12)`, cyclic
values, runaway recursion through callbacks, operators, `toString` and generators). The default run
takes seconds; `KILLER_FUZZ_ITERS=200000 KILLER_FUZZ_SEED=7 cargo test --test robustness fuzz_long -- --ignored`
runs a long one.

| Limit | Value | Error |
|-------|-------|-------|
| Killer call depth (heap frames) | 100,000 | `stack overflow: recursion too deep` (catchable with `try/catch`) |
| Rust-level re-entry (callbacks, `__add__`, `toString`, generators) | 400 nested levels | same error |
| String built by `+` / `*` | 256 MB | `string is too large (limit 256 MB)` |
| List built by `+` / `*` | 16M elements | `list is too large` |
| `range()` materialised | 32M elements | `range is too large ... use a while loop` |
| `json_stringify` output | 64 MB | `output is too large` |
| Source nesting (`(`, `[`, `{`, blocks, lambdas) | 200 levels | `nesting is too deep` |
| Operators in one expression | 1,000 | `expression is too long` |
| `print` / `str` of a nested or self-referencing value | 64 levels, 2M containers | printed as `[...]` |

Depth 10,000 recursion keeps working. The checks are single integer comparisons on paths that
already allocate or push a frame, so the interpreter hot loop is unchanged.

**Thread-safety design.** `Value` contains `Rc<RefCell<..>>` (arrays, dicts, objects, sets,
closure captures), and `unsafe impl Send/Sync for Value` exists only so values can be *moved* into
a task. The rule that makes it sound: **no `Rc` is ever shared between OS threads.** Everything
that crosses a thread boundary is deep-copied with `Value::detached()`:

- arguments and the function value (with its captured variables) of `async_spawn` / `spawn` /
  `SpawnCall` / `SpawnCallDirect`; a snapshot of the global variables given to the task;
- the value handed to `spawn_thread` / `async_spawn(value)` / `spawn(value)` / `async_resolve`;
- channel messages (`chan_send`) and future results (`await`, `async_all`, `async_race`,
  `async_timeout` copy again when they read a slot that other threads can also read).

`detached()` keeps sharing and cycles *inside* the copy (an array that contains itself copies to an
array that contains its copy) and cuts nesting beyond 20,000 levels with `null`, so it terminates on
any value. Consequence, documented behaviour: a task works on **copies**; mutating an array, dict,
object or set inside a task is invisible to the parent, and the parent's later mutations are
invisible to the task. Use `mutex_*` cells (store plain data as text) or channels to communicate.
Pooled spawn VMs are fully reset between tasks, and task threads get a 128 MB (reserved, not
committed) stack. A task that cannot get a thread resolves its future with an error.
`tests/thread_safety.rs` covers arguments of every kind, globals, closures, channels, results,
cyclic values, 200-task stress runs and nested spawning.

**Capability audit.** Every builtin that touches files, the network, processes, native code or
language models must call the matching `security::require_*`. `tests/capabilities.rs` calls every
dispatcher name in `src/builtin.rs` under a restricted capability set against a scratch directory,
a local listener and marker-writing commands, and fails if anything observable happens.
Gaps found and fixed: `mmap_*`, `call_native`, `page_map` (now `require_ffi`), `env_get`,
`env_set`, `process_exit` (now `require_process_spawn`), `disk_read_block` / `disk_write_block`
(now file read / write; offset arithmetic no longer overflows), `fileCopy` / `renameFile` (also
need file read). `io_port_*` executed real ring-0 `in`/`out` instructions and `interrupts_*`/`wfi`
executed `cli`/`sti`/`hlt`, which fault the whole process in user mode: they are now an error and
simulated no-ops. Temp files for `@python` / `@c` / `@java` / `@rust` blocks used predictable names
in the shared temp directory (symlink / clobbering race): they now use `create_new` files and
private per-run directories with unpredictable names.

**Remaining risks.** Block scoping is real (fix in progress): a variable first assigned inside an `if` or loop body
is not visible after the block (Python and JavaScript `var` programmers are surprised by
`Undefined variable`); declare it before the block. Killer functions cannot assign to a global
without `global`, and assigning inside a function creates a local. `volatile_*` read through
unaligned pointers (fine on x86-64). Dropping a value nested hundreds of thousands of levels deep
recurses natively in Rust's `Drop`; the nesting limit keeps source literals far below that, but a
program can still build such a value in a loop. Embedders must run the VM on a large stack (as
`killer_super` does with 512 MB); the 400-level native re-entry limit assumes that.
