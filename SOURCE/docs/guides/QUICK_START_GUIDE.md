# Killer Language - Quick Start Guide

## ⚡ 5-Minute Introduction

Welcome to Killer! Your first programming language that supports **both Python-style and Go-style syntax** - write code your way.

---

## 🎯 Your First Program

### Python Style (Clean & Simple)
```killer
print("Hello, Killer!")
```

### Go Style (Explicit & Clear)
```killer
print("Hello, Killer!");
```

### Both are valid! Pick your favorite.

---

## 📚 Basic Concepts (In 10 Minutes)

### 1. Variables
```killer
// Indentation style
name = "Alice"
age = 30
score = 95.5

// Brace style
name = "Alice";
age = 30;
score = 95.5;
```

### 2. Functions

**Define with indentation:**
```killer
fn greet(name)
    "Hello, " + name

// Call it
print(greet("World"))  // Hello, World
```

**Define with braces:**
```killer
fn greet(name) {
    "Hello, " + name
}

// Call it
print(greet("World"))  // Hello, World
```

**Arrow functions (one-liners):**
```killer
// Simple arrow
add(a, b) => a + b
print(add(3, 5))  // 8

// Use in data structures
person = {
    "name": "Alice",
    "age": 30
}
```

### 3. Control Flow

**If/Else with indentation:**
```killer
score = 85

if score >= 90
    print("A")
else if score >= 80
    print("B")
else
    print("C")
```

**If/Else with braces:**
```killer
score = 85

if (score >= 90) {
    print("A")
} else if (score >= 80) {
    print("B")
} else {
    print("C")
}
```

### 4. Loops

**For loop with indentation:**
```killer
for i in range(5)
    print(i)  // 0, 1, 2, 3, 4
```

**For loop with braces:**
```killer
for (i in range(5)) {
    print(i)  // 0, 1, 2, 3, 4
}
```

**While loop:**
```killer
// Indentation
x = 0
while x < 3
    print(x)
    x = x + 1

// Braces
x = 0
while (x < 3) {
    print(x)
    x = x + 1
}
```

### 5. Data Structures

**Arrays:**
```killer
numbers = [1, 2, 3, 4, 5]
print(len(numbers))    // 5
print(numbers[0])      // 1
print(reverse(numbers)) // [5, 4, 3, 2, 1]
```

**Dictionaries:**
```killer
person = {
    "name": "Bob",
    "age": 25,
    "city": "NYC"
}

print(person["name"])      // "Bob"
print(len(person))         // 3
print(keys(person))        // ["name", "age", "city"]
print(values(person))      // ["Bob", 25, "NYC"]
```

---

## 🔧 Standard Library Reference

### Collections
| Function | Purpose | Example |
|----------|---------|---------|
| `len(x)` | Get length of array/dict/string | `len([1,2,3])` → 3 |
| `range(n)` | Create array [0..n-1] | `range(3)` → [0,1,2] |
| `range(a,b)` | Create array [a..b-1] | `range(2,5)` → [2,3,4] |
| `range(a,b,s)` | Create array with step | `range(0,10,2)` → [0,2,4,6,8] |
| `keys(dict)` | Get dictionary keys | `keys({"a":1})` → ["a"] |
| `values(dict)` | Get dictionary values | `values({"a":1})` → [1] |

### Type System
| Function | Purpose | Example |
|----------|---------|---------|
| `type(x)` | Get type name | `type(42)` → "number" |
| `str(x)` | Convert to string | `str(42)` → "42" |
| `int(x)` | Convert to integer | `int("42")` → 42 |

### String Methods
| Function | Purpose | Example |
|----------|---------|---------|
| `upper(s)` | Convert to uppercase | `upper("hello")` → "HELLO" |
| `lower(s)` | Convert to lowercase | `lower("HELLO")` → "hello" |
| `trim(s)` | Remove leading/trailing space | `trim("  hi  ")` → "hi" |
| `split(s, sep)` | Split string | `split("a,b,c", ",")` → ["a","b","c"] |
| `starts_with(s, p)` | Check prefix | `starts_with("hello", "he")` → true |
| `ends_with(s, e)` | Check suffix | `ends_with("hello", "lo")` → true |
| `contains(s, sub)` | Find substring | `contains("hello", "ell")` → true |
| `replace(s, old, new)` | Replace text | `replace("hello", "l", "L")` → "heLLo" |
| `index_of(s, sub)` | Find position | `index_of("hello", "l")` → 2 |

### Array Methods
| Function | Purpose | Example |
|----------|---------|---------|
| `push(arr, values...)` | Add elements | `push([1,2], 3)` → [1,2,3] |
| `pop(arr)` | Remove last | `pop([1,2,3])` → 2 |
| `reverse(arr)` | Reverse order | `reverse([1,2,3])` → [3,2,1] |
| `slice(arr, a, b)` | Extract subarray | `slice([1,2,3,4], 1, 3)` → [2,3] |
| `concat(a, b)` | Combine arrays | `concat([1,2], [3,4])` → [1,2,3,4] |
| `join(arr, sep)` | Join to string | `join([1,2,3], ",")` → "1,2,3" |
| `contains(arr, v)` | Check membership | `contains([1,2,3], 2)` → true |
| `index_of(arr, v)` | Find index | `index_of([1,2,3], 2)` → 1 |

