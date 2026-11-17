//! Differential tests for call parameters living in frame slots (see `docs/call_frames.md`): every
//! program runs once normally (parameters and captured variables resolved at compile time) and once
//! with `KILLER_NO_PARAM_SLOTS=1` (the old named `argN` scheme) and must print exactly the same, errors
//! and their line numbers included. Programs with a known answer also pin that answer.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run_with(src: &str, legacy: bool) -> String {
    let path = std::env::temp_dir().join(format!("killer_sd_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer_super"));
    cmd.arg(&path).arg("--run");
    if legacy {
        cmd.env("KILLER_NO_PARAM_SLOTS", "1");
    } else {
        cmd.env_remove("KILLER_NO_PARAM_SLOTS");
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let text = text.replace("\r\n", "\n").replace(&name, "<file>");
    // the instruction number in a runtime error may legitimately differ
    let mut cleaned = String::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find("(at instruction ") {
        cleaned.push_str(&rest[..i]);
        let tail = &rest[i..];
        rest = match tail.find(')') {
            Some(j) => &tail[j + 1..],
            None => "",
        };
    }
    cleaned.push_str(rest);
    cleaned.trim().to_string()
}

/// Run both ways, require identical output, return it.
fn same(src: &str) -> String {
    let slots = run_with(src, false);
    let legacy = run_with(src, true);
    assert_eq!(slots, legacy, "slot and legacy output differ for:\n{}", src);
    slots
}

#[test]
fn recursion_and_mutual_recursion() {
    let src = "fn fact(n) {
  if n <= 1 {
    return 1
  }
  return n * fact(n - 1)
}
fn sumto(xs, i) {
  if i >= len(xs) {
    return 0
  }
  return xs[i] + sumto(xs, i + 1)
}
fn depth(s, n) {
  if n == 0 {
    return s
  }
  return depth(s + \"x\", n - 1)
}
fn is_even(n) {
  if n == 0 {
    return true
  }
  return is_odd(n - 1)
}
fn is_odd(n) {
  if n == 0 {
    return false
  }
  return is_even(n - 1)
}
println(fact(10))
println(sumto([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0))
println(len(depth(\"\", 50)))
println(is_even(10))
println(is_odd(7))
println(is_even(7))
";
    assert_eq!(same(src), "3628800\n55\n50\ntrue\ntrue\nfalse");
}

#[test]
fn parameters_are_assignable_and_independent_per_call() {
    let src = "fn f(a, b = 5) {
  a = a + 1
  b = b * 2
  return a + b
}
fn g(first, ...rest) {
  return [first, len(rest)]
}
fn swap(a, b) {
  t = a
  a = b
  b = t
  return [a, b]
}
fn fill(xs, v) {
  xs[0] = v
  return xs
}
println(f(1))
println(f(1, 2))
println(g(1))
println(g(1, 2, 3))
println(swap(1, 2))
ys = [0, 0]
fill(ys, 9)
println(ys)
fn inner(n) {
  m = n + 1
  return m * 2
}
fn outer(n) {
  k = inner(n)
  return n + k
}
println(outer(3))
";
    assert_eq!(same(src), "12\n6\n[1, 0]\n[1, 2]\n[2, 1]\n[9, 0]\n11");
}

#[test]
fn closures_share_state_and_keep_their_own() {
    let src = "fn make() {
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
p = make()
inc = p[0]
get = p[1]
x = inc()
x = inc()
println(get())
q = make()
f2 = q[0]
y = f2()
println(y)
println(get())
fn counter(start) {
  c = start
  fn step(d) {
    c = c + d
    return c
  }
  return step
}
a = counter(10)
b = counter(100)
println(a(1))
println(a(1))
println(b(5))
println(a(1))
fn make_adder(k) {
  fn add(x) {
    return x + k
  }
  return add
}
add5 = make_adder(5)
s = 0
for i in range(10) {
  s = add5(s)
}
println(s)
fn compose(f, g) {
  return fn(x) { return f(g(x)) }
}
h = compose(add5, make_adder(3))
println(h(1))
";
    assert_eq!(same(src), "2\n1\n2\n11\n12\n105\n13\n50\n9");
}

#[test]
fn closures_in_loops_capture_per_iteration() {
    let src = "fs = []
for i in range(3) {
  push(fs, fn() { return i * 10 })
}
for f in fs {
  println(f())
}
fn build() {
  out = []
  for j in range(3) {
    push(out, fn(x) { return x + j })
  }
  return out
}
for g in build() {
  println(g(100))
}
fn mk(v) {
  return fn() { return v }
}
ws = []
for k in range(3) {
  push(ws, mk(k * 2))
}
for w in ws {
  println(w())
}
";
    same(src);
    assert_eq!(same("fs = []\nfor i in range(3) {\n  push(fs, fn() { return i * 10 })\n}\nprintln(fs[0]())\nprintln(fs[2]())\n"), "0\n20");
}

#[test]
fn methods_and_this() {
    let src = "class Counter {
  fn init(start, step = 1) {
    this.n = start
    this.step = step
  }
  fn bump(by) {
    this.n = this.n + by * this.step
    return this.n
  }
  fn twice(by) {
    this.bump(by)
    return this.bump(by)
  }
}
c = new Counter(5)
println(c.bump(2))
println(c.twice(1))
d = new Counter(0, 10)
println(d.bump(3))
println(c.n)
class Animal {
  fn init(name) {
    this.name = name
  }
  fn speak(times) {
    s = \"\"
    for i in range(times) {
      s = s + this.name
    }
    return s
  }
}
class Dog extends Animal {
  fn fetch(x) {
    return this.speak(x) + \"!\"
  }
}
dg = new Dog(\"ab\")
println(dg.fetch(3))
";
    assert_eq!(same(src), "7\n9\n30\n9\nababab!");
}

#[test]
fn generators_keep_their_parameters() {
    let src = "fn count(a, b) {
  n = a
  while n < b {
    yield n
    n = n + 1
  }
}
fn evens(xs) {
  for x in xs {
    if x % 2 == 0 {
      yield x
    }
  }
}
g = count(3, 7)
println(next(g))
h = count(10, 12)
println(next(h))
println(next(g))
println(next(h))
println(next(g))
t = 0
for v in evens([1, 2, 3, 4, 5, 6]) {
  t = t + v
}
println(t)
println(list(count(0, 4)))
";
    assert_eq!(same(src), "3\n10\n4\n11\n5\n12\n[0, 1, 2, 3]");
}

#[test]
fn spawn_and_async_tasks_receive_their_arguments() {
    let src = "fn add(a, b) {
  return a + b
}
fn sq(x) {
  return x * x
}
hs = []
for i in range(8) {
  push(hs, async_spawn(sq, i))
}
t = 0
for h in hs {
  t = t + async_await(h)
}
println(t)
println(async_await(async_spawn(add, 2, 5)))
fn mk(k) {
  fn f(x) {
    return x + k
  }
  return f
}
f = mk(10)
println(async_await(async_spawn(f, 5)))
";
    assert_eq!(same(src), "140\n7\n15");
}

#[test]
fn errors_inside_calls_keep_their_lines_and_finally_runs() {
    let src = "fn boom(a, b) {
  c = a / b
  return a[5] + c
}
fn mid(x) {
  return boom(x, 2)
}
try {
  mid(3)
} catch e {
  println(\"caught\")
}
fn guarded(n) {
  try {
    if n > 1 {
      throw \"big \" + str(n)
    }
    return n
  } catch e {
    return e
  } finally {
    println(\"finally \" + str(n))
  }
}
println(guarded(1))
println(guarded(5))
fn top(a) {
  return mid(a)
}
top(7)
";
    let out = same(src);
    assert!(out.contains("caught") && out.contains("finally 1") && out.contains("big 5"), "{out}");
}

#[test]
fn wrong_arity_and_missing_arguments() {
    same("fn f(a, b) {\n  return [a, b]\n}\nprintln(f(1))\nprintln(f(1, 2, 3))\n");
    same("fn g(a) {\n  return a\n}\nprintln(g())\nprintln(g(1, 2))\n");
    same("fn h(a, b) {\n  return a + b\n}\nh = h\nprintln(h(1, 2))\nprintln(h(1))\n");
}

#[test]
fn lambdas_decorators_and_higher_order_calls() {
    let src = "fn twice(f) {
  return fn(x) { return f(f(x)) }
}
fn inc(x) {
  return x + 1
}
println(twice(inc)(5))
println(list(map([1, 2, 3], fn(v) { return v * 3 })))
println(list(filter([1, 2, 3, 4], fn(v) { return v % 2 == 0 })))
println(sorted([3, 1, 2], fn(v) { return 0 - v }))
fn apply(f, a, b) {
  return f(a, b)
}
println(apply(fn(x, y) { return x * y }, 6, 7))
fn logged(f) {
  calls = 0
  fn wrapper(x) {
    calls = calls + 1
    return f(x) + calls
  }
  return wrapper
}
w = logged(inc)
println(w(10))
println(w(10))
println(reduce([1, 2, 3, 4], fn(a, b) { return a + b }, 0))
";
    same(src);
    assert_eq!(same("fn twice(f) {\n  return fn(x) { return f(f(x)) }\n}\nfn inc(x) {\n  return x + 1\n}\nprintln(twice(inc)(5))\n"), "7");
}

#[test]
fn deep_recursion_limit_behaves_the_same() {
    let src = "fn down(n) {
  if n == 0 {
    return 0
  }
  return 1 + down(n - 1)
}
println(down(1000))
fn forever(n) {
  return 1 + forever(n + 1)
}
forever(0)
";
    let out = same(src);
    assert!(out.starts_with("1000"), "{out}");
}

