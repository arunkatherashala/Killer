//! End-to-end checks of the `killer` driver: run / fmt / lint / test / version.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Dir(PathBuf);

impl Dir {
    fn new() -> Dir {
        let d = std::env::temp_dir().join(format!("killer_cli_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }
    fn killer(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_killer")).args(args).current_dir(&self.0).output().unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
            String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"),
        )
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn version_and_help() {
    let d = Dir::new();
    let (code, out, _) = d.killer(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), format!("killer {}", env!("CARGO_PKG_VERSION")));
    let (code, out, _) = d.killer(&["--help"]);
    assert_eq!(code, 0);
    for c in ["run", "fmt", "lint", "test", "repl", "lsp"] {
        assert!(out.contains(c), "help lacks {c}");
    }
    assert_eq!(d.killer(&["bogus"]).0, 2);
}

#[test]
fn run_program_and_errors() {
    let d = Dir::new();
    d.write("hello.killer", "print(\"hi \" + str(1 + 2))\n");
    let (code, out, _) = d.killer(&["run", "hello.killer"]);
    assert_eq!((code, out.trim()), (0, "hi 3"));
    let (code, out, _) = d.killer(&["hello.killer"]);
    assert_eq!((code, out.trim()), (0, "hi 3"));
    d.write("bad.killer", "x = 1\nprint(\n");
    assert_eq!(d.killer(&["run", "bad.killer"]).0, 1);
}

#[test]
fn fmt_check_then_write() {
    let d = Dir::new();
    let f = d.write("a.killer", "fn f() {\nreturn 1   \n}\n\n\n\nprint(f())\n");
    let (code, out, _) = d.killer(&["fmt", "--check", "a.killer"]);
    assert_eq!(code, 1);
    assert!(out.contains("would reformat"));
    assert_eq!(d.killer(&["fmt", "a.killer"]).0, 0);
    assert_eq!(std::fs::read_to_string(&f).unwrap().replace("\r\n", "\n"), "fn f() {\n  return 1\n}\n\nprint(f())\n");
    assert_eq!(d.killer(&["fmt", "--check", "a.killer"]).0, 0);
    // formatted program still runs the same
    assert_eq!(d.killer(&["run", "a.killer"]).1.trim(), "1");
}

#[test]
fn lint_reports_rules_and_exit_codes() {
    let d = Dir::new();
    d.write("l.killer", "prnt(1)\nfn f() {\n  return 1\n  print(2)\n}\nprint(f())\n");
    let (code, out, _) = d.killer(&["lint", "l.killer"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("l.killer:1:1: error [undefined-function]"), "{out}");
    assert!(out.contains("[unreachable-code]"), "{out}");
    d.write("m.killer", "# lint:ignore undefined-function\nprnt(1)\n");
    assert_eq!(d.killer(&["lint", "m.killer"]).0, 0);
    assert_eq!(d.killer(&["lint", "--allow", "undefined-function", "l.killer"]).0, 0);
}

#[test]
fn test_command_runs_blocks_and_writes_junit() {
    let d = Dir::new();
    d.write(
        "math_test.killer",
        "fn double(x) {\n  return x * 2\n}\n\nfn thrower() {\n  throw \"boom\"\n}\n\ntest \"doubles\" {\n  assert_eq(double(2), 4)\n}\n\ntest \"fails with diff\" {\n  assert_eq([1, 2, 3], [1, 2, 4])\n}\n\ntest \"throws\" {\n  assert_throws(thrower, \"boom\")\n}\n\nfn test_plain_function() {\n  assert_eq(double(5), 10)\n}\n",
    );
    d.write("test_legacy.killer", "x = 1\nassert(x == 1)\nprint(\"legacy ok\")\n");
    let (code, out, _) = d.killer(&["test", "--junit", "out.xml"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("ok    ./math_test.killer::doubles") || out.contains("math_test.killer::doubles"), "{out}");
    assert!(out.contains("FAIL") && out.contains("fails with diff"), "{out}");
    assert!(out.contains("index 2: left 3, right 4"), "{out}");
    assert!(out.contains("legacy ok"), "legacy files still run whole: {out}");
    assert!(out.contains("3 failed") == false && out.contains("1 failed"), "{out}");
    let xml = std::fs::read_to_string(d.0.join("out.xml")).unwrap();
    assert!(xml.contains("<testsuites tests=\"5\" failures=\"1\""), "{xml}");
    assert!(xml.contains("<failure message=\"assert_eq failed\">"), "{xml}");

    // filtering by name
    let (code, out, _) = d.killer(&["test", "doubles"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 passed, 0 failed, 1 total"), "{out}");
    // blocks are inert in a normal run
    assert_eq!(d.killer(&["run", "math_test.killer"]).0, 0);
}
