# 📚 KILLER v1.0 - USAGE GUIDE: BEGINNER TO ADVANCED

**Version:** 1.0  
**Date:** 2026-03-20  
**Binary:** killer.exe  
**Status:** Production Ready ✅

---

## 🎯 Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Syntax](#basic-syntax)
3. [Data Types](#data-types)
4. [Functions](#functions)
5. [Control Flow](#control-flow)
6. [Collections](#collections)
7. [Advanced Features](#advanced-features)
8. [Real-World Examples](#real-world-examples)
9. [Troubleshooting](#troubleshooting)

---

## 🚀 Getting Started

### Installation

Simply copy `killer.exe` to your system:
```
killer.exe (139 KB standalone binary)
```

No dependencies, no installation needed. Just run it!

### Your First Program

Create a file called `hello.killer`:

```killer
kfn main
    print("Hello, KILLER!")
```

Run it:
```bash
killer.exe hello.killer
```

**Output:**
```
Hello, KILLER!
```

### Verify Installation

Check the version:
```bash
killer.exe --version
```

---

## 📖 PART 1: BASIC SYNTAX

### 1.1 Hello World (Simplest Form)

```killer
kfn main
    print("Hello, World!")
```

**Concepts:**
- `kfn` = function declaration keyword
- `main` = entry point function
- `print()` = output to console
- Code is indentation-based (Python-style)

---

### 1.2 Variables and Assignment

```killer
kfn main
    x = 10              # integer (type inferred)
    y = 3.14            # float
    name = "Alice"      # string
    is_active = true    # boolean
    
    print(x)            # prints: 10
    print(name)         # prints: Alice
```

**Key Points:**
- No `let` keyword needed (implicit assignment)
- Types are automatically inferred
- Snake_case for variable names

---

### 1.3 Basic Operations

```killer
kfn main
    # Arithmetic
    a = 10
    b = 3
    print(a + b)        # 13
    print(a - b)        # 7
    print(a * b)        # 30
    print(a / b)        # 3 (integer division)
    print(a % b)        # 1 (modulo)
    
    # String operations
    greeting = "Hello"
    name = "World"
    print(greeting + " " + name)  # Hello World
    
    # Comparison
    print(a > b)        # true
    print(a == 10)      # true
    print(a != b)       # true
```

---

## 🔢 PART 2: DATA TYPES

### 2.1 Integers

```killer
kfn main
    small = 42
    large = 1000000
    negative = -50
    
    # Operations
    print(10 + 20)      # 30
    print(10 * 5)       # 50
    print(10 - 3)       # 7
```

### 2.2 Floats

```killer
kfn main
    pi = 3.14159
    temperature = -5.5
    
    print(pi * 2)       # 6.28318
    print(temperature)  # -5.5
```

### 2.3 Strings

```killer
kfn main
    # String literals
    name = "Alice"
    message = "Hello, World!"
    
    # String operations
    print(name + " is here")           # Alice is here
    print("Length: " + len(name))      # Length: 5
    
    # String methods
    upper = name.upper()               # ALICE
    lower = message.lower()            # hello, world!
    reversed_str = reverse(name)       # ecilA
```

### 2.4 Booleans

```killer
kfn main
    x = true
    y = false
    
    print(x and y)      # false
    print(x or y)       # true
    print(not x)        # false
```

---

## ⚙️ PART 3: FUNCTIONS

### 3.1 Basic Functions

```killer
kfn add(a: Int, b: Int) -> Int
    return a + b

kfn main
    result = add(5, 3)
    print(result)       # 8
```

**Syntax:**
- `kfn function_name(param: Type) -> ReturnType`
- Parameters have types
- Return type comes after `->`
- `return` keyword to return values

### 3.2 Functions Without Return Type

```killer
kfn greet(name: String)
    print("Hello, " + name + "!")

kfn main
    greet("Alice")      # Hello, Alice!
    greet("Bob")        # Hello, Bob!
```

### 3.3 Optional Types (Type Inference)

```killer
kfn multiply(a, b)         # types inferred
    return a * b

kfn main
    print(multiply(3, 4))   # 12
```

### 3.4 Multiple Parameters

```killer
kfn calculate(a: Int, b: Int, operation: String) -> Int
    if operation == "add"
        return a + b
    elif operation == "subtract"
        return a - b
    else
        return 0

kfn main
    print(calculate(10, 5, "add"))      # 15
    print(calculate(10, 5, "subtract")) # 5
```

### 3.5 Functions Calling Functions

```killer
kfn square(x: Int) -> Int
    return x * x

kfn sum_of_squares(a: Int, b: Int) -> Int
    return square(a) + square(b)

kfn main
    result = sum_of_squares(3, 4)
    print(result)       # 25 (9 + 16)
```

---

## 🔄 PART 4: CONTROL FLOW

### 4.1 If/Else

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

### 4.2 Loops - While

```killer
kfn main
    count = 0
    while count < 5
        print(count)
        count = count + 1
    # Output: 0 1 2 3 4
```

### 4.3 Loops - For

```killer
kfn main
    for i in 0..5        # 0 to 4 (5 excluded)
        print(i)
    # Output: 0 1 2 3 4
    
    for i in 1..11       # 1 to 10
        print(i)
```

### 4.4 Break and Continue

```killer
kfn main
    for i in 0..10
        if i == 3
            continue     # skip this iteration
        if i == 7
            break        # exit loop
        print(i)
    # Output: 0 1 2 4 5 6
```

---

## 📦 PART 5: COLLECTIONS

### 5.1 Lists

```killer
kfn main
    numbers = [1, 2, 3, 4, 5]
    
    # Access elements
    print(numbers[0])            # 1
    print(numbers[2])            # 3
    
    # List length
    print(len(numbers))          # 5
    
    # Add to list
    numbers.append(6)
    print(len(numbers))          # 6
```

### 5.2 Iterating Over Lists

```killer
kfn main
    fruits = ["apple", "banana", "cherry"]
    
    for fruit in fruits
        print(fruit)
    
    # Output:
    # apple
    # banana
    # cherry
```

### 5.3 List Operations

```killer
kfn main
    numbers = [3, 1, 4, 1, 5, 9]
    
    # Sort
    sorted_nums = sort(numbers)
    print(sorted_nums)           # [1, 1, 3, 4, 5, 9]
    
    # Reverse
    reversed_nums = reverse(numbers)
    print(reversed_nums)         # [9, 5, 1, 4, 1, 3]
    
    # Contains
    print(contains(numbers, 5))  # true
    print(contains(numbers, 7))  # false
```

### 5.4 Maps (Dictionaries)

```killer
kfn main
    person = {
        "name": "Alice",
        "age": 30,
        "city": "New York"
    }
    
    # Access values
    print(person["name"])        # Alice
    print(person["age"])         # 30
    
    # Add/Update
    person["email"] = "alice@example.com"
    person["age"] = 31
    
    # Check key exists
    print(contains(person, "name"))     # true
    print(contains(person, "phone"))    # false
```

---

## 🔥 PART 6: ADVANCED FEATURES

### 6.1 Nested Functions

```killer
kfn outer(x: Int) -> Int
    kfn inner(y: Int) -> Int
        return x + y
    return inner(5)

kfn main
    result = outer(10)
    print(result)        # 15
```

### 6.2 Recursion (Fibonacci)

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

### 6.3 Pattern Matching

```killer
kfn describe(value: Int) -> String
    if value == 0
        return "zero"
    elif value == 1
        return "one"
    elif value == 2
        return "two"
    else
        return "many"

kfn main
    print(describe(0))   # zero
    print(describe(1))   # one
    print(describe(5))   # many
```

### 6.4 String Interpolation (K-Strings)

```killer
kfn main
    name = "Alice"
    age = 30
    city = "NYC"
    
    message = "Name: $name, Age: $age, City: $city"
    print(message)
    # Output: Name: Alice, Age: 30, City: NYC
```

### 6.5 Higher-Order Functions

```killer
kfn apply_twice(f, value: Int) -> Int
    return f(f(value))

kfn double(x: Int) -> Int
    return x * 2

kfn main
    result = apply_twice(double, 5)
    print(result)        # 20 (5 * 2 * 2)
```

---

## 🌟 PART 7: REAL-WORLD EXAMPLES

### 7.1 Calculator

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
    print(calculate(10, 5, "*"))  # 50
    print(calculate(10, 5, "/"))  # 2
```

### 7.2 Factor Finder

```killer
kfn find_factors(n: Int) -> List
    factors = []
    for i in 1..n
        if n % i == 0
            factors.append(i)
    return factors

kfn main
    factors = find_factors(12)
    print(factors)       # [1, 2, 3, 4, 6, 12]
```

### 7.3 Grade Calculator

```killer
kfn calculate_grade(score: Int) -> String
    if score >= 90
        return "A"
    elif score >= 80
        return "B"
    elif score >= 70
        return "C"
    elif score >= 60
        return "D"
    else
        return "F"

kfn main
    grades = [95, 87, 73, 65, 92]
    for score in grades
        print("Score: " + score + " Grade: " + calculate_grade(score))
```

### 7.4 Word Counter

```killer
kfn count_words(text: String) -> Int
    words = text.split(" ")
    return len(words)

kfn main
    sentence = "The quick brown fox jumps over the lazy dog"
    count = count_words(sentence)
    print("Word count: " + count)  # 9
```

### 7.5 List Operations

```killer
kfn sum_list(numbers: List) -> Int
    total = 0
    for num in numbers
        total = total + num
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

### 7.6 User Profile System

```killer
kfn create_profile(name: String, email: String, age: Int)
    profile = {
        "name": name,
        "email": email,
        "age": age,
        "created": true
    }
    return profile

kfn display_profile(profile)
    print("Name: " + profile["name"])
    print("Email: " + profile["email"])
    print("Age: " + profile["age"])

kfn main
    user = create_profile("Alice", "alice@example.com", 30)
    display_profile(user)
```

---

## 🐛 PART 8: TROUBLESHOOTING

### Q: How do I debug my code?

**A:** Use `print()` statements liberally:
```killer
kfn main
    x = 10
    print("x = " + x)
    y = x * 2
    print("y = " + y)
```

### Q: What's the difference between `==` and `=`?

**A:** 
- `=` is **assignment** (store a value)
- `==` is **comparison** (check if equal)

```killer
x = 5        # assign 5 to x
print(x == 5)  # check if x equals 5 (true)
```

### Q: How do I format output?

**A:** Use string concatenation or K-strings:
```killer
name = "Alice"
age = 30
print(name + " is " + age + " years old")          # concatenation
print("$name is $age years old")                   # K-string
```

### Q: Can I create multi-line functions?

**A:** Yes! Use indentation (Python-style):
```killer
kfn complex_function(a: Int, b: Int)
    step1 = a + b
    step2 = step1 * 2
    step3 = step2 - a
    return step3
```

### Q: What's the maximum list size?

**A:** Lists can grow dynamically. No hard limit for normal use cases.

### Q: Can I have nested loops?

**A:** Yes!
```killer
kfn main
    for i in 0..3
        for j in 0..3
            print(i + "," + j + " ")
```

### Q: How do I exit a program early?

**A:** Use `return` from main:
```killer
kfn main
    if some_error
        return       # exit program
    # continue execution
```

---

## 📚 QUICK REFERENCE

### Basic Syntax

| Feature | Syntax | Example |
|---------|--------|---------|
| Function | `kfn name(param: Type) -> Type` | `kfn add(a: Int, b: Int) -> Int` |
| Variable | `name = value` | `x = 10` |
| String | `"text"` or `$interpolation` | `"Hello " + name` |
| List | `[item1, item2]` | `[1, 2, 3]` |
| Map | `{"key": value}` | `{"name": "Alice"}` |
| If/Else | `if...elif...else` | `if x > 0 then...` |
| Loop | `for i in range` or `while` | `for i in 0..10` |
| Return | `return value` | `return x + y` |

### Built-in Functions

| Function | Purpose | Example |
|----------|---------|---------|
| `print()` | Output to console | `print("Hello")` |
| `len()` | Get length | `len([1,2,3])` |
| `sort()` | Sort list | `sort(numbers)` |
| `reverse()` | Reverse list | `reverse(numbers)` |
| `contains()` | Check if exists | `contains(list, 5)` |
| `append()` | Add to list | `list.append(item)` |
| `upper()` | Uppercase string | `"hello".upper()` |
| `lower()` | Lowercase string | `"HELLO".lower()` |
| `split()` | Split string | `"a b c".split(" ")` |

---

## 🎓 LEARNING PROGRESSION

### Level 1: Beginner (1-2 hours)
- [ ] Hello World
- [ ] Variables and basic types
- [ ] Print and output
- [ ] Basic arithmetic

### Level 2: Intermediate (2-4 hours)
- [ ] Functions with parameters
- [ ] Control flow (if/else)
- [ ] Loops (for, while)
- [ ] Lists and basic iteration

### Level 3: Advanced (4-8 hours)
- [ ] Complex functions
- [ ] Nested structures
- [ ] Maps and dictionaries
- [ ] Real-world programs

### Level 4: Expert (1-2 weeks)
- [ ] Recursion and optimization
- [ ] Higher-order functions
- [ ] Advanced patterns
- [ ] Performance optimization

---

## 💡 Pro Tips

1. **Start Small:** Begin with simple functions before complex ones
2. **Test Often:** Run your code frequently to verify it works
3. **Read Documentation:** Check built-in functions when stuck
4. **Name Variables Clearly:** Use descriptive names like `total_count` not `tc`
5. **Comment Your Code:** Especially for complex logic
6. **Break Down Problems:** Divide complex tasks into smaller functions
7. **Use Indentation:** Consistent indentation is critical (Python-style)
8. **Practice:** Write small programs regularly

---

## 📝 EXAMPLE: PUTTING IT ALL TOGETHER

A complete program that uses most concepts:

```killer
# Calculate statistics for a list of numbers
kfn calculate_stats(numbers: List)
    if len(numbers) == 0
        print("Empty list")
        return
    
    # Calculate sum
    total = 0
    for num in numbers
        total = total + num
    
    # Calculate average
    average = total / len(numbers)
    
    # Find minimum and maximum
    min_val = numbers[0]
    max_val = numbers[0]
    for num in numbers
        if num < min_val
            min_val = num
        if num > max_val
            max_val = num
    
    # Display results
    print("Numbers: " + numbers)
    print("Count: " + len(numbers))
    print("Sum: " + total)
    print("Average: " + average)
    print("Min: " + min_val)
    print("Max: " + max_val)

kfn main
    data = [45, 23, 78, 12, 56, 89, 34]
    calculate_stats(data)
```

**Output:**
```
Numbers: [45, 23, 78, 12, 56, 89, 34]
Count: 7
Sum: 337
Average: 48
Min: 12
Max: 89
```

---

## 🚀 NEXT STEPS

1. **Write Your First Program:** Create a simple calculator
2. **Solve Problems:** Implement functions to solve coding challenges
3. **Build Projects:** Create a real-world application
4. **Optimize:** Improve your code's performance
5. **Share:** Show your code to others and get feedback

---

## 📞 SUPPORT

If you encounter issues:
1. Check this guide for examples
2. Review the error message carefully
3. Test with simpler code first
4. Use `print()` to debug

---

**Status:** ✅ Ready to Use  
**Binary:** killer.exe  
**Guide Version:** 1.0  
**Last Updated:** 2026-03-20

**Happy Coding! 🎉**
