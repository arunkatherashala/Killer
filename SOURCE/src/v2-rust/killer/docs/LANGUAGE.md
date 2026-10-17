# Killer language reference

Status: beta / pre-release (version as in `Cargo.toml`). This reference describes what the
interpreter actually does. Every `killer` example below is executed by `docs/check_examples.py`
(`# expect:` comments are the expected output, one line each), so the examples cannot drift from
the implementation. Anything that does not work is listed under [Known limitations](#known-limitations).

Run a program with `killer run file.killer` (the `killer` driver, see [Tooling](#tooling)) or
`killer_super file.killer --run`. Both use the same compiler and VM.

- [Values and types](#values-and-types)
- [Operators and precedence](#operators-and-precedence)
- [Big integers](#big-integers)
- [Comments](#comments)
- [Strings](#strings)
- [Lists, dicts and sets](#lists-dicts-and-sets)
- [Control flow](#control-flow)
- [Functions](#functions)
- [Classes](#classes)
- [Errors](#errors)
- [Generators](#generators)
- [Imports](#imports)
- [Async, channels and mutexes](#async-channels-and-mutexes)
- [HTTP servers and clients](#http-servers-and-clients)
- [Vectors, RAG and embeddings](#vectors-rag-and-embeddings)
- [Logging and environment](#logging-and-environment)
- [Packages](#packages)
- [Tests](#tests)
- [Tooling](#tooling)
- [Resource limits and capabilities](#resource-limits-and-capabilities)
- [Killer-specific values](#killer-specific-values)
- [Standard library](#standard-library)
- [Known limitations](#known-limitations)

## Values and types

Variables are created by assignment; there is no `let`/`var`. Blocks use braces and statements end
at the line break (semicolons are optional).

```killer
n = 42
pi_ish = 3.14
name = "killer"
ok = true
nothing = null
xs = [1, 2, 3]
d = {"a": 1}
s = set([1, 2, 2])
println(type(n))
println(type(pi_ish))
println(type(name))
println(type(ok))
println(type(nothing))
println(type(xs))
println(type(d))
println(type(s))
# expect: number
# expect: number
# expect: string
# expect: bool
# expect: null
# expect: array
# expect: dict
# expect: set
```

Integers and floats are both reported as `number`; integers print without a decimal point and
`10 / 5` is `2`, `10 / 4` is `2.5`. Hex literals (`0xFF`) are supported. Plain numbers are 64-bit
and **saturate** instead of overflowing (`2 ** 63` is `9223372036854775807`); for exact large
integers use [big integers](#big-integers) (`123n`).

`type_of(x)` is the same as `type(x)`. Conversions: `str`, `int`, `float`, `bool`.

```killer
println(int(3.9))
println(float("2.5") + 1)
println(str(1) + str(true))
println(bool(0))
println(0xFF)
println(1 == 1.0)
# expect: 3
# expect: 3.5
# expect: 1true
# expect: false
# expect: 255
# expect: true
```

`null` is the absence of a value. `x ?? y` yields `y` when `x` is null and `x?.field` is null when
`x` is null.

```killer
x = null
println(x ?? 5)
println(x?.a)
# expect: 5
# expect: null
```

## Operators and precedence

From tightest to loosest:

| Level | Operators |
|-------|-----------|
| 1 | call `f(x)`, index `a[i]`, slice `a[i:j]`, member `a.b`, `a?.b` |
| 2 | unary `-`, `!`, `not` |
| 3 | `**` (right associative; binds tighter than unary minus on its left: `-2 ** 2` is `-4`) |
| 4 | `*`, `/`, `%` |
| 5 | `+`, `-` |
| 6 | `<<`, `>>` |
| 7 | `&`, `^`, `\|` |
| 8 | `<`, `<=`, `>`, `>=`, `==`, `!=`, `in` (comparisons chain: `1 < 2 < 3`) |
| 9 | `&&` / `and` |
| 10 | `\|\|` / `or` |
| 11 | `??` |
| 12 | `c ? a : b`, `a if c else b` |

```killer
println(2 + 3 * 4 - 6 / 2)
println(2 ** 10)
println(2 ** 3 ** 2)
println(-2 ** 2)
println(17 % 5)
println(true && false)
println(true || false)
println(not false)
println(1 < 2 < 3)
println(6 & 3)
println(6 | 3)
println(6 ^ 3)
println(1 << 4)
println(32 >> 2)
println(2 in [1, 2, 3])
x = 5
println(x > 2 ? "big" : "small")
println("big" if x > 2 else "small")
# expect: 11
# expect: 1024
# expect: 512
# expect: -4
# expect: 2
# expect: false
# expect: true
# expect: true
# expect: true
# expect: 2
# expect: 7
# expect: 5
# expect: 16
# expect: 8
# expect: true
# expect: big
# expect: big
```

Assignment forms: `=`, `+=`, `-=`, `*=`, `/=`, `x++`, `x--`, and multiple assignment
`a, b = 1, 2` (so `a, b = b, a` swaps).

```killer
x = 10
x += 5
x -= 3
x *= 2
x /= 4
println(x)
a, b = 1, 2
a, b = b, a
println(a)
println(b)
i = 1
i++
println(i)
# expect: 6
# expect: 2
# expect: 1
# expect: 2
```

Division by zero is a runtime error, not infinity:

```killer
try {
  x = 1 / 0
} catch e {
  println("caught")
}
# expect: caught
```

## Big integers

A literal with an `n` suffix (`123n`, `0xFFn`) or `big(x)` makes an arbitrary-precision integer, type
name `bigint`. `factorial(n)`, `powmod(base, exp, mod)`, `divmod(a, b)`, `idiv(a, b)`, `isqrt`,
`bit_length` and `gcd` accept them, and so do `abs`, `pow`, `sorted`, comparisons, `str`, f-strings and
JSON output. Nothing overflows: `2n ** 100` has 31 digits.

```killer
println(2n ** 100)
println(factorial(25))
println(big(2) ** 70 + 1)
println(123456789012345678901234567890n * 987654321n)
println(powmod(4, 13, 497))
println(divmod(17, 5))
println(idiv(7, 2))
println(type(5n))
println(0xFFn)
println(factorial(20) == 2432902008176640000)
println(sorted([3n, 1, 2n]))
println(json_stringify({"n": 2n ** 70}))
# expect: 1267650600228229401496703205376
# expect: 15511210043330985984000000
# expect: 1180591620717411303425
# expect: 121932631124828532112482853211126352690
# expect: 445
# expect: [3, 2]
# expect: 3
# expect: bigint
# expect: 255
# expect: true
# expect: [1, 2, 3]
# expect: {"n":1180591620717411303424}
```

Mixed arithmetic rules:

- `+ - * **` with a bigint and an integral number give an exact bigint.
- A bigint combined with a non-integral float gives a float (precision is lost).
- `a / b` is an exact bigint when `b` divides `a`, otherwise a float.
- `%`, `idiv` and `divmod` use **floor** semantics (like Python): `-7n % 3n` is `2`. Plain Number `%`
  keeps C semantics: `-7 % 3` is `-1`.
- Truthiness is one rule everywhere (`if`, `while`, `not`, `&&`, `||`, ternary, `bool()`, `filter`, `all`,
  `any`): `0n` is falsy like `0`, as are `null`, `false`, `""`, `[]` and `{}`.
- `int(b)` of a bigint returns the bigint unchanged (like Python); `int("123...")` with more than 15
  digits and `json_parse` of such an integer also give a bigint. `float`, `str`, `abs`, `min`, `max`,
  `sum`, `round`, `sorted`, `range` (small values), `chr` (small values) accept bigints; `5n in [5]` and
  `[5].index_of(5n)` are true/0; a bigint dict key is its decimal string.
- A bigint equals and orders exactly against a number (`2n ** 64 == 18446744073709551616n`).
- `**` refuses absurd result sizes with an error; division by zero is a catchable error.

```killer
println(-7n % 3n)
println(-7 % 3)
println(6n / 2n)
println(7n % 3n)
println(2n ** 64 == 18446744073709551616n)
println(2n ** 64 > 1e19)
try {
  println(1n / 0n)
} catch e {
  println("caught")
}
# expect: 2
# expect: -1
# expect: 3
# expect: 1
# expect: true
# expect: true
# expect: caught
```

## Comments

Three comment styles exist: `#`, `//` and `--`, each running to the end of the line.

```killer
# hash comment
// slash comment
-- dash comment
x = 1 # trailing hash
y = 2 // trailing slash
z = 3 -- trailing dash
println(x + y + z)
# expect: 6
```

**`//` is always a comment, never integer division.** Because `x // 2` would silently become `x`, the
compiler rejects it: a `//` that follows an expression-ending token (a name, number, `)` or `]`) and is
followed by an operand (a number or `(`, a lowercase name glued to it, or a name already used earlier in
the program) is a compile error: "`//` starts a comment in Killer; use floor(a / b) for integer
division". `// some prose`, `//TODO`, full-line comments, strings and `@lang` blocks are unaffected.
Use `floor` (or `idiv(a, b)` for exact big-integer division):

```killer
println(floor(7 / 2))
# expect: 3
```

```killer
x = 7
y = x // 2
# expect-error: starts a comment in Killer
```

## Strings

Strings use `"..."` or `'...'`, support the escapes `\n \t \\ \"`, triple-quoted `"""..."""` for
multiple lines, indexing and slicing (negative indexes count from the end), `+` and `*`
(repetition), and iteration over characters. `f"..."` interpolates `{expr}`; `k"..."` is the same
with Killer's own string type.

```killer
s = "hello"
println(len(s))
println(s[1])
println(s[-1])
println(s[1:3])
println(upper(s))
println("ab" * 3)
println("a" < "b")
println(f"n is {1 + 2}")
println(k"n is {1 + 2}")
println('single')
println("tab:\there")
println(len("""a
b"""))
for c in "abc" {
  println(c)
}
# expect: 5
# expect: e
# expect: o
# expect: el
# expect: HELLO
# expect: ababab
# expect: true
# expect: n is 3
# expect: n is 3
# expect: single
# expect: tab:	here
# expect: 3
# expect: a
# expect: b
# expect: c
```

Common functions: `split`, `join`, `replace`, `trim`, `upper`, `lower`, `starts_with`,
`ends_with`, `contains`, `index_of`, `pad_left`, `pad_right`, `ord`, `chr`, `reverse`, `repeat`.
`len` counts characters, not bytes. Calling a builtin as a method also works for some
(`"abc".upper()`), but the function form is the supported one.

```killer
parts = split("a,b,c", ",")
println(len(parts))
println(join(parts, "-"))
println(replace("a-b", "-", "+"))
println(trim("  x  "))
println(index_of("hello", "l"))
println(starts_with("hello", "he"))
println(contains("hello", "ell"))
println(pad_left("7", 3, "0"))
println(ord("a"))
println(chr(98))
println(reverse("abc"))
println(len("héllo"))
# expect: 3
# expect: a-b-c
# expect: a+b
# expect: x
# expect: 2
# expect: true
# expect: true
# expect: 007
# expect: 97
# expect: b
# expect: cba
# expect: 5
```

`"a" + 1` concatenates (the number is converted), so `"n" + 1` is `"n1"`.

## Lists, dicts and sets

Lists and dicts are shared references: assigning `b = a` makes both names refer to the same
object, and functions can mutate their arguments. Use `copy(x)` for a shallow copy.

```killer
a = [3, 1, 2]
push(a, 4)
println(len(a))
println(a[0])
println(a[-1])
println(a[1:3])
println(pop(a))
insert(a, 0, 9)
println(a)
println(sorted(a))
println(reverse(a))
println([1, 2] + [3])
println(sum([1, 2, 3]))
println(min(4, 2, 9))
println(max(4, 2, 9))
println(2 in [1, 2, 3])
println([x * x for x in range(4)])
println([x for x in range(8) if x % 2 == 0])
m = [[1, 2], [3, 4]]
println(m[1][0])
b = a
push(b, 100)
println(len(a))
# expect: 4
# expect: 3
# expect: 4
# expect: [1, 2]
# expect: 4
# expect: [9, 3, 1, 2]
# expect: [1, 2, 3, 9]
# expect: [2, 1, 3, 9]
# expect: [1, 2, 3]
# expect: 6
# expect: 2
# expect: 9
# expect: true
# expect: [0, 1, 4, 9]
# expect: [0, 2, 4, 6]
# expect: 3
# expect: 5
```

`sorted(list, key_fn)` takes an optional key function; `map`, `filter`, `reduce`, `zip`,
`enumerate`, `any`, `all` are functions, not methods.

```killer
fn neg(x) {
  return 0 - x
}
fn even(x) {
  return x % 2 == 0
}
fn add(a, b) {
  return a + b
}
println(sorted([1, 3, 2], neg))
println(map([1, 2, 3], neg))
println(filter([1, 2, 3, 4], even))
println(reduce([1, 2, 3, 4], add, 0))
println(zip([1, 2], [3, 4]))
for i, x in enumerate(["a", "b"]) {
  println(str(i) + x)
}
# expect: [3, 2, 1]
# expect: [-1, -2, -3]
# expect: [2, 4]
# expect: 10
# expect: [[1, 3], [2, 4]]
# expect: 0a
# expect: 1b
```

Dicts: string, number or bool keys; a missing key reads as `null`. Insertion order is preserved.
`keys(d)`, `values(d)` and `entries(d)` (a list of `[key, value]`) give lists; `delete(d, k)`
removes a key; `in` tests for a key. `for k in d` yields the keys and `for k, v in d` yields key and
value (it iterates the `[key, value]` pairs; any non-dict is iterated as before).

```killer
d = {"a": 1}
d["b"] = 2
println(d["a"] + d["b"])
println(len(keys(d)))
println(d["zz"] == null)
println("a" in d)
println("z" in d)
println(values(d))
for e in entries(d) {
  println(e[0] + str(e[1]))
}
for k, v in d {
  println(k, v)
}
delete(d, "a")
println(len(keys(d)))
println({k: k * 2 for k in range(3)})
println({"x": {"y": 5}}["x"]["y"])
# expect: 3
# expect: 2
# expect: true
# expect: true
# expect: false
# expect: [1, 2]
# expect: a1
# expect: b2
# expect: a 1
# expect: b 2
# expect: 1
# expect: {0: 0, 1: 2, 2: 4}
# expect: 5
```

Sets are created with `set([...])` and are shared and mutable like lists. Operators and functions:
`|`/`set_union`, `&`/`set_intersection`, `set_difference`, `set_add`, `set_remove`, `set_has`,
`set_size`, `set_to_array`.

```killer
s = set([1, 2, 2, 3])
println(len(s))
set_add(s, 10)
println(set_has(s, 10))
println(set([1, 2]) | set([2, 3]))
println(set([1, 2]) & set([2, 3]))
# expect: 3
# expect: true
# expect: {1, 2, 3}
# expect: {2}
```

Tuples `(1, 2, 3)` are written like lists and indexed the same way. A function may return a list
and the caller unpacks it: `a, b = f()`.

## Control flow

The condition is not parenthesised and the body uses braces. A body may be on separate lines or on
the same line (`if c { stmt }`, also for `else`, `while`, `for` and `try`/`catch`): a brace after a
control header opens a block, never a dict literal. A dict literal may follow `in` (`for k, v in {"a": 1} {`, `if x in {"a": 1} {`). Named one-line definitions work too: `fn f(x) { return x }`, `class A { fn m() { return 1 } }`.

```killer
x = 7
if x < 5 {
  println("low")
} else if x < 10 {
  println("mid")
} else {
  println("high")
}
if x > 100 {
  println("huge")
} elif x > 5 {
  println("elif works")
}
i = 0
while i < 3 {
  i = i + 1
}
println(i)
if x > 5 { println("one line") } else { println("never") }
for n in range(2) { println(n) }
# expect: mid
# expect: elif works
# expect: 3
# expect: one line
# expect: 0
# expect: 1
```

Loops: `while`, `for x in iterable` (every iteration rebinds the loop variable, even when a variable of
that name already exists: after `i = 7`, `for i in range(2)` leaves `i == 1`; an empty loop leaves the old
value alone), `for k, v in dict`, `for i in range(...)`, C-style `for (i = 0; i < n; i++)`, and
`do { } while cond`. `break` and `continue` work in all of them. `range(n)`, `range(a, b)`,
`range(a, b, step)` build a list.

```killer
for i in range(0, 10, 3) {
  println(i)
}
t = 0
for (k = 0; k < 4; k++) {
  t = t + k
}
println(t)
n = 0
do {
  n = n + 1
} while n < 3
println(n)
for i in range(6) {
  if i == 1 {
    continue
  }
  if i == 4 {
    break
  }
  println(i)
}
# expect: 0
# expect: 3
# expect: 6
# expect: 9
# expect: 6
# expect: 3
# expect: 0
# expect: 2
# expect: 3
```

`match` compares a value against patterns with `=>`; `_` is the default. `switch` uses `case`
labels and runs only the matching case (no fall-through).

```killer
x = 2
match x {
  1 => println("one")
  2 => println("two")
  _ => println("other")
}
switch x {
  case 1:
    println("one")
  case 2:
    println("two")
}
# expect: two
# expect: two
```

## Scope

Killer follows Python's rule. A plain assignment `x = ...` creates a variable for the whole function
(for the whole file at top level); blocks (`if`, `else`, `while`, `for`, `try`, `catch`) do not open a
scope for it:

```killer
if ready {
  mode = "fast"
} else {
  mode = "slow"
}
println(mode)          // visible after the block

for item in items {
  last = item
}
println(last)          // the loop variable and `last` keep their final values
```

- `let x = ...` is block-scoped: it shadows an outer `x` and is gone after the closing brace.
- A `for` loop variable (and a `let` in the loop body) is a fresh binding per iteration for closures;
  a plain assigned variable is one variable per call.
- Comprehension variables never overwrite an outer variable of the same name.
- Parameters are local; assigning a top-level variable inside a function needs `global x`.
- A variable that was not assigned on the path actually taken reads as `null` (there is no
  "used before assignment" error; initialise it before the block if `null` would be a surprise).

## Functions

Functions are declared with `fn`. A function without `return` yields `null`. Parameters may have
defaults, there may be one variadic parameter `...xs` (a list), and calls may use keyword
arguments. Functions are values: store them in variables and dicts, pass them as arguments, and
use lambdas `fn(a) { ... }` or `(a) => expr`.

```killer
fn greet(name, greeting = "Hi") {
  return greeting + " " + name
}
println(greet("Sai"))
println(greet("Sai", "Yo"))
fn total(...xs) {
  t = 0
  for x in xs {
    t = t + x
  }
  return t
}
println(total(1, 2, 3))
fn sub(a, b) {
  return a - b
}
println(sub(b = 1, a = 10))
fn nothing() {
  x = 1
}
println(nothing())
f = fn(a) {
  return a * 3
}
println(f(4))
g = (a) => a * 3
println(g(5))
table = {"say": fn() {
  return "hi"
}}
println(table["say"]())
# expect: Hi Sai
# expect: Yo Sai
# expect: 6
# expect: 9
# expect: null
# expect: 12
# expect: 15
# expect: hi
```

Closures **share** the variables they capture. A local that a nested function assigns (or that the
enclosing function reassigns after the nested definition) lives in one cell for the whole call: every
closure created by that call, and the enclosing function itself, see the same variable. Loop variables
are captured per iteration (like JavaScript `let`): each closure created in a loop remembers its own
value. Assigning inside a function creates a local variable unless the name is declared `global`;
reading globals needs no declaration.

```killer
fn make() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  return inc
}
c = make()
println(c())
println(c())
total = 0
fn add_local(n) {
  total = total + n
}
add_local(5)
println(total)
fn counter() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  fn get() {
    return n
  }
  return [inc, get]
}
pair = counter()
pair[0]()
pair[0]()
println(pair[1]())
fns = []
for i in range(3) {
  push(fns, fn() {
    return i * 10
  })
}
println(fns[0](), fns[1](), fns[2]())
fn add_global(n) {
  global total
  total = total + n
}
add_global(5)
println(total)
# expect: 1
# expect: 2
# expect: 0
# expect: 2
# expect: 0 10 20
# expect: 5
```

Decorators wrap a function: `@loud` above `fn base` means `base = loud(base)`. Recursion works
(depth 50,000 is fine; the limit is 100,000 frames, see [Resource limits](#resource-limits-and-capabilities))
and self tail-calls run in constant stack depth. Parameters may carry
type annotations (`a: number`, `-> number`) which are checked at call time.

```killer
fn loud(f) {
  fn wrapper(x) {
    return f(x) + 1
  }
  return wrapper
}
@loud
fn base(x) {
  return x * 2
}
println(base(5))
fn add(a: number, b: number) -> number {
  return a + b
}
println(add(1, 2))
fn depth(n) {
  if n == 0 {
    return 0
  }
  return 1 + depth(n - 1)
}
println(depth(5000))
# expect: 11
# expect: 3
# expect: 5000
```

Built-in functions are first-class values: `f = print`, `map(xs, str)`, `sorted(xs, abs)` work. A bare
builtin name compiles to a lookup that prefers a variable of that name and falls back to a callable
wrapper around the builtin. Every registered builtin qualifies, including the library ones (`vec_new`, `mock_embed`, `llm_embed_batch`,
`log_info`, `serve`, `chunk_text`, `isDir`, ...). The original core builtins have a fixed arity; every other
builtin value forwards the arguments actually passed (up to 6), and trailing `null` arguments count as
"not passed", so `f = log_info; f("x")` keeps the optional parameters at their defaults (an explicit trailing
`null` cannot be passed through such a value; wrap the call in a `fn` when you need one).

```killer
f = len
println(f([1, 2, 3]))
println(map(["a", "b"], upper))
println(reduce([1, 5, 2], max, 0))
say = println
say("hi")
# expect: 3
# expect: [A, B]
# expect: 5
# expect: hi
```

Beyond the basics, `format("{} is {:.2}", name, x)` fills `{}`, `{N}` and `{:.N}` placeholders,
`substr(s, start[, len])` slices by character count, `time_now()` is the current time as a number, and
`to_float`, `to_int`, `to_string`, `type_of`, `parse_int`, `parse_float` and the snake_case file names
(`file_read`, `file_write`, `file_append`, `file_exists`, `file_delete`) are aliases of the longer
builtins.

```killer
println(format("{} + {} = {}", 1, 2, 3))
println(format("{1} {0}", "a", "b"))
println(format("{:.3}", 2))
println(substr("hello", 1, 3))
println(to_float("2.5") + 1)
println(type(time_now()))
println(parse_int("42") + 1)
# expect: 1 + 2 = 3
# expect: b a
# expect: 2.000
# expect: ell
# expect: 3.5
# expect: number
# expect: 43
```

## Classes

```killer
class Animal {
  fn init(name) {
    this.name = name
  }
  fn speak() {
    return this.name + " makes a sound"
  }
}
a = new Animal("Rex")
println(a.speak())
println(a instanceof Animal)
println(a.missing)
# expect: Rex makes a sound
# expect: true
# expect: null
```

Instances are created with `new`, `init` is the constructor and `this` the receiver. Objects are
references. Inheritance uses `extends`; `super.method(...)` calls the parent version, including
`super.init(...)`.

```killer
class Animal {
  fn init(name) {
    this.name = name
  }
  fn speak() {
    return this.name + " makes a sound"
  }
}
class Dog extends Animal {
  fn init(name, breed) {
    super.init(name)
    this.breed = breed
  }
  fn speak() {
    return super.speak() + " loudly"
  }
  fn fetch() {
    return this.name + " fetches"
  }
}
d = new Dog("Rex", "lab")
println(d.speak())
println(d.fetch())
println(d.breed)
e = d
e.name = "Max"
println(d.name)
# expect: Rex makes a sound loudly
# expect: Rex fetches
# expect: lab
# expect: Max
```

`static fn` defines a method called on the class. Operator overloading uses dunder methods such as
`__add__`; `toString()` controls how `println` shows an object. Returning `this` allows chaining.

```killer
class V {
  fn init(x) {
    this.x = x
  }
  fn __add__(o) {
    return new V(this.x + o.x)
  }
  fn toString() {
    return "V(" + str(this.x) + ")"
  }
  static fn twice(v) {
    return v * 2
  }
  fn bump() {
    this.x = this.x + 1
    return this
  }
}
r = new V(1) + new V(2)
println(r.x)
println(r)
println(V.twice(4))
println(new V(0).bump().bump().x)
# expect: 3
# expect: V(3)
# expect: 8
# expect: 2
```

## Errors

`throw` any value; `try { } catch e { }` binds it; `finally` runs on **every** exit of its `try`: normal
end, caught or uncaught error, `return` (the value is computed before `finally` runs), `break` and
`continue`. A `return` or `throw` inside `finally` overrides the pending one. Runtime errors (division
by zero, bad index, calling an undefined function) are also catchable. An uncaught error prints
`ERROR: ...` with the file and line to stderr and exits non-zero.

```killer
try {
  throw "boom"
} catch e {
  println("caught " + str(e))
}
try {
  throw {"code": 42}
} catch e {
  println(e["code"])
}
try {
  println("a")
} finally {
  println("b")
}
try {
  throw "x"
} catch e {
  println("c")
} finally {
  println("f")
}
fn bad() {
  throw "deep"
}
fn mid() {
  bad()
}
try {
  mid()
} catch e {
  println("got " + str(e))
}
assert(1 == 1)
println("ok")
fn f() {
  try {
    return "from try"
  } finally {
    println("cleanup")
  }
}
println(f())
for j in range(3) {
  try {
    if j == 1 {
      continue
    }
    println("body", j)
  } finally {
    println("fin", j)
  }
}
# expect: caught boom
# expect: 42
# expect: a
# expect: b
# expect: c
# expect: f
# expect: got deep
# expect: ok
# expect: cleanup
# expect: from try
# expect: body 0
# expect: fin 0
# expect: fin 1
# expect: body 2
# expect: fin 2
```

An uncaught error carries the line number:

```killer
println("before")
println(never_defined)
# expect-error: main.killer:2
```

## Generators

A function containing `yield` is a generator. It is lazy: values are produced one at a time as the
`for` loop asks for them, so infinite generators work when the loop `break`s.

```killer
fn gen() {
  yield 1
  yield 2
}
for x in gen() {
  println(x)
}
fn naturals() {
  n = 0
  while true {
    yield n
    n = n + 1
  }
}
for x in naturals() {
  if x > 2 {
    break
  }
  println(x)
}
# expect: 1
# expect: 2
# expect: 0
# expect: 1
# expect: 2
```

## Imports

`import "file.killer"` includes another file at compile time and makes its top-level functions,
classes and variables available, as if its text had been written at that place (error lines still
point into the imported file). Paths are relative to the importing file; a file is included once, so
repeated or circular imports are skipped. `import name` (bare) and `import "name/sub.killer"` also
resolve through installed packages (see [Packages](#packages)). An import that cannot be found is a
compile error that lists the places searched.

```killer
# file: helper.killer
fn helper_add(a, b) {
  return a + b
}
```

```killer
import "helper.killer"
println(helper_add(2, 3))
# expect: 5
```

## Async, channels and mutexes

`async fn` declares an asynchronous function and `await` waits for its result. `spawn f()` starts a
function concurrently and returns a handle that `await` accepts. The functional forms are
`async_spawn(f)`, `async_await(handle)`. Channels pass values between tasks and a mutex guards a
shared value.

**Tasks get deep copies.** Arguments, the function's captured variables, a snapshot of the globals,
channel messages and results are all copied across the thread boundary (no object is ever shared between
OS threads). Mutating a list, dict, object or set inside a task is invisible to the parent and the
parent's later changes are invisible to the task; communicate with channels, `mutex_*` cells or the
returned value. `async_spawn(f, args...)` passes arguments.

```killer
async fn five() {
  return 5
}
println(await five())
fn w() {
  return 7
}
h = spawn w()
println(await h)
h2 = async_spawn(w)
println(async_await(h2))
ch = chan_new()
chan_send(ch, 9)
println(chan_recv(ch))
m = mutex_new(1)
mutex_add(m, 4)
println(mutex_get(m))
mutex_set(m, 9)
println(mutex_get(m))
items = [1, 2]
fn work(xs) {
  push(xs, 99)
  return len(xs)
}
h3 = async_spawn(work, items)
println(async_await(h3))
println(len(items))
# expect: 5
# expect: 7
# expect: 7
# expect: 9
# expect: 5
# expect: 9
# expect: 3
# expect: 2
```

## HTTP servers and clients

**Server.** `serve(port, handler[, opts])` binds a TCP port and runs the accept loop on the calling
thread, one request at a time. The handler receives a dict `{method, path, query, headers, body,
params}` (header names are lower case) and returns a string (200 `text/plain`), a dict with `status`
(and optional `headers` and `body`), or any other dict or list (sent as JSON). A handler that throws
produces a `500` and the server keeps going. `route(method, path, fn)` registers handlers instead of one
catch-all: `:name` segments land in `req["params"]`, a trailing `*` matches the rest, and `serve(port,
null)` dispatches to the routes (unmatched requests get `404`). Options: `host` (default `127.0.0.1`;
`"0.0.0.0"` exposes the server), `max_requests`, `idle_timeout` (seconds). `serve_once(port, handler)`
serves one request, `server_stop()` called from a handler stops after the current response,
`routes_clear()` forgets the routes and `free_port()` returns an unused local port. `serve` returns the
number of requests served. There is no TLS in the server; put a reverse proxy in front for HTTPS.

**Client.** `http_get(url[, headers, timeout])` returns the body text. `http_post`, `http_put`,
`http_delete`, `http_patch`, `http_head`, `http_get_json` and `http_download` are the other verbs.
`http_request({method, url, headers, body, timeout, retries})` returns `{status, headers, body}`; a dict
body is sent as JSON, and failures on connection errors or `5xx` are retried with exponential backoff.
The client is a hardened wrapper around `curl` (which must be installed, especially for HTTPS): the URL,
headers and body are passed on curl's **stdin**, never on its command line, so API keys do not show up
in the process list; TLS certificates are verified by default (`insecure: true` opts out); plain `http://`
is refused except for `localhost`, `127.0.0.1` and `::1` (or `allow_http: true`); header values with
control characters are rejected; every request has a timeout.

A server and its client cannot share one thread, so the example starts the client as a task. (Tasks get
copies, so the port is passed as an argument.)

```killer
port = free_port()
route("GET", "/hello/:name", fn(req) {
  return "hello " + req["params"]["name"]
})
route("POST", "/sum", fn(req) {
  data = json_parse(req["body"])
  return {"total": data["a"] + data["b"]}
})
fn client(port) {
  base = "http://127.0.0.1:" + str(port)
  a = http_get(base + "/hello/ada", {}, 10)
  b = http_request({"method": "POST", "url": base + "/sum", "body": {"a": 2, "b": 3}, "timeout": 10})
  c = http_get(base + "/nope", {}, 10)
  return [a, b, c]
}
h = async_spawn(client, port)
n = serve(port, null, {"max_requests": 3})
r = async_await(h)
println("served", n)
println(r[0])
println(r[1]["status"], r[1]["body"])
println(r[2])
routes_clear()
# expect: served 3
# expect: hello ada
# expect: 200 {"total":5}
# expect: Not Found: GET /nope
```

## Vectors, RAG and embeddings

Offline building blocks for retrieval-augmented generation, with no model required.

- `vec_new(dim)` creates a store handle; `vec_add(store, id, vector[, meta])` inserts or replaces an entry
  (`meta` is any JSON-like dict); `vec_search(store, query, k[, filter])` returns the `k` nearest entries by
  cosine similarity as `[{id, score, meta}]`, best first, optionally only those whose metadata matches
  `filter`; `vec_get`, `vec_delete`, `vec_len`, `vec_clear`, `vec_free`; `vec_save(store, path)` and
  `vec_load(path)` use a validated binary file.
- `cosine(a, b)`, `dot(a, b)`, `norm(v)` and `top_k(scores, k)` (indexes of the largest scores).
- `chunk_text(text, size, overlap)` makes overlapping character windows that avoid cutting words;
  `chunk_by_sentence(text, max_chars)` packs whole sentences; `count_tokens(text)` is an
  **approximation** (letters cost one token per 4 characters, digits one per 3, punctuation one each),
  usually within 10-20 percent of GPT-style counts, good for budgeting but not exact;
  `build_rag_prompt(question, chunks)` builds a grounded prompt from strings, `{text: ...}` dicts or search hits.
- `mock_embed(text, dim)` is a deterministic offline embedding for tests and demos. `llm_embed_batch`
  and `llm_embed(opts, text)` call a real provider (`provider`, `model`, `base_url` for any
  OpenAI-compatible server, `key_env` naming the environment variable that holds the key); they use the
  hardened client above.

```killer
store = vec_new(3)
vec_add(store, "a", [1, 0, 0], {"tag": "x"})
vec_add(store, "b", [0, 1, 0], {"tag": "y"})
hits = vec_search(store, [0.9, 0.1, 0], 1)
println(hits[0]["id"], round(hits[0]["score"] * 100) / 100, hits[0]["meta"]["tag"])
println(vec_len(store))
println(cosine([1, 0], [1, 1]))
println(dot([1, 2], [3, 4]), norm([3, 4]), top_k([0.1, 0.9, 0.5], 2))
chunks = chunk_text("one two three four five six seven eight nine ten", 20, 5)
println(len(chunks), chunks[0])
println(chunk_by_sentence("Hi there. How are you? Fine.", 15))
println(count_tokens("Hello, world! 12345"))
println(len(mock_embed("hello", 8)), mock_embed("hello", 8) == mock_embed("hello", 8))
println(build_rag_prompt("What is Killer?", ["Killer is a scripting language."]))
# expect: a 0.99 x
# expect: 2
# expect: 0.7071067811865475
# expect: 11 5 [1, 2]
# expect: 3 one two three four
# expect: [Hi there., How are you?, Fine.]
# expect: 8
# expect: 8 true
# expect: Answer the question using only the context below. If the context does not contain the answer, say you do not know.
# expect:
# expect: Context:
# expect: [1] Killer is a scripting language.
# expect:
# expect: Question: What is Killer?
# expect: Answer:
```

`examples/rag_demo.killer` combines all of this with `serve` into a small question-answering service.

## Logging and environment

`log_debug`, `log_info`, `log_warn` and `log_error(message[, fields])` write one line to stderr. The
minimum level comes from the `KILLER_LOG` environment variable (`debug`, `info`, `warn`, `error`, `off`;
default `info`) or `log_set_level(name)`. `KILLER_LOG_FORMAT=json` switches to one JSON object per line.
`load_env(path)` reads a `.env` file (`KEY=VALUE`, `export KEY=VALUE`, `#` comments, quotes) into the
process environment without overriding variables that already exist, and returns the names it set (never
the values); read them with `env_get(name)`.

```killer
writeFile("demo.env", "A_KEY=1\nexport B=\"two words\"\n# comment\n")
println(load_env("demo.env"))
println(env_get("B"))
log_set_level("warn")
log_info("not shown")
log_warn("shown on stderr", {"code": 7})
# expect: [A_KEY, B]
# expect: two words
```

## Packages

A project is a folder with a `killer.toml` manifest; dependencies are pinned in `killer.lock` with a
SHA-256 per package. The `killer_pkg` tool manages them:

```text
killer_pkg new app              # application: killer.toml, src/main.killer, tests/, .gitignore
killer_pkg new mathx --lib      # library: entry src/lib.killer
killer_pkg add mathx --path ../mathx        # also: add name@^1.2 | --git URL [--rev R] | --dev
killer_pkg install              # resolve, download, verify, write killer.lock, fill .killer/packages
killer_pkg update [name]        # re-resolve to the newest allowed versions
killer_pkg list | remove NAME
killer_pkg publish --registry URL --token T [--dry-run]   # registry: file:///dir, a directory, or http://host:port
killer_pkg check                # re-hash installed packages against killer.lock
```

```toml
[package]
name = "app"
version = "0.1.0"
entry = "src/main.killer"

[dependencies]
mathx = { path = "../mathx" }       # or  libA = "^1.2"  or  libC = { git = "https://host/c.git", rev = "abc" }
```

Version requirements are `^1.2`, `~1.2.3`, `>=1.0, <2.0`, `=1.2.3`, `1.2.3` and `*`; conflicts name the
packages involved. Imports are resolved at **compile time**: after `killer_pkg install`,
`import mathx` inlines the package's entry file (and `import "mathx/sub.killer"` a file inside it), so its
functions are called by name. Lookup order: the importing file's own folder, then `packages/<name>` and `.killer/packages/<name>` in
each folder from the script's up to the project root (nearest `killer.toml`), then path dependencies
from the manifest and lock file. Registries are served by
`kpm-registry` (a directory of immutable releases over HTTP, token-protected publish) or are simply a
directory used as a `file://` registry.

## Tests

A `test "name" { ... }` block defines a test; so does any top-level zero-argument `fn test_*()`. A plain
run (`killer run`) only defines them; `killer test` runs each one inside its own `try/catch`, so one
failure never stops the others, and prints a table. `assert_eq(a, b)` fails with both values and where
they first differ, `assert_throws(fn, "text")` passes when the function throws an error containing the
text, and `assert`, `assert_ne`, `assert_true`, `assert_false`, `assert_contains`, `assert_nil` are also
available.

```killer
fn double(x) {
  return x * 2
}
test "doubles" {
  assert_eq(double(2), 4)
}
test "rejects bad input" {
  assert_throws(fn() {
    throw "boom"
  }, "boom")
}
println("tests are only defined here")
# expect: tests are only defined here
```

```text
$ killer test                      # every test in ./ (test blocks, fn test_*, legacy test_*.killer files)
  ok    t.killer::doubles (0 ms)
  FAIL  t.killer::fails
          assert_eq failed
            left:  4
            right: 5
3 passed, 1 failed, 4 total in 0.01s
$ killer test --junit out.xml doubles     # filter = substring of "file::test name"; JUnit XML for CI
$ killer test --list
```

The exit code is 1 when any test fails.

## Tooling

One driver, `killer`, covers everyday work (`cargo build --bin killer`):

| Command | What it does |
|---------|--------------|
| `killer run file.killer` (or `killer file.killer`) | run a program |
| `killer fmt [--check] paths` | re-indent brace code (2 spaces), strip trailing whitespace, collapse blank lines; behaviour-preserving and idempotent; `--check` exits 1 if a file would change |
| `killer lint [--deny-warnings] [--allow RULE] [--rules] paths` | lint; exit 1 on errors (or any warning with `--deny-warnings`) |
| `killer test [--junit FILE] [--list] [filter] [paths]` | run tests (above) |
| `killer repl` | interactive prompt; variables persist, a line ending in `{` continues until braces balance |
| `killer lsp` | language server over stdio |
| `killer --version` | print the version |

**Lint rules** (`killer lint --rules`): `undefined-function` (error: call of a name that is not a function,
class, variable or builtin), `unused-variable`, `unreachable-code` (after `return`/`throw`/`break`/`continue`),
`shadowed-builtin`, `floor-division-comment` (`a // 2` is a comment). Suppress a finding with
`# lint:ignore RULE` on the line or alone on the line above (no rule id ignores every rule), or
`# lint:ignore-file RULE` for a whole file.

**Editor support.** `killer_lsp` (also `killer lsp`) is a language server built on the real compiler:
diagnostics with the user's line numbers, completion of keywords, builtins and your own functions, hover
with one-line docs, go-to-definition and the document outline. `editors/vscode` is a VS Code extension
(syntax highlighting, snippets including `test` blocks, and the language client); see its README for
installation. **Debugger.** `killer_dap` is a Debug Adapter Protocol server (stdio) with line
breakpoints, step in/over/out, continue and variable inspection; point any DAP client at it. It reports
only the innermost scope plus globals and a single stack frame.

## Resource limits and capabilities

Whatever a script contains, the compiler and VM report a Killer error instead of crashing or hanging:

| Limit | Value | Result |
|-------|-------|--------|
| Call depth | 100,000 frames | catchable `stack overflow: recursion too deep` |
| Re-entry through callbacks, `__add__`, `toString`, generators | 400 levels | same error |
| String built by `+`, `*`, `repeat`, `pad_*` | 256 MB | `string is too large` |
| List built by `+` or `*` | 16M elements | `list is too large` |
| `range(n)` materialised | 32M elements | `range is too large ... use a while loop` |
| `json_stringify` output | 64 MB | `output is too large` |
| Source nesting (brackets, blocks, lambdas) | 200 levels | `nesting is too deep` |
| Operators in one expression | 1,000 | `expression is too long` |

```killer
fn forever(n) {
  return forever(n + 1) + 1
}
try {
  forever(0)
} catch e {
  println(e)
}
try {
  x = repeat("ab", 1e12)
} catch e {
  println("too large")
}
# expect: stack overflow: recursion too deep
# expect: too large
```

Capabilities: every builtin that touches files, the network, processes, native code (FFI) or language
models first checks a per-thread `CapabilitySet` (`allow_file_read`, `allow_file_write`,
`allow_network`, `allow_process_spawn`, `allow_native_jit`, `allow_llm`, `allow_ffi`). The `killer`
binaries run with everything allowed (`CapabilitySet::trusted_local()`); an embedder that runs untrusted
scripts installs `CapabilitySet::restricted()` (everything denied) or its own set with
`CapabilityScopeGuard`, and `tests/capabilities.rs` checks that every dispatcher name honours it.

## Killer-specific values

Killer has three-valued logic (trits `T_POS`, `T_ZERO`, `T_NEG`), uncertain numbers declared with
`believe x = 10 ± 1` that propagate error bounds, plus signals, qubits and AI helpers. The
three-valued comparison of an uncertain number returns a trit.

```killer
believe a = 10 ± 1
believe b = 20 ± 2
println(a + b)
println(a > 5)
println(a > 10)
println(T_POS)
println(trit_and(T_POS, T_ZERO))
# expect: 30 ± 3
# expect: T_POS
# expect: T_ZERO
# expect: T_POS
# expect: T_ZERO
```

## HTTP server: one request at a time

`serve(port, handler[, opts])` runs the accept loop and every handler on the VM thread, so
**requests are handled strictly one after another**; a slow handler delays the requests queued
behind it (there is no `workers` option). Guards that keep one bad client or handler from wedging
the process:

- `read_timeout` (seconds, default 10): the total time a client has to send its request. An idle
  or dribbling connection gets `408 Request Timeout` and the next client is served.
- `request_timeout` (seconds, default 60, `0` turns it off): wall-clock deadline for one handler
  call, checked on loop back-edges. A handler stuck in `while true` is aborted and the client gets
  `503`; the server keeps running. A single blocking builtin (`sleep_ms(1e6)`) cannot be interrupted.
- `server_stop()` is process-wide: call it from a handler or from a task on another thread (for
  example a watchdog started with `async_spawn`). It interrupts reads and streams within about
  200 ms; a handler that is mid-call finishes (or hits its deadline) first.
- `stream_timeout` (seconds, default 300) bounds a Server-Sent Events reply.
- Other options: `host`, `max_requests`, `idle_timeout`.

## Streaming model output

`llm_stream(opts, prompt)` returns a lazy generator of text chunks (deltas): the request goes out
when the first chunk is asked for and the reply is read as it arrives, so output shows up while the
model is still writing. `break` stops consuming; `gen_close(g)` also kills the underlying `curl`.
`llm_stream_cb(opts, prompt, fn)` calls `fn(chunk)` for every chunk (return `false` to stop) and
returns the whole text. `opts`: `provider` (`openai` default, `openai_compatible`, `groq`,
`anthropic`, `ollama`, `mock`), `model`, `base_url`, `key` or `key_env`, `system`, `max_tokens`,
`temperature`, `timeout` (seconds for the whole stream, default 120). `prompt` is a string or an
array of `{role, content}` dicts. Wire formats: OpenAI-compatible SSE (`data: {...}` ... `[DONE]`),
Anthropic SSE (`content_block_delta`), Ollama NDJSON. Keys and prompts travel on curl's stdin, never
its command line. `provider: "mock"` needs no network (`reply`, `chunks`, `delay_ms`, `fail_after`).

```killer
for chunk in llm_stream({"provider": "mock", "chunks": ["str", "eam", "ing"]}, "hi") {
  println(chunk)
}
# expect: str
# expect: eam
# expect: ing
```

A `serve` handler can stream too: return `{stream: generator_or_array, headers: {...}}` and the
server sends `text/event-stream`, one `data:` event per item, flushing each (a string is the data; a
dict with only `event`/`data`/`id` becomes a named event; other values are sent as JSON). A
generator that raises ends the stream with an `event: error`; a client that disconnects closes the
generator. The serve option `stream_timeout` (seconds, default 300) bounds a stream.

```killer skip
fn ticker() {
  for i in range(3) {
    yield {"event": "tick", "data": {"n": i}}
  }
}
serve(8080, fn(req) { return {"stream": ticker()} })
```

## Profiling

`killer profile script.killer [--html out.html] [--top N] [--lines N]` (or
`killer_super script.killer --profile [--profile-html out.html]`) runs the script and prints a table
of functions sorted by self time (calls, self ms, total ms, self %) and the hottest source lines
(hit counts and time). `--html` writes a self-contained flame graph page (inline CSS and
JavaScript, click a box to zoom). In a script, `profile_start()` / `profile_stop()` bracket a
region and `profile_report(format)` returns `"text"` (default), `"html"`, `"functions"` (array of
`{name, calls, self_ms, total_ms, self_pct}`) or `"lines"` (array of `{file, line, hits, ms}`).
Self time excludes callees; total time counts a recursive function once. The native JIT is switched
off while profiling so every call and loop iteration is counted; with profiling off the cost is one
branch per instruction. Spawned tasks run on their own VMs and are not profiled.

```killer
fn square(x) {
  return x * x
}
profile_start()
total = 0
for i in range(5) {
  total = total + square(i)
}
profile_stop()
for row in profile_report("functions") {
  if row["name"] == "square" {
    println(row["calls"])
  }
}
# expect: 5
```

## Standard library

The full list of registered builtin names, grouped, with one-line descriptions, is in
[STDLIB_INDEX.md](STDLIB_INDEX.md). It is generated from `const BUILTINS` in `src/compiler.rs` and the
`NAMES` tables of the net/AI modules by `docs/gen_stdlib_index.py`. Every registered name is callable
(`parity/REGISTERED_BUT_DEAD.txt` is empty). Many AI, device and OS-level builtins need external services
or hardware and are not exercised by the examples here.

## Known limitations

Observed, not hypothetical:

- **Block scoping**: a variable first assigned inside an `if`, `while` or `for` body is not visible after
  the block (`Undefined variable`). Assign it before the block. A fix is in progress.
- **Plain integers saturate**: `2 ** 63` and `9223372036854775807 + 1` give `9223372036854775807`, and a
  float of 2^63 or more prints saturated too; use `n` literals / `big()` for exact values.
- **Number `%` keeps C semantics** (`-7 % 3` is `-1`) while bigints use floor semantics (`-7n % 3n` is `2`).
- **Closures created before the enclosing function first assigns a captured variable** keep the old copy
  semantics instead of sharing it.
- `//` is a comment, so there is no integer-division operator (use `floor(a / b)` or `idiv`).
- `len(s)` and `s[i]` on very long strings are O(n) per call.
- `count_tokens` is an approximation, not a model tokenizer.
- The HTTP server has no TLS and serves one request at a time; the HTTP client needs `curl` for HTTPS.
- The debugger shows one stack frame.
- The x86-64 JIT only handles pure numeric functions; on other CPU architectures everything runs in the
  interpreter (same results, slower).
- Performance claims: only `bench/RESULTS.md` is measured; anything else is not measured.
