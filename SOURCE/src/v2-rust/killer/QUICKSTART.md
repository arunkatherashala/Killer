# Killer Language -- 5-Minute Quick Start

This guide gets you from zero to running Killer programs in five minutes.
For the full language reference see `DOCS.md`.

---

## Step 1: Build

From the `killer` directory (where `Cargo.toml` lives):

```
cargo build --release
```

The compiled binaries appear in `target/release/`:

| Binary | Purpose |
|---|---|
| `killer_super` | Compile and run `.killer` source files |
| `killer_repl` | Interactive REPL |

---

## Step 2: Run a script

```
cargo run --bin killer_super -- my_program.killer
```

Or, after `cargo build --release`:

```
./target/release/killer_super my_program.killer
```

On Windows:

```
target\release\killer_super.exe my_program.killer
```

---

## Step 3: REPL

```
cargo run --bin killer_repl
```

Type `.help` at the prompt for a command list. Type `.exit` or `.quit` to quit.

---

## Step 4: Your first program

Create `hello.killer`:

```killer
-- Variables
let name = "Killer"
let version = 1

-- Interpolated string
let msg = K"Hello from {name} v{version}!"
println(msg)

-- Function
fn add(a, b) {
    return a + b
}
println(add(3, 4))

-- Array and loop
let nums = [1, 2, 3, 4, 5]
for n in nums {
    println(n * n)
}
```

Run it:

```
cargo run --bin killer_super -- hello.killer
```

Expected output:

```
Hello from Killer v1!
7
1
4
9
16
25
```

---

## Step 5: Your first Kore query

Create `kore_demo.killer`:

```killer
-- Write sample data into a .kore file
kore_write("sales.kore", "region:str,amount:float", "North,1200\nSouth,800\nEast,950\nWest,1100")

-- Query it with SQL syntax
let result = kore_query_table(
    "SELECT region, amount FROM sales.kore ORDER BY amount DESC"
)
println(result)

-- Read back one column
let regions = kore_read_col("sales.kore", "region")
println(regions)
```

Run it:

```
cargo run --bin killer_super -- kore_demo.killer
```

---

## Key language facts

- File extension: `.killer`
- Comments: `//` or `--`
- Variables: `let x = value`
- Functions: `fn name(params) { body }`
- Print: `println(value)`
- Types: Number, String, Bool, Null, Array, Dict, Set
- No semicolons required (newlines delimit statements)
- Indentation is optional but braces `{}` are required for blocks

---

## What to read next

- `DOCS.md` -- complete language reference with all builtins and examples
- `examples/` -- working sample programs
- `KALA_SMOKE.md` -- Kala AI engine quick reference
