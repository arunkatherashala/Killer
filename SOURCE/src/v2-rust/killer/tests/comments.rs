use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_cm_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).trim().replace("\r\n", "\n")
}

#[test]
fn trailing_hash_comments_are_ignored() {
    let src = "x = 5   # five\nprintln(x)  # print it\n# a full-line comment\nprintln(x + 1)\n";
    assert_eq!(run(src), "5\n6");
}

#[test]
fn hash_inside_strings_is_kept() {
    let src = "s = \"a # b\"\nprintln(s)\nprintln(\"#tag\")\n";
    assert_eq!(run(src), "a # b\n#tag");
}

#[test]
fn trailing_hash_comments_work_in_functions_and_blocks() {
    let src = "fn f(n) {   # a function\n  t = 0   # accumulator\n  for i in range(n) {   # loop\n    t = t + i   # add\n  }\n  return t  # done\n}\nprintln(f(5))\n";
    assert_eq!(run(src), "10");
}

#[test]
fn slash_and_dash_comments_still_work() {
    let src = "println(1) // slash\nprintln(2) -- dash\n";
    assert_eq!(run(src), "1\n2");
}
