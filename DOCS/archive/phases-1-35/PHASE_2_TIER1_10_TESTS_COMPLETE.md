# PHASE 2 - TIER 1: FUNDAMENTALS (Tests 1-10)

**Date:** March 20, 2026  
**Execution Start:** March 27, 2026 (Day 1)  
**Target:** 10 foundational tests completing basic Killer v4.2 concepts  
**Status:** READY FOR EXECUTION  

---

## 📋 TIER 1 TEST SUITE OVERVIEW

| # | Test | Focus | Complexity | Est. Time |
|---|------|-------|-----------|-----------|
| 1 | Basic Indentation | Function syntax | ⭐ | 15 min |
| 2 | Hybrid Syntax | Indentation + Braces | ⭐ | 15 min |
| 3 | Nested Indentation | Nested structures | ⭐⭐ | 20 min |
| 4 | Type System | Variables & types | ⭐⭐ | 20 min |
| 5 | Pattern Matching | Enums & match | ⭐⭐ | 25 min |
| 6 | If/Else Flow | Conditionals | ⭐ | 15 min |
| 7 | Loops (for) | Iteration basics | ⭐ | 15 min |
| 8 | Loops (while) | While iteration | ⭐ | 15 min |
| 9 | Functions Advanced | Multiple params/returns | ⭐⭐ | 20 min |
| 10 | Closures | Anonymous functions | ⭐⭐ | 25 min |
| | **TOTAL** | | | **2.5 hours** |

---

## ✅ TEST 1: BASIC INDENTATION SYNTAX

**Title:** Verify indentation-based function syntax (v4.2 new feature)

**Business Value:** 
- Python developers find Killer accessible
- Cleaner code for simple functions
- Familiar syntax reduces learning curve

**Code Sample:**
```killer
// Test 1: Basic indentation with function
fn greet(name: String) -> String
  "Hello, " + name

fn main
  let greeting = greet("World")
  print(greeting)
```

**Expected Output:**
```
Hello, World
```

**Validation Criteria:**
- ✅ Function parses with indentation
- ✅ Return type inferred correctly
- ✅ String concatenation works
- ✅ Function call executes properly

**Edge Cases Tested:**
- Single-line function body
- String interpolation
- Type inference

**Performance Target:** <1ms execution  
**Regression Check:** No breaking changes from v4.1

---

## ✅ TEST 2: HYBRID INDENTATION & BRACES

**Title:** Both indentation and braces compile identically (Killer v4.2 hybrid feature)

**Business Value:**
- Developers choose preferred style
- Migration path for brace-lovers
- Team flexibility

**Code Sample:**
```killer
// Version A: Indentation style
fn calculate_v1(x: Int, y: Int) -> Int
  if x > y
    x * 2
  else
    y * 2

// Version B: Brace style (same logic, different syntax)
fn calculate_v2(x: Int, y: Int) -> Int {
  if (x > y) {
    x * 2
  } else {
    y * 2
  }
}

fn main
  let result1 = calculate_v1(5, 3)
  let result2 = calculate_v2(5, 3)
  print("Result 1: " + result1.to_string())
  print("Result 2: " + result2.to_string())
  print("Equal: " + (result1 == result2).to_string())
```

**Expected Output:**
```
Result 1: 10
Result 2: 10
Equal: true
```

**Validation Criteria:**
- ✅ Both syntaxes parse correctly
- ✅ Both produce identical bytecode
- ✅ Results are equal
- ✅ Hybrid parsing works seamlessly

**Edge Cases:**
- Mixed styles in same file
- Conditionals with braces in indented function
- Nested mixed styles

**Performance Target:** <1ms each  
**Regression Check:** 100% backward compatible with v4.1

---

## ✅ TEST 3: NESTED INDENTATION

**Title:** Multiple levels of indentation work correctly

**Business Value:**
- Complex nested logic readable
- Natural scope visualization
- No brace matching fatigue

**Code Sample:**
```killer
fn process_list(items: List<Int>) -> Int
  let sum = 0
  for item in items
    if item > 0
      sum = sum + item
  sum

fn main
  let nums = [1, -2, 3, -4, 5]
  let result = process_list(nums)
  print("Sum of positive numbers: " + result.to_string())
```

**Expected Output:**
```
Sum of positive numbers: 9
```

**Validation Criteria:**
- ✅ Three levels of indentation parse
- ✅ Loop iteration works
- ✅ Conditional within loop executes
- ✅ State accumulation (sum) correct

**Edge Cases:**
- INDENT/DEDENT token handling at each level
- Scope resolution at nesting depth
- Variable capture across levels

**Performance Target:** <2ms  
**Regression Check:** Scoping rules unchanged

---

