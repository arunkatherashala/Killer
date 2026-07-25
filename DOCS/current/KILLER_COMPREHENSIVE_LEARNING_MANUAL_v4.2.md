# KILLER LANGUAGE: COMPREHENSIVE LEARNING MANUAL

**Version:** 4.2  
**Date:** March 20, 2026  
**For:** Team Learning & Public Market Release  
**Status:** Production Ready

---

# TABLE OF CONTENTS

1. Introduction & Getting Started
2. Core Language Fundamentals
3. Type System & Variables
4. Control Flow
5. Functions & Closures
6. Collections & Data Structures
7. Pattern Matching & Enumerations
8. Object-Oriented Programming
9. Actors & Concurrency
10. Streams & Window Aggregation
11. Error Handling
12. Advanced Patterns
13. Best Practices
14. Real-world Examples
15. Troubleshooting & FAQ

---

# CHAPTER 1: INTRODUCTION & GETTING STARTED

## 1.1 What is the Killer Language?

The Killer language is a modern, high-performance systems programming language designed for real-time applications, concurrent systems, and stream processing. It combines:

- **Elegant syntax** similar to Python and Rust
- **Built-in concurrency** through the actor model
- **Predictable performance** with < 1ms p99 latencies
- **Strong type system** with type inference
- **Real-world patterns** for microservices, analytics, and distributed systems

### Design Philosophy

**"Simple for simple things, powerful for complex things"**

Killer was designed with a hybrid syntax approach:
- **Simple code** uses clean indentation (like Python)
- **Complex code** can use braces for clarity (like C/Rust)
- Everything is backward compatible

## 1.2 Language History & Evolution

| Version | Release | Features |
|---------|---------|----------|
| v1.0 | 2024 | Core language, basic types |
| v2.0 | 2024 | Generators, bytecode VM |
| v3.0 | 2025 | Full OOP, exceptions |
| v4.0 | 2025 | Actors, async/await |
| **v4.2** | **2026** | **Hybrid syntax, streaming** |

## 1.3 Key Strengths vs Other Languages

```
Feature         | Killer | Python | Go    | Rust
â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
Syntax          | â˜…â˜…â˜…â˜…â˜… | â˜…â˜…â˜…â˜…â˜… | â˜…â˜…â˜…  | â˜…â˜…â˜…
Concurrency     | â˜…â˜…â˜…â˜…â˜… | â˜…â˜†â˜†â˜†â˜† | â˜…â˜…â˜…â˜… | â˜…â˜…â˜…â˜…
Performance     | â˜…â˜…â˜…â˜…â˜† | â˜…â˜…â˜†â˜†â˜† | â˜…â˜…â˜…â˜… | â˜…â˜…â˜…â˜…â˜…
Learning Curve  | â˜…â˜…â˜…â˜…â˜† | â˜…â˜…â˜…â˜…â˜… | â˜…â˜…â˜…â˜… | â˜…â˜…â˜…
Real-time Ready | â˜…â˜…â˜…â˜…â˜… | â˜…â˜…â˜†â˜†â˜† | â˜…â˜…â˜…â˜† | â˜…â˜…â˜…â˜…â˜…
```

## 1.4 Common Use Cases

âœ… **Real-time systems** - <1ms latency critical applications  
âœ… **Microservices** - API servers, gateways, workers  
âœ… **Stream processing** - Analytics, event aggregation  
âœ… **Concurrent systems** - Multi-threaded applications  
âœ… **Data processing** - MapReduce, batch jobs  
âœ… **Teaching** - Learn concurrency fundamentals  

## 1.5 Installation & Setup

### Windows
```powershell
# Download
curl -O https://killer-lang.org/killer-windows-latest.exe

# Install
.\killer-windows-latest.exe

# Verify
killer --version
```

### macOS
```bash
brew install killer-lang
killer --version
```

### Linux
```bash
wget https://killer-lang.org/killer-linux-latest.tar.gz
tar xzf killer-linux-latest.tar.gz
sudo ./install.sh
killer --version
```

## 1.6 Your First Killer Program

Create file: `hello.killer`

```killer
kfn main
  print("Hello, Killer World!")
```

Run it:
```bash
killer run hello.killer
```

Output:
```
Hello, Killer World!
```

**Congratulations!** You've written your first Killer program.

---

# CHAPTER 2: CORE LANGUAGE FUNDAMENTALS

## 2.1 Indentation-Based Syntax

Killer uses Python-style indentation. Blocks are defined by indentation levels:

```killer
kfn greet(name: String)
  let greeting = "Hello, " + name
  print(greeting)
  
  if name.len() > 0
    print("Your name is: " + name)
  else
    print("You have no name")
```

### Key Rules

**Two spaces per indentation level** (never tabs):
```killer
kfn example
  print("Level 1")
  if true
    print("Level 2")
    if true
      print("Level 3")
```

**Empty lines don't affect indentation:**
```killer
kfn process
  let x = 1
  print("Starting")
  
  let y = 2  # Empty line above doesn't break context
  print("Done")
```

## 2.2 Hybrid Syntax with Braces

For complex logic, you can optionally use braces. Both styles work together:

```killer
# STYLE 1: Pure Indentation (Simple)
kfn calculate(x: Int)
  if x > 0
    return x * 2
  else
    return x * -2

# STYLE 2: With Braces (Complex)
kfn calculate(x: Int) {
  if x > 0 {
    return x * 2
  } else {
    return x * -2
  }
}

# STYLE 3: Hybrid (Mixed) - Both work!
kfn calculate(x: Int) {
  if x > 0
    return x * 2
  else
    return x * -2
}
```

**Best Practice:** Use indentation for simple functions, braces when nesting gets deep (3+ levels).

## 2.3 Comments

Single-line comments with `#`:
```killer
# This is a comment
kfn process
  let x = 1  # Inline comment
  print(x)   # Another comment
```

Documentation comments with `##`:
```killer
## Calculates the sum of two numbers.
## Args: a (Int), b (Int)
## Returns: Sum as Int
kfn add(a: Int, b: Int) -> Int
  a + b
```

## 2.4 Variables and Constants

### Variables (Mutable)

```killer
kfn demo
  # Type annotation explicit
  let x: Int = 5
  x = 10  # Can be reassigned
  
  # Type inference
  let y = "Hello"  # Killer knows it's String
  y = "World"      # Can change
```

### Constants (Immutable)

```killer
let PI = 3.14159  # Global constant
let MAX_SIZE = 1000

kfn circle_area(radius: Float) -> Float
  PI * radius * radius
```

### Naming Conventions

```killer
# Variables and functions: snake_case
let user_count = 42
kfn calculate_total() -> Int
  ...

# Constants: UPPER_CASE
let MAX_CONNECTIONS = 1000

# Types and Enums: PascalCase
type User
  name: String
  age: Int

enum Status
  Active
  Inactive
```

