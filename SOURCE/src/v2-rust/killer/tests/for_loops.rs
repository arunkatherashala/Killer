use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_for_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).trim().replace("\r\n", "\n")
}

#[test]
fn continue_and_break_work_in_for_loops() {
    let src = "fn f() {\n  out = \"\"\n  for i in range(8) {\n    if i == 2 {\n      continue\n    }\n    if i == 6 {\n      break\n    }\n    out = out + str(i) + \",\"\n  }\n  return out\n}\nprintln(f())\n";
    assert_eq!(run(src), "0,1,3,4,5,");
}

#[test]
fn continue_works_at_top_level_over_arrays() {
    let src = "total = 0\nfor x in [10, 20, 30, 40] {\n  if x == 20 {\n    continue\n  }\n  total = total + x\n}\nprintln(total)\n";
    assert_eq!(run(src), "80");
}

#[test]
fn nested_loops_and_empty_iterables() {
    let src = "fn f() {\n  n = 0\n  for i in range(4) {\n    for j in range(i) {\n      n = n + 1\n    }\n  }\n  for k in [] {\n    n = n + 100\n  }\n  for k in range(0) {\n    n = n + 100\n  }\n  return n\n}\nprintln(f())\n";
    assert_eq!(run(src), "6");
}

#[test]
fn loop_variable_stays_visible_and_arrays_of_strings_work() {
    let src = "words = [\"a\", \"b\", \"c\"]\nout = \"\"\nfor w in words {\n  out = out + w + \"-\"\n}\nprintln(out)\nprintln(w)\n";
    assert_eq!(run(src), "a-b-c-\nc");
}

#[test]
fn push_while_iterating_sees_live_length() {
    // the iterator checks the current length on every step, like the previous implementation
    let src = "a = [1, 2]\nn = 0\nfor x in a {\n  n = n + 1\n  if n < 5 {\n    push(a, x)\n  }\n}\nprintln(n)\nprintln(len(a))\n";
    assert_eq!(run(src), "6\n6");
}

#[test]
fn for_over_array_args_in_hot_native_function_matches_interpreter() {
    // called enough times to be compiled natively; the result must equal the interpreted one
    let src = "fn total(a) {\n  s = 0\n  for x in a {\n    if x % 2 == 0 {\n      continue\n    }\n    s = s + x\n  }\n  return s\n}\nfn main() {\n  data = [1, 2, 3, 4, 5, 6, 7]\n  r = 0\n  for i in range(100) {\n    r = r + total(data)\n  }\n  return r\n}\nprintln(main())\n";
    assert_eq!(run(src), "1600");
}
