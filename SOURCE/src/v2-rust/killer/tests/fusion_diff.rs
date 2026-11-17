//! Differential tests for the bytecode shortcuts (blocks without a runtime scope, `for x in range(..)`
//! without building the array, fused instructions): every program runs once normally and once with
//! `KILLER_NO_FUSION=1` (which compiles without any shortcut) and must print exactly the same, errors
//! and their line numbers included. Programs with a known answer also pin that answer.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run_with(src: &str, no_fusion: bool) -> String {
    let path = std::env::temp_dir().join(format!("killer_fd_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer_super"));
    cmd.arg(&path).arg("--run");
    if no_fusion {
        cmd.env("KILLER_NO_FUSION", "1");
    } else {
        cmd.env_remove("KILLER_NO_FUSION");
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_file(&path);
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    // the temp file name differs between the two runs
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    // the instruction number in a runtime error differs when instructions were fused away
    let text = text.replace("\r\n", "\n").replace(&name, "<file>");
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
    let fused = run_with(src, false);
    let plain = run_with(src, true);
    assert_eq!(fused, plain, "fused and unfused output differ for:\n{}", src);
    fused
}

#[test]
fn break_continue_return_inside_scopeless_blocks() {
    let src = "fn f(n) {
  out = 0
  for i in range(n) {
    if i == 2 {
      continue
    }
    if i == 7 {
      break
    }
    if i == 5 {
      if n > 3 {
        out = out + 100
      }
    }
    out = out + i
  }
  return out
}
fn g(xs) {
  for x in xs {
    if x > 2 {
      if x == 4 {
        return x * 10
      }
    }
  }
  return -1
}
println(f(10))
println(f(4))
println(g([1, 2, 3, 4, 5]))
println(g([1, 2]))
k = 0
while k < 10 {
  k = k + 1
  if k == 3 {
    continue
  }
  if k == 8 {
    break
  }
}
println(k)
";
    assert_eq!(same(src), "119
4
40
-1
8");
}

#[test]
fn blocks_that_bind_names_keep_their_scope() {
    // `let`, closures over the loop variable and nested loops must behave exactly as before
    let src = "fn make() {
  fs = []
  for i in range(3) {
    fn f() {
      return i * 10
    }
    push(fs, f)
  }
  return fs
}
r = make()
println(r[0]())
println(r[1]())
println(r[2]())
fn h() {
  if true {
    let q = 1
  }
  return q
}
println(h())
for a in range(2) {
  for b in range(2) {
    println(a * 10 + b)
  }
}
println(a)
println(b)
";
    same(src);
}

#[test]
fn nested_blocks_early_exits_pop_the_right_scopes() {
    let src = "fn f() {
  total = 0
  for i in range(5) {
    for j in range(5) {
      if j == 3 {
        break
      }
      if i == 3 {
        return total
      }
      total = total + 1
    }
    if i == 1 {
      continue
    }
    total = total + 10
  }
  return total
}
println(f())
t = 0
for i in range(4) {
  for j in range(4) {
    if j > i {
      break
    }
    t = t + 1
  }
}
println(t)
";
    assert_eq!(same(src), "29\n10");
}

#[test]
fn runtime_error_lines_inside_scopeless_blocks() {
    let src = "xs = [1, 2, 3]
total = 0
for i in range(5) {
  if i == 3 {
    total = total + xs.nope(1, 2)
  }
  total = total + i
}
println(total)
";
    let out = same(src);
    assert!(out.contains(":5"), "error should name line 5: {}", out);
    let src2 = "fn f(a) {
  n = 0
  for i in range(3) {
    n = n + a[i]
  }
  return n
}
println(f(7))
";
    let out2 = same(src2);
    assert!(out2.contains(":4") || out2.contains("line 4"), "error should name line 4: {}", out2);
}

#[test]
fn exceptions_in_loops_and_try_blocks() {
    let src = "fn f() {
  caught = 0
  for i in range(6) {
    try {
      if i % 2 == 0 {
        throw \"even\"
      }
      caught = caught + 100
    } catch e {
      caught = caught + 1
    }
  }
  return caught
}
println(f())
r = 0
for i in range(10) {
  try {
    if i == 4 {
      throw \"x\"
    }
    r = r + i
  } catch e {
    r = r + 1000
    break
  }
}
println(r)
";
    assert_eq!(same(src), "303\n1006");
}

