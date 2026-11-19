//! Differential tests for read-only closure captures (see `docs/call_frames.md`, "Round 7"): a closure
//! whose body provably never assigns a captured name skips the write-back at `Ret`. Every program runs
//! once normally and once with `KILLER_NO_RO_CAPTURES=1` and must print exactly the same, errors and
//! their line numbers included. Programs with a known answer also pin that answer.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run_with(src: &str, off: Option<&str>) -> String {
    let path = std::env::temp_dir().join(format!("killer_cd_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer_super"));
    cmd.arg(&path).arg("--run");
    cmd.env_remove("KILLER_NO_RO_CAPTURES").env_remove("KILLER_NO_CAP_SLOTS");
    if let Some(var) = off {
        cmd.env(var, "1");
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

/// Run with both optimizations, without capture slots, and without read-only captures at all;
/// require identical output, return it.
fn same(src: &str) -> String {
    let on = run_with(src, None);
    let no_slots = run_with(src, Some("KILLER_NO_CAP_SLOTS"));
    let off = run_with(src, Some("KILLER_NO_RO_CAPTURES"));
    assert_eq!(on, off, "optimized and legacy output differ for:\n{}", src);
    assert_eq!(on, no_slots, "capture-slot and read-only-only output differ for:\n{}", src);
    on
}

#[test]
fn shared_counters_and_accumulators() {
    let src = r#"fn make() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  fn add(d) {
    n += d
    return n
  }
  fn get() {
    return n
  }
  return [inc, add, get]
}
p = make()
p[0]()
p[0]()
println(p[2]())
println(p[1](10))
println(p[2]())
q = make()
println(q[0]())
println(p[2]())
"#;
    assert_eq!(same(src), r#"2
12
12
1
12"#);
}

#[test]
fn closures_stored_in_collections_and_loops() {
    let src = r#"fn mk(k, m) {
  fn f(x) {
    return x * m + k
  }
  return f
}
fs = []
for i in range(4) {
  push(fs, mk(i, i + 1))
}
t = 0
for f in fs {
  t = t + f(10)
}
println(t)
println(fs[3](2))
d = {"a": mk(1, 1), "b": mk(2, 2)}
println(d["a"](5))
println(d["b"](5))
lst = [mk(3, 3), mk(4, 4)]
println(lst[1](1))
"#;
    assert_eq!(same(src), r#"106
11
6
12
8"#);
}

#[test]
fn three_level_nesting() {
    let src = r#"fn outer(a) {
  b = a * 2
  fn mid(c) {
    d = c + b
    fn inner(e) {
      return a + b + d + e
    }
    return inner
  }
  return mid
}
m = outer(1)
i = m(10)
println(i(100))
println(i(200))
m2 = outer(5)
m3 = m2(1)
println(m3(1))
fn lvl1(x) {
  fn lvl2(y) {
    fn lvl3(z) {
      x = x + 1
      return x + y + z
    }
    return lvl3
  }
  return lvl2
}
l2 = lvl1(1)
f3 = l2(2)
println(f3(3))
println(f3(3))
"#;
    assert_eq!(same(src), r#"115
215
27
7
8"#);
}

#[test]
fn recursion_through_captured_function() {
    let src = r#"fn mkfact() {
  fn fact(n) {
    if n <= 1 {
      return 1
    }
    return n * fact(n - 1)
  }
  return fact
}
f = mkfact()
println(f(10))
fn mkfib(limit) {
  fn fib(n) {
    if n < 2 {
      return n
    }
    return fib(n - 1) + fib(n - 2)
  }
  return fib
}
g = mkfib(0)
println(g(15))
fn walker(depth) {
  fn go(n, acc) {
    if n == 0 {
      return acc
    }
    return go(n - 1, acc + depth)
  }
  return go
}
w = walker(3)
println(w(100, 0))
"#;
    assert_eq!(same(src), r#"3628800
610
300"#);
}

#[test]
fn decorators_and_wrappers() {
    let src = r#"fn logged(f, tag) {
  fn wrapper(x) {
    return tag + ":" + str(f(x))
  }
  return wrapper
}
fn double(x) {
  return x * 2
}
d1 = logged(double, "A")
d2 = logged(d1, "B")
println(d1(4))
println(d2(4))
fn counting(f) {
  calls = 0
  fn w(x) {
    calls = calls + 1
    return f(x) + calls
  }
  return w
}
c = counting(double)
println(c(1))
println(c(1))
println(c(1))
fn twice(f) {
  return fn(x) { return f(f(x)) }
}
println(twice(double)(3))
println(twice(twice(double))(1))
"#;
    assert_eq!(same(src), r#"A:8
B:A:8
3
4
5
12
16"#);
}

