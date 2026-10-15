# Killer tutorial

Fourteen short lessons. Every example is executed by `docs/check_examples.py`, and the lines starting
with `# expect:` show what the program prints. Save an example as `lesson.killer` and run it with
`killer run lesson.killer` (or `killer_super lesson.killer --run`). The details are in [LANGUAGE.md](LANGUAGE.md).

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

`break` leaves a loop and `continue` skips to the next round. Short bodies fit on one line
(`for n in range(3) { println(n) }`), and every `for ... in` rebinds its loop variable, even if the name
is already in use.

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
for key, value in person {
  println(key + " = " + str(value))
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
saved = []
for i in range(3) {
  push(saved, fn() {
    return i
  })
}
println(saved[0](), saved[2]())
fn fact(n) {
  if n <= 1 {
    return 1
  }
  return n * fact(n - 1)
}
println(fact(10))
println(fact(25n))
# expect: 3
# expect: 0 2
# expect: 3628800
# expect: 15511210043330985984000000
```

Closures share the variables they capture, and each loop iteration gets its own copy of the loop
variable. Numbers that outgrow 64 bits need the `n` suffix, which is why `fact(25n)` is exact.

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

## Lesson 11: A tiny HTTP API

`serve` runs a web server on the current thread and `route` maps a method and path to a function. The
function receives a dict describing the request (`params` holds `:name` parts of the path) and returns
text (200), a dict with a `status`, or any other dict (sent as JSON). A server and its client cannot
share a thread, so the client runs as a task started with `async_spawn`; tasks get copies of their
arguments, so the port is passed in.

```killer
port = free_port()
route("GET", "/greet/:name", fn(req) {
  return {"message": "hello " + req["params"]["name"]}
})
route("POST", "/add", fn(req) {
  body = json_parse(req["body"])
  if body["a"] == null {
    return {"status": 400, "body": {"error": "missing a"}}
  }
  return {"sum": body["a"] + body["b"]}
})
fn client(port) {
  base = "http://127.0.0.1:" + str(port)
  one = http_request({"url": base + "/greet/ada", "timeout": 10})
  two = http_request({"method": "POST", "url": base + "/add", "body": {"a": 1, "b": 2}, "timeout": 10})
  bad = http_request({"method": "POST", "url": base + "/add", "body": {"b": 2}, "timeout": 10})
  return [one, two, bad]
}
task = async_spawn(client, port)
serve(port, null, {"max_requests": 3})
replies = async_await(task)
for r in replies {
  println(r["status"], r["body"])
}
routes_clear()
# expect: 200 {"message":"hello ada"}
# expect: 200 {"sum":3}
# expect: 400 {"error":"missing a"}
```

The client refuses plain `http://` except to localhost and checks TLS certificates for `https://`
(it needs `curl` installed). Secrets such as API keys travel on curl's stdin, never on a command line.
Pass `{"host": "0.0.0.0"}` as the options to `serve` to accept outside connections (there is no TLS in
the server; use a reverse proxy).

## Lesson 12: Retrieval-augmented generation, offline

RAG is: split documents into chunks, turn each chunk into a vector, find the chunks closest to a
question, and put them in the prompt. `mock_embed` is a deterministic stand-in for a real embedding
model, so this runs with no network; swap in `llm_embed_batch` for a provider. `count_tokens` is an
approximation.

```killer
docs = [
  "To reset your password open Settings, choose Security and press Reset. A link is emailed to you.",
  "Invoices are generated on the first day of each month and can be downloaded as PDF files.",
  "Orders ship within two business days. Express delivery takes two days."
]
store = vec_new(64)
n = 0
for doc in docs {
  for piece in chunk_text(doc, 60, 10) {
    vec_add(store, "chunk" + str(n), mock_embed(piece, 64), {"text": piece})
    n = n + 1
  }
}
println(vec_len(store) == n)
question = "How do I reset my password?"
hits = vec_search(store, mock_embed(question, 64), 2)
println(len(hits))
println(contains(hits[0]["meta"]["text"], "password") or contains(hits[0]["meta"]["text"], "Reset"))
prompt = build_rag_prompt(question, hits)
println(contains(prompt, "Question: How do I reset my password?"))
println(count_tokens(prompt) > 20)
# expect: true
# expect: 2
# expect: true
# expect: true
# expect: true
```

`vec_save(store, "index.kvec")` and `vec_load` persist the index, and `examples/rag_demo.killer` wraps
the same steps in a `serve` loop that answers `POST /ask`.

## Lesson 13: Packages

A package is a folder with a `killer.toml`. Create one and use it from an application (shell session, not
a Killer program):

```text
killer_pkg new mathx --lib          # mathx/killer.toml, src/lib.killer
killer_pkg new app                  # app/killer.toml, src/main.killer
cd app
killer_pkg add mathx --path ../mathx
killer_pkg install                  # writes killer.lock with a SHA-256 per package
killer_pkg check                    # re-hashes what is installed against the lock
killer run src/main.killer
```

`src/main.killer` then imports by name. The import is resolved when the program is compiled and the
package's entry file is inlined, so its functions are called directly (a missing package is a compile
error, not a runtime surprise). The same mechanism works for any file next to your program:

```killer
# file: mathx.killer
fn mathx_double(x) {
  return x * 2
}
```

```killer
import "mathx.killer"
println(mathx_double(21))
# expect: 42
```

Registry dependencies use version requirements (`killer_pkg add mathx@^1.2`), and `killer_pkg publish
--registry file:///path/to/registry` publishes a release (releases are immutable).

## Lesson 14: Testing

`test "name" { }` blocks hold tests; `assert_eq` shows both sides when it fails and `assert_throws`
checks that a function throws. A normal run only defines the tests, `killer test` runs them (each one
isolated, so a failure never hides the others).

```killer
fn clamp(x, lo, hi) {
  if x < lo {
    return lo
  }
  if x > hi {
    return hi
  }
  return x
}
test "clamp keeps values in range" {
  assert_eq(clamp(5, 0, 10), 5)
  assert_eq(clamp(-3, 0, 10), 0)
  assert_eq(clamp(99, 0, 10), 10)
}
test "division by zero throws" {
  assert_throws(fn() {
    x = 1 / 0
  })
}
println("defined, not run")
# expect: defined, not run
```

```text
$ killer test clamp
  ok    lesson.killer::clamp keeps values in range (0 ms)
2 passed, 0 failed, 2 total
```

Format and lint your code with `killer fmt` and `killer lint`; both are described in
[LANGUAGE.md](LANGUAGE.md#tooling), together with the language server and the VS Code extension.

Where next: the language reference ([LANGUAGE.md](LANGUAGE.md)), the standard library index
([STDLIB_INDEX.md](STDLIB_INDEX.md)) and the runnable programs under `tests/` and `parity/realworld/`.
