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
}
