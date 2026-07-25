# Killer Programming Language - Complete Documentation

**Version:** v2.0  
**Status:** Production Ready  
**Test Coverage:** 100% (48/48 tests passing)  
**Build Date:** March 2026

---

## Table of Contents

1. [Introduction](#introduction)
2. [Installation & Setup](#installation--setup)
3. [Quick Start](#quick-start)
4. [Core Syntax](#core-syntax)
5. [Variables & Data Types](#variables--data-types)
6. [Operators](#operators)
7. [Control Flow](#control-flow)
8. [Functions](#functions)
9. [Object-Oriented Programming](#object-oriented-programming)
10. [Built-in Objects](#built-in-objects)
11. [String Methods](#string-methods)
12. [Array Methods](#array-methods)
13. [Advanced Features](#advanced-features)
14. [Type System](#type-system)
15. [Error Handling](#error-handling)
16. [Regular Expressions](#regular-expressions)
17. [Examples](#examples)
18. [Transpilation](#transpilation)

---

## Introduction

**Killer** is a modern programming language designed from scratch with focus on **simplicity, readability, and modern syntax**. It combines features from JavaScript, Python, and other modern languages.

### Key Features

- ✅ **Modern Syntax:** Arrow functions, template literals, classes
- ✅ **Full OOP:** Classes, inheritance, static methods, getters/setters
- ✅ **Functional Programming:** Arrow functions, closures, higher-order functions
- ✅ **Powerful Built-ins:** Math object, String methods (14+), Array methods (9+)
- ✅ **Type System:** Dynamic typing with typeof, instanceof, Array.isArray()
- ✅ **Multi-target:** Execute directly or transpile to Python/JavaScript
- ✅ **Error Handling:** try/catch/finally blocks
- ✅ **Regular Expressions:** Full regex support
- ✅ **No Dependencies:** Pure Python implementation

---

## Installation & Setup

### Requirements

- Python 3.6 or higher
- No external dependencies

### Setup

```bash
# Clone or download the Killer repository
git clone <killer-repo>
cd killer

# Verify installation
python main.py examples/01_hello.killer
```

---

## Quick Start

### Running Your First Program

Create a file `hello.killer`:

```killer
print("Hello, World!");
x = 10;
y = 20;
print(`Sum: ${x + y}`);
```

Run it:

```bash
python main.py hello.killer
```

### Three Execution Modes

```bash
# 1. Direct Interpretation (fastest for small scripts)
python main.py script.killer

# 2. Transpile to Python
python main.py --python script.killer

# 3. Transpile to JavaScript
python main.py --js script.killer
```

### Interactive Mode

```bash
python main.py
>>> x = 10
>>> print(x * 2)
20
>>> 
```

---

## Core Syntax

### Comments

```killer
# Single line comment
x = 10;  # Comment after code

# Multi-line by using multiple comments
# This is line 1
# This is line 2
# This is line 3
```

### Statements & Semicolons

Semicolons are **optional** but recommended:

```killer
x = 10          # Valid
y = 20;         # Also valid
```

### Indentation

**Not required** (unlike Python). Use braces `{}` for blocks:

```killer
if x > 5 {
    print("x is greater than 5");
}
```

---

## Variables & Data Types

### Variable Declaration

```killer
x = 10;              # Number
name = "Alice";      # String
is_active = true;    # Boolean
pi = 3.14159;        # Float
```

### Data Types

| Type | Example | Typeof |
|------|---------|--------|
| Number | `42`, `3.14` | `"number"` |
| String | `"hello"`, `'world'` | `"string"` |
| Boolean | `true`, `false` | `"boolean"` |
| Array | `[1, 2, 3]` | `"array"` |
| Object | `{x: 10, y: 20}` | `"object"` |
| Null | `null` | `"null"` |
| Function | `x => x * 2` | `"function"` |

### Constants (Convention)

```killer
# Killer doesn't have const keyword, use UPPERCASE by convention
PI = 3.14159;
MAX_SIZE = 100;
```

---

## Operators

### Arithmetic Operators

```killer
a = 10;
b = 3;

print(a + b);      # 13 - Addition
print(a - b);      # 7  - Subtraction
print(a * b);      # 30 - Multiplication
print(a / b);      # 3.333... - Division
print(a % b);      # 1  - Modulo
print(a ** b);     # 1000 - Exponentiation (Power)
```

### Comparison Operators

```killer
x = 10;
y = 20;

print(x == y);     # false - Equal to
print(x != y);     # true  - Not equal to
print(x < y);      # true  - Less than
print(x > y);      # false - Greater than
print(x <= y);     # true  - Less than or equal
print(x >= y);     # false - Greater than or equal
```

### Logical Operators

```killer
a = true;
b = false;

print(a && b);     # false - AND (both must be true)
print(a || b);     # true  - OR (at least one true)
print(!a);         # false - NOT (negation)

# Short-circuit evaluation
x = 5;
if (x > 0 && x < 10) {
    print("x is between 0 and 10");
}
```

### Compound Assignment Operators

```killer
x = 10;

x += 5;            # x = x + 5 = 15
x -= 3;            # x = x - 3 = 12
x *= 2;            # x = x * 2 = 24
x /= 4;            # x = x / 4 = 6
x %= 5;            # x = x % 5 = 1

print(x);          # 1
```

### Increment/Decrement

```killer
x = 10;

x++;               # x = 11 - Post-increment
++x;               # x = 12 - Pre-increment

x--;               # x = 11 - Post-decrement
--x;               # x = 10 - Pre-decrement
```

### String Concatenation

```killer
first = "Hello";
last = "World";

result = first + " " + last;
print(result);     # "Hello World"

# Repeat string
print("Ha" * 3);   # "HaHaHa"
```

### Ternary Operator

```killer
age = 20;
status = age >= 18 ? "Adult" : "Minor";
print(status);     # "Adult"

# Nested ternary
score = 85;
grade = score >= 90 ? "A" : score >= 80 ? "B" : "C";
print(grade);      # "B"
```

### Type Operators

#### typeof

```killer
print(typeof 42);              # "number"
print(typeof "hello");         # "string"
print(typeof true);            # "boolean"
print(typeof [1, 2, 3]);       # "array"
print(typeof {x: 10});         # "object"
print(typeof null);            # "null"
print(typeof (x => x * 2));    # "function"
```

#### instanceof

```killer
arr = [1, 2, 3];
print(arr instanceof Array);   # true

str = "hello";
print(str instanceof String);  # false (string is primitive)

class Animal {}
obj = new Animal();
print(obj instanceof Animal);  # true
```

---

## Control Flow

### if/else Statements

```killer
age = 20;

if age >= 18 {
    print("Adult");
} else {
    print("Minor");
}

# Else if
score = 85;
if score >= 90 {
    print("A");
} else if score >= 80 {
    print("B");
} else if score >= 70 {
    print("C");
} else {
    print("F");
}
```

### switch/case Statements

```killer
day = 3;

switch day {
    case 1: print("Monday");
    case 2: print("Tuesday");
    case 3: print("Wednesday");
    case 4: print("Thursday");
    case 5: print("Friday");
    case 6:
    case 7: print("Weekend");
    default: print("Invalid day");
}

# Switch with fall-through (cases without break run next case)
```

### while Loops

```killer
x = 0;
while x < 5 {
    print(x);
    x = x + 1;
}
# Output: 0 1 2 3 4

# Infinite loop (with break)
while true {
    print("infinite");
    break;
}
```

### do-while Loops

```killer
x = 0;
do {
    print(x);
    x = x + 1;
} while x < 3;
# Output: 0 1 2

# Executes at least once even if condition is false
```

### for Loops

```killer
# Traditional for loop
for i in 0..5 {
    print(i);
}
# Output: 0 1 2 3 4 5

# For loop with step
for i in 0..10..2 {
    print(i);
}
# Output: 0 2 4 6 8 10

# For loop over array
arr = [10, 20, 30];
for item in arr {
    print(item);
}
# Output: 10 20 30
```

### break and continue

```killer
# break - exits the loop
for i in 0..10 {
    if i == 5 {
        break;
    }
    print(i);
}
# Output: 0 1 2 3 4

# continue - skips rest of iteration
for i in 0..5 {
    if i == 2 {
        continue;
    }
    print(i);
}
# Output: 0 1 3 4 5
```

### Range Operator

```killer
# Basic range (inclusive)
0..5          # 0, 1, 2, 3, 4, 5

# Range with step
0..10..2      # 0, 2, 4, 6, 8, 10
10..0..-1     # 10, 9, 8, 7, ... 0 (descending)

# Using in for loops
for i in 0..5 {
    print(i);
}

# Using in array literals
arr = [0..5];
print(arr);    # [0, 1, 2, 3, 4, 5]
```

---

## Functions

### Function Declaration

```killer
function add(a, b) {
    return a + b;
}

result = add(5, 3);
print(result);     # 8
```

### Function with No Return

```killer
function greet(name) {
    print(`Hello, ${name}!`);
}

greet("Alice");    # Hello, Alice!
```

### Default Parameters

```killer
function introduce(name = "Guest", age = 18) {
    print(`${name} is ${age} years old`);
}

introduce("Bob", 25);     # Bob is 25 years old
introduce("Charlie");     # Charlie is 18 years old
introduce();              # Guest is 18 years old
```

### Arrow Functions

#### Single Parameter (Implicit Return)

```killer
square = x => x * x;
print(square(5));         # 25
```

#### Multiple Parameters (Implicit Return)

```killer
add = (a, b) => a + b;
print(add(10, 5));        # 15
```

#### No Parameters

```killer
greet = () => "Hello!";
print(greet());           # Hello!
```

#### Multiple Statements (Explicit Return)

```killer
multiply = (a, b) => {
    result = a * b;
    return result;
};

print(multiply(4, 5));    # 20
```

### Closures

```killer
function counter() {
    x = 0;
    return () => {
        x = x + 1;
        return x;
    };
}

count = counter();
print(count());          # 1
print(count());          # 2
print(count());          # 3
```

### Higher-Order Functions

```killer
# Function taking a function as parameter
function apply(fn, value) {
    return fn(value);
}

double = x => x * 2;
result = apply(double, 5);
print(result);          # 10

# Function returning a function
function multiplier(factor) {
    return x => x * factor;
}

triple = multiplier(3);
print(triple(4));       # 12
```

### Recursive Functions

```killer
function factorial(n) {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

print(factorial(5));    # 120
```

---

## Object-Oriented Programming

### Objects (Dictionaries)

```killer
person = {
    name: "Alice",
    age: 30,
    city: "New York"
};

print(person.name);           # Alice
print(person["age"]);         # 30

# Modifying properties
person.age = 31;
person["city"] = "Boston";
```

### Classes

#### Basic Class

```killer
class Animal {
    constructor(name, species) {
        this.name = name;
        this.species = species;
    }
    
    describe() {
        return `${this.name} is a ${this.species}`;
    }
    
    speak() {
        return "Some sound";
    }
}

dog = new Animal("Buddy", "Dog");
print(dog.describe());        # Buddy is a Dog
print(dog.speak());           # Some sound
```

### Inheritance

```killer
class Vehicle {
    constructor(brand) {
        this.brand = brand;
    }
    
    start() {
        return `${this.brand} is starting`;
    }
}

class Car extends Vehicle {
    constructor(brand, model) {
        this.brand = brand;
        this.model = model;
    }
    
    start() {
        return `${this.brand} ${this.model} is starting`;
    }
    
    drive() {
        return "Driving...";
    }
}

car = new Car("Toyota", "Camry");
print(car.start());           # Toyota Camry is starting
print(car.drive());           # Driving...
```

### Static Methods

```killer
class MathHelper {
    static add(a, b) {
        return a + b;
    }
    
    static multiply(a, b) {
        return a * b;
    }
}

print(MathHelper.add(5, 3));         # 8
print(MathHelper.multiply(4, 7));    # 28
```

### Getters

```killer
class Rectangle {
    constructor(width, height) {
        this.width = width;
        this.height = height;
    }
    
    get area() {
        return this.width * this.height;
    }
    
    get perimeter() {
        return 2 * (this.width + this.height);
    }
}

rect = new Rectangle(5, 10);
print(rect.area);             # 50
print(rect.perimeter);        # 30
```

### Setters

```killer
class Account {
    constructor(balance = 0) {
        this.balance = balance;
    }
    
    get funds() {
        return this.balance;
    }
    
    set funds(amount) {
        if amount >= 0 {
            this.balance = amount;
        }
    }
}

account = new Account(100);
print(account.funds);         # 100
account.funds = 200;
print(account.funds);         # 200
```

---

## Built-in Objects

### Math Object

#### Constants

```killer
print(Math.PI);               # 3.14159265...
print(Math.E);                # 2.71828182...
```

#### Methods

```killer
# Absolute value
print(Math.abs(-10));         # 10

# Square root
print(Math.sqrt(16));         # 4

# Power
print(Math.pow(2, 3));        # 8

# Maximum value
print(Math.max(5, 3, 9, 1));  # 9

# Minimum value
print(Math.min(5, 3, 9, 1));  # 1

# Rounding
print(Math.round(3.7));       # 4
print(Math.floor(3.7));       # 3
print(Math.ceil(3.2));        # 4

# Random number (0 to 1)
print(Math.random());         # e.g., 0.573...
```

### Array Methods

#### Access

```killer
arr = [10, 20, 30, 40, 50];

print(arr[0]);                # 10 (first element)
print(arr.length);            # 5
```

#### Transformation

```killer
numbers = [1, 2, 3, 4, 5];

# map - transform each element
doubled = numbers.map(x => x * 2);
print(doubled);               # [2, 4, 6, 8, 10]

# filter - keep matching elements
evens = numbers.filter(x => x % 2 == 0);
print(evens);                 # [2, 4]
```

#### Reduction

```killer
numbers = [1, 2, 3, 4, 5];

# reduce - combine all elements
sum = numbers.reduce((acc, val) => acc + val, 0);
print(sum);                   # 15

# Product
product = numbers.reduce((a, b) => a * b, 1);
print(product);               # 120
```

#### Search

```killer
numbers = [1, 2, 3, 4, 5];

# find - first matching element
first_gt_3 = numbers.find(x => x > 3);
print(first_gt_3);            # 4

# some - any matching element?
has_even = numbers.some(x => x % 2 == 0);
print(has_even);              # true

# every - all match?
all_positive = numbers.every(x => x > 0);
print(all_positive);          # true
```

#### Modification

```killer
arr = [1, 2, 3, 4, 5];

# reverse
arr.reverse();
print(arr);                   # [5, 4, 3, 2, 1]

# sort
nums = [3, 1, 4, 1, 5, 9];
nums.sort();
print(nums);                  # [1, 1, 3, 4, 5, 9]

# splice - remove/insert elements
arr = [1, 2, 3, 4, 5];
arr.splice(2, 2);             # Remove 2 elements starting at index 2
print(arr);                   # [1, 2, 5]
```

#### Combination

```killer
arr1 = [1, 2];
arr2 = [3, 4];

combined = arr1.concat(arr2);
print(combined);              # [1, 2, 3, 4]
```

#### String Conversion

```killer
arr = [1, 2, 3];

joined = arr.join("-");
print(joined);                # "1-2-3"

joined2 = arr.join(", ");
print(joined2);               # "1, 2, 3"
```

#### Array Checking

```killer
print(Array.isArray([1, 2]));         # true
print(Array.isArray("hello"));        # false
print(Array.isArray({x: 1}));         # false
```

---

## String Methods

### Case Conversion

```killer
str = "Hello World";

print(str.toUpperCase());     # "HELLO WORLD"
print(str.toLowerCase());     # "hello world"
```

### Search

```killer
str = "Hello, World!";

# indexOf - first occurrence
print(str.indexOf("o"));      # 4
print(str.indexOf("World"));  # 7

# includes - contains?
print(str.includes("World")); # true
print(str.includes("xyz"));   # false
```

### Extract

```killer
str = "Hello, World!";

# slice
print(str.slice(0, 5));       # "Hello"
print(str.slice(7, 12));      # "World"
print(str.slice(-6));         # "World!"

# substring
print(str.substring(0, 5));   # "Hello"

# charAt - character at index
print(str.charAt(0));         # "H"
print(str.charAt(6));         # "W"
```

### Trim

```killer
str = "  Hello  ";

print(str.trim());            # "Hello"
print(str.trimStart());       # "Hello  "
print(str.trimEnd());         # "  Hello"
```

### Split

```killer
str = "apple, banana, cherry";

parts = str.split(", ");
print(parts);                 # ["apple", "banana", "cherry"]

chars = str.split("");
print(chars);                 # ["a", "p", "p", "l", "e", ...]
```

### Replace

```killer
str = "Hello World";

print(str.replace("World", "Killer"));      # "Hello Killer"
print(str.replace("l", "L"));               # "HeLLo World" (first only)
```

### Match

```killer
str = "I have 2 apples and 5 oranges";

# Using regex
numbers = str.match(/\d+/);
print(numbers);               # ["2"]

all_numbers = str.match(/\d+/g);  # global flag
print(all_numbers);           # ["2", "5"]
```

### Repeat

```killer
str = "Ha";
print(str.repeat(3));         # "HaHaHa"
```

### Check Start/End

```killer
str = "Hello, World!";

print(str.startsWith("Hello"));     # true
print(str.endsWith("!"));           # true
```

---

## Advanced Features

### Template Literals

```killer
name = "Alice";
age = 30;

# Simple interpolation
greeting = `Hello, ${name}!`;
print(greeting);              # Hello, Alice!

# Expression in template
calculation = `2 + 3 = ${2 + 3}`;
print(calculation);           # 2 + 3 = 5

# Multi-line strings
text = `Line 1
Line 2
Line 3`;
print(text);

# Nested expressions
score = 85;
grade = `Score: ${score}, Grade: ${score >= 80 ? "B" : "C"}`;
print(grade);                 # Score: 85, Grade: B
```

### Regular Expressions

```killer
# Basic pattern
pattern = /hello/;
print(pattern.test("hello world"));    # true
print(pattern.test("HELLO"));          # false

# Case insensitive
pattern2 = /hello/i;
print(pattern2.test("HELLO"));         # true

# Matching
text = "I am 25 years old";
numbers = text.match(/\d+/);
print(numbers);               # ["25"]

# Common patterns
pattern3 = /[a-z]+/;          # lowercase letters
pattern4 = /[0-9]+/;          # digits
pattern5 = /\w+/;             # word characters
pattern6 = /\d{3}-\d{2}-\d{4}/;  # SSN format
```

---

## Type System

### Type Checking Functions

```killer
# parseInt - string to integer
x = parseInt("42");
print(x);                     # 42
print(typeof x);              # "number"

# parseFloat - string to float
y = parseFloat("3.14");
print(y);                     # 3.14

# String - convert to string
s = String(42);
print(s);                     # "42"
print(typeof s);              # "string"

# Number - convert to number
n = Number("3.14");
print(n);                     # 3.14

# Boolean - convert to boolean
b = Boolean(1);
print(b);                     # true

# isNaN - is not a number?
print(isNaN(NaN));            # true
print(isNaN(42));             # false

# isFinite - is finite number?
print(isFinite(100));         # true
print(isFinite(Infinity));    # false
```

### Type Coercion

```killer
# Implicit coercion
print("5" + 3);               # "53" (string concatenation)
print("5" - 3);               # 2 (numeric operation)
print(true + 1);              # 2 (true is 1)
print(false + 5);             # 5 (false is 0)

# Explicit coercion
print(Number("42") + 8);      # 50
print(String(42) + " items"); # "42 items"
```

---

## Error Handling

### try/catch/finally

```killer
try {
    x = 10;
    y = 20;
    result = x / y;
    print(result);
} catch e {
    print(`Error: ${e}`);
} finally {
    print("Cleanup code");
}

# Output:
# 0.5
# Cleanup code
```

### Error Scenarios

```killer
try {
    # Division by zero doesn't throw in Killer (returns Infinity)
    result = 10 / 0;
    print(result);           # Infinity
    
    # Array access out of bounds returns null
    arr = [1, 2, 3];
    print(arr[10]);          # null
    
} catch e {
    print(`Caught error: ${e}`);
} finally {
    print("Always runs");
}
```

---

## Examples

### Example 1: Variables and Arithmetic

```killer
# Declare variables
x = 10;
y = 20;
z = x + y;

print(z);                     # 30
print(`${x} + ${y} = ${z}`);  # 10 + 20 = 30

# Using operators
a = 5;
b = 3;
print(a ** b);                # 125 (5^3)
```

### Example 2: Functions and Arrow Functions

```killer
# Regular function
function greet(name) {
    return `Hello, ${name}!`;
}

# Arrow function
square = x => x * x;

# Default parameters
function calculate(a, b = 10) {
    return a + b;
}

print(greet("World"));        # Hello, World!
print(square(5));             # 25
print(calculate(5));          # 15
print(calculate(5, 20));      # 25
```

### Example 3: Arrays and Array Methods

```killer
numbers = [1, 2, 3, 4, 5];

# map
doubled = numbers.map(x => x * 2);
print(doubled);               # [2, 4, 6, 8, 10]

# filter
evens = numbers.filter(x => x % 2 == 0);
print(evens);                 # [2, 4]

# reduce
sum = numbers.reduce((a, b) => a + b, 0);
print(sum);                   # 15

# find
first_gt_3 = numbers.find(x => x > 3);
print(first_gt_3);            # 4
```

### Example 4: Classes and Inheritance

```killer
class Animal {
    constructor(name) {
        this.name = name;
    }
    
    speak() {
        return `${this.name} makes a sound`;
    }
}

class Dog extends Animal {
    speak() {
        return `${this.name} barks`;
    }
    
    static getSpecies() {
        return "Canis familiaris";
    }
}

dog = new Dog("Buddy");
print(dog.speak());           # Buddy barks
print(Dog.getSpecies());      # Canis familiaris
```

### Example 5: Control Flow

```killer
score = 85;

# if/else
if score >= 90 {
    print("A");
} else if score >= 80 {
    print("B");
} else if score >= 70 {
    print("C");
} else {
    print("F");
}

# for loop with range
for i in 0..5 {
    print(i);
}

# Array iteration
arr = ["Apple", "Banana", "Cherry"];
for item in arr {
    print(item);
}
```

### Example 6: String Methods

```killer
text = "Killer Programming Language";

# Case conversion
print(text.toUpperCase());    # KILLER PROGRAMMING LANGUAGE
print(text.toLowerCase());    # killer programming language

# Search
print(text.indexOf("Program"));    # 7
print(text.includes("Language"));  # true

# Extract
print(text.slice(0, 6));      # Killer
print(text.slice(-8));        # Language

# Split
words = text.split(" ");
print(words);                 # ["Killer", "Programming", "Language"]
```

---

## Transpilation

### Transpile to Python

```bash
python main.py --python script.killer > script.py
python script.py
```

Example Killer code:
```killer
arr = [1, 2, 3, 4, 5];
doubled = arr.map(x => x * 2);
print(doubled);
```

Becomes Python:
```python
arr = [1, 2, 3, 4, 5]
doubled = arr.map(lambda x: x * 2)
print(doubled)
```

### Transpile to JavaScript

```bash
python main.py --js script.killer > script.js
node script.js
```

Example Killer code:
```killer
greet = name => `Hello, ${name}!`;
print(greet("World"));
```

Becomes JavaScript:
```javascript
const greet = (name) => `Hello, ${name}!`;
console.log(greet("World"));
```

---

## Project Structure

```
killer/
├── main.py                 # Entry point
├── src/
│   ├── lexer.py           # Tokenizer (555 lines)
│   ├── parser.py          # Syntax analyzer (1085 lines)
│   ├── interpreter.py     # Execution engine (1304 lines)
│   ├── python_generator.py    # Python transpiler (759 lines)
│   └── javascript_generator.py # JS transpiler (535 lines)
├── examples/              # 16 example files
│   ├── 01_hello.killer
│   ├── 02_variables.killer
│   ...
│   └── 16_phase2_oop.killer
├── index.killer           # Full language showcase (272 lines)
├── view.killer            # Killer file viewer
├── index.html             # Interactive web demo
├── view.html              # Code viewer (formatted)
├── code.html              # Minimal code viewer
└── README.md              # This file
```

---

## Testing

Run all tests:

```bash
python tests/python/test_all_phases.py
```

Expected output:
```
Testing 16 examples across 3 modes...
[01_hello.killer] [OK] [OK] [OK]
...
[16_phase2_oop.killer] [OK] [OK] [OK]

Results: 48/48 PASS (100%)
[SUCCESS] ALL TESTS PASSED!
```

---

## Language Statistics

| Metric | Count |
|--------|-------|
| Total Lines of Code | 9000+ |
| Lexer Tokens | 70+ |
| AST Node Types | 25+ |
| String Methods | 14 |
| Array Methods | 9 |
| Math Functions | 9 |
| Examples | 16 |
| Test Cases | 48 |
| Pass Rate | 100% |
| Features | 60+ |

---

## Features Checklist

### Phase 1 (MVP)
- ✅ Variables & arithmetic
- ✅ Types & operators
- ✅ Control flow
- ✅ Functions & closures
- ✅ Arrays & objects
- ✅ String operations

### Phase 2 (Modern Syntax)
- ✅ Arrow functions
- ✅ Template literals
- ✅ Default parameters
- ✅ Classes & constructors
- ✅ Inheritance
- ✅ Static methods
- ✅ Getters & setters

### Phase 3 (Advanced)
- ✅ Switch/case statements
- ✅ Do-while loops
- ✅ Array methods (map, filter, reduce)
- ✅ String methods (14+)
- ✅ Regular expressions
- ✅ Error handling
- ✅ Type system
- ✅ Multi-target transpilation

---

## Common Patterns

### Working with Arrays

```killer
# Create array
arr = [1, 2, 3, 4, 5];

# Transform
result = arr.map(x => x * 2).filter(x => x > 5);
print(result);  # [6, 8, 10]

# Accumulate
sum = arr.reduce((a, b) => a + b, 0);
print(sum);     # 15
```

### Working with Objects

```killer
person = {
    name: "Alice",
    age: 30,
    email: "alice@example.com"
};

# Access
print(person.name);     # Alice

# Modify
person.age = 31;

# Add property
person.city = "NYC";
```

### Working with Strings

```killer
text = "Hello, World!";

# Chain methods
result = text
    .toLowerCase()
    .replace("world", "killer")
    .split(" ")
    .join("-");

print(result);  # hello,-killer!
```

---

## Troubleshooting

### Issue: "Undefined variable"
**Solution:** Check spelling and ensure variable is declared before use

### Issue: "Type error"
**Solution:** Verify types match with typeof - use explicit type conversion

### Issue: "Parser error at line X"
**Solution:** Check bracket/brace matching and statement syntax

### Issue: "Cannot call method on null"
**Solution:** Add null checks before calling methods

---

## Resources

- **Examples Directory:** `examples/` - 16 complete working examples
- **Showcase:** `index.killer` - 272 lines demonstrating all features
- **Web Demo:** `index.html` - Interactive browser-based demo
- **Source Code:** `src/` - Full interpreter source code

---

## Contributing

To extend Killer:

1. **Add tokens** in `lexer.py`
2. **Add AST nodes** in `parser.py`
3. **Add parsing logic** in `parser.py`
4. **Add interpretation** in `interpreter.py`
5. **Update transpilers** in `python_generator.py` and `javascript_generator.py`
6. **Add test examples** in `examples/`

---

## License

Educational project - Free to use and modify

---

## Version History

**v2.0 (Current)**
- Complete OOP with inheritance, static methods, getters/setters
- Arrow functions, template literals, default parameters
- 48/48 tests passing (100%)
- Production ready

**v1.0 (Initial)**
- Core language features
- Basic functions and objects
- Interpreter and transpilers

---

## Support

For issues or questions, refer to the examples in the `examples/` directory or run `python main.py --help`

---

**Last Updated:** March 2026  
**Status:** Production Ready ✅  
**Test Coverage:** 100% (48/48)