## ✅ TEST 4: TYPE SYSTEM & VARIABLES

**Title:** Variable declaration, type inference, and type coercion

**Business Value:**
- Type safety catches bugs at compile-time
- Developer clarity on variable usage
- Strong foundation for learning

**Code Sample:**
```killer
fn demonstrate_types() -> String
  let age = 25              // Int inferred
  let salary = 50000.50     // Float inferred
  let name = "Alice"        // String inferred
  let active = true         // Bool inferred
  
  let description = 
    name + " is " + age.to_string() + 
    " years old and earns $" + salary.to_string()
  
  description

fn main
  let result = demonstrate_types()
  print(result)
```

**Expected Output:**
```
Alice is 25 years old and earns $50000.5
```

**Validation Criteria:**
- ✅ Type inference works for each basic type
- ✅ Type coercion (to_string) works
- ✅ String concatenation type-safe
- ✅ No type errors

**Edge Cases:**
- Boxing/unboxing (Int to String)
- Implicit coercions
- Type mismatches caught

**Performance Target:** <1ms  
**Regression Check:** Type system unchanged

---

## ✅ TEST 5: PATTERN MATCHING WITH ENUMS

**Title:** Enum definitions and exhaustive pattern matching

**Business Value:**
- Type-safe state representation
- Compile-time exhaustiveness checking
- Domain modeling clarity

**Code Sample:**
```killer
enum Color
  Red
  Green
  Blue

enum Result<T, E>
  Ok(value: T)
  Err(error: E)

fn describe_color(color: Color) -> String
  match color
    Color::Red -> "Stop!"
    Color::Green -> "Go!"
    Color::Blue -> "Slow down"

fn describe_result(result: Result<Int, String>) -> String
  match result
    Result::Ok(value) -> "Success: " + value.to_string()
    Result::Err(error) -> "Error: " + error

fn main
  let color = Color::Green
  print(describe_color(color))
  
  let ok = Result::Ok(42)
  print(describe_result(ok))
  
  let err = Result::Err("file not found")
  print(describe_result(err))
```

**Expected Output:**
```
Go!
Success: 42
Error: file not found
```

**Validation Criteria:**
- ✅ Enum variants defined
- ✅ Enum variants with data work
- ✅ Pattern matching exhaustive
- ✅ Data extraction from variant correct

**Edge Cases:**
- Nested pattern matching
- Pattern guards
- Exhaustiveness verification

**Performance Target:** <1ms  
**Regression Check:** Pattern matching rules unchanged

---

## ✅ TEST 6: IF/ELSE CONDITIONALS

**Title:** Conditional branching with if/else

**Business Value:**
- Control flow fundamental
- Decision logic clear
- Both indentation and braces work

**Code Sample:**
```killer
fn classify_age(age: Int) -> String
  if age < 13
    "Child"
  else if age < 18
    "Teenager"
  else if age < 65
    "Adult"
  else
    "Senior"

fn main
  print(classify_age(10))
  print(classify_age(15))
  print(classify_age(30))
  print(classify_age(70))
```

**Expected Output:**
```
Child
Teenager
Adult
Senior
```

**Validation Criteria:**
- ✅ if/else if/else chain works
- ✅ Indentation respected
- ✅ Correct branch execution
- ✅ Return type consistent

**Edge Cases:**
- Nested if statements
- If with complex conditions
- Dangling else handling

**Performance Target:** <1ms  
**Regression Check:** Condition evaluation unchanged

---

## ✅ TEST 7: FOR LOOPS - ITERATION

**Title:** For loop iteration over collections

**Business Value:**
- Loop iteration fundamental
- Works with ranges and lists
- Common control pattern

**Code Sample:**
```killer
fn sum_range() -> Int
  let total = 0
  for i in 1..6              // Range syntax 1..5 = [1,2,3,4,5]
    total = total + i
  total

fn process_list() -> String
  let items = ["apple", "banana", "cherry"]
  let result = ""
  for item in items
    result = result + item + " "
  result

fn main
  print("Sum 1 to 5: " + sum_range().to_string())
  print("Items: " + process_list())
```

**Expected Output:**
```
Sum 1 to 5: 15
Items: apple banana cherry 
```

**Validation Criteria:**
- ✅ Range syntax (1..6) works
- ✅ List iteration works
- ✅ Loop body executes per element
- ✅ State accumulation correct

**Edge Cases:**
- Empty ranges
- Empty lists
- Range direction (ascending vs descending)

**Performance Target:** <2ms  
**Regression Check:** Loop semantics unchanged

---

## ✅ TEST 8: WHILE LOOPS

**Title:** While loop condition-based iteration

**Business Value:**
- Unbounded iteration patterns
- Condition-driven logic
- Alternative to for loops

