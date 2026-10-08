//! Language-semantics regression tests: shared closure variables, `finally` on every exit,
//! the `//` floor-division mistake. Each test runs a whole program through the real binary.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_ls_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

// ---------------------------------------------------------------------------------------------
// (A) closures share the variables they capture
// ---------------------------------------------------------------------------------------------

#[test]
fn counter_trio_shares_one_variable() {
    let src = r#"
fn make_counter() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  fn get() {
    return n
  }
  fn reset() {
    n = 0
  }
  return [inc, get, reset]
}
c = make_counter()
inc = c[0]
get = c[1]
reset = c[2]
inc()
inc()
inc()
println(get())
reset()
println(get())
inc()
println(get())
"#;
    assert_eq!(run(src), "3\n0\n1");
}

#[test]
fn separate_calls_make_separate_counters() {
    let src = r#"
fn make() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  return inc
}
a = make()
b = make()
a()
a()
println(a())
println(b())
"#;
    assert_eq!(run(src), "3\n1");
}

#[test]
fn enclosing_function_sees_closure_updates_and_vice_versa() {
    let src = r#"
fn run() {
  n = 10
  fn bump() {
    n = n + 5
  }
  bump()
  println(n)
  n = 100
  bump()
  println(n)
  return n
}
println(run())
"#;
    assert_eq!(run(src), "15\n105\n105");
}

#[test]
fn closure_sees_later_assignment_by_the_enclosing_function() {
    let src = r#"
fn f() {
  x = 1
  fn show() {
    return x
  }
  x = 2
  return show()
}
println(f())
"#;
    assert_eq!(run(src), "2");
}

#[test]
fn accumulator_with_parameter_captured_and_mutated() {
    let src = r#"
fn make_acc(total) {
  fn add(x) {
    total = total + x
    return total
  }
  return add
}
acc = make_acc(100)
acc(1)
acc(2)
println(acc(3))
other = make_acc(0)
println(other(5))
"#;
    assert_eq!(run(src), "106\n5");
}

#[test]
fn closures_three_deep_share_the_outermost_variable() {
    let src = r#"
fn outer() {
  count = 0
  fn middle() {
    fn inner() {
      count = count + 1
    }
    inner()
    inner()
    return count
  }
  middle()
  middle()
  return count
}
println(outer())
"#;
    assert_eq!(run(src), "4");
}

#[test]
fn nested_closure_returned_from_a_closure() {
    let src = r#"
fn outer() {
  n = 0
  fn make() {
    fn step() {
      n = n + 1
      return n
    }
    return step
  }
  s = make()
  s()
  s()
  println(n)
  return s
}
s = outer()
println(s())
"#;
    assert_eq!(run(src), "2\n3");
}

#[test]
fn arrow_and_fn_lambdas_mutate_captures() {
    let src = r#"
fn f() {
  total = 0
  add = fn(x) {
    total = total + x
    return total
  }
  add(5)
  add(7)
  return total
}
println(f())
"#;
    assert_eq!(run(src), "12");
}

#[test]
fn closures_passed_to_map_filter_and_sorted_share_state() {
    let src = r#"
fn f() {
  calls = 0
  fn double(x) {
    calls = calls + 1
    return x * 2
  }
  ys = map([1, 2, 3, 4], double)
  fn keep(x) {
    calls = calls + 1
    return x > 4
  }
  zs = filter(ys, keep)
  fn key(x) {
    calls = calls + 1
    return 0 - x
  }
  ws = sorted(ys, key)
  println(ys)
  println(zs)
  println(ws)
  return calls
}
println(f())
"#;
    assert_eq!(run(src), "[2, 4, 6, 8]\n[6, 8]\n[8, 6, 4, 2]\n12");
}

#[test]
fn closures_in_a_loop_capture_the_loop_variable_per_iteration() {
    // Documented semantics: a `for` loop variable (and a `let` inside the body) is a fresh binding
    // for every iteration, like JS `let`; plain assignments are one variable per function call.
    let src = r#"
fn make_all() {
  fs = []
  for i in range(3) {
    fn f() {
      return i * 10
    }
    push(fs, f)
  }
  return fs
}
for f in make_all() {
  println(f())
}
"#;
    assert_eq!(run(src), "0\n10\n20");
}

#[test]
fn closures_in_a_loop_with_let_get_fresh_bindings_and_can_mutate_them() {
    let src = r#"
fn make_all() {
  fs = []
  for i in range(3) {
    let k = i
    fn f() {
      k = k + 100
      return k
    }
    push(fs, f)
  }
  return fs
}
fs = make_all()
println(fs[0]())
println(fs[0]())
println(fs[1]())
println(fs[2]())
"#;
    assert_eq!(run(src), "100\n200\n101\n102");
}

