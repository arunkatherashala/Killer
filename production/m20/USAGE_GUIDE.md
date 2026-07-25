# 📚 KILLER v1.0 - USAGE GUIDE

**Complete Guide: Beginner to Advanced**

---

## 📖 TABLE OF CONTENTS

1. [Getting Started](#getting-started)
2. [Beginner Basics](#beginner-basics) (1-2 hours)
3. [Intermediate Features](#intermediate-features) (2-4 hours)
4. [Advanced Topics](#advanced-topics) (4+ hours)
5. [Real-World Examples](#real-world-examples)
6. [Troubleshooting](#troubleshooting)

---

## 🚀 GETTING STARTED

### Installation

killer.exe is a standalone binary. No installation needed!

```bash
# Just run it
killer.exe program.killer
```

### Your First Program

Create `hello.killer`:
```killer
kfn main
    print("Hello, KILLER!")
```

Run it:
```bash
killer.exe hello.killer
```

Output:
```
Hello, KILLER!
```

---

## 🔢 BEGINNER BASICS (1-2 Hours)

### 1. Variables and Types

```killer
kfn main
    # Numbers
    age = 25
    temperature = -5.5
    
    # Strings
    name = "Alice"
    message = "Hello, World!"
    
    # Booleans
    is_active = true
    is_deleted = false
    
    # Print them
    print(age)
    print(name)
    print(is_active)
```

### 2. Arithmetic

```killer
kfn main
    a = 10
    b = 3
    
    print(a + b)    # 13
    print(a - b)    # 7
    print(a * b)    # 30
    print(a / b)    # 3
    print(a % b)    # 1
```

### 3. String Operations

```killer
kfn main
    first = "Hello"
    second = "World"
    
    # Concatenation
    greeting = first + " " + second
    print(greeting)         # Hello World
    
    # Length
    print(len(greeting))    # 11
    
    # Methods
    print(first.upper())    # HELLO
    print(second.lower())   # world
```

### 4. If/Else

```killer
kfn main
    age = 18
    
    if age < 13
        print("Child")
    elif age < 18
        print("Teen")
    else
        print("Adult")
```

### 5. Loops - For

```killer
kfn main
    # Count to 10
    for i in 0..10
        print(i)
    
    # Print list items
    fruits = ["apple", "banana", "cherry"]
    for fruit in fruits
        print(fruit)
```

### 6. Loops - While

```killer
kfn main
    count = 0
    while count < 5
        print(count)
        count = count + 1
```

---

## 🔧 INTERMEDIATE FEATURES (2-4 Hours)

### 1. Functions with Parameters

```killer
kfn add(a: Int, b: Int) -> Int
    return a + b

kfn greet(name: String)
    print("Hello, " + name + "!")

kfn main
    print(add(5, 3))        # 8
    greet("Alice")          # Hello, Alice!
```

### 2. Lists

```killer
kfn main
    numbers = [1, 2, 3, 4, 5]
    
    # Access elements
    print(numbers[0])       # 1
    print(numbers[2])       # 3
    
    # Add elements
    numbers.append(6)
    
    # Length
    print(len(numbers))     # 6
    
    # Iterate
    for n in numbers
        print(n)
```

### 3. List Operations

```killer
kfn main
    numbers = [3, 1, 4, 1, 5, 9]
    
    sorted_nums = sort(numbers)
    print(sorted_nums)      # [1, 1, 3, 4, 5, 9]
    
    reversed_nums = reverse(numbers)
    print(reversed_nums)    # [9, 5, 1, 4, 1, 3]
    
    if contains(numbers, 5)
        print("Found 5!")
```

### 4. Maps (Dictionaries)

```killer
kfn main
    person = {
        "name": "Alice",
        "age": 30,
        "city": "NYC"
    }
    
    print(person["name"])   # Alice
    
    person["email"] = "alice@example.com"
    
    for key in keys(person)
        print(key + ": " + person[key])
```

### 5. Multiple Functions

```killer
kfn square(x: Int) -> Int
    return x * x

kfn sum_of_squares(a: Int, b: Int) -> Int
    return square(a) + square(b)

kfn main
    result = sum_of_squares(3, 4)
    print(result)           # 25
```

### 6. Function Return Values

```killer
kfn is_even(n: Int) -> Bool
    return n % 2 == 0

kfn get_category(age: Int) -> String
    if age < 18
        return "minor"
    else
        return "adult"

kfn main
    print(is_even(4))       # true
    print(get_category(25)) # adult
```

---

## 🔥 ADVANCED TOPICS (4+ Hours)

### 1. Recursion (Fibonacci)

```killer
kfn fibonacci(n: Int) -> Int
    if n <= 1
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

kfn main
    for i in 0..11
        print(fibonacci(i) + " ")
    # Output: 0 1 1 2 3 5 8 13 21 34 55
```

### 2. Nested Functions

```killer
kfn outer(x: Int) -> Int
    kfn inner(y: Int) -> Int
        return x + y
    return inner(5)

kfn main
    result = outer(10)
    print(result)    # 15
```

### 3. Complex Data Structures

```killer
kfn main
    # List of maps
    people = [
        {"name": "Alice", "age": 30},
        {"name": "Bob", "age": 25},
        {"name": "Charlie", "age": 35}
    ]
    
    for person in people
        print(person["name"] + " is " + person["age"])
```

### 4. String Interpolation (K-Strings)

```killer
kfn main
    name = "Alice"
    age = 30
    
    message = "Name: $name, Age: $age"
    print(message)
    # Output: Name: Alice, Age: 30
```

### 5. Higher-Order Functions

```killer
kfn apply_twice(f, value: Int) -> Int
    return f(f(value))

kfn double(x: Int) -> Int
    return x * 2

kfn main
    result = apply_twice(double, 5)
    print(result)    # 20 (5 * 2 * 2)
```

---

## 🌟 REAL-WORLD EXAMPLES

### Calculator

```killer
kfn calculate(a: Int, b: Int, op: String) -> Int
    if op == "+"
        return a + b
    elif op == "-"
        return a - b
    elif op == "*"
        return a * b
    elif op == "/"
        if b != 0
            return a / b
    return 0

kfn main
    print(calculate(10, 5, "+"))  # 15
    print(calculate(10, 5, "-"))  # 5
```

### Grade System

```killer
kfn calculate_grade(score: Int) -> String
    if score >= 90
        return "A"
    elif score >= 80
        return "B"
    elif score >= 70
        return "C"
    else
        return "F"

kfn main
    scores = [95, 87, 73, 65]
    for score in scores
        grade = calculate_grade(score)
        print("Score: " + score + " Grade: " + grade)
```

### Statistics

```killer
kfn sum_list(numbers: List) -> Int
    total = 0
    for n in numbers
        total = total + n
    return total

kfn average(numbers: List) -> Int
    if len(numbers) == 0
        return 0
    return sum_list(numbers) / len(numbers)

kfn main
    data = [10, 20, 30, 40, 50]
    print("Sum: " + sum_list(data))      # 150
    print("Average: " + average(data))   # 30
```

### Word Counter

```killer
kfn count_words(text: String) -> Int
    words = text.split(" ")
    return len(words)

kfn main
    sentence = "The quick brown fox"
    count = count_words(sentence)
    print("Words: " + count)  # 4
```

---

## 🐛 TROUBLESHOOTING

### Q: Syntax Error - "Expected indent"
**A:** Check indentation. Use 4 spaces, not tabs.

### Q: Type Mismatch
**A:** Make sure parameter types match what you're passing.

```killer
kfn add(a: Int, b: Int)    # expects Int
result = add(5, "hello")   # ERROR: "hello" is String
```

### Q: Function Not Found
**A:** Make sure function is defined before main, or defined in main.

### Q: Index Out of Bounds
**A:** Check array size before accessing.

```killer
nums = [1, 2, 3]
print(nums[5])  # ERROR: only indices 0-2 exist
```

### Q: "return" statement issues
**A:** Make sure return type matches.

```killer
kfn get_number() -> Int
    return "hello"      # ERROR: should return Int
```

---

## 📈 LEARNING PROGRESSION

### Day 1: Basics
- [ ] Hello World
- [ ] Variables (int, string, bool)
- [ ] Arithmetic operations
- [ ] print() function

### Day 2: Control Flow
- [ ] If/else
- [ ] While loops
- [ ] For loops
- [ ] Break/continue

### Day 3: Functions
- [ ] Simple functions
- [ ] Parameters and return types
- [ ] Multiple functions together
- [ ] Return values

### Day 4: Collections
- [ ] Lists basics
- [ ] List operations
- [ ] Maps/dictionaries
- [ ] Iterating collections

### Day 5: Advanced
- [ ] Recursion
- [ ] Nested functions
- [ ] String interpolation
- [ ] Real-world programs

---

## 💡 PRO TIPS

1. **Test Frequently:** Run code often to verify it works
2. **Use print() for Debugging:** Print intermediate values
3. **Start Small:** Build simple programs before complex ones
4. **Name Variables Clearly:** Use descriptive names
5. **Comment Your Code:** Especially for complex logic
6. **Break Down Problems:** Divide tasks into smaller functions
7. **Consistent Indentation:** Critical for Python-style syntax

---

## 🎓 NEXT STEPS

1. **Try:** Run the programs above
2. **Modify:** Change variables and see what happens
3. **Combine:** Mix features together
4. **Build:** Create your own program
5. **Share:** Use with your team

---

**Status:** ✅ Complete Learning Guide  
**Ready to:** Start coding with KILLER!