#[test]
fn eval_and_locals_of_nested_calls() {
    let src = "fn f(a, b) {
  return eval(\"1 + 2\") + a + b
}
println(f(10, 20))
fn g(n) {
  return n * 2
}
println(g(g(g(1))))
fn h(a) {
  x = a
  fn inner(b) {
    return x + b
  }
  x = x + 100
  return inner(1)
}
println(h(1))
";
    same(src);
}

#[test]
fn hot_accumulator_loops_in_functions() {
    let src = "fn total(n) {
  s = 0
  i = 0
  while i < n {
    s = s + i * 2
    i = i + 1
  }
  return s
}
println(total(1000))
fn fsum(n, k) {
  s = 0
  for i in range(n) {
    s = s + i * k
  }
  return s
}
println(fsum(100, 3))
fn big(n) {
  s = 1
  i = 0
  while i < n {
    s = s * 3
    i = i + 1
  }
  return s
}
println(big(40))
fn fib(n) {
  if n < 2 {
    return n
  }
  return fib(n - 1) + fib(n - 2)
}
println(fib(20))
fn collatz(n) {
  steps = 0
  while n != 1 {
    if n % 2 == 0 {
      n = n / 2
    } else {
      n = 3 * n + 1
    }
    steps = steps + 1
  }
  return steps
}
println(collatz(27))
";
    same(src);
    let out = same("fn total(n) {\n  s = 0\n  i = 0\n  while i < n {\n    s = s + i * 2\n    i = i + 1\n  }\n  return s\n}\nprintln(total(1000))\nprintln(total(10))\nprintln(total(3000000))\n");
    assert_eq!(out, "999000\n90\n8999997000000");
}
