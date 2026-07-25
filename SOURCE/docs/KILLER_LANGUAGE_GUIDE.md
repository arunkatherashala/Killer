# Killer Language — User Guide

> Version 2.1.0 "Enterprise" | Updated March 31, 2026

This is the **single entry-point** document for learning **Killer** — a systems-grade programming language with a Rust-native runtime, zero external dependencies, and first-class support for balanced ternary, AI integration, and columnar data compression.

Pipeline: `lexer` → `stmt_parser` → `compiler` → `optimizer` → `vm`

---

## 1. Install & Run

### Building from source

```bash
cd SOURCE/src/v2-rust/killer
cargo build --release
```

The binary is at `target/release/killer-native.exe` (Windows) or `target/release/killer-native` (Linux/Mac).

### Running a program

```bash
killer-native my_program.killer
```

### Embedding the runtime

```rust
use killer_native::run_killer_source;

fn main() {
    let code = r#"print("Hello from Killer")"#;
    run_killer_source(code);
}
```

---

## 2. Hello World

```killer
print("Hello, Killer")
println("Hello with newline")
```

K-strings support interpolation:

```killer
let name = "World"
println(K"Hello, {name}!")
```

---

## 3. Variables & Assignment

Use `let` for bindings. Variables are block-scoped (Python-style indentation).

```killer
let x = 1
let y = x + 2
let greeting = "hello"
```

### Destructuring

Array and object destructuring is supported:

```killer
let [a, b, c] = [10, 20, 30]
let {name, age} = {name: "Alice", age: 30}
println(K"{name} is {age}")
```

### Compiler internals

The compiler lowers locals to **slots** (`LoadSlot`/`StoreSlot`) for O(1) access. Stores to a slot that is never read are turned into `Pop` by the dead-store elimination pass.

---

## 4. Functions

```killer
kfn add(a, b):
    return a + b

println(add(3, 4))   # 7
```

### Tail-call optimization

If the last statement in a function calls **itself**, the compiler emits a `TailCall` opcode — the VM reuses the current activation frame instead of growing the call stack. This enables deep recursion without stack overflow:

```killer
kfn factorial(n, acc):
    if n <= 1:
        return acc
    return factorial(n - 1, n * acc)

println(factorial(100000, 1))   # No stack overflow
```

### Closures / function expressions

```killer
let double = |x|:
    return x * 2

println(double(5))   # 10
```

---

## 5. Control Flow

### Conditionals

```killer
if x > 10:
    println("big")
else:
    println("small")
```

### While loops

```killer
let i = 0
while i < 5:
    println(i)
    i = i + 1
```

### For loops

```killer
for item in [1, 2, 3]:
    println(item)

for i in range(10):
    println(i)
```

### Do-while

```killer
do:
    println("runs at least once")
while false
```

Backward branches are where the VM checks the **wall-clock execution budget** (if configured) to stop runaway loops.

---

## 6. Pattern Matching

Killer supports `match` expressions with pattern arms, guards, and destructuring:

```killer
match value:
    42 => println("the answer")
    "hello" => println("greeting")
    [x, y] => println(K"pair: {x}, {y}")
    {name, age} if age > 18 => println(K"{name} is an adult")
    _ => println("wildcard catch-all")
```

Patterns can be:
- **Literals**: numbers, strings, booleans, null
- **Identifiers**: bind the matched value to a name
- **Wildcards**: `_` matches anything without binding
- **Arrays**: `[a, b, c]` destructures arrays by index
- **Objects**: `{key: pattern}` destructures objects by key
- **Guards**: `pattern if condition =>` adds a boolean filter

---

## 7. Types & Literals

| Type | Syntax | Example |
|------|--------|---------|
| Number | `42`, `3.14` | `let pi = 3.14` |
| String | `"hello"` | `let s = "hi"` |
| K-String | `K"..."` | `K"value is {x}"` |
| Bool | `true`, `false` | `let ok = true` |
| Null | `null` | `let n = null` |
| Array | `[a, b]` | `let arr = [1, 2, 3]` |
| Dict | `{k: v}` | `let d = {x: 1, y: 2}` |
| Trit | `T_POS`, `T_NEG`, `T_ZERO` | Balanced ternary values |
| Signal | `signal_create(...)` | Trit + confidence + source |
| Qubit | `qubit_create(...)` | Quantum-inspired superposition |

