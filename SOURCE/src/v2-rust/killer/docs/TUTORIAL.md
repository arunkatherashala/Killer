# Killer tutorial

Ten short lessons. Every example is executed by `docs/check_examples.py`, and the lines starting
with `# expect:` show what the program prints. Save an example as `lesson.killer` and run it with
`killer_super lesson.killer --run`. The details are in [LANGUAGE.md](LANGUAGE.md).

## Lesson 1: Hello, values and variables

`println` prints its arguments separated by spaces. Variables appear by assignment.

```killer
println("hello, killer")
age = 30
price = 9.5
println("age:", age, "price:", price)
println(age * 2)
# expect: hello, killer
# expect: age: 30 price: 9.5
# expect: 60
```

## Lesson 2: Strings

```killer
name = "Ada"
println("Hello, " + name + "!")
println(f"{name} has {len(name)} letters")
println(upper(name))
println(name[0])
println(split("a,b,c", ","))
# expect: Hello, Ada!
# expect: Ada has 3 letters
# expect: ADA
# expect: A
# expect: [a, b, c]
```

## Lesson 3: Decisions

Conditions have no parentheses and bodies use braces on their own lines.

```killer
score = 72
if score >= 90 {
  println("A")
} else if score >= 70 {
  println("C or better")
} else {
  println("keep trying")
}
println(score >= 50 ? "pass" : "fail")
# expect: C or better
# expect: pass
```

## Lesson 4: Loops

```killer
total = 0
for n in range(1, 6) {
  total = total + n
}
println(total)
count = 3
while count > 0 {
  println(count)
  count = count - 1
}
for word in ["red", "green"] {
  println(word)
}
# expect: 15
# expect: 3
# expect: 2
# expect: 1
# expect: red
# expect: green
```

`break` leaves a loop and `continue` skips to the next round. Use a fresh loop variable name for
each `for ... in` loop (see the limitations in LANGUAGE.md).

## Lesson 5: Lists

```killer
nums = [5, 3, 8]
push(nums, 1)
println(nums)
println(len(nums))
println(sorted(nums))
println(nums[0])
println(nums[-1])
squares = [x * x for x in range(5)]
println(squares)
println(sum(squares))
# expect: [5, 3, 8, 1]
# expect: 4
# expect: [1, 3, 5, 8]
# expect: 5
# expect: 1
# expect: [0, 1, 4, 9, 16]
# expect: 30
```

## Lesson 6: Dictionaries

```killer
person = {"name": "Ada", "year": 1815}
person["field"] = "maths"
println(person["name"])
println(person["missing"])
println("year" in person)
for key in keys(person) {
  println(key + " = " + str(person[key]))
}
# expect: Ada
# expect: null
# expect: true
# expect: name = Ada
# expect: year = 1815
# expect: field = maths
```

## Lesson 7: Functions

Functions can have default values and take other functions as arguments.

```killer
fn power(base, exp = 2) {
  return base ** exp
}
println(power(5))
println(power(2, 10))
fn apply_twice(f, x) {
  return f(f(x))
}
fn inc(n) {
  return n + 1
}
println(apply_twice(inc, 5))
double = (v) => v * 2
println(map([1, 2, 3], double))
# expect: 25
# expect: 1024
# expect: 7
# expect: [2, 4, 6]
```

## Lesson 8: Closures and recursion

```killer
fn counter() {
  n = 0
  fn next() {
    n = n + 1
    return n
  }
  return next
}
tick = counter()
tick()
tick()
println(tick())
fn fact(n) {
  if n <= 1 {
    return 1
  }
  return n * fact(n - 1)
}
println(fact(10))
# expect: 3
# expect: 3628800
```

## Lesson 9: Classes

```killer
class Account {
  fn init(owner, balance) {
    this.owner = owner
    this.balance = balance
  }
  fn deposit(amount) {
    this.balance = this.balance + amount
    return this
  }
  fn toString() {
    return this.owner + ": " + str(this.balance)
  }
}
class Savings extends Account {
  fn add_interest() {
    this.balance = this.balance * 1.5
  }
}
a = new Savings("Ada", 100)
a.deposit(50).deposit(50)
a.add_interest()
println(a)
# expect: Ada: 300
```

## Lesson 10: Errors, generators and files

Errors are values you can `throw` and `catch`. Generators `yield` values lazily. Files are read and
written with `writeFile` and `readFile`.

```killer
fn safe_div(a, b) {
  if b == 0 {
    throw "cannot divide by zero"
  }
  return a / b
}
try {
  println(safe_div(10, 2))
  println(safe_div(1, 0))
} catch e {
  println("error: " + str(e))
} finally {
  println("done")
}
fn evens() {
  n = 0
  while true {
    yield n
    n = n + 2
  }
}
for ev in evens() {
  if ev > 6 {
    break
  }
  println(ev)
}
writeFile("notes.txt", "saved")
println(readFile("notes.txt"))
# expect: 5
# expect: error: cannot divide by zero
# expect: done
# expect: 0
# expect: 2
# expect: 4
# expect: 6
# expect: saved
```

Where next: the language reference ([LANGUAGE.md](LANGUAGE.md)), the standard library index
([STDLIB_INDEX.md](STDLIB_INDEX.md)) and the runnable programs under `tests/` and `parity/realworld/`.