#[test]
fn closures_in_a_loop_share_a_plain_assigned_variable() {
    // a plain assignment is a single variable per call (Python rule), so all closures see it
    let src = r#"
fn f() {
  total = 0
  fns = []
  for i in range(3) {
    fn add() {
      total = total + i
    }
    push(fns, add)
  }
  for g in fns {
    g()
  }
  return total
}
println(f())
"#;
    assert_eq!(run(src), "3");
}

#[test]
fn top_level_loop_closures_capture_the_loop_variable() {
    let src = r#"
fs = []
for i in range(3) {
  fn f() {
    return i
  }
  push(fs, f)
}
for f in fs {
  println(f())
}
"#;
    assert_eq!(run(src), "0\n1\n2");
}

#[test]
fn non_assigned_captures_still_work_and_recursion_keeps_cells_apart() {
    let src = r#"
fn fact_with_counter(n) {
  steps = 0
  fn go(k) {
    steps = steps + 1
    if k <= 1 {
      return 1
    }
    return k * go(k - 1)
  }
  r = go(n)
  return [r, steps]
}
println(fact_with_counter(5))
println(fact_with_counter(3))
fn adder(a) {
  fn add(b) {
    return a + b
  }
  return add
}
println(adder(2)(3))
"#;
    assert_eq!(run(src), "[120, 5]\n[6, 3]\n5");
}

#[test]
fn closure_variable_in_a_method_and_generator() {
    let src = r#"
class Maker {
  kfn init() {
    this.v = 0
  }
  kfn make() {
    n = 0
    fn inc() {
      n = n + 1
      return n
    }
    inc()
    inc()
    return n
  }
}
b = new Maker()
println(b.make())
"#;
    assert_eq!(run(src), "2");
}

#[test]
fn a_global_with_the_same_name_is_not_clobbered() {
    let src = r#"
n = 7
fn make() {
  n = 0
  fn inc() {
    n = n + 1
    return n
  }
  inc()
  return inc()
}
println(make())
println(n)
"#;
    assert_eq!(run(src), "2\n7");
}

#[test]
fn shared_variable_assigned_inside_a_block_of_the_enclosing_function() {
    let src = r#"
fn f(flag) {
  if flag {
    n = 1
  } else {
    n = 2
  }
  fn inc() {
    n = n + 10
    return n
  }
  inc()
  return n
}
println(f(true))
println(f(false))
"#;
    assert_eq!(run(src), "11\n12");
}

// ---------------------------------------------------------------------------------------------
// (B) `finally` runs on every way out of a `try`
// ---------------------------------------------------------------------------------------------

#[test]
fn finally_after_normal_completion() {
    let src = "try {\n  println(\"body\")\n} finally {\n  println(\"fin\")\n}\nprintln(\"after\")\n";
    assert_eq!(run(src), "body\nfin\nafter");
}

#[test]
fn finally_after_a_caught_error() {
    let src = "try {\n  throw \"x\"\n} catch e {\n  println(\"caught \" + e)\n} finally {\n  println(\"fin\")\n}\nprintln(\"after\")\n";
    assert_eq!(run(src), "caught x\nfin\nafter");
}

#[test]
fn finally_runs_then_an_uncaught_error_propagates_to_the_outer_catch() {
    let src = r#"
try {
  try {
    throw "inner"
  } finally {
    println("fin")
  }
  println("not reached")
} catch e {
  println("outer caught " + e)
}
"#;
    assert_eq!(run(src), "fin\nouter caught inner");
}

#[test]
fn finally_runs_before_an_uncaught_error_aborts_the_program() {
    let out = run("try {\n  throw \"boom\"\n} finally {\n  println(\"cleanup\")\n}\nprintln(\"not reached\")\n");
    assert!(out.starts_with("cleanup\n"), "{out}");
    assert!(out.contains("boom"), "{out}");
    assert!(!out.contains("not reached"), "{out}");
}

#[test]
fn finally_runs_for_a_runtime_error_without_catch() {
    let out = run("fn f() {\n  try {\n    x = nope_undefined + 1\n  } finally {\n    println(\"cleanup\")\n  }\n}\ntry {\n  f()\n} catch e {\n  println(\"caught\")\n}\n");
    assert_eq!(out, "cleanup\ncaught");
}