## 2.5 Print Statements

Basic printing:
```killer
print("Hello")
print(42)
print(3.14)
print(true)
```

String concatenation:
```killer
let name = "Alice"
print("Hello, " + name)     # "Hello, Alice"
print("Number: " + "42")    # "Number: 42"
```

Converting to strings:
```killer
let count = 42
print("Count: " + count.to_string())

let pi = 3.14
print("Pi: " + pi.to_string())
```

Formatted output:
```killer
let x = 10
let y = 20
print("x=" + x.to_string() + ", y=" + y.to_string())
# Output: x=10, y=20
```

---

# CHAPTER 3: TYPE SYSTEM & VARIABLES

## 3.1 Primitive Types

### Integer (Int)

```killer
let count: Int = 42
let negative: Int = -100
let zero: Int = 0

# No limit on size (arbitrary precision)
let big: Int = 999999999999999999
```

Operations:
```killer
let a = 10
let b = 3

let sum = a + b          # 13
let difference = a - b   # 7
let product = a * b      # 30
let quotient = a / b     # 3 (integer division)
let remainder = a % b    # 1
let power = a ^ 2        # 100
```

### Float

```killer
let pi: Float = 3.14159
let e: Float = 2.71828
let price: Float = 99.99
```

Operations:
```killer
let x = 10.5
let y = 3.0

let sum = x + y          # 13.5
let product = x * y      # 31.5
let ratio = x / y        # 3.5
```

Conversion:
```killer
let x: Int = 42
let y: Float = x.to_float()        # 42.0

let a: Float = 3.14
let b: Int = a.to_int()            # 3 (truncates)
```

### String

```killer
let greeting: String = "Hello"
let name: String = "World"
let message = greeting + ", " + name
```

String methods:
```killer
let text = "Killer"

let length = text.len()             # 6
let upper = text.upper()            # "KILLER"
let lower = text.lower()            # "killer"
let contains = text.contains("ill") # true
```

String slicing:
```killer
let word = "Helloworld"
let first_letter = word[0]          # "H"
let first_five = word[0..5]         # "Hello"
```

### Boolean

```killer
let is_active: Bool = true
let is_complete: Bool = false

# Logical operations
let a = true
let b = false

print(a && b)    # false (AND)
print(a || b)    # true (OR)
print(!a)        # false (NOT)
```

## 3.2 Type Inference

Killer automatically detects types - **no annotations needed**:

```killer
# Simple, clean style - type inference automatic
let x = 42              # Int inferred
let y = 3.14            # Float inferred
let name = "Alice"      # String inferred
let active = true       # Bool inferred

kfn add(a, b)          # Parameter types inferred: Int, Int
  a + b

print(add(10, 5))      # 15
```

## 3.3 Type Coercion

Automatic conversion in expressions:
```killer
let x: Int = 10
let y: Float = 3.5
let result = x.to_float() + y    # Float: 13.5

let count = 42
print("Count: " + count.to_string())  # String concatenation
```

## 3.4 Complex Types

### Tuples (Multi-value)

```killer
# Create tuple
let point: (Int, Int) = (10, 20)
let person = ("Alice", 30, 5.9)

# Access elements
let x = point[0]        # 10
let y = point[1]        # 20

# Destructure
let (name, age, height) = person
print(name)             # "Alice"
print(age)              # 30
```

Multiple return values (simple style):
```killer
kfn divide(a, b)        # Types inferred from usage: Int, Int
  (a / b, a % b)

let (quotient, remainder) = divide(17, 5)
print(quotient)         # 3
print(remainder)        # 2
```

## 3.5 Type Annotations Best Practices

**When to use explicit types:**
```killer
# Function parameters: always explicit
kfn greet(name: String, age: Int) -> String
  "Hello, " + name

# Complex logic: help readers
let users: List<String> = List()
let config: Map<String, Int> = Map()
```

**When type inference is fine:**
```killer
# Simple assignments
let x = 42
let name = "Alice"

# Inside functions where context is clear
kfn process
  let total = 0
  let items = []
```

---

# CHAPTER 4: CONTROL FLOW

## 4.1 If/Else Statements

Basic conditional:
```killer
let age = 20

if age >= 18
  print("Adult")
else
  print("Minor")
```

Chaining conditions:
```killer
let score = 75

if score >= 90
  print("A")
else if score >= 80
  print("B")
else if score >= 70
  print("C")
else
  print("F")
```

With braces (optional):
```killer
if age >= 18 {
  print("Adult")
} else {
  print("Minor")
}
```

Expressions (return value):
```killer
let status = if age >= 18 "Adult" else "Minor"
print(status)  # "Adult" or "Minor"
```

## 4.2 Comparison Operators

```killer
let x = 10
let y = 20

print(x == y)   # false (equal)
print(x != y)   # true (not equal)
print(x < y)    # true (less than)
print(x > y)    # false (greater than)
print(x <= y)   # true (less or equal)
print(x >= y)   # false (greater or equal)
```

String comparison:
```killer
let a = "apple"
let b = "apple"
let c = "banana"

print(a == b)   # true
print(a == c)   # false
print(a != c)   # true
```

## 4.3 Logical Operators

AND (`&&`):
```killer
if age >= 18 && has_license
  print("Can drive")
```

OR (`||`):
```killer
if day == "Saturday" || day == "Sunday"
  print("Weekend!")
```

NOT (`!`):
```killer
if !is_complete
  print("Still working on it")
```

Combining:
```killer
if (age >= 18 && has_license) || is_professional_driver
  print("Authorized to drive")
```

## 4.4 For Loops

Range iteration:
```killer
# Loop from 0 to 9
for i in 0..10
  print(i)

# Loop from 1 to 5
for i in 1..6
  print(i)
```

Collection iteration:
```killer
let names = ["Alice", "Bob", "Charlie"]

for name in names
  print(name)

# Output:
# Alice
# Bob
# Charlie
```

Enumerated iteration:
```killer
let items = ["apple", "banana", "cherry"]

for i in 0..items.len()
  print(i.to_string() + ": " + items[i])

# Output:
# 0: apple
# 1: banana
# 2: cherry
```

## 4.5 While Loops

```killer
let count = 0

while count < 5
  print(count)
  count = count + 1
```

With braces:
```killer
let x = 10
while x > 0 {
  print(x)
  x = x - 1
}
```

Break and continue:
```killer
let i = 0
while true
  if i == 5
    break        # Exit loop
  if i == 2
    i = i + 1
    continue     # Skip to next iteration
  
  print(i)
  i = i + 1
```

## 4.6 Loop Control

**Break:** Exit loop immediately
```killer
for i in 0..100
  if i == 10
    break        # Exit when i becomes 10
```

**Continue:** Skip to next iteration
```killer
for i in 0..10
  if i == 5
    continue     # Skip printing 5
  print(i)
```