#[test]
fn generators_with_loops_inside_blocks() {
    let src = "fn evens(n) {
  for i in range(n) {
    if i % 2 == 0 {
      yield i
    }
  }
}
out = []
for v in evens(9) {
  push(out, v)
}
println(out)
";
    assert_eq!(same(src), "[0, 2, 4, 6, 8]");
}

#[test]
fn range_loop_shapes() {
    let src = "for i in range(3) { println(i) }
println(i)
for i in range(2, 5) { println(i) }
for i in range(5, 2) { println(99) }
for i in range(0) { println(98) }
for i in range(-3) { println(97) }
for i in range(-2, 2) { println(i) }
for i in range(3.0) { println(i) }
for i in range(1.0, 3.0) { println(i) }
n = 4
m = 1
for i in range(m, n - 1) { println(i * 100) }
for i in range(n) {
  if i == 1 { continue }
  if i == 3 { break }
  println(i)
}
println(i)
for i in range(1000000000000) { break }
";
    let out = same(src);
    assert!(out.starts_with("0\n1\n2\n2\n2\n3\n4\n-2\n-1\n0\n1\n0\n1\n2\n1\n2\n100\n200\n0\n2\n3"), "{}", out);
    assert!(out.contains("range is too large"), "{}", out);
}

#[test]
fn range_with_fractional_and_non_numeric_arguments_behaves_as_before() {
    same("for i in range(2.5) { println(i) }\nprintln(i)\n");
    same("for i in range(1.5, 4.5) { println(i) }\n");
    same("for i in range(\"a\") { println(i) }\n");
    same("for i in range(null) { println(i) }\n");
    same("for i in range(1, 2, 3, 4) { println(i) }\n");
    same("x = 3\nfor i in range(x + 0.5) { println(i) }\n");
}

#[test]
fn script_defined_range_is_called_normally() {
    let src = "fn range(n) {
  return [10, 20, n]
}
for i in range(7) {
  println(i)
}
println([i * 2 for i in range(5)])
";
    assert_eq!(same(src), "10\n20\n7\n[20, 40, 10]");
}

#[test]
fn range_loops_in_comprehensions_functions_and_nested_calls() {
    let src = "fn squares(n) {
  return [i * i for i in range(n)]
}
fn pairs(n) {
  return [[a, b] for a in range(n) for b in range(a)]
}
println(squares(5))
println(pairs(4))
println([x for x in range(2, 8) if x % 3 == 0])
fn nested(n) {
  s = 0
  for i in range(n) {
    for j in range(i, n) {
      s = s + i * j
    }
  }
  return s
}
println(nested(6))
def_total = 0
for i in range(100) {
  def_total = def_total + nested(3)
}
println(def_total)
";
    same(src);
}

#[test]
fn modifying_the_loop_variable_does_not_change_the_iteration() {
    let src = "out = []
for i in range(5) {
  push(out, i)
  i = i + 10
}
println(out)
println(i)
fn f() {
  out = []
  for i in range(4) {
    i = i * 2
    push(out, i)
  }
  return out
}
println(f())
";
    assert_eq!(same(src), "[0, 1, 2, 3, 4]\n14\n[0, 2, 4, 6]");
}

#[test]
fn closures_capture_range_loop_variables_per_iteration() {
    let src = "fs = []
for i in range(3) {
  push(fs, fn() { return i * 10 })
}
println(fs[0]())
println(fs[1]())
println(fs[2]())
";
    same(src);
}

#[test]
fn objects_methods_and_scopes_in_loops() {
    let src = "class Counter {
  fn init(step) {
    this.n = 0
    this.step = step
  }
  fn inc() {
    this.n = this.n + this.step
  }
  fn get() {
    return this.n
  }
}
c = new Counter(3)
for i in range(5) {
  c.inc()
}
println(c.get())
objs = []
for i in range(3) {
  push(objs, new Counter(i))
}
t = 0
for o in objs {
  o.inc()
  o.inc()
  t = t + o.get()
}
println(t)
";
    assert_eq!(same(src), "15\n6");
}
