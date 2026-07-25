# Killer Language Reference

## Table of Contents
1. [Basics](#basics)
2. [Types](#types)
3. [Operators](#operators)
4. [Control Flow](#control-flow)
5. [Functions](#functions)
6. [Objects and Classes](#objects-and-classes)
7. [Advanced Features](#advanced-features)
8. [Standard Library](#standard-library)
9. [Error Handling](#error-handling)

---

## Basics

### Comments
```killer
// Single-line comment
/* Multi-line
   comment */
```

### Variables
```killer
let x = 10          // Immutable variable
x = 20              // Implicit reassignment (declaration optional)
let y = "hello"     // String
let z = true        // Boolean
let a = null        // Null value
```

### Variable Scoping
Killer uses lexical scoping with proper shadowing and nested block support:

```killer
let x = 10
{
  let x = 20        // Shadows outer x
  print(x)          // 20
}
print(x)            // 10 (outer x still in scope)
```

---

## Types

### Primitives
- **Number**: IEEE 754 64-bit float
  ```killer
  let n = 3.14
  let i = 42        // Also a number (not distinct integer type)
  ```

- **String**: UTF-8 text
  ```killer
  let s = "hello"
  let interpolated = "x = {x}"  // String interpolation with {}
  ```

- **Boolean**: `true` or `false`
  ```killer
  let b = true
  if b { print("yes") }
  ```

- **Null**: Undefined/missing value
  ```killer
  let missing = null
  if missing == null { print("no value") }
  ```

### Collections
- **Array**: Ordered list (0-indexed)
  ```killer
  let arr = [1, 2, 3]
  arr[0]              // 1
  arr.push(4)         // arr is now [1, 2, 3, 4]
  ```

- **Dictionary**: Key-value map
  ```killer
  let obj = { "name": "Alice", "age": 30 }
  obj["name"]         // "Alice"
  obj.name            // Also "Alice" (dot notation)
  ```

### Advanced Types (Nova Galaxy)

- **Trit**: Balanced ternary (-1, 0, +1)
  ```killer
  let t = T(+1)       // Create trit
  let logic = t && T(-1)  // Trit AND
  ```

- **Signal**: Value with confidence
  ```killer
  let sig = Signal(+1, 0.95, "confident")
  print(sig.value)        // +1
  print(sig.confidence)   // 0.95
  ```

- **Qubit**: Probabilistic bit
  ```killer
  let q = Qubit(0.5)  // 50% chance to measure as 1
  if measure(q) {     // Probabilistic measurement
    print("got 1")
  } else {
    print("got 0")
  }
  ```

- **Tryte**: 6-trit word (word in balanced ternary)
  ```killer
  let w = Tryte([+1, 0, -1, +1, 0, -1])
  ```

---

## Operators

### Arithmetic
```killer
x + y       // Addition
x - y       // Subtraction
x * y       // Multiplication
x / y       // Division (floating point)
x // y      // Floor division (integer result)
x % y       // Modulo (remainder)
x ^ y       // Exponentiation
```

### Comparison
```killer
x == y      // Equality
x != y      // Inequality
x > y       // Greater than
x >= y      // Greater than or equal
x < y       // Less than
x <= y      // Less than or equal
```

### Logical
```killer
a && b      // Logical AND (short-circuit)
a || b      // Logical OR (short-circuit)
!a          // Logical NOT
```

### Bitwise (on trits)
```killer
t1 && t2    // Trit AND (min)
t1 || t2    // Trit OR (max)
!t          // Trit NOT (negate)
```

### String Operators
```killer
"hello" + " " + "world"  // String concatenation
"x = {10}"               // String interpolation
```

### Assignment
```killer
x = 5               // Assign
x += 1              // x = x + 1
x -= 1              // x = x - 1
x *= 2              // x = x * 2
x /= 2              // x = x / 2
```

---

## Control Flow

### if/else
```killer
if condition {
  // then branch
} else {
  // else branch
}

// Shorthand
if x > 0 { print("positive") } else { print("non-positive") }
```

### while
```killer
while condition {
  // body
  if special_case { break }
}

// Do-while
do {
  // body
} while condition
```

### for
```killer
for i in range(10) {
  print(i)  // 0, 1, 2, ..., 9
}

for item in array {
  print(item)
}

for key in obj.keys() {
  print(obj[key])
}

// C-style for
for (let i = 0; i < 10; i = i + 1) {
  print(i)
}
```

### switch/case
```killer
switch x {
  case 1: print("one")
  case 2: print("two")
  default: print("other")
}
```

### match (Pattern Matching)
```killer
match value {
  [1, x, 3] => print("middle is", x)
  {name: n, age: a} => print(n, "is", a, "years old")
  null => print("no value")
  _ => print("other")
}
```

### break/continue
```killer
for i in range(10) {
  if i == 5 { break }      // Exit loop
  if i == 2 { continue }   // Skip to next iteration
  print(i)
}
```

---

## Functions

### Declaration
```killer
def greet(name) {
  return "Hello, " + name
}

def add(a, b) {
  return a + b
}

// Without explicit return, last expression is returned
def multiply(a, b) {
  a * b
}
```

### Calling
```killer
greet("Alice")     // "Hello, Alice"
add(3, 4)          // 7
```

### Anonymous Functions (Lambdas)
```killer
let double = def(x) { x * 2 }
double(5)          // 10

// In array operations
arr = [1, 2, 3].map(def(x) { x * 2 })  // [2, 4, 6]
```

### Recursion
```killer
def factorial(n) {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}

factorial(5)  // 120
```

### Variable Capture (Closures)
```killer
def make_adder(x) {
  def adder(y) {
    return x + y
  }
  return adder
}

add5 = make_adder(5)
add5(3)        // 8
```

---

## Objects and Classes

### Object Literals
```killer
person = {
  name: "Alice",
  age: 30,
  greet: def(self) { print("Hi, I'm", self.name) }
}

person.name        // "Alice"
person.age         // 30
person.greet()     // Prints: Hi, I'm Alice
```

### Classes
```killer
class Animal {
  def __init__(name) {
    self.name = name
  }
  
  def speak() {
    print(self.name, "speaks")
  }
}

class Dog extends Animal {
  def speak() {
    print(self.name, "barks")
  }
}

dog = new Dog("Buddy")
dog.speak()          // Buddy barks
```

### Properties and Methods
```killer
class Rectangle {
  def __init__(width, height) {
    self.width = width
    self.height = height
  }
  
  def area() {
    return self.width * self.height
  }
  
  def perimeter() {
    return 2 * (self.width + self.height)
  }
}

rect = new Rectangle(3, 4)
print(rect.area())       // 12
print(rect.perimeter())  // 14
```

---

## Advanced Features

### Exception Handling
```killer
try {
  // risky code
  x = 1 / 0
} catch error {
  print("Error:", error)
} finally {
  print("cleanup")
}

throw "custom error"
```

### Generators (Yield)
```killer
def count_up_to(n) {
  for i in range(n) {
    yield i
  }
}

for num in count_up_to(5) {
  print(num)  // 0, 1, 2, 3, 4
}
```

### Concurrency

#### Async Functions
```killer
async def fetch_data() {
  // Runs in background thread
  return get_data_from_api()
}

result = fetch_data()  // Returns immediately with future
// Later...
value = await result   // Block until complete
```

#### Spawn (Fire & Forget)
```killer
spawn expensive_computation()  // Runs in background, result discarded
```

### Module System (Import/Export)

#### Importing
```killer
// Import all exports from a module
import "json"

// Selective import
import { parse, stringify } from "json"

// Import with alias
import "collections" as coll
coll.map(arr, fn)

// Import from packages directory
import "my-package"
```

#### Exporting
```killer
// Export specific names
export helper, process, format

// In another file
import { helper } from "my-module"
```

### Data Quality
```killer
// Mark a value with quality metadata
quality x = compute_value()

// Access quality information
if x has_quality {
  print("Quality:", x.quality_score)
}
```

---

## Standard Library

Killer includes a comprehensive standard library. See [stdlib/README.md](../stdlib/README.md) for complete documentation.

### Common Modules

#### io
```killer
import "io"
content = io.read_file("data.txt")
io.write_file("output.txt", content)
```

#### json
```killer
import { parse, stringify } from "json"
obj = parse('{"name": "Alice"}')
json_str = stringify(obj)
```

#### collections
```killer
import "collections"
arr = [1, 2, 3]
doubled = arr.map(def(x) { x * 2 })  // [2, 4, 6]
evens = arr.filter(def(x) { x % 2 == 0 })  // [2]
```

#### math
```killer
import { sqrt, sin, PI } from "math"
x = sqrt(16)  // 4
y = sin(PI / 2)  // 1.0
```

#### string
```killer
import "string"
upper = "hello".uppercase()  // "HELLO"
words = "a,b,c".split(",")   // ["a", "b", "c"]
```

---

## Error Handling

### Runtime Errors
```killer
try {
  x = undefined_var   // Error!
} catch e {
  print("Caught:", e)
}
```

### Type Conversions
```killer
str(42)         // "42"
str(true)       // "true"
str([1,2,3])    // "[1, 2, 3]"

num("42")       // 42
num("3.14")     // 3.14

type(x)         // "number", "string", "array", "dict", etc.
```

### Null Coalescing
```killer
x = missing ?? default_value
```

---

## Best Practices

1. **Use meaningful names**: `total_price` not `tp`
2. **Keep functions small**: Single responsibility principle
3. **Use type hints in comments**: `// x: number`
4. **Handle errors**: Always check for null/error cases
5. **Use stdlib**: Don't reinvent the wheel
6. **Modularize**: Break code into small files with imports
7. **Performance**: Use slotted operations for hot loops

---

## Language Limits

| Feature | Limit | Notes |
|---------|-------|-------|
| Recursion Depth | 10,000 | Prevents stack overflow |
| String Length | 1GB | Practical limit (memory) |
| Array Size | 1GB elements | Practical limit |
| Variable Name Length | Unlimited | Path: `a.b.c.d...` |
| File Size | Unlimited | Streamed parsing |

---

## See Also

- [Standard Library Reference](../stdlib/README.md)
- [Quickstart Guide](./quickstart.md)
- [Examples](../examples/)
- [KPM Package Manager](../tools/kpm/)
