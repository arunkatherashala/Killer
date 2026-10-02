use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("killer_append_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&path)
        .arg("--run")
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).trim().replace("\r\n", "\n")
}

#[test]
fn appends_literals_and_str_calls() {
    let src = "fn f() {\n  s = \"\"\n  for i in range(5) {\n    s = s + \"<\" + str(i) + \">\"\n  }\n  return s\n}\nprintln(f())\n";
    assert_eq!(run(src), "<0><1><2><3><4>");
}

#[test]
fn appends_at_top_level_and_with_quotes_and_parens() {
    let src = "s = \"a\"\ns = s + \"say \\\"hi\\\"\" + str((1 + 2) * 3)\nprintln(s)\n";
    assert_eq!(run(src), "asay \"hi\"9");
}

#[test]
fn non_string_slot_uses_the_generic_add() {
    // slot holds a number: AppendSlot must fall back to the normal Add semantics
    let a = run("fn f() {\n  x = 5\n  x = x + \"a\"\n  return x\n}\nprintln(f())\n");
    let b = run("fn g() {\n  y = 5\n  z = y + \"a\"\n  return z\n}\nprintln(g())\n");
    assert_eq!(a, b);
}

#[test]
fn mixed_precedence_is_untouched() {
    // `s + a - b` must not be split into appends; numeric accumulation is unchanged too
    let src = "fn f() {\n  t = 0\n  for i in range(4) {\n    t = t + i * 2\n  }\n  n = 10\n  n = n + 5 - 3\n  return str(t) + \"/\" + str(n)\n}\nprintln(f())\n";
    assert_eq!(run(src), "12/12");
}

#[test]
fn building_does_not_alias() {
    let src = "fn f() {\n  a = \"x\"\n  b = a\n  a = a + \"y\"\n  return b + \"|\" + a\n}\nprintln(f())\n";
    assert_eq!(run(src), "x|xy");
}