**Return:** Exit function from loop
```killer
kfn find_first_even(numbers: List<Int>) -> Int
  for num in numbers
    if num % 2 == 0
      return num
  return -1       # Not found
```

---

# CHAPTER 5: FUNCTIONS & CLOSURES

## 5.1 Function Basics

Simple function (types auto-inferred):
```killer
kfn greet()
  print("Hello!")

greet()  # Output: Hello!
```

With parameters (type inference from usage):
```killer
kfn greet(name)        # Type inferred: String
  print("Hello, " + name)

greet("Alice")         # Output: Hello, Alice
greet("Bob")           # Output: Hello, Bob
```

With explicit return type:
```killer
kfn add(a, b)          # Parameter types inferred: Int, Int
  a + b

let result = add(10, 5)
print(result)          # 15
```

With explicit type annotations (optional, for clarity):
```killer
kfn add(a: Int, b: Int) -> Int
  a + b
```

## 5.2 Multiple Parameters

**Simple Killer style - no type annotations:**
```killer
kfn calculate(x, y, operation)   # Types inferred automatically
  if operation == "add"
    return x + y
  else if operation == "multiply"
    return x * y
  else
    return 0

print(calculate(10, 5, "add"))          # 15
print(calculate(10, 5, "multiply"))     # 50
```

**Optional: Explicit types (if you want clarity - mix and match):**
```killer
kfn calculate(x: Int, y, operation)    # Only annotate what helps clarity
  if operation == "add"
    return x + y
  else if operation == "multiply"
    return x * y
  else
    return 0
```

## 5.3 Multiple Return Values

Using tuples (types auto-inferred):
```killer
kfn divide(dividend, divisor)        # Types inferred: Int, Int
  (dividend / divisor, dividend % divisor)

let (quotient, remainder) = divide(17, 5)
print("17 / 5: " + quotient.to_string() + " R" + remainder.to_string())
# Output: 17 / 5: 3 R2
```

## 5.4 Default Returns

Functions automatically return last expression (type inferred):
```killer
kfn square(x)                # Type inference: Int
  x * x

print(square(5))             # 25
```

With explicit type annotation (optional):
```killer
kfn square(x: Int) -> Int
  x * x
```

## 5.5 Closures (Anonymous Functions)

Create inline functions (no type annotations needed):
```killer
let double = |x| x * 2      # Types inferred
print(double(5))             # 10

let add = |x, y| x + y
print(add(3, 4))             # 7
```

With explicit type annotations (optional):
```killer
let double: |Int| -> Int = |x| x * 2
let add: |Int, Int| -> Int = |x, y| x + y
```

Capturing variables (Lexical Scope):
```killer
kfn create_multiplier(factor: Int) -> |Int| -> Int
  return |x| x * factor

let times_three = create_multiplier(3)
print(times_three(5))   # 15
print(times_three(10))  # 30
```

## 5.6 Higher-Order Functions

Functions that take functions as parameters (types auto-inferred):
```killer
kfn apply_twice(f, value)   # Types inferred from usage
  let first = f(value)
  f(first)

let add_one = |x| x + 1
let result = apply_twice(add_one, 5)
print(result)                # 7 (5 + 1 + 1)
```

Functions that return functions:
```killer
kfn create_adder(n: Int) -> |Int| -> Int
  return |x| x + n

let add_five = create_adder(5)
print(add_five(10))     # 15
print(add_five(20))     # 25
```

## 5.7 Function Composition

```killer
kfn compose(f, g)                   # Types inferred
  return |x| f(g(x))

let add_one = |x| x + 1
let multiply_two = |x| x * 2

let add_then_multiply = compose(multiply_two, add_one)
print(add_then_multiply(5))  # (5 + 1) * 2 = 12
```

---

# CHAPTER 6: COLLECTIONS & DATA STRUCTURES

## 6.1 Lists

Creating lists (types inferred from content):
```killer
# With initial values - types auto-inferred
let fruits = ["apple", "banana", "cherry"]  # Type: List<String>
let nums = [1, 2, 3, 4, 5]                    # Type: List<Int>

# Empty lists with explicit types (optional)
let items: List<String> = List()
let numbers: List<Int> = List()
```

List operations:
```killer
let nums = [10, 20, 30]

# Add element
nums.append(40)         # [10, 20, 30, 40]

# Get length
let size = nums.len()   # 4

# Access element
let first = nums[0]     # 10
let last = nums[nums.len() - 1]  # 40

# Check contains
let has_20 = nums.contains(20)   # true
```

List iteration:
```killer
let names = ["Alice", "Bob", "Charlie"]

# Simple iteration
for name in names
  print(name)

# With index
for i in 0..names.len()
  print(i.to_string() + ": " + names[i])
```

List methods:
```killer
let nums = [3, 1, 4, 1, 5, 9, 2, 6]

# Filter (keep elements where condition is true)
let evens = []
for n in nums
  if n % 2 == 0
    evens.append(n)
# evens = [4, 2, 6]

# Map (transform each element)
let doubled = []
for n in nums
  doubled.append(n * 2)
# doubled = [6, 2, 8, 2, 10, 18, 4, 12]

# Sum
let total = 0
for n in nums
  total = total + n
# total = 31
```

## 6.2 Maps (Dictionaries)

Creating maps (types inferred):
```killer
# Map syntax - types auto-inferred from content
let user = {
  "name": "Alice",
  "email": "alice@example.com"
}   # Type: Map<String, String>

let scores = {"alice": 90, "bob": 85}   # Type: Map<String, Int>

# Empty maps with explicit types (optional)
let config: Map<String, Int> = Map()
let settings: Map<String, String> = Map()
```

Map operations:
```killer
let user = Map<String, String>()

# Insert/update
user["name"] = "Alice"
user["email"] = "alice@example.com"
user["age"] = "30"

# Retrieve
let name = user["name"]         # "Alice"
let email = user["email"]       # "alice@example.com"

# Check if key exists
if user.contains("name")
  print("Name exists")

# Get length
let field_count = user.len()    # 3
```

Map iteration:
```killer
let scores = Map<String, Int>()
scores["alice"] = 90
scores["bob"] = 85
scores["charlie"] = 92

# Iterate over keys
for key in scores.keys()
  print(key + ": " + scores[key].to_string())

# Output:
# alice: 90
# bob: 85
# charlie: 92
```

## 6.3 Tuples

Fixed-size collections with mixed types (inferred from values):
```killer
# 2-tuple - types inferred
let point = (10, 20)           # Type: (Int, Int)
let x = point[0]               # 10
let y = point[1]               # 20

# 3-tuple - types inferred
let person = ("Alice", 30, "Engineer")  # Type: (String, Int, String)
let name = person[0]           # "Alice"
let age = person[1]            # 30
let job = person[2]            # "Engineer"

# Destructuring
let (n, a, j) = person
print(n)                       # Alice
```