#[test]
fn generators_capturing_variables() {
    let src = r#"fn gen(k) {
  i = 0
  while i < 3 {
    yield i * k
    i = i + 1
  }
}
fn mkgen(base) {
  fn inner(n) {
    j = 0
    while j < n {
      yield base + j
      j = j + 1
    }
  }
  return inner
}
println(list(gen(5)))
g = mkgen(100)
println(list(g(3)))
a = g(2)
b = g(2)
println(next(a))
println(next(b))
println(next(a))
println(next(b))
"#;
    assert_eq!(same(src), r#"[0, 5, 10]
[100, 101, 102]
100
100
101
101"#);
}

#[test]
fn spawned_tasks_capture_values() {
    let src = r#"fn mk(k) {
  fn f(x) {
    return x + k
  }
  return f
}
f = mk(10)
println(async_await(async_spawn(f, 5)))
hs = []
for i in range(6) {
  push(hs, async_spawn(mk(i), 100))
}
t = 0
for h in hs {
  t = t + async_await(h)
}
println(t)
fn mkc() {
  n = 0
  fn bump(d) {
    n = n + d
    return n
  }
  return bump
}
b = mkc()
println(async_await(async_spawn(b, 3)))
println(b(4))
"#;
    assert_eq!(same(src), r#"15
615
3
4"#);
}

#[test]
fn try_finally_and_error_lines() {
    let src = r#"fn mk(k) {
  fn f(x) {
    try {
      if x < 0 {
        throw "neg"
      }
      return x + k
    } catch (e) {
      return e + str(k)
    } finally {
      println("fin" + str(k))
    }
  }
  return f
}
f = mk(7)
println(f(1))
println(f(-1))
fn mk2(k) {
  fn g(x) {
    return x / k
  }
  return g
}
h = mk2(0)
try {
  println(h(1))
} catch (e) {
  println("caught")
}
fn mk3(k) {
  fn g(x) {
    y = x + k
    return y + nosuchvar
  }
  return g
}
q = mk3(1)
q(2)
"#;
    let got = same(src);
    assert!(got.starts_with("fin7\n8\nfin7\nneg7\ncaught"), "{}", got);
    assert!(got.contains(":34"), "{}", got);
}

#[test]
fn eval_inside_closures() {
    let src = r#"fn mk(k) {
  fn f(x) {
    return eval("x + k")
  }
  return f
}
try {
  println(mk(3)(4))
} catch (e) {
  println("eval-scope-error")
}
fn mk3(k) {
  fn f(x) {
    y = eval("2 + 3")
    return x + k + y
  }
  return f
}
f = mk3(5)
println(f(1))
println(f(1))
"#;
    assert_eq!(same(src), r#"eval-scope-error
11
11"#);
}