### Balanced ternary (unique to Killer)

Killer has native trit/tryte types — a three-valued logic system:

```killer
let a = trit_pos()         # +1
let b = trit_neg()         # -1
let c = trit_and(a, b)     # -1 (Kleene AND)
let w = trit_word(a)       # "True"
println(w)
```

### Signals — trit + confidence + source

```killer
let sig = signal_create(trit_pos(), 0.95, "sensor_1")
println(signal_to_str(sig))
let fused = signal_and(sig, sig)
println(signal_confident(fused, 0.5))   # true
```

### Fuzzy logic

```killer
let t = fuzzy_threshold(0.78, 0.55)     # trit (pos if > 0.55)
let w = trit_word(t)                     # "True"
```

---

## 8. Classes & Methods

```killer
class Animal:
    init(name, sound):
        self.name = name
        self.sound = sound

    speak():
        println(K"{self.name} says {self.sound}")

class Dog extends Animal:
    fetch(item):
        println(K"{self.name} fetches {item}")

let d = new Dog("Rex", "Woof")
d.speak()            # Rex says Woof
d.fetch("ball")      # Rex fetches ball
```

---

## 9. Error Handling

```killer
try:
    let result = risky_operation()
    println(result)
catch(e):
    println(K"Error: {e}")
finally:
    println("cleanup done")
```

`throw("message")` raises an exception caught by the nearest `try/catch`.

---

## 10. Async & Concurrency

### Spawn (fire-and-forget)

```killer
spawn long_running_task()
```

### Async functions

```killer
async fn fetch_data():
    return http_get("https://api.example.com/data")

let future = fetch_data()
let result = await future
```

---

## 11. Builtins (285+)

Full reference: `BUILTIN_REFERENCE.md`. Key categories:

| Category | Examples |
|----------|---------|
| I/O | `print`, `println`, `read_file`, `write_file` |
| Math | `abs`, `sqrt`, `pow`, `floor`, `ceil`, `round` |
| Strings | `len`, `split`, `trim`, `upper`, `lower`, `replace` |
| Arrays | `push`, `pop`, `sort`, `map`, `filter`, `reduce` |
| JSON | `parse_json`, `to_json` |
| HTTP | `http_get`, `http_post`, `http_put`, `http_delete` |
| Trit | `trit_pos`, `trit_neg`, `trit_and`, `trit_or`, `trit_not` |
| Signal | `signal_create`, `signal_and`, `signal_or`, `signal_confident` |
| Qubit | `qubit_create`, `qubit_measure`, `qubit_hadamard` |
| AI/LLM | `kala_ask`, `khlm_with_tools`, `tool_list` |
| Nova | `nova_compress`, `nova_decompress`, `nova_info` |

---

## 12. AI Integration (Kala)

Killer has built-in LLM support via the KhLM 5-tier intelligence router:

```killer
# Simple inference (Ollama local)
let answer = kala_ask("What is the meaning of life?")
println(answer)

# Tool-calling (model picks tools automatically)
let result = khlm_with_tools("Read data.json and summarize it")
println(result)
```

### Setup

```powershell
./scripts/kala-setup.ps1
```

### Supported providers

| Tier | Provider | Model |
|------|----------|-------|
| 1 | Local regex | Pattern matching (fastest) |
| 2 | Ollama | phi3:mini, llama3 (local) |
| 3 | OpenAI | gpt-4o (cloud) |
| 4 | Anthropic | claude-3 (cloud) |
| 5 | Groq | llama-70b (cloud, fast) |

---

## 13. Nova Compression

Nova columnar compression engine — **84% reduction**, beating Apache Parquet:

```killer
nova_compress("data.csv", "data.kore")
let info = nova_info("data.kore")
println(info)
let data = nova_decompress("data.kore")
```