---

## 🎨 Style Guide

### Recommended Practices

**1. Pick ONE style per project**
```killer
// DON'T MIX INCONSISTENTLY
fn bad_mixed_code()
    if x > 0 {
        y = 10
    }
    print(y)

// DO: Be consistent within files
fn good_indentation()
    if x > 0
        y = 10
    print(y)

fn good_braces() {
    if (x > 0) {
        y = 10
    }
    print(y)
}
```

**2. Use `killer fmt` to auto-format**
```bash
# Format a file
killer fmt mycode.killer

# Check without changing
killer fmt --check mycode.killer
```

**3. Naming conventions**
```killer
// Variables and functions: snake_case
my_variable = 10
fn my_function() => "result"

// Dictionary keys: stay as-is
person = {
    "firstName": "Alice",  // camelCase if in source data
    "full_name": "Alice Smith"  // snake_case for app code
}
```

**4. Formatting tips**
```killer
// Good: readable spacing
fn calculate(a, b)
    result = a + b
    result * 2

// Good: descriptive names
user_scores = [95, 87, 92]
high_scores = []

for score in user_scores
    if score > 90
        high_scores.push(score)

print(high_scores)
```

---

## 🚀 Next Steps

### Learn More
1. **Examples:** Check `examples/killer_showcase_examples.killer`
2. **Full Docs:** See `DUAL_SYNTAX_ARCHITECTURE.md`
3. **Standard Library:** `STDLIB_REFERENCE.md`

### Try It Out
```bash
# Create your first file
echo 'print("Hello, Killer!")' > hello.killer

# Run it
killer hello.killer

# Format it
killer fmt hello.killer
```

### Common Patterns

**Process a list:**
```killer
numbers = [1, 2, 3, 4, 5]
results = []

for num in numbers
    if num > 2
        results.push(num * 2)

print(results)  // [6, 8, 10]
```

**Transform data:**
```killer
data = [
    {"name": "Alice", "score": 95},
    {"name": "Bob", "score": 87},
    {"name": "Charlie", "score": 92}
]

top_performers = []
for person in data {
    if person["score"] > 90
        top_performers.push(person["name"])
}

print(top_performers)  // ["Alice", "Charlie"]
```

**String manipulation:**
```killer
text = "hello world"
cleaned = trim(text)
upper_text = upper(cleaned)
words = split(upper_text, " ")
result = join(words, "-")
print(result)  // "HELLO-WORLD"
```

---

## ❓ FAQ

**Q: Can I mix Python and Go style in the same file?**  
A: Yes, but we recommend choosing one per project and using `killer fmt` to keep it consistent.

**Q: What's the difference between the two styles?**  
A: None! They compile to identical bytecode. It's purely about readability preference.

**Q: Which style is "better"?**  
A: Neither! Use what feels natural. Pythonistas love indentation; Go developers prefer braces. Killer supports both.

**Q: How do I know what's a builtin vs user-defined function?**  
A: Builtins are in the standard library (len, range, upper, etc.). The IDE will show you which is which.

**Q: Can I use both styles in the same program?**  
A: Yes! Though we recommend keeping each file/project consistent with `killer fmt`.

**Q: What types exist in Killer?**  
A: `number`, `string`, `bool`, `array`, `dict`, `null`. Use `type(x)` to check.

**Q: Do I need semicolons?**  
A: No! They're optional. `x = 5` and `x = 5;` both work.

---

## 🌟 Cool Tricks

**Function references (arrow style):**
```killer
square(x) => x * x
print(square(5))  // 25
```

**Implicit returns (last expression):**
```killer
fn calculate(a, b)
    a + b  // No 'return' needed!

print(calculate(3, 4))  // 7
```

**Nested data structures:**
```killer
company = {
    "name": "TechCorp",
    "teams": [
        {"name": "Backend", "members": 5},
        {"name": "Frontend", "members": 4}
    ]
}

print(company["teams"][0]["name"])  // "Backend"
```

---

## 📞 Get Help

- **Discord:** Join our community
- **GitHub Issues:** Report bugs or ask questions
- **Documentation:** Full API reference available
- **Examples:** Check the `examples/` directory

---

## 🎓 Learning Path

1. **Beginner:** Variables, basic types, print
2. **Intermediate:** Functions, control flow, arrays/dicts
3. **Advanced:** Complex data structures, recursion, algorithms
4. **Expert:** Building real applications

**Time to first "Hello, World!":** 1 minute  
**Time to productive programmer:** 1-2 hours  
**Time to expert developer:** 1-2 weeks  

---

## ✨ Remember

Killer is designed to be **simple first, powerful when needed**. Start with basic examples, gradually explore more features. The language will feel familiar no matter which style you prefer.

**Happy coding! 🚀**