#[test]
fn comprehension_forms() {
    let src = r#"xs = [1, 2, 3, 4, 5, 6]
a = [x * 2 for x in xs]
b = [x for x in xs if x % 2 == 0]
c = [[i, j] for i in range(3) for j in range(2) if i != j]
d = {x: x * x for x in xs if x > 3}
e = {x % 3 for x in xs}
println(a)
println(b)
println(c)
println(d)
println(len(e))
k = 10
f = [x + k for x in xs]
println(f)
fn g(n) {
  return [i * n for i in range(4)]
}
println(g(3))
fn h(n) {
  m = n + 1
  fn q(x) {
    return [x * m + t for t in range(3)]
  }
  return q
}
println(h(2)(5))
nested = [[y * x for y in range(3)] for x in range(3)]
println(nested)
"#;
    assert_eq!(same(src), r#"[2, 4, 6, 8, 10, 12]
[2, 4, 6]
[[0, 1], [1, 0], [2, 0], [2, 1]]
{4: 16, 5: 25, 6: 36}
3
[11, 12, 13, 14, 15, 16]
[0, 3, 6, 9]
[15, 16, 17]
[[0, 0, 0], [0, 1, 2], [0, 2, 4]]"#);
}

#[test]
fn comprehension_variable_scoping() {
    let src = r#"fn f() {
  r = [q for q in range(3)]
  return r
}
println(f())
i = 100
z = [i for i in range(3)]
println(z)
println(i)
for w in range(2) {
  tmp = [w * v for v in range(3)]
  println(tmp)
}
fn g() {
  s = 0
  v = 55
  l = [v for v in range(4)]
  return [v, l]
}
println(g())
fn lam(x) {
  return fn(y) { return [x + y for y in range(2)] }
}
println(lam(10)(0))
"#;
    assert_eq!(same(src), r#"[0, 1, 2]
[0, 1, 2]
100
[0, 0, 0]
[0, 1, 2]
[55, [0, 1, 2, 3]]
[10, 11]"#);
}

#[test]
fn comprehension_variable_leak_is_preserved() {
    let src = r#"r = [q for q in range(3)]
println(r)
try {
  println(q)
} catch (e) {
  println("q not visible")
}
fn f() {
  r2 = [u * 2 for u in range(3)]
  try {
    return u
  } catch (e) {
    return "u not visible"
  }
}
println(f())
d = {a: a + 1 for a in range(3)}
println(d)
try {
  println(a)
} catch (e) {
  println("a not visible")
}
n = 5
s = [n for n in range(2)]
println(n)
"#;
    assert_eq!(same(src), r#"[0, 1, 2]
2
2
{0: 1, 1: 2, 2: 3}
2
5"#);
}

#[test]
fn augmented_assignment_captures() {
    let src = r#"fn make_counter() {
  count = 0
  fn inc() {
    count += 1
    return count
  }
  fn dec() {
    count -= 1
    return count
  }
  fn reset() {
    count = 0
    return count
  }
  return {"inc": inc, "dec": dec, "reset": reset}
}
c = make_counter()
c["inc"]()
c["inc"]()
println(c["dec"]())
println(c["inc"]())
println(c["reset"]())
println(c["inc"]())
fn acc(total) {
  fn add(x) {
    total += x
    return total
  }
  fn peek() {
    return total
  }
  return [add, peek]
}
p = acc(100)
p[0](5)
p[0](6)
println(p[1]())
"#;
    assert_eq!(same(src), r#"1
2
0
1
111"#);
}

#[test]
fn mutation_through_captured_collections() {
    let src = r#"fn mk(arr, d) {
  fn push2(x) {
    push(arr, x)
    d["n"] = len(arr)
    return len(arr)
  }
  fn total() {
    s = 0
    for v in arr {
      s = s + v
    }
    return s
  }
  return [push2, total]
}
a = [1]
dd = {"n": 0}
p = mk(a, dd)
p[0](5)
p[0](6)
println(p[1]())
println(a)
println(dd["n"])
fn mkstr(s) {
  fn up() {
    return s + "!"
  }
  fn rep(n) {
    t = ""
    for i in range(n) {
      t = t + s
    }
    return t
  }
  return [up, rep]
}
z = mkstr("ab")
println(z[0]())
println(z[1](3))
"#;
    assert_eq!(same(src), r#"12
[1, 5, 6]
3
ab!
ababab"#);
}

#[test]
fn higher_order_builtins() {
    let src = r#"fn sorter(key) {
  return fn(a) { return a * key }
}
xs = [3, 1, 2]
println(sorted(xs, fn(v) { return -v }))
m = 4
println(map(xs, fn(v) { return v + m }))
println(filter(xs, fn(v) { return v > m - 3 }))
k = sorter(2)
println(sorted(xs, k))
fn mkp(th) {
  return fn(v) { return v > th }
}
println(filter(xs, mkp(1)))
"#;
    assert_eq!(same(src), r#"[3, 2, 1]
[7, 5, 6]
[3, 2]
[1, 2, 3]
[3, 2]"#);
}

#[test]
fn reassigned_and_nested_assigned_captures() {
    let src = r#"fn f() {
  x = 1
  fn g() {
    return x
  }
  x = 2
  return g()
}
println(f())
fn a(v) {
  fn b() {
    fn c() {
      v = v + 1
      return v
    }
    return c()
  }
  return b
}
bb = a(1)
println(bb())
println(bb())
"#;
    assert_eq!(same(src), r#"2
2
3"#);
}

#[test]
fn rebinding_shapes_keep_write_back() {
    let src = r#"fn mk1(k) {
  fn f(n) {
    r = [k]
    for k in range(n) {
      r = r
    }
    return r
  }
  return f
}
f1 = mk1(10)
println(f1(3))
println(f1(5))
fn mk2(k) {
  fn f() {
    r = k
    k++
    return r
  }
  return f
}
f2 = mk2(1)
println(f2())
println(f2())
fn mk3(k) {
  fn f() {
    r = k
    [k, z] = [k + 10, 0]
    return r
  }
  return f
}
f3 = mk3(1)
println(f3())
println(f3())
fn mk4(k) {
  fn f() {
    r = k
    ys = [k * 2 for k in range(2)]
    return [r, ys]
  }
  return f
}
f4 = mk4(7)
println(f4())
println(f4())
fn mk5(k, m) {
  fn f() {
    r = [k, m]
    if r[0] > 0 { k = k + 1 } else { m = m + 1 }
    return r
  }
  return f
}
f5 = mk5(1, 5)
println(f5())
println(f5())
fn mk7(k) {
  fn f() {
    r = k
    k = k * 2; r = r + 1
    return r
  }
  return f
}
f7 = mk7(3)
println(f7())
println(f7())
"#;
    let got = same(src);
    assert!(got.ends_with("4\n7"), "{}", got);
}

#[test]
fn catch_binding_keeps_write_back() {
    let src = r#"fn mk6(k) {
  fn f() {
    r = k
    try {
      throw "x"
    } catch (k) {
      r = r + 100
    }
    return r
  }
  return f
}
f6 = mk6(1)
println(f6())
println(f6())
"#;
    let got = same(src);
    assert!(got.starts_with("101"), "{}", got);
}