Multiple return values with tuples:
```killer
kfn get_user_stats() -> (String, Int, Float)
  let name = "Alice"
  let age = 30
  let salary = 75000.50
  (name, age, salary)

let (user_name, user_age, user_salary) = get_user_stats()
```

## 6.4 Nested Collections

Lists of lists (types inferred):
```killer
let matrix = [              # Type: List<List<Int>>
  [1, 2, 3],
  [4, 5, 6],
  [7, 8, 9]
]

print(matrix[0][0])     # 1
print(matrix[2][2])     # 9

# Iterate
for row in matrix
  for value in row
    print(value)
```

Maps of lists (types inferred):
```killer
let student_grades = {          # Type: Map<String, List<Int>>
  "alice": [90, 85, 88],
  "bob": [75, 80, 78]
}

for student in student_grades.keys()
  let grades = student_grades[student]
  let total = 0
  for grade in grades
    total = total + grade
  print(student + " average: " + (total / grades.len()).to_string())
```

## 6.5 Collection Performance

Processing large lists efficiently:
```killer
kfn process_large_list() -> Int
  let items = List<Int>()
  
  # Add 1000 items
  for i in 0..1000
    items.append(i)
  
  # Calculate sum
  let total = 0
  for item in items
    total = total + item
  
  total

let result = process_large_list()  # 499500 (sum of 0..999)
```

Best practices:
- Preallocate size if known
- Use maps for fast lookups (O(1))
- Use lists for ordered data
- Avoid unnecessary copies

---

# CHAPTER 7: PATTERN MATCHING & ENUMERATIONS

## 7.1 Enums (Enumerations)

Simple enum (types inferred):
```killer
enum Color
  Red
  Green
  Blue

let my_color = Color::Red

match my_color           # Pattern matching (types inferred)
  Color::Red -> print("Stop")
  Color::Green -> print("Go")
  Color::Blue -> print("Think")
```

Parameterized enums (types inferred from values):
```killer
enum Message
  Text(content)          # String type inferred
  Number(value)          # Int type inferred  
  Empty

let msg1 = Message::Text("Hello")
let msg2 = Message::Number(42)
let msg3 = Message::Empty
```

## 7.2 Pattern Matching

Basic pattern matching (types auto-inferred):
```killer
enum TrafficLight
  Red
  Yellow
  Green

let light = TrafficLight::Red

match light
  TrafficLight::Red -> print("Stop")
  TrafficLight::Yellow -> print("Prepare")
  TrafficLight::Green -> print("Go")
```

Extracting values from enums (types inferred):
```killer
enum Status
  Success(msg)           # String type inferred
  Error(code)            # Int type inferred

let status = Status::Success("User created")

match status
  Status::Success(data) -> print("Success: " + data)
  Status::Error(code) -> print("Error: " + code.to_string())
```

## 7.3 Match Guards

Adding conditions to patterns:
```killer
enum Response
  Value(num: Int)

let responses = [Response::Value(100), Response::Value(5), Response::Value(50)]

for resp in responses
  match resp
    Response::Value(num) if num > 50 -> print("Large: " + num.to_string())
    Response::Value(num) if num > 0 -> print("Small: " + num.to_string())
    Response::Value(_) -> print("Zero or negative")
```

## 7.4 Nested Patterns

Matching nested structures:
```killer
enum Tree
  Empty
  Node(value: Int, left: Tree, right: Tree)

kfn sum_tree(tree: Tree) -> Int
  match tree
    Tree::Empty -> 0
    Tree::Node(val, left, right) -> val + sum_tree(left) + sum_tree(right)

# Create a tree:
#       1
#      / \
#     2   3
let tree = Tree::Node(1, 
  Tree::Node(2, Tree::Empty, Tree::Empty),
  Tree::Node(3, Tree::Empty, Tree::Empty))

print(sum_tree(tree))   # 6
```

## 7.5 Option Type

Safe handling of optional values:
```killer
enum Option<T>
  Some(value: T)
  None

kfn find_user(id: Int) -> Option<String>
  if id == 1
    Option::Some("Alice")
  else if id == 2
    Option::Some("Bob")
  else
    Option::None

kfn greet_user(id: Int)
  match find_user(id)
    Option::Some(name) -> print("Hello, " + name)
    Option::None -> print("User not found")

greet_user(1)     # "Hello, Alice"
greet_user(99)    # "User not found"
```

## 7.6 Result Type

Error handling with Result:
```killer
enum Result<T, E>
  Ok(value: T)
  Err(error: E)

kfn divide(a: Int, b: Int) -> Result<Int, String>
  if b == 0
    Result::Err("Division by zero")
  else
    Result::Ok(a / b)

kfn safe_divide(a: Int, b: Int)
  match divide(a, b)
    Result::Ok(result) -> print("Result: " + result.to_string())
    Result::Err(error) -> print("Error: " + error)

safe_divide(20, 4)   # "Result: 5"
safe_divide(20, 0)   # "Error: Division by zero"
```

## 7.7 Pattern Matching Best Practices

Rule 1: Always handle all cases
```killer
enum Color
  Red
  Green
  Blue

kfn paint(color: Color)
  match color
    Color::Red -> print("Painting red")
    Color::Green -> print("Painting green")
    Color::Blue -> print("Painting blue")
    # No default needed - all cases covered
```

Rule 2: Use pattern alternatives for similar cases
```killer
enum Error
  NotFound
  Unauthorized
  Forbidden
  BadRequest

kfn handle_error(error: Error)
  match error
    Error::NotFound | Error::Forbidden -> print("Access denied")
    Error::Unauthorized -> print("Please login")
    Error::BadRequest -> print("Invalid input")
```

---

# CHAPTER 8: OBJECT-ORIENTED PROGRAMMING

## 8.1 Structs (Composite Types)

Defining structs (field types can be explicit or inferred):
```killer
type Point
  x
  y

let p = Point(x: 10, y: 20)   # Types inferred: (Int, Int)
print(p.x)                      # 10
```

With explicit types (optional, for clarity):
```killer
type User
  name: String
  age: Int
  email: String

kfn create_user(name, age, email)   # Types inferred from call
  User(name: name, age: age, email: email)
```

Creating methods:
```killer
type Rectangle
  width: Int
  height: Int

kfn area(rect: Rectangle) -> Int
  rect.width * rect.height

kfn perimeter(rect: Rectangle) -> Int
  (rect.width + rect.height) * 2

let r = Rectangle(width: 10, height: 5)
print("Area: " + area(r).to_string())       # 50
print("Perimeter: " + perimeter(r).to_string())  # 30
```

## 8.2 Immutable Data

All data is immutable by default (types inferred):
```killer
type User
  name
  age

let user = User(name: "Alice", age: 30)  # Types inferred

# Cannot modify:
# user.name = "Bob"   # ERROR: Cannot assign to immutable field
```

