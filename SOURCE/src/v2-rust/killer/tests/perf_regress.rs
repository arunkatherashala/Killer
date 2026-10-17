//! Regression tests for the string-indexing and global-lookup performance fixes.
//! They assert behaviour and use generous time limits (not exact timings).
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_pr_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

fn timed(src: &str, limit_secs: u64) -> String {
    let t = Instant::now();
    let out = run(src);
    assert!(t.elapsed() < Duration::from_secs(limit_secs), "took {:?}", t.elapsed());
    out
}

#[test]
fn string_index_semantics_ascii_and_unicode() {
    let src = "s = \"hello\"\nu = \"h\u{e9}llo w\u{f6}rld\"\nprintln(s[0])\nprintln(s[-1])\nprintln(s[5])\nprintln(s[-6])\nprintln(len(s))\nprintln(u[1])\nprintln(u[-4])\nprintln(u[9])\nprintln(u[10])\nprintln(u[11])\nprintln(len(u))\nprintln(u[1:4])\nprintln(u[7:100])\n";
    assert_eq!(run(src), "h\no\nnull\nnull\n5\n\u{e9}\n\u{f6}\nl\nd\nnull\n11\n\u{e9}ll\n\u{f6}rld");
}

#[test]
fn string_index_in_functions_slots_and_params() {
    let src = "g = \"ab\u{e9}d\"\nfn f(t) {\n  return t[2] + str(len(t))\n}\nfn h() {\n  loc = \"x\u{e9}yz\"\n  i = 1\n  return loc[i] + loc[-1] + str(len(loc))\n}\nfn k() {\n  return g[2] + str(len(g))\n}\nprintln(f(g))\nprintln(h())\nprintln(k())\nprintln(g[2])\n";
    assert_eq!(run(src), "\u{e9}4\n\u{e9}z4\n\u{e9}4\n\u{e9}");
}

#[test]
fn long_ascii_loop_with_len_and_index() {
    let src = "s = \"abcdefghijklmnopqrstuvwxyz\"\nfor k in range(11) {\n  s = s + s\n}\nc = 0\ni = 0\nwhile i < len(s) {\n  if s[i] == \"e\" {\n    c = c + 1\n  }\n  i = i + 1\n}\nprintln(len(s))\nprintln(c)\n";
    assert_eq!(timed(src, 120), "53248\n2048");
}

#[test]
fn long_unicode_loop_with_len_and_index() {
    let src = "u = \"abcdefghijklmnopqrstuvwxyz\"\nfor k in range(10) {\n  u = u + u\n}\nt = \"\u{e9}\" + u\nc = 0\ni = 0\nwhile i < len(t) {\n  if t[i] == \"e\" {\n    c = c + 1\n  }\n  i = i + 1\n}\nprintln(len(t))\nprintln(c)\nprintln(t[0])\n";
    assert_eq!(timed(src, 120), "26625\n1024\n\u{e9}");
}

#[test]
fn deep_recursion_reading_a_global_is_linear() {
    let src = "step = 1\nfn deep(n) {\n  if n == 0 {\n    return 0\n  }\n  return step + deep(n - 1)\n}\nprintln(deep(10000))\n";
    assert_eq!(timed(src, 60), "10000");
}

#[test]
fn function_does_not_see_callers_locals() {
    let src = "fn inner() {\n  return secret\n}\nfn outer() {\n  secret = 42\n  return inner()\n}\ntry {\n  println(outer())\n} catch e {\n  println(\"undefined\")\n}\n";
    assert_eq!(run(src), "undefined");
}

#[test]
fn globals_closures_generators_and_try_still_resolve() {
    let src = "base = 100\nfn make(n) {\n  fn add(x) {\n    return x + n + base\n  }\n  return add\n}\nf = make(5)\nprintln(f(1))\nfn gen() {\n  for i in range(3) {\n    yield i + base\n  }\n}\nfor v in gen() {\n  println(v)\n}\nfn thrower() {\n  throw \"boom\"\n}\nfn safe() {\n  try {\n    thrower()\n  } catch e {\n    return base + 1\n  }\n}\nprintln(safe())\nprintln(map([1, 2], fn(x) { return x + base }))\nfn counter() {\n  total = 0\n  fn bump() {\n    total = total + 1\n  }\n  bump()\n  bump()\n  return total\n}\nprintln(counter())\n";
    assert_eq!(run(src), "106\n100\n101\n102\n101\n[101, 102]\n2");
}