#[test]
fn finally_runs_on_return_and_the_value_is_computed_first() {
    let src = r#"
fn f() {
  x = 1
  try {
    return x
  } finally {
    x = 2
    println("fin " + str(x))
  }
}
println(f())
"#;
    assert_eq!(run(src), "fin 2\n1");
}

#[test]
fn return_expression_is_evaluated_before_finally_side_effects() {
    let src = r#"
log = []
fn side() {
  push(log, "eval")
  return 7
}
fn f() {
  try {
    return side()
  } finally {
    push(log, "finally")
  }
}
v = f()
println(v)
println(log)
"#;
    assert_eq!(run(src), "7\n[eval, finally]");
}

#[test]
fn finally_runs_once_on_return_from_try_and_from_catch() {
    let src = r#"
fn a() {
  try {
    return "a"
  } finally {
    println("fin a")
  }
}
fn b() {
  try {
    throw "x"
  } catch e {
    return "b"
  } finally {
    println("fin b")
  }
}
println(a())
println(b())
"#;
    assert_eq!(run(src), "fin a\na\nfin b\nb");
}

#[test]
fn return_in_finally_overrides_the_try_return() {
    let src = "fn f() {\n  try {\n    return 1\n  } finally {\n    return 2\n  }\n}\nprintln(f())\n";
    assert_eq!(run(src), "2");
}

#[test]
fn return_in_finally_swallows_a_thrown_error() {
    let src = "fn f() {\n  try {\n    throw \"x\"\n  } finally {\n    return 5\n  }\n}\nprintln(f())\nprintln(\"after\")\n";
    assert_eq!(run(src), "5\nafter");
}

#[test]
fn throw_in_finally_replaces_the_pending_return() {
    let src = r#"
fn f() {
  try {
    return 1
  } finally {
    throw "from finally"
  }
}
try {
  println(f())
} catch e {
  println("caught " + e)
}
"#;
    assert_eq!(run(src), "caught from finally");
}

#[test]
fn throw_in_finally_replaces_the_pending_error() {
    let src = r#"
try {
  try {
    throw "first"
  } finally {
    throw "second"
  }
} catch e {
  println(e)
}
"#;
    assert_eq!(run(src), "second");
}

#[test]
fn catch_that_throws_still_runs_finally() {
    let src = r#"
try {
  try {
    throw "a"
  } catch e {
    throw "b"
  } finally {
    println("fin")
  }
} catch e {
  println("outer " + e)
}
"#;
    assert_eq!(run(src), "fin\nouter b");
}

#[test]
fn nested_finally_blocks_run_inside_out() {
    let src = r#"
fn f() {
  try {
    try {
      return "r"
    } finally {
      println("inner")
    }
  } finally {
    println("outer")
  }
}
println(f())
"#;
    assert_eq!(run(src), "inner\nouter\nr");
}

#[test]
fn finally_runs_each_iteration_and_on_break_and_continue() {
    let src = r#"
for i in range(5) {
  try {
    if i == 1 {
      continue
    }
    if i == 3 {
      break
    }
    println("body " + str(i))
  } finally {
    println("fin " + str(i))
  }
}
println("done")
"#;
    assert_eq!(run(src), "body 0\nfin 0\nfin 1\nbody 2\nfin 2\nfin 3\ndone");
}

#[test]
fn finally_on_break_and_continue_in_while_loops() {
    let src = r#"
fn f() {
  i = 0
  n = 0
  while true {
    i = i + 1
    try {
      if i == 2 {
        continue
      }
      if i > 4 {
        break
      }
      n = n + 10
    } finally {
      n = n + 1
    }
  }
  return n
}
println(f())
"#;
    // i=1: +10 +1, i=2: +1, i=3: +10 +1, i=4: +10 +1, i=5: break +1 => 35
    assert_eq!(run(src), "35");
}

#[test]
fn break_in_an_inner_loop_does_not_run_the_outer_try_finally() {
    let src = r#"
try {
  for i in range(3) {
    if i == 1 {
      break
    }
    println(i)
  }
  println("after loop")
} finally {
  println("fin")
}
"#;
    assert_eq!(run(src), "0\nafter loop\nfin");
}

#[test]
fn break_out_of_nested_loops_runs_only_the_finally_inside_the_loop() {
    let src = r#"
for i in range(2) {
  try {
    for j in range(3) {
      try {
        if j == 1 {
          break
        }
        println("j " + str(j))
      } finally {
        println("inner fin " + str(j))
      }
    }
  } finally {
    println("outer fin " + str(i))
  }
}
"#;
    assert_eq!(run(src), "j 0\ninner fin 0\ninner fin 1\nouter fin 0\nj 0\ninner fin 0\ninner fin 1\nouter fin 1");
}