Creating modified copies (types auto-inferred):
```killer
kfn birthday(user)
  let new_age = user.age + 1
  User(name: user.name, age: new_age)

let alice = User(name: "Alice", age: 30)
let alice_after = birthday(alice)
# alice is still 30
# alice_after is 31
```

## 8.3 Type Aliases

Creating aliases for complex types:
```killer
type UserId = Int
type UserName = String
type UserDatabase = Map<UserId, UserName>

kfn add_user(db: UserDatabase, id: UserId, name: UserName)
  db[id] = name

kfn get_user(db: UserDatabase, id: UserId) -> String
  db[id]
```

---

# CHAPTER 9: ACTORS & CONCURRENCY

## 9.1 Actor Model Introduction

The actor model is Killer's concurrency foundation:
- Each actor is a lightweight process
- Actors communicate via message passing
- No shared mutable state
- Non-blocking by default

## 9.2 Creating Actors

Simple actor:
```killer
actor Counter
  let count = 0
  
  handle increment(value: Int)
    count = count + value
  
  handle get_count() -> Int
    count

kfn main
  let counter = Counter::spawn()
  counter.increment(5).await
  counter.increment(3).await
  let result = counter.get_count().await
  print("Count: " + result.to_string())
```

Actor with state:
```killer
actor UserService
  let users = Map<String, String>()
  
  handle register(user_id: String, name: String) -> String
    users[user_id] = name
    "User " + user_id + " registered"
  
  handle get_user(user_id: String) -> String
    if users.contains(user_id)
      users[user_id]
    else
      "Not found"

kfn main
  let service = UserService::spawn()
  print(service.register("u1", "Alice").await)
  print(service.get_user("u1").await)
```

## 9.3 Message Passing

Sending messages without waiting:
```killer
actor Logger
  handle log(message: String)
    print("[LOG] " + message)

kfn main
  let logger = Logger::spawn()
  
  # Send without waiting
  logger.log("Starting")
  logger.log("Processing")
  logger.log("Done")
```

Waiting for responses:
```killer
actor Calculate
  handle add(a: Int, b: Int) -> Int
    a + b

kfn main
  let calc = Calculate::spawn()
  
  # Wait for result with .await
  let result = calc.add(10, 20).await
  print("Result: " + result.to_string())
```

## 9.4 Concurrent Operations

Multiple actors working together (types inferred):
```killer
actor Worker
  handle process(task_id)
    "Task " + task_id.to_string() + " completed"

kfn main
  let workers = []
  
  # Spawn multiple workers
  for i in 0..5
    workers.append(Worker::spawn())
  
  # Send work to all
  for i in 0..5
    let result = workers[i].process(i).await
    print(result)
```

## 9.5 Synchronization

Semaphore pattern (types auto-inferred):
```killer
actor Semaphore
  let permits = 3     # Int type inferred
  
  handle acquire()
    if permits > 0
      permits = permits - 1
      true
    else
      false
  
  handle release()
    permits = permits + 1

kfn main
  let sem = Semaphore::spawn()
  
  print(sem.acquire().await.to_string())  # true
  print(sem.acquire().await.to_string())  # true
  print(sem.acquire().await.to_string())  # true
  print(sem.acquire().await.to_string())  # false (no permits left)
  
  sem.release().await
  print(sem.acquire().await.to_string())  # true
```

## 9.6 Real-world Actor Example

Order processing service (types auto-inferred):
```killer
actor OrderProcessor
  let orders = {}          # Map<String, String> inferred
  let total_revenue = 0.0  # Float inferred
  
  handle create_order(order_id, amount)
    orders[order_id] = "pending"
    total_revenue = total_revenue + amount
    "Order " + order_id + " created"
  
  handle ship_order(order_id)
    if orders.contains(order_id)
      orders[order_id] = "shipped"
      true
    else
      false
  
  handle get_revenue()
    total_revenue

kfn main
  let processor = OrderProcessor::spawn()
  
  print(processor.create_order("ORD1", 99.99).await)
  print(processor.create_order("ORD2", 149.99).await)
  
  processor.ship_order("ORD1").await
  
  print("Revenue: $" + processor.get_revenue().await.to_string())
```

---

# CHAPTER 10: STREAMS & WINDOW AGGREGATION

## 10.1 Tumbling Windows

Fixed-size time windows:

```killer
struct Event
  timestamp: Int
  value: Int

kfn tumbling_window(events: List<Event>, window_size: Int) -> Map<Int, Int>
  let result = Map<Int, Int>()
  
  for event in events
    # Calculate which window this event belongs to
    let window_id = (event.timestamp / window_size) * window_size
    
    # Add to window
    if result.contains(window_id)
      result[window_id] = result[window_id] + event.value
    else
      result[window_id] = event.value
  
  result

kfn main
  let events = List<Event>()
  events.append(Event(50, 10))
  events.append(Event(150, 20))
  events.append(Event(200, 15))
  events.append(Event(250, 25))
  
  let windows = tumbling_window(events, 100)
  
  for window_id in windows.keys()
    print("Window [" + window_id.to_string() + "): " 
          + windows[window_id].to_string())
```

## 10.2 Windowed Aggregation

Computing statistics within windows:
```killer
struct Measurement
  time: Int
  sensor_id: String
  temperature: Float

kfn window_statistics(measurements: List<Measurement>, 
                     window_ms: Int) -> Map<String, Float>
  let windows = Map<String, List<Float>>()
  
  # Group measurements into windows
  for m in measurements
    let window_key = ((m.time / window_ms) * window_ms).to_string()
    let full_key = window_key + "-" + m.sensor_id
    
    if windows.contains(full_key)
      windows[full_key].append(m.temperature)
    else
      let temps = List<Float>()
      temps.append(m.temperature)
      windows[full_key] = temps
  
  # Calculate averages
  let result = Map<String, Float>()
  for key in windows.keys()
    let temps = windows[key]
    let sum = 0.0
    for t in temps
      sum = sum + t
    result[key] = sum / temps.len().to_float()
  
  result

kfn main
  let measurements = List<Measurement>()
  measurements.append(Measurement(100, "sensor1", 22.5))
  measurements.append(Measurement(120, "sensor1", 23.0))
  measurements.append(Measurement(200, "sensor1", 21.5))
  
  let averages = window_statistics(measurements, 100)
  
  for key in averages.keys()
    print(key + ": " + averages[key].to_string())
```

## 10.3 Stream Processing Pipeline

