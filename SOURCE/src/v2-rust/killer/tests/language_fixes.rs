//! Regression tests for language bugs and gaps fixed in batch F3a.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_lf_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn zero_bigint_is_falsy_everywhere() {
    let src = "z = 0n\nw = 5n\nprintln(bool(z))\nprintln(bool(w))\nprintln(not z)\nprintln(z && 1)\nprintln(z || 7)\nprintln(1 if z else 2)\nif z { println(\"t\") } else { println(\"f\") }\nprintln(len(filter([0n, 1n, 2n], fn(v) { return v })))\nprintln(any([0n, 0]))\nprintln(all([1n, 0n]))\nprintln(Boolean(z))\n";
    assert_eq!(run(src), "false\ntrue\ntrue\nfalse\ntrue\n2\nf\n2\nfalse\nfalse\nfalse");
}