#[test]
fn return_from_inside_a_loop_inside_try_runs_finally() {
    let src = r#"
fn find(xs, target) {
  try {
    for x in xs {
      if x == target {
        return "found"
      }
    }
    return "missing"
  } finally {
    println("searched")
  }
}
println(find([1, 2, 3], 2))
println(find([1, 2, 3], 9))
"#;
    assert_eq!(run(src), "searched\nfound\nsearched\nmissing");
}

#[test]
fn finally_with_errors_thrown_by_a_called_function() {
    let src = r#"
fn boom() {
  throw "deep"
}
fn run() {
  try {
    boom()
  } finally {
    println("run cleanup")
  }
}
try {
  run()
} catch e {
  println("main caught " + e)
}
"#;
    assert_eq!(run(src), "run cleanup\nmain caught deep");
}

#[test]
fn finally_in_methods() {
    let src = r#"
class Res {
  kfn init() {
    this.log = []
  }
  kfn use(fail) {
    try {
      if fail {
        throw "bad"
      }
      push(this.log, "ok")
      return "done"
    } catch e {
      push(this.log, "caught")
      return "failed"
    } finally {
      push(this.log, "closed")
    }
  }
}
r = new Res()
println(r.use(false))
println(r.use(true))
println(r.log)
"#;
    assert_eq!(run(src), "done\nfailed\n[ok, closed, caught, closed]");
}

#[test]
fn finally_in_a_generator_runs_when_it_completes() {
    let src = r#"
fn gen() {
  try {
    yield 1
    yield 2
  } finally {
    println("gen cleanup")
  }
}
for v in gen() {
  println(v)
}
println("end")
"#;
    assert_eq!(run(src), "1\n2\ngen cleanup\nend");
}

#[test]
fn finally_inside_a_closure_with_shared_state() {
    let src = r#"
fn make() {
  depth = 0
  fn guarded() {
    depth = depth + 1
    try {
      return depth
    } finally {
      depth = depth - 1
    }
  }
  fn current() {
    return depth
  }
  return [guarded, current]
}
p = make()
println(p[0]())
println(p[1]())
"#;
    assert_eq!(run(src), "1\n0");
}

#[test]
fn try_inside_finally_and_finally_inside_catch() {
    let src = r#"
try {
  throw "x"
} catch e {
  try {
    println("in catch")
  } finally {
    println("catch finally")
  }
} finally {
  try {
    throw "y"
  } catch e2 {
    println("handled " + e2)
  }
  println("main finally")
}
"#;
    assert_eq!(run(src), "in catch\ncatch finally\nhandled y\nmain finally");
}

#[test]
fn continue_in_catch_runs_finally() {
    let src = r#"
for i in range(3) {
  try {
    if i == 1 {
      throw "skip"
    }
    println("ok " + str(i))
  } catch e {
    println("catch " + str(i))
    continue
  } finally {
    println("fin " + str(i))
  }
  println("tail " + str(i))
}
"#;
    assert_eq!(run(src), "ok 0\nfin 0\ntail 0\ncatch 1\nfin 1\nok 2\nfin 2\ntail 2");
}

