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
b = Maker()
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

