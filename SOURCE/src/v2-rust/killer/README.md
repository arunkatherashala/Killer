# Killer Language

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-beta-orange.svg)](#status)

**Killer** is a from-scratch scripting language with a bytecode VM, written in Rust with **zero external
crates**. It has Python-like syntax (braces for blocks), closures, classes, generators, exceptions with
file and line numbers, big integers, a large builtin library, an optional x86-64 JIT for numeric code
(other architectures use the interpreter), and a few unusual features: three-valued logic, uncertain
numbers, and offline building blocks for AI programs.

## Status

**Beta / pre-release.** The version is the one in `Cargo.toml` (1.0.0). The language is exercised by 166
parity probes (all pass), 18 real-world programs and about 3,250 Rust tests; the honest list of what
does not work yet is in [`parity/PARITY_REPORT.md`](parity/PARITY_REPORT.md) and the *Known limitations*
below. Performance numbers are only quoted from [`bench/RESULTS.md`](bench/RESULTS.md); anything else is
"not measured". Many older documents under the repository's `DOCS/` folder are historical session
reports and are not authoritative.

## Features

- **Language**: functions, lambdas, closures that share captured variables, classes with inheritance and
  operator overloading, lazy generators, `try`/`catch`/`finally`, `match`/`switch`, comprehensions,
  destructuring, default and keyword arguments, `for k, v in dict`, one-line blocks, builtins as values,
  optional type annotations, `async`/`spawn`/channels/mutexes (tasks get deep copies).
- **Numbers**: 64-bit numbers plus arbitrary-precision integers (`123n`, `factorial`, `powmod`, `divmod`).
- **Standard library**: 711 registered builtin names ([`docs/STDLIB_INDEX.md`](docs/STDLIB_INDEX.md)):
  strings, collections, files, JSON/CSV, dates, regex, crypto, concurrency, HTTP.
- **Network and AI**: a real HTTP server (`serve`, `route`), a hardened curl-backed HTTP client (TLS on by
  default, secrets via stdin), a vector store, text chunking, offline embeddings, RAG prompt building,
  structured logging, an LLM client.
- **Tooling**: the `killer` driver (`run`, `fmt`, `lint`, `test`, `repl`, `lsp`), a test framework
  (`test "name" { }`), a formatter and linter, a language server, a VS Code extension, a Debug Adapter
  Protocol server (`killer_dap`).
- **Packages**: `killer.toml`, `killer.lock` with checksums, semver resolution, `killer_pkg`
  (`new/add/install/publish/check`), compile-time `import`.
- **Safety**: recursion, allocation and nesting limits that produce catchable errors instead of crashes,
  a deterministic fuzz harness, capability checks on every builtin that touches files, network, processes
  or native code.
- **Unusual**: trits (`T_POS`, `T_ZERO`, `T_NEG`) with Kleene logic, `believe x = 10 ± 1` uncertain numbers.

## Install

**Prebuilt binaries** (tagged releases; Linux x64, Windows x64, macOS x64 and arm64) with checksum
verification, installed to `~/.killer/bin`:

```bash
sh install.sh            # Linux / macOS (see --help, --dry-run)
```
```powershell
./install.ps1            # Windows PowerShell
```

The scripts are in the repository root; they download from the project's GitHub releases.

**From source** (Rust stable, no other dependencies; `curl` is needed at run time for HTTPS):

```bash
cd SOURCE/src/v2-rust/killer
cargo build --release --bin killer --bin killer_super --bin killer_pkg --bin killer_lsp --bin killer_dap
# binaries are in target/release/
```

## Quickstart

```bash
echo 'println("hello, killer")' > hello.killer
killer run hello.killer              # same as: killer_super hello.killer --run
killer repl                          # interactive prompt
killer fmt --check .                 # formatting check
killer lint .                        # lint
killer test                          # run test "..." { } blocks and fn test_*()
killer_pkg new myapp                 # a project with killer.toml
```

```killer
fn greet(name, greeting = "Hello") {
  return greeting + ", " + name + "!"
}
println(greet("Ada"))

counts = {}
for word in split("the cat and the hat", " ") {
  counts[word] = get(counts, word, 0) + 1
}
for word, n in counts {
  println(word, n)
}
println(factorial(30))
```

```killer
port = free_port()
route("GET", "/hello/:name", fn(req) {
  return "hello " + req["params"]["name"]
})
serve(port, null, {"max_requests": 1})   # then: curl http://127.0.0.1:<port>/hello/ada
```

Next: the [tutorial](docs/TUTORIAL.md) (14 lessons), the [language reference](docs/LANGUAGE.md), the
[standard library index](docs/STDLIB_INDEX.md), the [architecture guide for contributors](docs/ARCHITECTURE.md)
and [`DOCS.md`](DOCS.md).

## Architecture in one picture

```
source -> imports -> typecheck -> test blocks -> control-flow / lambda / sugar passes -> compile_block
       -> bytecode -> optimizer -> VM (stack machine, interpreter loop)  [+ x86-64 JIT for hot numeric code]
```

Details, file by file, in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Performance

Killer has two execution tiers: a bytecode interpreter for everything, and a built-in x86-64 JIT for pure
numeric functions and loops (anything it cannot prove safe runs in the interpreter; on other CPU
architectures the JIT is disabled). Measurements against CPython (non-numeric workloads, geometric-mean
time ratio 2.14 -> 1.81 after the first optimization round, plus CPU-time before/after for a second round)
are in [`bench/RESULTS.md`](bench/RESULTS.md); reproduce with `python bench/run.py` (set `KILLER_BIN`).
That file also lists where Killer is still slower than CPython (comprehensions, method calls, string
indexing). No other performance claim is made here.

## Calling C libraries (FFI)

```killer
lib = ffi_open("msvcrt.dll")                 # libc.so.6 / libm.so.6 on Linux
println(ffi_call(lib, "pow", "dd>d", 2, 10)) # 1024
println(ffi_call(lib, "strlen", "s>l", "killer"))

buf = ffi_alloc(16)                          # bounds-checked native memory
ffi_poke(buf, 0, "i32", 1234)
println(ffi_peek(buf, 0, "i32"))
ffi_free(buf)
ffi_close(lib)
```

Signatures are `<args>><ret>` with `i`/`l` integer, `p` pointer, `d` double, `s` string and
`v` void. A call is either all doubles (up to 4) or all integer/pointer/string (up to 6); mixed
signatures, structs by value and callbacks are not supported yet. FFI needs the `allow_ffi`
capability, which sandboxed scripts do not get.

## Optional type annotations

```killer
fn add(a: number, b: number) -> number {
  return a + b
}
count: number = 10
add("one", 2)        # type error (line 5): argument 1 of add() expects number, got string
```

Types are `number`, `string`, `bool`, `array`, `dict` and `any`. The checker is gradual: it only
reports errors it can prove, and code without annotations is untouched.

## Uncertain values and three-valued comparisons

A measurement is a range, and Killer treats it as one. `believe x = v ± m` creates a value that is
guaranteed to lie in `[v - m, v + m]`. Arithmetic returns a range that *encloses every possible
result*, and comparisons answer with a three-valued `Trit` instead of guessing:

```killer
believe length = 10 ± 1       # somewhere in [9, 11]
believe width  = 4 ± 0.5      # somewhere in [3.5, 4.5]

area = length * width
println(area)                 # 40 ± 9.5
println(area > 20)            # T_POS:  true for every possible value
println(area > 40)            # T_ZERO: cannot be decided
println(area < 10)            # T_NEG:  false for every possible value

if area > 20 { println("certainly larger than 20") }   # runs
if area > 40 { println("not printed: not certain") }    # does not run
```

- `+ - * /` propagate the margin; dividing by a range that includes zero, or taking `sqrt` of a
  range that includes negatives, is an error rather than a silent wrong answer.
- `< <= > >= == !=` return `T_POS` (certain), `T_NEG` (impossible) or `T_ZERO` (overlapping
  ranges). `if` only takes the branch for `T_POS`, so decisions are made only when they are safe.
- `uncertain(v, m)` builds one directly; `unc_value`, `unc_margin`, `unc_lo`, `unc_hi` read the parts.
- `&&`, `||` and `!` use Kleene three-valued logic on these results: AND is the minimum, OR the
  maximum, and NOT of an undecided answer is still undecided (`T_ZERO`). A plain boolean mixed in
  counts as certainly true or false.
- Plain numbers are exact (margin 0), so they mix freely with uncertain values.
- The margin is a worst-case bound (interval arithmetic), not a statistical standard deviation.

### Statistical values: `gauss(mean, sigma)`

`believe` gives a guaranteed worst-case range. When your errors are *statistical* (independent,
normally distributed, like repeated measurements), use `gauss`: errors add in quadrature, so the
result is tighter than the worst case.

```killer
a = gauss(10, 3)
b = gauss(20, 4)
println(a + b)                # 30 ± 5σ     (sqrt(3² + 4²), not 3 + 4)
println(a < b)                # T_POS:  the means are 2 combined sigmas apart
println(a > 5)                # T_ZERO: only 1.67 sigmas, so undecided
println(prob_gt(b, a))        # 0.9772...   P(b > a)

total = gauss(0, 0)
for i in range(9) { total = total + gauss(10, 1) }
println(total)                # 90 ± 3σ     (sigma grows as sqrt(n), not n)
```

- Comparisons are decided only when the means differ by at least 1.96 combined sigmas (about 95%
  confidence); otherwise they return `T_ZERO`. `prob_gt(a, b)` / `prob_lt(a, b)` give the probability.
- Read the parts with `unc_value(x)` and `unc_sigma(x)`.
- Errors are assumed **independent**, so `x - x` is not zero. Division and `sqrt` are first-order
  approximations and are refused when the divisor / argument is not clearly away from zero.
- Interval (`±`) and statistical (`gauss`) values cannot be mixed in one expression; that is an error.

### Examples

Runnable programs in [`examples/uncertainty/`](examples/uncertainty/) (each has a checked-in
`.expected` output that the test suite verifies):

| Program | Shows |
|---------|-------|
| `lab_density.killer` | identifying a metal from measurements, statistical vs guaranteed errors, `T_ZERO` as "cannot be told apart" |
| `sensor_alerts.killer` | a thermostat that only alarms when sure and asks for a re-check otherwise; Kleene `&&` `||` `!` |
| `shipping_weight.killer` | will a batch exceed a truck's capacity?, errors adding in quadrature, `prob_gt` |
| `project_budget.killer` | a cost estimate as ranges checked against several budgets, with a 10% contingency |

```
killer_super examples/uncertainty/sensor_alerts.killer --run
```

The constants `T_POS`, `T_ZERO` and `T_NEG` are plain names. Use `x == T_ZERO` to test for "undecided"
(an `if` on a three-valued result runs only for `T_POS`).

## Known limitations

- Block scoping: a variable first assigned inside an `if` or loop body is not visible after the block
  (fix in progress); assign it before the block.
- Plain 64-bit numbers saturate on overflow; use `n` literals / `big()` for exact large integers.
- The JIT is x86-64 only and covers pure numeric code; everything else is interpreted.
- The HTTP server has no TLS and serves one request at a time; the HTTP client needs `curl` (HTTPS).
- `len(s)` and `s[i]` are O(n) on very long strings; `count_tokens` is an approximation.
- The debugger shows one stack frame.
- Dicts and objects are shared references (like Python or JavaScript): `b = a` aliases, and a dict or
  object that contains itself is never garbage-collected.
- The FFI is minimal (see above) and has no safety net against a wrong signature.
- About 190 source files in `src/` are never compiled (see `parity/PARITY_REPORT.md`).
- Killer is a one-person project with a young package ecosystem.

## Binaries

| Binary | Purpose |
|--------|---------|
| `killer` | The driver: `run`, `fmt`, `lint`, `test`, `repl`, `lsp` |
| `killer_super` | Compiler/runner: `killer_super file.killer --run` |
| `killer_pkg` | Package manager: `new`, `add`, `install`, `publish`, `check` |
| `killer_lsp` | Language server (stdio) |
| `killer_dap` | Debug Adapter Protocol server (stdio) |
| `kpm-registry` | File-backed package registry served over HTTP |
| `killer-repl`, `killer-playground` | Older REPL and web playground |
| `killer-mcp` | MCP server: AI assistant tool bridge |
| `ghost_hive`, `kore_bench`, `killer_ui_serve` | Distributed VM coordinator, format benchmarks, chat UI |

## Testing

```bash
cd SOURCE/src/v2-rust/killer
cargo test --tests                                   # lib, binaries and all integration tests (long)
cargo build --bins
KILLER_BIN=$PWD/target/debug/killer_super python parity/run_parity.py    # 166 probes
KILLER_BIN=$PWD/target/debug/killer_super python docs/check_examples.py  # every doc example
python docs/check_links.py                                               # relative links in the docs
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Issues and PRs welcome.

## License

MIT, see [LICENSE](LICENSE).
