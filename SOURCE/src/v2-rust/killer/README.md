# Killer Language

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-1884%20unit%20%2B%20integration%20passing-brightgreen.svg)](#testing)
[![Version](https://img.shields.io/badge/version-2.1-blueviolet.svg)](#)

**Killer** is a high-performance, AI-native scripting language with a Rust-powered runtime. It combines Python-style expressiveness with native-speed execution via JIT compilation, trinary (3-valued) logic, built-in AI primitives, and a rich 600+ function standard library — all with zero external crate dependencies.

> Created by [Sai Arun Kumar Katherashala](https://github.com/arunkatherashala)

---

## What makes Killer unique?

| Feature | Other languages | Killer |
|---------|----------------|--------|
| Logic system | Binary (true/false) | **Trinary** (T_POS / T_NEG / T_ZERO) |
| Runtime | GC'd or manual | **Rust VM + tri-color cycle GC** |
| JIT | Optional/external | **Built-in x86-64 JIT** for pure numeric functions and arrays (x86-64 only) |
| AI primitives | Library calls | **Native `believe`, `async_spawn`, LLM client** |
| Dependencies | Many crates | **Zero external crate dependencies** |
| Package manager | Separate tool | **Built-in KPM** with local/registry install |

---

## Quick Start

```bash
# Build
cargo build --release

# Run a Killer program
cargo run --bin killer_super -- hello.killer --run

# Interactive REPL
cargo run --bin killer-repl

# Online playground
cargo run --bin killer-playground
# → open http://127.0.0.1:3000
```

---

## Language Overview

### Hello World

```killer
println("Hello, Killer!")
```

### Variables and arithmetic

```killer
x = 42
y = x * 2 + 1
println("Result: " + str(y))
```

### Functions

```killer
fn fib(n) {
  if n <= 1 { n }
  else { fib(n-1) + fib(n-2) }
}

for i in range(10) {
  println("fib(" + str(i) + ") = " + str(fib(i)))
}
```

### Arrays and list comprehensions

```killer
nums = [1, 2, 3, 4, 5]
doubled = [x * 2 for x in nums if x > 2]
println(str(doubled))   # [6, 8, 10]
```

### Trinary (3-valued) logic

```killer
a = T_POS    # positive / true
b = T_NEG    # negative / false
c = T_ZERO   # indeterminate / unknown

result = a AND c    # T_ZERO (Kleene logic)
println(str(result))
```

### Uncertain values

```killer
temp = believe 98.6 ± 0.4    # value with uncertainty bounds
println(str(temp))
```

### Async concurrency

```killer
fn worker(id) {
  println("Worker " + str(id) + " starting")
  id * 100
}

handle = async_spawn(fn() { worker(1) })
result = async_await(handle)
println("Got: " + str(result))
```

### JSON and HTTP

```killer
data = json_parse('{"name":"Killer","score":10}')
println(data["name"])

resp = http_get("https://api.example.com/data")
parsed = json_parse(resp)
```

### File I/O

```killer
writeFile("output.txt", "Hello from Killer!")
content = readFile("output.txt")
println(content)
```

---

## Architecture

```
Source (.killer)
    │
    ▼
Lexer → Tokens
    │
    ▼
Parser → AST
    │
    ▼ constant folding (compile-time)
Compiler → Bytecode
    │
    ▼ peephole optimizer
Optimizer → Bytecode (optimized)
    │
    ▼
VM (register-based, threaded dispatch)
    │         │
    ▼         ▼
JIT x86-64  GC (tri-color mark-and-sweep)
```

- **VM**: Register-style stack VM, ~60 opcodes, interpreter dispatch
- **JIT**: Hot loops compiled to native x86-64 (threshold: 500 iterations)
- **GC**: Tri-color mark-and-sweep on top of `Rc<RefCell<>>` to collect reference cycles
- **Optimizer**: Two-pass: AST-level constant folding + bytecode peephole pass

---

## Standard Library (600+ functions)

| Category | Functions |
|----------|-----------|
| Math | `abs`, `sqrt`, `pow`, `sin`, `cos`, `floor`, `ceil`, `log`, `gcd`, `lcm`, … |
| Strings | `len`, `split`, `join`, `trim`, `replace`, `upper`, `lower`, `contains`, `startsWith`, … |
| Arrays | `push`, `pop`, `sort`, `reverse`, `filter`, `map`, `reduce`, `zip`, `enumerate`, … |
| File I/O | `readFile`, `writeFile`, `appendFile`, `fileExists`, `listDir`, `mkdir`, `pathJoin`, … |
| HTTP | `http_get`, `http_post`, `http_put`, `http_delete`, `http_get_json`, `http_with_headers`, … |
| JSON | `json_parse`, `json_stringify` |
| Date/Time | `date_now`, `date_parse`, `date_format`, `date_diff`, `date_add`, `timestamp`, … |
| Concurrency | `async_spawn`, `async_await`, `chan_send`, `chan_recv`, `mutex_new`, … |
| AI | `llm_complete`, `llm_embed`, `vector_store`, `vector_search`, `khlm_route`, … |
| Crypto | `sha256`, `hmac_sha256`, `base64_encode`, `base64_decode`, `aes_encrypt`, … |
| System | `env`, `exit`, `args`, `sleep`, `gc_stats`, `jit_stats`, … |

---

## Binaries

| Binary | Purpose |
|--------|---------|
| `killer_super` | Main compiler/runner — execute `.killer` files |
| `killer-repl` | Interactive REPL with persistent variables and `.gc`/`.jit` commands |
| `killer-playground` | Web playground server at `http://127.0.0.1:3000` |
| `kpm-registry` | KPM package registry server |
| `killer-mcp` | MCP server — AI assistant tool bridge |
| `killer_ui_serve` | Kala UI chat server |
| `ghost_hive` | Ghost VM distributed coordinator |
| `kore_bench` | KORE/Nova format benchmarks |

---

## Package Manager (KPM)

```killer
# In a .killer file or REPL
kpm_install("math-utils")
kpm_install("http-helpers@1.2.0")

import math_utils
println(math_utils.factorial(10))
```

```bash
# Start the registry
cargo run --bin kpm-registry
# → http://127.0.0.1:8080
```

---

## Performance

Killer has two execution tiers:

- A **bytecode interpreter** for everything.
- A **built-in x86-64 JIT** that compiles *pure numeric* functions (numbers, arrays of numbers,
  loops, `range`, `for ... in`, `sqrt/abs/floor/ceil/min/max`, calls to other such functions) to
  machine code. Anything it cannot prove safe runs in the interpreter, and native code that hits
  division by zero, a bad index or runaway recursion falls back to the interpreter automatically.

Measured against CPython on the same machine, same algorithm, identical output (best of 5 runs;
timings include process start-up and vary from run to run, so read the ratios, not the seconds):

| Benchmark | Killer | Python | Result |
|-----------|--------|--------|--------|
| Bubble sort, 4000 numbers | 0.24 s | 1.66 s | 6.8x faster |
| Collatz chains (loops, `%`) | 0.87 s | 4.06 s | 4.7x faster |
| Count primes below 300k (`for`, `sqrt`) | 0.33 s | 0.49 s | 1.5x faster |
| Dot product of arrays | 0.16 s | 0.22 s | 1.4x faster |
| String building (`s = s + "x"`) | 0.14 s | 0.17 s | 1.3x faster |
| `fib(30)` | 0.19 s | 0.24 s | 1.3x faster |
| Top-level `while` loop | 0.43 s | 0.37 s | about level |
| Array `push` loop | 0.23 s | 0.14 s | 1.6x slower |
| Dict counting | 0.29 s | 0.12 s | 2.5x slower |
| Object method calls | 0.40 s | 0.12 s | 3.3x slower |

**Honest summary:** numeric and string-building code is faster than CPython; code dominated by
dicts, objects and array building is still slower, because the JIT does not handle those yet.
Killer is not a replacement for C, C++, Java or .NET on performance.

Run the benchmarks yourself with `cargo bench --bench vm_runtime`.

---

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

## Known limitations

- The JIT is x86-64 only and covers pure numeric code; everything else is interpreted.
- Dicts and objects are shared references (like Python or JavaScript): `b = a` aliases, and a
  dict or object that contains itself is never garbage-collected.
- The FFI is minimal (see above) and has no safety net against a wrong signature.
- Killer has no package ecosystem to speak of and is a one-person project.

---

## Testing

```bash
# Unit tests (1884 tests)
cargo test --lib

# Integration tests
cargo test --test pipeline_conformance
cargo test --test trit_three_valued

# All tests
cargo test
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get started. Issues and PRs welcome!

---

## License

MIT — see [LICENSE](LICENSE)
