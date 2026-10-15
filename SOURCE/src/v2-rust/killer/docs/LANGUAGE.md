# Killer language reference

Status: beta / pre-release (version as in `Cargo.toml`). This reference describes what the
interpreter actually does. Every `killer` example below is executed by `docs/check_examples.py`
(`# expect:` comments are the expected output, one line each), so the examples cannot drift from
the implementation. Anything that does not work is listed under [Known limitations](#known-limitations).

Run a program with `killer_super file.killer --run`.

- [Values and types](#values-and-types)
- [Operators and precedence](#operators-and-precedence)
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
`10 / 5` is `2`, `10 / 4` is `2.5`. Hex literals (`0xFF`) are supported. Integers are 64-bit and
**saturate** instead of overflowing or growing (see limitations).

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

**`//` is always a comment, never integer division.** Writing `7 // 2` is a parse error
("`//` starts a comment in Killer; use `floor(a / b)` for integer division"). Use `floor`:

```killer
println(floor(7 / 2))
# expect: 3
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
removes a key; `in` tests for a key. Iterating a dict with `for k in d` yields its keys; to get
pairs iterate `entries(d)`.

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

The condition is not parenthesised and the body always uses braces **on separate lines** (a whole
`if c { stmt }` on one line is not supported; see limitations).

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
# expect: mid
# expect: elif works
# expect: 3
```

Loops: `while`, `for x in iterable`, `for i in range(...)`, C-style `for (i = 0; i < n; i++)`, and
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

Closures capture variables by reference and keep their state. Assigning inside a function creates
a local variable unless the name is declared `global`; reading globals needs no declaration.

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
fn add_global(n) {
  global total
  total = total + n
}
add_global(5)
println(total)
# expect: 1
# expect: 2
# expect: 0
# expect: 5
```

Decorators wrap a function: `@loud` above `fn base` means `base = loud(base)`. Recursion works
(5000 levels deep is tested) and self tail-calls run in constant stack depth. Parameters may carry
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

Built-in functions are not first-class values: `f = print` is an "undefined variable" error.
Wrap them: `f = fn(x) { return upper(x) }`.

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

`throw` any value; `try { } catch e { }` binds it; `finally` always runs. Runtime errors (division
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
# expect: caught boom
# expect: 42
# expect: a
# expect: b
# expect: c
# expect: f
# expect: got deep
# expect: ok
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

`import "file.killer"` runs another file and makes its top-level functions and variables available.
Paths are relative to the importing file.

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
# expect: 5
# expect: 7
# expect: 7
# expect: 9
# expect: 5
```

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

The full list of the 710 registered builtin names, grouped, with one-line descriptions, is in
[STDLIB_INDEX.md](STDLIB_INDEX.md). It is generated from `const BUILTINS` in `src/compiler.rs` by
`docs/gen_stdlib_index.py`. Some registered names are not implemented (listed in
`parity/REGISTERED_BUT_DEAD.txt`, flagged in the index). Many AI, device and OS-level builtins need
external services or hardware and are not exercised by the examples here.

## Known limitations

These were observed while writing this document (not hypothetical):

- **Integer overflow saturates**: `2 ** 63` and `9223372036854775807 + 1` both give
  `9223372036854775807`. There is no arbitrary-precision integer; `2 ** 62` is exact.
- **One-line blocks are not parsed**: `if x > 1 { println(x) }` and
  `for i in xs { println(i) }` on a single line fail with "unsupported expression"; put the body on
  following lines.
- **A `for ... in` loop does not rebind a variable that already exists**: after `i = 7`,
  `for i in range(3) { println(i) }` prints `7` three times and leaves `i == 7` (same in
  functions, and after a C-style `for (i = 0; ...)` that used the same name). Use a fresh loop
  variable name.
- `//` is a comment, so there is no integer-division operator (use `floor(a / b)`).
- **`for k, v in d` over a dict** does not give key/value pairs (`v` is `null`); use
  `for e in entries(d)`.
- **Builtins are not first-class values** (`f = print` fails).
- `format(...)`, `parse_int`, `parse_float`, `file_read`/`file_write` and about fifty other names
  are registered but not implemented (see the index); use f-strings, `int`, `float`, `readFile`,
  `writeFile`.
- The x86-64 JIT only handles pure numeric functions; on other CPU architectures everything runs
  in the interpreter (same results, slower).
- Performance claims: only `bench/RESULTS.md` is measured; anything else is not measured.
