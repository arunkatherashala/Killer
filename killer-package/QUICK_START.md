# ⚡ Killer - Quick Start (5 Minutes)

## Installation
Simply extract `killer.exe` from this package. No other dependencies needed!

---

## Example 1: Hello World

**File**: `hello.killer`
```killer
kfn main() {
  println("Hello, Killer!")
}

main()
```

**Run**:
```bash
killer.exe hello.killer
```

**Output**:
```
Hello, Killer!
```

---

## Example 2: Variables & Math

**File**: `math.killer`
```killer
kfn add(a, b) {
  a + b
}

result = add(5, 3)
println("5 + 3 = " + result.to_string())

# Fibonacci
kfn fib(n) {
  if n <= 1 {
    n
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

print("Fibonacci(10) = ")
println(fib(10).to_string())
```

**Run**:
```bash
killer.exe math.killer
```

**Output**:
```
5 + 3 = 8
Fibonacci(10) = 55
```

---

## Example 3: Collections (Lists)

**File**: `lists.killer`
```killer
numbers = [1, 2, 3, 4, 5]

for num in numbers {
  println(num.to_string())
}

# Sum
sum = 0
for n in numbers {
  sum = sum + n
}

println("Sum = " + sum.to_string())
```

**Run**:
```bash
killer.exe lists.killer
```

---

## Example 4: Pattern Matching

**File**: `matching.killer`
```killer
enum Color {
  Red,
  Green,
  Blue
}

kfn describe(color) {
  match color {
    Color::Red -> println("It's red!")
    Color::Green -> println("It's green!")
    Color::Blue -> println("It's blue!")
  }
}

describe(Color::Red)
describe(Color::Blue)
```

**Run**:
```bash
killer.exe matching.killer
```

---

## Key Syntax

```killer
# Variables (no 'let' needed)
x = 42

# Functions
kfn add(a, b) {
  a + b
}

# Output
println("text")      # print with newline
print("text")        # print without newline

# Conditionals
if x > 0 {
  println("positive")
}

# Loops
for item in list {
  println(item.to_string())
}

# Types (optional)
kfn double(x: Int) -> Int {
  x * 2
}

# Collections
list = [1, 2, 3]
map = {"key": "value"}
```

---

## Next Steps

1. **Try the examples** in `examples/` folder
2. **Read docs/SYNTAX.md** for complete reference
3. **Explore docs/ACTORS.md** for concurrency patterns

**Happy coding! 🔥**
