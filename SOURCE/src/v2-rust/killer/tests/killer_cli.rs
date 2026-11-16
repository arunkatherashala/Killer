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

#[test]
fn run_resolves_imports_next_to_the_script_and_names_it_in_errors() {
    let d = Dir::new();
    std::fs::create_dir_all(d.0.join("sub")).unwrap();
    d.write("sub/helper.killer", "fn five() {\n  return 5\n}\n");
    d.write("sub/main.killer", "import \"helper.killer\"\nprintln(five())\nprintln(nope)\n");
    let (code, out, err) = d.killer(&["run", "sub/main.killer"]);
    assert_eq!(code, 1);
    assert_eq!(out.trim(), "5");
    assert!(err.contains("main.killer:3"), "{err}");
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t);
        } else if p.extension().map_or(false, |x| x == "killer") {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

fn comment_lines(dir: &std::path::Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            comment_lines(&p, out);
        } else if let Ok(s) = std::fs::read_to_string(&p) {
            for l in s.lines() {
                let t = l.trim();
                if t.starts_with('#') || t.starts_with("//") || t.starts_with("--") {
                    out.push(t.to_string());
                }
            }
        }
    }
}

#[test]
fn fmt_over_examples_converges_and_check_is_idempotent_and_keeps_comments() {
    let d = Dir::new();
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let work = d.0.join("ex");
    copy_dir(&examples, &work);
    let mut before = Vec::new();
    comment_lines(&work, &mut before);
    assert!(!before.is_empty(), "examples should contain comments");

    let (code, out, err) = d.killer(&["fmt", "ex"]);
    assert_eq!(code, 0, "fmt failed: {out}{err}");
    // after one pass the tree is stable: --check passes and a second pass changes nothing
    let (code, out, err) = d.killer(&["fmt", "--check", "ex"]);
    assert_eq!(code, 0, "fmt --check must pass after formatting: {out}{err}");
    let mut after = Vec::new();
    comment_lines(&work, &mut after);
    assert_eq!(before, after, "formatting must keep every comment line");

    // and --check reports an unformatted file with exit code 1 without touching it
    d.write("ex/messy.killer", "fn f(a) {\nreturn a   \n}\n");
    let (code, out, _) = d.killer(&["fmt", "--check", "ex/messy.killer"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("would reformat"), "{out}");
    assert_eq!(std::fs::read_to_string(d.0.join("ex/messy.killer")).unwrap(), "fn f(a) {\nreturn a   \n}\n");
}

#[test]
fn lint_rule_ids_and_every_suppression_comment_form() {
    let d = Dir::new();
    // each rule fires with its id
    d.write("a.killer", "prnt(1)\n");
    assert!(d.killer(&["lint", "a.killer"]).1.contains("[undefined-function]"));
    d.write("b.killer", "fn f() {\n  unused_thing = 1\n  return 2\n}\nprint(f())\n");
    assert!(d.killer(&["lint", "b.killer"]).1.contains("[unused-variable]"));
    d.write("c.killer", "fn f() {\n  return 1\n  print(2)\n}\nprint(f())\n");
    assert!(d.killer(&["lint", "c.killer"]).1.contains("[unreachable-code]"));
    d.write("e.killer", "len = 3\nprint(len)\n");
    assert!(d.killer(&["lint", "e.killer"]).1.contains("[shadowed-builtin]"));

    // suppression with `#`, `//` and `--` comment markers, same line and line above, and whole file
    for (i, src) in [
        "prnt(1) # lint:ignore undefined-function\n",
        "prnt(1) // lint:ignore undefined-function\n",
        "prnt(1) -- lint:ignore undefined-function\n",
        "// lint:ignore undefined-function\nprnt(1)\n",
        "-- lint:ignore-file undefined-function\nprnt(1)\nprnt(2)\n",
        "# lint:ignore\nprnt(1)\n",
    ]
    .iter()
    .enumerate()
    {
        let name = format!("s{i}.killer");
        d.write(&name, src);
        let (code, out, _) = d.killer(&["lint", &name]);
        assert_eq!(code, 0, "suppression form {src:?} should silence the rule: {out}");
    }
    // suppressing another rule id does not silence this one
    d.write("t.killer", "prnt(1) # lint:ignore unused-variable\n");
    assert_eq!(d.killer(&["lint", "t.killer"]).0, 1);
}
