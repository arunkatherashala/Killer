//! Regression tests for front-end (parser / compiler / typecheck) bugs.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_ff_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn grouping_paren_ignores_parens_in_strings() {
    assert_eq!(run("s = \"z\"\nx = (\"(\" + s)\ny = (\")\" + s)\nprintln(x)\nprintln(y)\n"), "(z\n)z");
}

#[test]
fn escaped_quote_does_not_break_comment_stripping() {
    assert_eq!(run("println(\"\\\"http://x\")\n"), "\"http://x");
    assert_eq!(run("fn main() {\n    println(\"\\\"http://x\")\n}\nmain()\n"), "\"http://x");
}

#[test]
fn inf_and_nan_are_ordinary_variable_names() {
    assert_eq!(run("inf = 5\nprint(inf)\nnan = 7\nprintln(nan + 1)\nprintln(INF > 1000)\n"), "5\n8\ntrue");
}

#[test]
fn dash_comment_after_floor_division_text_is_ignored() {
    assert_eq!(run("lo = 2\nhi = 8\n-- mid = (lo + hi) // 2\nprintln(lo + hi)\n"), "10");
}

#[test]
fn import_inside_string_or_polyglot_block_is_not_resolved() {
    let out = run("t = \"\"\"a\nimport nonexistent_pkg_zz\nb\"\"\"\nprintln(len(t) > 0)\n");
    assert_eq!(out, "true");
    let out = run("@python {\nimport nonexistent_pkg_zz\n}\nprintln(1)\n");
    assert!(!out.contains("cannot find import"), "{out}");
    // with python installed the block really ran: python itself reports the missing module
    if run("println(polyglot_check(\"python\"))") == "true" {
        assert!(out.contains("ModuleNotFoundError") && out.contains("POLYGLOT_E_EXIT"), "{out}");
    }
}

#[test]
fn typecheck_ternary_does_not_report_false_errors() {
    let src = "fn f(n: number) -> number { return n <= 1 ? n : f(n - 1) }\nn = 3\ns: string = n > 0 ? \"a\" : \"b\"\nprintln(f(3))\nprintln(s)\n";
    assert_eq!(run(src), "1\na");
}

#[test]
fn typecheck_class_method_does_not_shadow_builtin_arity() {
    let src = "class Box {\n  fn push(item: number) {\n    this.v = item\n  }\n}\nxs = [1]\npush(xs, 2)\nprintln(len(xs))\n";
    assert_eq!(run(src), "2");
}

#[test]
fn nested_functions_with_the_same_name_in_two_outers() {
    let src = "fn a() {\n  fn helper() {\n    return 1\n  }\n  return helper() + 10\n}\nfn b() {\n  fn helper() {\n    return 2\n  }\n  return helper() + 20\n}\nprintln(a())\nprintln(b())\n";
    assert_eq!(run(src), "11\n22");
}

#[test]
fn if_as_last_statement_returns_branch_value() {
    let src = "kfn fib(n) { if n <= 1 { n } else { fib(n - 1) + fib(n - 2) } }\nprintln(fib(10))\n\
               fn sign(x) {\n  if x < 0 {\n    \"neg\"\n  } else if x == 0 {\n    \"zero\"\n  } else {\n    \"pos\"\n  }\n}\n\
               println(sign(-3))\nprintln(sign(0))\nprintln(sign(5))\n\
               fn nested(a, b) {\n  if a {\n    if b { 1 } else { 2 }\n  } else {\n    3\n  }\n}\n\
               println(nested(true, false))\nprintln(nested(false, false))\n";
    assert_eq!(run(src), "55\nneg\nzero\npos\n2\n3");
}

#[test]
fn if_tail_value_keeps_other_behavior() {
    // no final else: still null; branch ending in an assignment: null; explicit return: unchanged;
    // an `if` that is not the last statement: its value is discarded
    let src = "fn a(x) {\n  if x { 1 }\n}\nprintln(a(false))\n\
               fn b(x) {\n  y = 0\n  if x { y = 5 } else { y = 6 }\n}\nprintln(b(true))\n\
               fn c(x) {\n  if x { return 7 } else { return 8 }\n}\nprintln(c(true))\nprintln(c(false))\n\
               fn d(x) {\n  if x { 1 } else { 2 }\n  3\n}\nprintln(d(true))\n";
    assert_eq!(run(src), "null\nnull\n7\n8\n3");
}

#[test]
fn optional_chaining_chains_left_to_right() {
    let src = "d = {\"b\": {\"c\": 1}}\nn = null\nprintln(d?.b?.c)\nprintln(d?.b.c)\nprintln(n?.b?.c)\nprintln(n?.b.c)\nprintln(\"abc\"?.upper())\n\
               fn f(o) {\n  return o?.b?.c\n}\nprintln(f(d))\nprintln(f(null))\n";
    assert_eq!(run(src), "1\n1\nnull\nnull\nABC\n1\nnull");
}

#[test]
fn compound_assignment_and_chained_comparison_evaluate_once() {
    let src = "counter = [0]\nfn next() {\n  counter[0] = counter[0] + 1\n  return 0\n}\n\
               xs = [10, 20]\nxs[next()] += 5\nprintln(xs[0])\nprintln(counter[0])\n\
               xs[next()]++\nprintln(xs[0])\nprintln(counter[0])\n\
               fn f() {\n  counter[0] = counter[0] + 1\n  return 5\n}\n\
               counter[0] = 0\nprintln(0 < f() < 10)\nprintln(counter[0])\n\
               counter[0] = 0\nprintln(10 < f() < 20)\nprintln(counter[0])\n\
               counter[0] = 0\nprintln(0 < f() < 10 < 3 < f())\nprintln(counter[0])\n\
               println(1 in [1, 2] == true)\n";
    assert_eq!(run(src), "15\n1\n16\n2\ntrue\n1\nfalse\n1\nfalse\n1\ntrue");
}

#[test]
fn switch_empty_case_warns_and_does_not_fall_through() {
    let src = "x = 1\nswitch x {\n  case 1:\n  case 2:\n    println(\"two\")\n}\nswitch 2 {\n  case 1, 2:\n    println(\"shared\")\n}\n";
    let out = run(src);
    assert!(out.contains("warning") && out.contains("no fall-through"), "{out}");
    assert!(!out.contains("two"), "{out}");
    assert!(out.contains("shared"), "{out}");
}

#[test]
fn nested_function_and_lambda_values_survive_scoped_registration() {
    // Nested functions are registered as `outer/inner` and under the plain name; both entries share
    // one start, so a function-name table that kept the scoped spelling lost the value depending on
    // hash-iteration order. Run several times (each process has its own hash seed).
    let src = "fn outer() {\n  fn helper(x) {\n    return x + 1\n  }\n  return map([1, 2], helper)\n}\n\
               fn other() {\n  return map([1, 2], x => x * 10)\n}\n\
               println(outer())\nprintln(other())\n";
    for _ in 0..8 {
        assert_eq!(run(src), "[2, 3]\n[10, 20]");
    }
}