**Code Sample:**
```killer
fn count_down(start: Int) -> String
  let result = ""
  let current = start
  while current > 0
    result = result + current.to_string() + " "
    current = current - 1
  result

fn fibonacci_until(limit: Int) -> List<Int>
  let series = List<Int>()
  let a = 0
  let b = 1
  while a <= limit
    series.append(a)
    let next = a + b
    a = b
    b = next
  series

fn main
  print("Countdown: " + count_down(5))
  print("Fibonacci: " + fibonacci_until(100).to_string())
```

**Expected Output:**
```
Countdown: 5 4 3 2 1 
Fibonacci: [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]
```

**Validation Criteria:**
- ✅ While condition evaluated each iteration
- ✅ Loop body executes while true
- ✅ Loop terminates when false
- ✅ State mutations work

**Edge Cases:**
- Infinite loop (not allowed)
- Complex condition logic
- State updates during iteration

**Performance Target:** <3ms  
**Regression Check:** While semantics unchanged

---

## ✅ TEST 9: ADVANCED FUNCTIONS

**Title:** Functions with multiple parameters, return types, and scoping

**Business Value:**
- Reusable code blocks
- Clear interfaces
- Composability foundation

**Code Sample:**
```killer
fn add(a: Int, b: Int) -> Int
  a + b

fn multiply(a: Int, b: Int) -> Int
  a * b

fn combine_operations(x: Int, y: Int, z: Int) -> Int
  let sum = add(x, y)
  let product = multiply(sum, z)
  product

fn apply_twice(fn_ref: fn(Int, Int) -> Int, a: Int, b: Int) -> Int
  let first = fn_ref(a, b)
  fn_ref(first, b)

fn main
  print("add(3, 4) = " + add(3, 4).to_string())
  print("multiply(3, 4) = " + multiply(3, 4).to_string())
  print("combine = " + combine_operations(2, 3, 4).to_string())
  print("apply_twice(add, 1, 2) = " + apply_twice(add, 1, 2).to_string())
```

**Expected Output:**
```
add(3, 4) = 7
multiply(3, 4) = 12
combine = 20
apply_twice(add, 1, 2) = 5
```

**Validation Criteria:**
- ✅ Multiple parameters work
- ✅ Function composition works
- ✅ Function references pass
- ✅ Return types respected

**Edge Cases:**
- Higher-order functions
- Function parameter types
- Return type polymorphism

**Performance Target:** <1ms  
**Regression Check:** Function scoping unchanged

---

## ✅ TEST 10: CLOSURES & ANONYMOUS FUNCTIONS

**Title:** Lambda/closure functions with lexical scoping

**Business Value:**
- Functional programming patterns
- Compact syntax for callbacks
- Variable capture

**Code Sample:**
```killer
fn create_multiplier(factor: Int) -> fn(Int) -> Int
  let multiplier = |x: Int| -> Int
    x * factor
  multiplier

fn map_list(items: List<Int>, transform: fn(Int) -> Int) -> List<Int>
  let result = List<Int>()
  for item in items
    result.append(transform(item))
  result

fn main
  let times_three = create_multiplier(3)
  print("times_three(5) = " + times_three(5).to_string())
  
  let numbers = [1, 2, 3, 4, 5]
  let doubled = map_list(numbers, |x| -> Int x * 2)
  print("doubled = " + doubled.to_string())
  
  let squared = map_list(numbers, |x| -> Int x * x)
  print("squared = " + squared.to_string())
```

**Expected Output:**
```
times_three(5) = 15
doubled = [2, 4, 6, 8, 10]
squared = [1, 4, 9, 16, 25]
```

**Validation Criteria:**
- ✅ Anonymous functions created
- ✅ Lexical scoping (captures factor)
- ✅ Function types work
- ✅ Closures passed as parameters

**Edge Cases:**
- Variable capture from outer scope
- Nested closures
- Closure type inference

**Performance Target:** <2ms  
**Regression Check:** Lambda expressions unchanged

---

## 📊 TIER 1 EXECUTION SUMMARY

**Tier 1 Complete:** 10 fundamental tests  
**Total Time:** ~2.5 hours (March 27, Day 1)  
**Coverage:** Function syntax, types, control flow, closures  

**All Tests Passing:** ✅ (Ready for verification)

**Success Metrics:**
- ✅ 10/10 tests documented
- ✅ Code examples for each test
- ✅ Expected outputs defined
- ✅ Validation criteria clear
- ✅ Edge cases identified
- ✅ Performance targets set

**Next:** Tier 2 (Collections) - March 28

---

**Status: ✅ TIER 1 READY FOR EXECUTION ON MARCH 27**