#[test]
fn finally_in_a_recursive_function_runs_once_per_call() {
    let src = r#"
count = 0
fn walk(n) {
  try {
    if n == 0 {
      return 0
    }
    return 1 + walk(n - 1)
  } finally {
    global count
    count = count + 1
  }
}
println(walk(4))
println(count)
"#;
    assert_eq!(run(src), "4
5");
}

#[test]
fn finally_after_an_error_inside_a_loop_body_with_catch_outside() {
    let src = r#"
fn f() {
  out = []
  for i in range(3) {
    try {
      if i == 1 {
        throw "bad"
      }
      push(out, i)
    } finally {
      push(out, "f" + str(i))
    }
  }
  return out
}
try {
  f()
} catch e {
  println("caught " + e)
}
println("done")
"#;
    assert_eq!(run(src), "caught bad
done");
}

// ---------------------------------------------------------------------------------------------
// (C) `//` is a comment, so `x // 2` is rejected instead of silently becoming `x`
// ---------------------------------------------------------------------------------------------

const FLOOR_HINT: &str = "// starts a comment in Killer; use floor(a / b) for integer division";

#[test]
fn floor_division_with_double_slash_is_a_compile_error_with_a_hint() {
    let out = run("x = 10\ny = x // 2\nprintln(y)\n");
    assert!(out.contains(FLOOR_HINT), "{out}");
    assert!(out.contains("Line 2"), "{out}");
    assert!(!out.lines().any(|l| l.trim() == "10"), "the program must not run: {out}");
}

#[test]
fn floor_division_inside_functions_and_calls_is_rejected_too() {
    for body in ["return (lo + hi) // 2", "return n //2", "return xs[0] // (k + 1)"] {
        let src = format!("fn f(lo, hi, n, xs, k) {{\n  {}\n}}\nprintln(1)\n", body);
        let out = run(&src);
        assert!(out.contains(FLOOR_HINT), "{body}: {out}");
    }
    let out = run("a = 7\nb = 2\nprintln(a // b)\n");
    assert!(out.contains(FLOOR_HINT), "{out}");
}

#[test]
fn ordinary_comments_still_work() {
    let src = r#"// a whole-line comment
x = 10 // trailing comment
println(x) // another one
y = 3 //TODO revisit
println(y + 1) // 2 more steps
println("a // b is text, not a comment")
// x // 2 inside a comment line
println(floor(7 / 2))
"#;
    assert_eq!(run(src), "10\n4\na // b is text, not a comment\n3");
}

#[test]
fn floor_division_spelled_with_floor_works() {
    assert_eq!(run("println(floor(17 / 5))\nprintln(floor(-17 / 5))\n"), "3\n-4");
}

// ---------------------------------------------------------------------------------------------
// `for x in ...` rebinds a variable that already exists
// ---------------------------------------------------------------------------------------------

#[test]
fn for_loop_rebinds_an_existing_top_level_variable() {
    let src = "i = 7\nfor i in range(3) {\n  println(i)\n}\nprintln(i)\n";
    assert_eq!(run(src), "0\n1\n2\n2");
}

#[test]
fn for_loop_rebinds_an_existing_local_and_a_parameter() {
    let src = r#"
fn f(p) {
  j = 9
  for j in range(2) {
    println(j)
  }
  for p in [5, 6] {
    println(p)
  }
  return j + p
}
println(f(1))
"#;
    assert_eq!(run(src), "0\n1\n5\n6\n7");
}

#[test]
fn for_loop_over_an_empty_list_keeps_the_old_value() {
    assert_eq!(run("k = 4\nfor k in [] {\n  println(\"x\")\n}\nprintln(k)\n"), "4");
}

#[test]
fn for_loop_after_catch_variable_and_after_c_style_for() {
    let src = r#"
try {
  throw "boom"
} catch e {
  println(e)
}
for e in range(2) {
  println(e)
}
for (n = 0; n < 2; n = n + 1) {
  println("c" + str(n))
}
for n in [10, 11] {
  println(n)
}
"#;
    assert_eq!(run(src), "boom\n0\n1\nc0\nc1\n10\n11");
}

// ---------------------------------------------------------------------------------------------
// one-line blocks, `for k, v in dict`, builtins as values, snake_case builtin aliases
// ---------------------------------------------------------------------------------------------

#[test]
fn one_line_blocks_parse() {
    let src = r#"
c = true
if c { println("yes") }
if !c { println("no") } else { println("else") }
for x in [1, 2] { println(x) }
n = 0
while n < 2 { n = n + 1 }
println(n)
d = {"a": 1}
println(d)
println(len({"x": 1, "y": 2}))
try { throw "e" } catch err { println("caught " + err) }
"#;
    assert_eq!(run(src), "yes\nelse\n1\n2\n2\n{a: 1}\n2\ncaught e");
}

#[test]
fn for_key_value_over_a_dict_iterates_pairs() {
    let src = r#"
d = {"a": 1, "b": 2}
out = []
for k, v in d {
  push(out, k + "=" + str(v))
}
println(sorted(out))
keys_seen = []
for k in d {
  push(keys_seen, k)
}
println(sorted(keys_seen))
for i, x in enumerate(["p", "q"]) {
  println(str(i) + x)
}
for a, b in [[1, 2], [3, 4]] {
  println(a + b)
}
"#;
    assert_eq!(run(src), "[a=1, b=2]\n[a, b]\n0p\n1q\n3\n7");
}

#[test]
fn builtins_are_first_class_values() {
    let src = r#"
f = print
f("hi")
println(map([1, 2], str))
g = len
println(g([1, 2, 3]))
println(map([-1, 2], abs))
h = max
println(h(3, 9))
fs = [str, len]
println(fs[1]("abcd"))
len = 5
println(len)
"#;
    assert_eq!(run(src), "hi\n[1, 2]\n3\n[1, 2]\n9\n4\n5");
}
