//! Real-application trial: the five example apps under examples/apps/ run through the `killer`
//! driver (`killer test`, `killer run`), plus regression tests for every bug the trial found
//! (see examples/apps/FRICTION.md).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn norm(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace("\r\n", "\n")
}

fn killer(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_killer")).current_dir(cwd).args(args).output().unwrap()
}

fn killer_pkg(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_killer_pkg")).current_dir(cwd).args(args).output().unwrap()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("killer_apps_{}_{}_{}", tag, std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let f = self.0.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, text).unwrap();
        f
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app_dir(name: &str) -> PathBuf {
    crate_dir().join("examples/apps").join(name)
}

/// Run `killer test` inside an app and return (success, stdout).
fn app_tests(name: &str) -> (bool, String) {
    let out = killer(&app_dir(name), &["test"]);
    (out.status.success(), norm(&out.stdout) + &norm(&out.stderr))
}

// ---------------------------------------------------------------------------------------------
// regression tests for bugs found by the trial
// ---------------------------------------------------------------------------------------------

#[test]
fn fstring_escapes_and_literal_braces() {
    let t = Tmp::new("fstr");
    let f = t.write(
        "a.killer",
        r#"x = 5
println(f"a\nb {x}")
println(f"t\tq \"{x}\" \\")
println(f"{{lit}} {x}")
println(k"k\n{x}")
"#,
    );
    let out = killer(&t.0, &["run", f.to_str().unwrap()]);
    assert_eq!(norm(&out.stdout), "a\nb 5\nt\tq \"5\" \\\n{lit} 5\nk\n5\n", "{}", norm(&out.stderr));
}

#[test]
fn killer_run_resolves_imports_relative_to_the_script() {
    let t = Tmp::new("imp");
    t.write("src/helper.killer", "fn helper_twice(x) {\n  return x * 2\n}\n");
    t.write("src/main.killer", "import \"helper.killer\"\nprintln(helper_twice(21))\n");
    let elsewhere = Tmp::new("cwd");
    let out = killer(&elsewhere.0, &["run", t.0.join("src/main.killer").to_str().unwrap()]);
    assert_eq!(norm(&out.stdout).trim(), "42", "{}", norm(&out.stderr));
}

#[test]
fn killer_test_resolves_imports_relative_to_the_test_file() {
    let t = Tmp::new("timp");
    t.write("src/lib.killer", "fn lib_inc(x) {\n  return x + 1\n}\n");
    t.write("tests/test_lib.killer", "import \"../src/lib.killer\"\ntest \"inc\" {\n  assert_eq(lib_inc(1), 2)\n}\n");
    let out = killer(&t.0, &["test"]);
    assert!(out.status.success(), "{}{}", norm(&out.stdout), norm(&out.stderr));
    assert!(norm(&out.stdout).contains("ok"), "{}", norm(&out.stdout));
}

#[test]
fn scaffolded_project_tests_are_discovered_by_killer_test() {
    let t = Tmp::new("scaf");
    assert!(killer_pkg(&t.0, &["new", "demo"]).status.success());
    let out = killer(&t.0.join("demo"), &["test"]);
    let text = norm(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("arithmetic works"), "{text}");
}

#[test]
fn killer_test_finds_test_blocks_in_any_file_of_a_tests_folder() {
    let t = Tmp::new("tdir");
    t.write("tests/checks.killer", "test \"one\" {\n  assert_eq(1, 1)\n}\n");
    t.write("tests/helper.killer", "fn helper() {\n  return 1\n}\n");
    let out = killer(&t.0, &["test"]);
    let text = norm(&out.stdout);
    assert!(text.contains("checks.killer::one"), "{text}");
    assert!(!text.contains("helper.killer"), "{text}");
}

#[test]
fn eprintln_writes_to_stderr_only() {
    let t = Tmp::new("eprint");
    let f = t.write("e.killer", "println(\"out\")\neprintln(\"bad\", 1, [2])\neprint(\"x\")\n");
    let out = killer(&t.0, &["run", f.to_str().unwrap()]);
    assert_eq!(norm(&out.stdout).trim(), "out");
    assert_eq!(norm(&out.stderr).trim(), "bad 1 [2]\nx");
}