Real-time data processing:
```killer
struct LogEntry
  level: String
  message: String
  timestamp: Int

actor LogProcessor
  let error_count = 0
  let window_errors = Map<Int, Int>()  # window_id -> count
  
  handle process(log: LogEntry) -> String
    # Count errors
    if log.level == "ERROR"
      error_count = error_count + 1
      
      let window_id = (log.timestamp / 1000) * 1000
      if window_errors.contains(window_id)
        window_errors[window_id] = window_errors[window_id] + 1
      else
        window_errors[window_id] = 1
    
    "Logged: " + log.message
  
  handle get_stats() -> (Int, Int)
    (error_count, window_errors.len())

kfn main
  let processor = LogProcessor::spawn()
  
  let logs = List<LogEntry>()
  logs.append(LogEntry("ERROR", "Connection failed", 100))
  logs.append(LogEntry("INFO", "Started", 150))
  logs.append(LogEntry("ERROR", "Timeout", 200))
  
  for log in logs
    processor.process(log).await
  
  let (total_errors, window_count) = processor.get_stats().await
  print("Total errors: " + total_errors.to_string())
```

## 10.4 Real-World Analytics Example

Web metrics aggregation:
```killer
struct PageView
  user_id: String
  page: String
  duration_ms: Int
  timestamp: Int

actor Analytics
  let page_metrics = Map<String, Int>()
  let total_views = 0
  
  handle track_view(view: PageView)
    total_views = total_views + 1
    
    if page_metrics.contains(view.page)
      page_metrics[view.page] = page_metrics[view.page] + 1
    else
      page_metrics[view.page] = 1
  
  handle get_report() -> String
    let report = "Page Views by Page:\n"
    for page in page_metrics.keys()
      report = report + page + ": " + page_metrics[page].to_string() + "\n"
    report

kfn main
  let analytics = Analytics::spawn()
  
  analytics.track_view(PageView("u1", "/home", 1000, 100)).await
  analytics.track_view(PageView("u2", "/about", 500, 150)).await
  analytics.track_view(PageView("u1", "/home", 1200, 200)).await
  
  print(analytics.get_report().await)
```

---

# CHAPTER 11: ERROR HANDLING

## 11.1 Result Type

Safe error handling:
```killer
enum Result<T, E>
  Ok(value: T)
  Err(error: E)

kfn safe_divide(a: Int, b: Int) -> Result<Int, String>
  if b == 0
    Result:: ("Division by zero")
  else
    Result::Ok(a / b)

kfn main
  match safe_divide(20, 4)
    Result::Ok(result) -> print("20 / 4 = " + result.to_string())
    Result::Err(error) -> print("Error: " + error)
  
  match safe_divide(20, 0)
    Result::Ok(result) -> print("20 / 0 = " + result.to_string())
    Result::Err(error) -> print("Error: " + error)
```

## 11.2 Option Type

Handling optional values:
```killer
enum Option<T>
  Some(value: T)
  None

kfn find_user(users: Map<String, String>, id: String) -> Option<String>
  if users.contains(id)
    Option::Some(users[id])
  else
    Option::None

kfn main
  let users = Map<String, String>()
  users["u1"] = "Alice"
  users["u2"] = "Bob"
  
  match find_user(users, "u1")
    Option::Some(name) -> print("Found: " + name)
    Option::None -> print("Not found")
  
  match find_user(users, "u99")
    Option::Some(name) -> print("Found: " + name)
    Option::None -> print("Not found")
```

## 11.3 Error Propagation

Chaining error handling:
```killer
kfn read_config() -> Result<String, String>
  # Simulate reading a config file
  Result::Err("File not found")

kfn validate_config(config: String) -> Result<Bool, String>
  if config.len() > 0
    Result::Ok(true)
  else
    Result::Err("Empty config")

kfn setup() -> Result<Bool, String>
  match read_config()
    Result::Ok(config) ->
      match validate_config(config)
        Result::Ok(valid) -> Result::Ok(valid)
        Result::Err(e) -> Result::Err(e)
    Result::Err(e) -> Result::Err(e)

kfn main
  match setup()
    Result::Ok(_) -> print("Setup successful")
    Result::Err(error) -> print("Setup failed: " + error)
```

## 11.4 Custom Errors

Creating meaningful error types:
```killer
enum AppError
  FileNotFound(filename: String)
  ParseError(message: String)
  DatabaseError(message: String)

kfn handle_user_request(file_path: String) -> Result<String, AppError>
  if file_path.len() == 0
    return Result::Err(AppError::FileNotFound("path is empty"))
  
  # Simulate parsing
  if file_path.contains("invalid")
    return Result::Err(AppError::ParseError("Invalid format"))
  
  Result::Ok("User data")

kfn main
  match handle_user_request("")
    Result::Ok(data) -> print("Data: " + data)
    Result::Err(AppError::FileNotFound(f)) ->
      print("File not found: " + f)
    Result::Err(AppError::ParseError(m)) ->
      print("Parse error: " + m)
    Result::Err(AppError::DatabaseError(m)) ->
      print("Database error: " + m)
```

---

# CHAPTER 12: ADVANCED PATTERNS

## 12.1 The Builder Pattern

Creating complex objects:
```killer
type HttpRequest
  method: String
  path: String
  headers: Map<String, String>
  body: String

actor HttpRequestBuilder
  let method = "GET"
  let path = "/"
  let headers = Map<String, String>()
  let body = ""
  
  handle with_method(m: String) -> HttpRequest
    method = m
    HttpRequest(method: method, path: path, headers: headers, body: body)
  
  handle with_path(p: String) -> HttpRequest
    path = p
    HttpRequest(method: method, path: path, headers: headers, body: body)
  
  handle with_header(key: String, value: String) -> HttpRequest
    headers[key] = value
    HttpRequest(method: method, path: path, headers: headers, body: body)
  
  handle build() -> HttpRequest
    HttpRequest(method: method, path: path, headers: headers, body: body)

kfn main
  let builder = HttpRequestBuilder::spawn()
  let req = builder
    .with_method("POST").await
    .with_path("/api/users").await
    .with_header("Content-Type", "application/json").await
    .build().await
```

## 12.2 The Factory Pattern

Creating objects of different types:
```killer
enum Vehicle
  Car(doors: Int)
  Motorcycle()
  Truck(capacity: Int)

kfn create_vehicle(vehicle_type: String) -> Vehicle
  if vehicle_type == "car"
    Vehicle::Car(doors: 4)
  else if vehicle_type == "motorcycle"
    Vehicle::Motorcycle()
  else if vehicle_type == "truck"
    Vehicle::Truck(capacity: 5000)
  else
    Vehicle::Car(doors: 2)

kfn describe_vehicle(vehicle: Vehicle) -> String
  match vehicle
    Vehicle::Car(doors) -> "Car with " + doors.to_string() + " doors"
    Vehicle::Motorcycle() -> "Motorcycle"
    Vehicle::Truck(capacity) -> "Truck with " + capacity.to_string() + " kg capacity"

kfn main
  let car = create_vehicle("car")
  let bike = create_vehicle("motorcycle")
  
  print(describe_vehicle(car))
  print(describe_vehicle(bike))
```

## 12.3 The Observer Pattern

