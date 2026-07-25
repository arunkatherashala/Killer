# Killer Syntax Reference

## Variables

```killer
# No 'let' keyword needed
x = 42
name = "Killer"
ready = true

# With type annotations (optional)
count: Int = 100
message: String = "Hello"
```

## Functions

```killer
# Basic function
kfn add(a, b) {
  a + b
}

# With type annotations
kfn multiply(x: Int, y: Int) -> Int {
  x * y
}

# Function call
result = add(3, 5)
```

## Control Flow

```killer
# If/Else
if x > 0 {
  println("Positive")
} else if x < 0 {
  println("Negative")
} else {
  println("Zero")
}

# Loops
for i in [1, 2, 3] {
  println(i.to_string())
}

# While (note: not commonly used in Killer)
```

## Collections

```killer
# Lists
numbers = [1, 2, 3, 4, 5]
first = numbers[0]

for item in numbers {
  println(item.to_string())
}

# Maps
data = {"name": "Killer", "version": "1.1"}
value = data["name"]
```

## Enums & Pattern Matching

```killer
enum Status {
  Active,
  Inactive,
  Pending
}

match status {
  Status::Active -> println("Running")
  Status::Inactive -> println("Stopped")
  Status::Pending -> println("Waiting")
}
```

## Strings

```killer
# String literals
text = "Hello"

# String concatenation
greeting = "Hello, " + "Killer!"

# Convert to string
num = 42
str = num.to_string()
```

## Output

```killer
# Print without newline
print("Loading...")

# Print with newline
println("Done!")

# Print values
value = 123
println(value.to_string())
```

## Comments

```killer
# This is a comment

# Multi-line comments use multiple #
# Line 1
# Line 2
```

## Type System

```killer
# Basic types
Int      # 32-bit integer
String   # Text
Bool     # true/false
List<T>  # List of type T
Map<K,V> # Map of key-value pairs

# Type annotations
kfn process(data: Int, name: String) -> String {
  name + ": " + data.to_string()
}
```

---

For more patterns, see **ACTORS.md** and **PERFORMANCE.md**.
