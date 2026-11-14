//! The eval memory cap through the real CLI binaries, which declare `memcap::CountingAlloc` themselves (or get it
//! from the `mem-cap` feature): the live-bytes check must be active there, not just the pre-checks.
use std::process::Command;

fn run(bin: &str, code: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("killer_evalmem_{}_{}", std::process::id(), bin.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.killer");
    std::fs::write(&f, code).unwrap();
    let mut c = Command::new(bin);
    if bin.contains("killer_super") {
        c.arg(&f).arg("--run");
    } else {
        c.arg("run").arg(&f);
    }
    let o = c.output().unwrap();
    (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}

// Many small allocations: no single site is pre-checked, so only the periodic live-bytes check can stop this.
const SMALL_ALLOCS: &str = r#"try {
  exec("l = []\ni = 0\nwhile i < 40000000 {\n  l.push('item' + str(i))\n  i = i + 1\n}\nlen(l)", null, {"max_memory_mb": 8})
} catch e {
  println(e)
}
"#;
const HUGE_REPEAT: &str = r#"try {
  eval("len('a' * 100000000000)")
} catch e {
  println(e)
}
"#;

fn check(bin: &str) {
    let (o, e) = run(bin, SMALL_ALLOCS);
    assert!(format!("{o}{e}").contains("memory limit exceeded (8 MB)"), "{bin}: {o} {e}");
    let (o, e) = run(bin, HUGE_REPEAT);
    assert!(format!("{o}{e}").contains("too large") || format!("{o}{e}").contains("limit"), "{bin}: {o} {e}");
}

#[test]
fn killer_super_enforces_the_live_bytes_cap() {
    check(env!("CARGO_BIN_EXE_killer_super"));
}

#[test]
fn killer_enforces_the_live_bytes_cap() {
    check(env!("CARGO_BIN_EXE_killer"));
}