Reactive event handling:
```killer
actor EventBus
  let listeners = List<|String| -> String>()
  
  handle subscribe(listener: |String| -> String)
    listeners.append(listener)
  
  handle emit(event: String) -> String
    let results = ""
    for listener in listeners
      results = results + listener(event) + "; "
    results

kfn main
  let bus = EventBus::spawn()
  
  let handler1 = |event: String| "Handler1 got: " + event
  let handler2 = |event: String| "Handler2 got: " + event
  
  bus.subscribe(handler1).await
  bus.subscribe(handler2).await
  
  print(bus.emit("user_login").await)
```

## 12.4 The Pipeline Pattern

Chaining operations:
```killer
kfn pipeline(data: Int, 
            f1: |Int| -> Int,
            f2: |Int| -> Int,
            f3: |Int| -> Int) -> Int
  f3(f2(f1(data)))

kfn main
  let add_one = |x| x + 1
  let double = |x| x * 2
  let square = |x| x * x
  
  # (5 + 1) * 2 = 12, then 12 * 12 = 144
  let result = pipeline(5, add_one, double, square)
  print(result)  # 144
```

---

# CHAPTER 13: BEST PRACTICES

## 13.1 Code Style

### Naming Conventions

```killer
# Variables and functions: snake_case
let user_count = 0
kfn calculate_average() -> Float
  ...

# Constants: UPPER_CASE
let MAX_CONNECTIONS = 1000
let PI = 3.14159

# Types and Enums: PascalCase
type User
  name: String

enum Status
  Active
  Inactive
```

### Indentation

```killer
#âœ… Good: Two spaces per level
kfn calculate
  let x = 10
  if x > 5
    print("Large")

# âŒ Bad: Inconsistent indentation
kfn calculate
  let x = 10
    if x > 5
       print("Large")
```

## 13.2 Function Design

### Keep Functions Small

```killer
# âœ… Good: Single responsibility
kfn validate_email(email: String) -> Bool
  email.contains("@")

kfn validate_age(age: Int) -> Bool
  age >= 18

# âŒ Bad: Too much responsibility
kfn validate_user(name: String, email: String, age: Int) -> Bool
  if name.len() > 0 && email.contains("@") && age >= 18
    true
  else
    false
```

### Return Early

```killer
# âœ… Good: Return early for error cases
kfn process(data: String) -> Result<String, String>
  if data.len() == 0
    return Result::Err("Empty data")
  
  if !data.contains("valid")
    return Result::Err("Invalid data")
  
  Result::Ok("Processed: " + data)
```

## 13.3 Error Handling

### Always Handle Errors

```killer
# âœ… Good: Explicit error handling
match divide(20, 5)
  Result::Ok(result) -> print("Result: " + result.to_string())
  Result::Err(error) -> print("Error: " + error)

# âŒ Bad: Ignoring potential errors
let result = divide(20, 5)
print(result)  # What if it's an error?
```

### Use Specific Error Types

```killer
# âœ… Good: Specific errors
enum UserError
  NotFound
  AlreadyExists
  InvalidEmail

kfn create_user(email: String) -> Result<String, UserError>
  ...

# âŒ Bad: Generic errors
kfn create_user(email: String) -> String
  # Can't distinguish between errors
  ...
```

## 13.4 Performance Considerations

### Avoid Unnecessary Copies

```killer
# âœ… Good: Pass by reference patterns
kfn process_list(items: List<Int>) -> Int
  let sum = 0
  for item in items
    sum = sum + item
  sum

# âŒ Potentially inefficient: Creating new lists
kfn process_list(items: List<Int>) -> List<Int>
  let result = []
  for item in items
    result.append(item * 2)
  result
```

### Use Appropriate Data Structures

```killer
# âœ… Good: Map for fast lookups
let users = Map<String, String>()
if users.contains(user_id)
  let name = users[user_id]

# Potentially inefficient: List for lookups
let users_list = ["alice", "bob", "charlie"]
let found = false
for user in users_list
  if user == "bob"
    found = true
```

## 13.5 Concurrency Patterns

### Avoid Blocking Operations

```killer
# âœ… Good: Non-blocking with async
let result = worker.process(task).await

# âŒ Bad: Would block
let result = blocking_operation()
```

### Handle Actor Failures

```killer
# âœ… Good: Graceful degradation
match service.request(data).await
  Result::Ok(response) -> print(response)
  Result::Err(e) -> print("Service unavailable: using cache")

# âŒ Bad: Assuming service always succeeds
let response = service.request(data).await
print(response)
```

---

# CHAPTER 14: REAL-WORLD EXAMPLES

## 14.1 Web API Server

```killer
actor ApiServer
  let routes = Map<String, |String| -> String>()
  let request_count = 0
  
  handle register_route(path: String, handler: |String| -> String)
    routes[path] = handler
  
  handle handle_request(method: String, path: String, body: String) -> String
    request_count = request_count + 1
    
    if routes.contains(path)
      let handler = routes[path]
      "{\"status\": 200, \"data\": \"" + handler(body) + "\"}"
    else
      "{\"status\": 404, \"error\": \"Not found\"}"
  
  handle get_metrics() -> String
    "Requests handled: " + request_count.to_string()

kfn main
  let api = ApiServer::spawn()
  
  # Register routes
  api.register_route("/api/users", |body| "Users: " + body).await
  api.register_route("/api/posts", |body| "Posts: " + body).await
  
  # Handle requests
  let resp1 = api.handle_request("GET", "/api/users", "").await
  let resp2 = api.handle_request("GET", "/api/posts", "").await
  let resp3 = api.handle_request("GET", "/not-found", "").await
  
  print(resp1)
  print(resp2)
  print(resp3)
  print(api.get_metrics().await)
```

## 14.2 Rate Limiter

```killer
actor RateLimiter
  let request_times = Map<String, List<Int>>()
  let window_ms = 1000
  let max_requests = 100
  
  handle allow_request(client_id: String, now: Int) -> Bool
    if !request_times.contains(client_id)
      let times = List<Int>()
      times.append(now)
      request_times[client_id] = times
      return true
    
    let times = request_times[client_id]
    
    # Remove old requests outside window
    let filtered = List<Int>()
    for t in times
      if (now - t) < window_ms
        filtered.append(t)
    
    if filtered.len() < max_requests
      filtered.append(now)
      request_times[client_id] = filtered
      true
    else
      false

kfn main
  let limiter = RateLimiter::spawn()
  
  # Simulate requests
  for i in 0..120
    let now = i * 10  # Stagger by 10ms
    let allowed = limiter.allow_request("client1", now).await
    if !allowed
      print("Request " + i.to_string() + " rate limited")
```

## 14.3 Data Processing Pipeline

