//! Package manager and module resolution tests. Everything happens in temporary directories and
//! drives the real binaries (`killer_super` to run scripts, `killer_pkg` for the package manager).
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let d = std::env::temp_dir().join(format!("killer_pk_{}_{}_{}", std::process::id(), tag, NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
    fn write(&self, rel: &str, text: &str) -> &Tmp {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
        self
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn norm(out: &std::process::Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

/// Run a script with killer_super from `cwd`; returns combined output.
fn run_script(script: &Path, cwd: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(script).arg("--run").current_dir(cwd).output().unwrap();
    norm(&out)
}

#[allow(dead_code)]
fn pkg(cwd: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_killer_pkg")).args(args).current_dir(cwd).output().unwrap();
    (out.status.success(), norm(&out))
}

// ------------------------------------------------------------ module resolution

#[test]
fn bare_import_resolves_an_installed_package_by_its_entry() {
    let t = Tmp::new("bare");
    t.write("killer.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n");
    t.write(".killer/packages/greeter/killer.toml", "[package]\nname = \"greeter\"\nversion = \"1.0.0\"\nentry = \"src/lib.killer\"\n");
    t.write(".killer/packages/greeter/src/lib.killer", "fn greet(n) {\n  return \"hi \" + n\n}\n");
    t.write("src/main.killer", "import greeter\nprintln(greet(\"sai\"))\n");
    assert_eq!(run_script(&t.path("src/main.killer"), &t.0), "hi sai");
}

#[test]
fn quoted_subpath_import_and_packages_dir_and_include_once() {
    let t = Tmp::new("sub");
    t.write("packages/util/killer.toml", "[package]\nname = \"util\"\nversion = \"1.0.0\"\nentry = \"lib.killer\"\n");
    t.write("packages/util/lib.killer", "fn base() {\n  return 40\n}\n");
    t.write("packages/util/extra/more.killer", "fn more() {\n  return 2\n}\n");
    t.write("main.killer", "import util\nimport \"util/extra/more.killer\"\nimport util\nprintln(base() + more())\n");
    assert_eq!(run_script(&t.path("main.killer"), &t.0), "42");
}

#[test]
fn path_dependency_from_the_manifest_resolves_without_install() {
    let t = Tmp::new("pathdep");
    t.write("libb/killer.toml", "[package]\nname = \"libb\"\nversion = \"0.2.0\"\nentry = \"b.killer\"\n");
    t.write("libb/b.killer", "fn bee() {\n  return \"bzz\"\n}\n");
    t.write("app/killer.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nlibb = { path = \"../libb\" }\n");
    t.write("app/main.killer", "import libb\nprintln(bee())\n");
    assert_eq!(run_script(&t.path("app/main.killer"), &t.path("app")), "bzz");
}

#[test]
fn missing_import_is_a_compile_error_naming_the_import_and_where_it_looked() {
    let t = Tmp::new("missing");
    t.write("killer.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n");
    t.write("main.killer", "println(\"before\")\nimport nonexistent_package\nprintln(\"after\")\n");
    let out = run_script(&t.path("main.killer"), &t.0);
    assert!(out.contains("cannot find import nonexistent_package"), "{out}");
    assert!(out.contains("main.killer:2"), "{out}");
    assert!(out.contains(".killer/packages/nonexistent_package"), "{out}");
    assert!(!out.contains("before") && !out.contains("after"), "nothing may run: {out}");
    let t2 = Tmp::new("missing_q");
    t2.write("main.killer", "import \"nope/thing.killer\"\nprintln(1)\n");
    let out = run_script(&t2.path("main.killer"), &t2.0);
    assert!(out.contains("cannot find import \"nope/thing.killer\""), "{out}");
}

#[test]
fn legacy_standard_library_names_still_compile() {
    let t = Tmp::new("legacy");
    t.write("main.killer", "import math\nprintln(sqrt(16))\n");
    assert_eq!(run_script(&t.path("main.killer"), &t.0), "4");
}

#[test]
fn errors_inside_a_package_point_at_the_package_file_and_line() {
    let t = Tmp::new("srcmap");
    t.write("killer.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n");
    t.write(".killer/packages/broken/killer.toml", "[package]\nname = \"broken\"\nversion = \"1.0.0\"\nentry = \"src/lib.killer\"\n");
    t.write(".killer/packages/broken/src/lib.killer", "fn ok() {\n  return 1\n}\nfn boom() {\n  return undefined_thing + 1\n}\n");
    t.write("main.killer", "import broken\nprintln(ok())\nprintln(boom())\n");
    let out = run_script(&t.path("main.killer"), &t.0);
    assert!(out.contains(".killer/packages/broken/src/lib.killer"), "{out}");
    assert!(out.contains(":5") || out.contains("line 5"), "{out}");
}