#[test]
fn exceptions_recursion_and_split_readers_writers() {
    let src = r#"fn mk(a, b) {
  fn f(x) {
    if x > 0 {
      throw "pos"
    }
    return a + b + x
  }
  return f
}
f = mk(1, 2)
total = 0
for i in range(5) {
  try {
    total = total + f(i - 2)
  } catch (e) {
    total = total + 1000
  }
}
println(total)
fn mkr(n) {
  fn depth(d) {
    if d == 0 {
      return n
    }
    return depth(d - 1) + 1
  }
  return depth
}
println(mkr(5)(200))
fn outer() {
  v = 10
  fn rd() {
    return v + 1
  }
  fn wr() {
    v = v * 2
    return v
  }
  r1 = rd()
  wr()
  r2 = rd()
  return [r1, r2]
}
println(outer())
"#;
    assert_eq!(same(src), r#"2006
205
[11, 21]"#);
}

#[test]
fn hot_closure_loops_stay_correct() {
    let src = r#"fn mk(k) {
  fn add(x) {
    return x + k
  }
  return add
}
a = mk(3)
s = 0
for i in range(200000) {
  s = a(s)
}
println(s)
t = 9007199254740000
for j in range(2000) {
  t = a(t)
}
println(t)
fn compose(f, g) {
  return fn(x) { return f(g(x)) }
}
h = compose(a, mk(4))
u = 0
for i in range(50000) {
  u = u + h(i) % 7
}
println(u)
"#;
    let got = same(src);
    assert_eq!(got, "600000\n9007199254746000\n149997");
}

#[test]
fn capture_slots_with_params_defaults_methods_and_callbacks() {
    let src = r#"fn mk(a, b, c) {
  d = a + b
  e = [a, b, c]
  fn f(x, y) {
    t = x * 2
    u = y + a
    return [t + u + b + c + d, e, len(e)]
  }
  return f
}
f = mk(1, 2, 3)
println(f(10, 20))
println(f(1, 1))
g = mk(5, 6, 7)
println(g(0, 0))
println(f(2, 2))
fn mk2(base, scale) {
  fn apply(xs) {
    return [v * scale + base for v in xs]
  }
  fn label(n) {
    return "{base}:{n}:{scale}"
  }
  fn nested(z) {
    fn inner(w) {
      return base + scale + z + w
    }
    return inner(1)
  }
  return [apply, label, nested]
}
fs = mk2(100, 3)
println(fs[0]([1, 2, 3]))
println(fs[1](7))
println(fs[2](10))
fn mk3(k, h) {
  fn call(x) {
    return h(x) + k
  }
  return call
}
fn sq(x) {
  return x * x
}
c3 = mk3(1, sq)
println(c3(5))
println(map([1, 2, 3], c3))
println(sorted([3, 1, 2], mk3(0, fn(v) { return -v })))
fn mk4(n, acc) {
  fn go(i) {
    if i == 0 {
      return acc
    }
    return go(i - 1) + n
  }
  return go
}
println(mk4(2, 100)(10))
fn mk5(p = 5, q = 7) {
  fn f(x, y = 1) {
    return x + y + p + q
  }
  return f
}
println(mk5()(1))
println(mk5(1, 2)(1, 10))
println(mk5(1)(0))
class Box {
  kfn init(v) {
    this.v = v
  }
  kfn getter(m) {
    fn f(x) {
      return x * m
    }
    return f
  }
}
b = new Box(4)
gf = b.getter(3)
println(gf(5))
fn mk6(items, key) {
  fn find(name) {
    for it in items {
      if it[key] == name {
        return it
      }
    }
    return null
  }
  return find
}
find = mk6([{"n": "a", "v": 1}, {"n": "b", "v": 2}], "n")
println(find("b"))
println(find("z"))
"#;
    assert_eq!(same(src), r#"[49, [1, 2, 3], 3]
[12, [1, 2, 3], 3]
[29, [5, 6, 7], 3]
[15, [1, 2, 3], 3]
[103, 106, 109]
{base}:{n}:{scale}
114
26
[2, 5, 10]
[3, 2, 1]
120
14
14
9
15
{n: b, v: 2}
null"#);
}