```killer
struct DataRecord
  id: String
  value: Int
  timestamp: Int

actor DataPipeline
  let input_count = 0
  let output_count = 0
  let error_count = 0
  
  handle process(record: DataRecord) -> Result<String, String>
    input_count = input_count + 1
    
    # Validate
    if record.id.len() == 0
      error_count = error_count + 1
      return Result::Err("Invalid ID")
    
    if record.value < 0
      error_count = error_count + 1
      return Result::Err("Negative value")
    
    # Process
    let processed = "Record: " + record.id + " Value: " 
                  + record.value.to_string()
    output_count = output_count + 1
    
    Result::Ok(processed)
  
  handle get_stats() -> (Int, Int, Int)
    (input_count, output_count, error_count)

kfn main
  let pipeline = DataPipeline::spawn()
  
  let records = List<DataRecord>()
  records.append(DataRecord("r1", 100, 1000))
  records.append(DataRecord("r2", 200, 1100))
  records.append(DataRecord("", -50, 1200))  # Invalid
  
  for record in records
    match pipeline.process(record).await
      Result::Ok(msg) -> print(msg)
      Result::Err(e) -> print("Error: " + e)
  
  let (inp, out, err) = pipeline.get_stats().await
  print("Input: " + inp.to_string() + ", Output: " + out.to_string() 
        + ", Errors: " + err.to_string())
```

---

# CHAPTER 15: TROUBLESHOOTING & FAQ

## 15.1 Common Problems

### Problem: Type Mismatch

```killer
# âŒ Error
let x = "hello"
let y = x + 5  # ERROR: Cannot add String and Int
```

**Solution:** Convert to same type
```killer
# âœ… Correct
let x = "hello"
let y = x + 5.to_string()  # "hello5"
```

### Problem: Undefined Variable

```killer
# âŒ Error
if x > 10
  print("Large")
# x is not defined
```

**Solution:** Declare before use
```killer
# âœ… Correct
let x = 15
if x > 10
  print("Large")
```

### Problem: Pattern Not Exhaustive

```killer
# âŒ Error
enum Color
  Red
  Blue
  Green

let color = Color::Red
match color
  Color::Red -> print("Red")
  # ERROR: Missing cases for Blue and Green
```

**Solution:** Handle all cases
```killer
# âœ… Correct
match color
  Color::Red -> print("Red")
  Color::Blue -> print("Blue")
  Color::Green -> print("Green")
```

## 15.2 Performance Issues

### Problem: Memory Grows Unbounded

```killer
# âŒ Bad: Infinite list accumulation
actor MemoryLeaker
  let data = []
  
  handle add(item: Int)
    data.append(item)  # Never cleared!
```

**Solution:** Limit or clear data
```killer
# âœ… Good: Bounded size
actor BoundedCache
  let cache = Map<String, Int>()
  let max_size = 1000
  
  handle add(key: String, value: Int)
    if cache.len() >= max_size
      # Remove oldest entry
      if cache.contains(key)
        cache.remove(key)
    cache[key] = value
```

### Problem: Slow List Operations

```killer
# âš ï¸ Potentially slow: Linear search
let items = [1, 2, 3, ..., 1000000]
if items.contains(999999)
  print("Found")
```

**Solution:** Use Map for lookups
```killer
# âœ… Faster: O(1) lookup
let items_map = Map<Int, Bool>()
for item in items
  items_map[item] = true

if items_map.contains(999999)
  print("Found")
```

## 15.3 FAQ (Frequently Asked Questions)

**Q: Can I use tabs for indentation?**  
A: No, Killer requires spaces (2 per level). Most editors can be configured to insert spaces instead of tabs.

**Q: Is Killer dynamically or statically typed?**  
A: Statically typed with type inference. You don't have to write types everywhere, but they're enforced at compile time.

**Q: How does Killer compare in performance to Rust?**  
A: Rust is faster (~5x), but Killer is good enough for most real-time systems. It's easier to learn and use.

**Q: Can actors communicate across network?**  
A: Not in v4.2, but it's planned for v5.0. Currently actors run in-process.

**Q: Is there a garbage collector?**  
A: Yes, but it's non-generational and creates rare, predictable pauses. Perfect for systems with latency budgets of 5-100ms.

**Q: How to handle very large files?**  
A: Stream processing. Read chunks, process, discard. Don't load entire file in memory.

**Q: Can I call C libraries?**  
A: Not in v4.2, planned for v4.4 (FFI - Foreign Function Interface).

**Q: What's the max number of concurrent actors?**  
A: Tested up to 100,000+ actors. Practical limit depends on system resources.

---

# APPENDIX A: QUICK REFERENCE

## Language Keywords

```
fn, let, if, else, for, while, match, actor, handle, return, break, continue, true, false
```

## Built-in Types

```
Int, Float, String, Bool, List<T>, Map<K, V>, Option<T>, Result<T, E>
```

## Operators

```
Arithmetic:  +, -, *, /, %, ^
Comparison: ==, !=, <, >, <=, >=
Logical:    &&, ||, !
String:     +, .len(), .upper(), .lower(), .contains()
```

## Common Methods

```
.len()              - Length of list/string/map
.append()           - Add to list
.contains()         - Check membership
.to_string()        - Convert to string
.to_int()           - Convert to int
.to_float()         - Convert to float
.keys()             - Get map keys
```

---

# APPENDIX B: EXAMPLE PROGRAMS

## HelloWorld

```killer
kfn main
  print("Hello, Killer!")
```

## Fibonacci

```killer
kfn fibonacci(n: Int) -> Int
  if n <= 1
    n
  else
    fibonacci(n - 1) + fibonacci(n - 2)

kfn main
  for i in 0..10
    print(fibonacci(i).to_string() + " ")
```

## Bubble Sort

```killer
kfn bubble_sort(items: List<Int>) -> List<Int>
  let n = items.len()
  
  for i in 0..n
    for j in 0..(n-i-1)
      if items[j] > items[j+1]
        # Swap
        let temp = items[j]
        items[j] = items[j+1]
        items[j+1] = temp
  
  items

kfn main
  let unsorted = [64, 34, 25, 12, 22, 11, 90]
  let sorted = bubble_sort(unsorted)
  print(sorted)
```

---

# APPENDIX C: RESOURCES

## Official Documentation
https://killer-lang.org/docs

## Community Forum
https://forum.killer-lang.org

## GitHub Repository
https://github.com/killer-lang/killer

## Package Registry
https://pkg.killer-lang.org

---

**END OF MANUAL**

---

# DOCUMENT INFORMATION

**Title:** Killer Language Comprehensive Learning Manual  
**Version:** 4.2  
**Date:** March 20, 2026  
**Total Pages:** 100+ (estimated)  
**Author:** Killer Development Team  
**Status:** Production Ready for Team Training & Market Release  

**This manual contains:**
- âœ… 15 comprehensive chapters
- âœ… 100+ code examples
- âœ… Best practices and patterns
- âœ… Real-world examples
- âœ… Troubleshooting guide
- âœ… Quick reference
- âœ… Appendices

**Perfect for:**
- Team learning and onboarding
- Self-study and independent learning
- Market publication and distribution
- Training curriculum development