### Codecs

| Codec | Use case |
|-------|----------|
| RLE | Repeated values (status columns) |
| Delta | Sequential IDs, timestamps |
| Dict | Low-cardinality strings |
| BDICT | Bit-packed dictionary (int/float/str) |
| CDELTA | Constant-delta (sequential IDs = 2 bytes total) |
| BITS | Boolean columns (8 rows per byte) |
| NOVT | Trit arrays (2 bits per trit) |
| NOVD | Dense base-3 (1.585 bits per trit) |

---

## 14. Security Sandbox

Killer enforces **capability-based security** at every I/O boundary:

```rust
let mut vm = VirtualMachine::new();
vm.set_capabilities(CapabilitySet {
    allow_file_read: false,
    allow_file_write: false,
    allow_network: true,
    allow_process_spawn: false,
    allow_llm: true,
    ..CapabilitySet::default()
});
```

Every builtin that performs I/O calls a `require_*()` check:
- `require_file_read()` / `require_file_write()` — filesystem
- `require_network()` — HTTP, TCP
- `require_process_spawn()` — child processes
- `require_llm()` — AI/LLM inference

### Execution budget

```rust
vm.set_execution_budget(ExecutionBudget {
    max_instruction_steps: 50_000_000,
    max_wall_ms: 120_000,
    max_heap_bytes: 256 * 1024 * 1024,
});
```

Step budget checked every opcode. Wall-clock checked on backward jumps (cheap).

---

## 15. Formatter & Linter

### Format

```bash
killer-native --format my_program.killer
```

30+ style rules: indentation, spacing, line breaks, keywords, trailing commas, braces, import ordering.

### Lint

The linter reports `INFO`, `WARN`, `ERROR` with suggestions and line/column tracking.

---

## 16. Tool Calling

Register tools for LLM agents to invoke:

```killer
# List available tools
println(tool_list())

# Let the LLM use tools to answer
let prompt = "Read data.json and tell me the answer field"
println(khlm_with_tools(prompt))
```

The engine intercepts `TOOL_CALL: {"name":"readFile","args":[...]}` lines from the model, executes the tool via `builtin_dispatch`, and feeds `TOOL_RESULT` back until the model's answer is final.

---

## 17. Import & Modules

```killer
import math
import "utils/helpers"
```

---

## 18. Performance Notes

| Metric | Value |
|--------|-------|
| Loop iteration rate | 13–50M iter/sec |
| Startup overhead | ~60ms |
| Instructions per loop iter | 4 (fused) |
| Value enum size | 40 bytes |
| Recursive call rate | ~440K calls/sec |
| String formatting | ~400K K"" ops/sec |

The optimizer applies:
- **Constant folding** (numeric, boolean, trit)
- **Dead-store elimination** (unused `StoreSlot` → `Pop`)
- **Peephole fusion** (`LoadSlot+ConstNum+Add+StoreSlot` → `AddSlotConst`)
- **`LtSlotConst`** — fused loop condition (no stack ops)

---

## 19. Roadmap

| Feature | Status |
|---------|--------|
| Pattern matching (`match`) | Done |
| Destructuring (`let [a,b] = ...`) | Done |
| Tail-call optimization | Done |
| Dead-store elimination | Done |
| Constant folding | Done |
| Capability sandbox | Done |
| Execution budget | Done |
| x86-64 JIT (hot loops) | Done |
| Nova compression v3 | Done |
| Formatter (30+ rules) | Done |
| Linter (3 severity levels) | Done |
| NaN-boxing | In progress |
| Parser merge | Planned |

---

## 20. Related Docs

- **`KILLER_GRAMMAR.ebnf`** — Formal EBNF grammar
- **`BUILTIN_REFERENCE.md`** — All 285+ builtins
- **`UNSAFE_AUDIT.md`** — Where `unsafe` lives and why
- **`FUZZING.md`** — Parser fuzzing setup
- **`IMPROVEMENT_10_TRACKER.md`** — Progress tracker
