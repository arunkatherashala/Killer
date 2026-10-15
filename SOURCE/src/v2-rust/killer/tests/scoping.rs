//! Variable scope: plain assignment is function-scoped (module-scoped at top level), like Python;
//! `let` stays block-scoped; loop variables are fresh per iteration for closures.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_sc_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn if_else_assignment_is_visible_after_the_block_at_top_level() {
    assert_eq!(run("c = true\nif c { x = 1 } else { x = 2 }\nprintln(x)\nd = false\nif d { y = 1 } else { y = 2 }\nprintln(y)\n"), "1\n2");
}

#[test]
fn for_loop_assignment_and_variable_survive_the_loop() {
    assert_eq!(run("for i in [1, 2, 3] { last = i }\nprintln(last)\nprintln(i)\n"), "3\n3");
}

#[test]
fn assignment_inside_function_blocks_is_function_scoped() {
    let src = "fn f(c) {\n  if c { y = 1 } else { y = 2 }\n  for k in [4, 5] { z = k }\n  n = 0\n  while n < 3 {\n    w = n * 10\n    n = n + 1\n  }\n  return [y, z, w, k]\n}\nprintln(f(true))\nprintln(f(false))\n";
    assert_eq!(run(src), "[1, 5, 20, 5]\n[2, 5, 20, 5]");
}

#[test]
fn assignment_in_nested_blocks_and_try_catch() {
    let src = "fn g() {\n  if true {\n    if true {\n      deep = 7\n    }\n  }\n  try {\n    t = 1\n    throw \"boom\"\n  } catch e {\n    c = 2\n  }\n  return [deep, t, c]\n}\nprintln(g())\n";
    assert_eq!(run(src), "[7, 1, 2]");
}

#[test]
fn variable_after_block_does_not_clobber_other_locals() {
    // slots of block-created variables must not be reused by later variables
    let src = "fn h() {\n  a = 1\n  if true { b = 2\n    c = 3 }\n  d = 4\n  e = 5\n  return [a, b, c, d, e]\n}\nprintln(h())\n";
    assert_eq!(run(src), "[1, 2, 3, 4, 5]");
}

#[test]
fn let_is_still_block_scoped() {
    let out = run("fn f() {\n  if true {\n    let q = 1\n  }\n  return q\n}\nprintln(f())\n");
    assert!(out.contains("Undefined variable"), "{}", out);
    let out = run("if true {\n  let q = 1\n}\nprintln(q)\n");
    assert!(out.contains("Undefined variable"), "{}", out);
}

#[test]
fn closures_still_capture_loop_variables_per_iteration() {
    let src = "fn make_all() {\n  fs = []\n  for i in range(3) {\n    fn f() {\n      return i * 10\n    }\n    push(fs, f)\n    last = i\n  }\n  push(fs, last)\n  return fs\n}\nr = make_all()\nprintln(r[0]())\nprintln(r[1]())\nprintln(r[2]())\nprintln(r[3])\n";
    assert_eq!(run(src), "0\n10\n20\n2");
}

#[test]
fn comprehension_variable_does_not_clobber_a_variable_assigned_in_a_block() {
    let src = "fn f() {
  if true { v = 100 }
  xs = [v * 2 for v in [1, 2, 3]]
  return [xs, v]
}
println(f())
";
    assert_eq!(run(src), "[[2, 4, 6], 100]");
}

#[test]
fn params_and_globals_behave_as_before() {
    let src = "total = 0\nfn add(p) {\n  global total\n  if p > 0 { total = total + p }\n  p = p + 1\n  return p\n}\nprintln(add(5))\nprintln(total)\n";
    assert_eq!(run(src), "6\n5");
}

#[test]
fn variable_assigned_only_on_the_untaken_path_reads_as_null() {
    // documented: a name that was never assigned on this path reads as null (no per-read check)
    assert_eq!(run("fn f(c) {\n  if c { x = 1 }\n  return x\n}\nprintln(f(true))\nprintln(f(false))\n"), "1\nnull");
}
