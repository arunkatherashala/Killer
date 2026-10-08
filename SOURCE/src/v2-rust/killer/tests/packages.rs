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

// ------------------------------------------------------------ package manager CLI, end to end

fn file_url(p: &Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/'))
}

fn ok(cwd: &Path, args: &[&str]) -> String {
    let (good, out) = pkg(cwd, args);
    assert!(good, "killer_pkg {:?} failed: {}", args, out);
    out
}

fn fail(cwd: &Path, args: &[&str]) -> String {
    let (good, out) = pkg(cwd, args);
    assert!(!good, "killer_pkg {:?} should have failed: {}", args, out);
    out
}

/// Create library `name` at `version` in `ws/<name>` with the given source and publish it.
fn publish_lib(ws: &Tmp, reg: &str, name: &str, version: &str, deps: &str, src: &str) {
    if !ws.path(name).exists() {
        ok(&ws.0, &["new", name, "--lib"]);
    }
    ws.write(&format!("{name}/killer.toml"), &format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nentry = \"src/lib.killer\"\n\n[dependencies]\n{deps}"));
    ws.write(&format!("{name}/src/lib.killer"), src);
    ok(&ws.0, &["-C", name, "publish", "--registry", reg]);
}

const LIB_A_1_0: &str = "fn lib_a_add(a, b) {\n  return a + b\n}\n";
const LIB_A_1_1: &str = "fn lib_a_add(a, b) {\n  return a + b\n}\nfn lib_a_mul(a, b) {\n  return a * b\n}\n";

#[test]
fn new_scaffolds_a_project() {
    let ws = Tmp::new("new");
    ok(&ws.0, &["new", "my-app"]);
    for f in ["killer.toml", "src/main.killer", "tests/basic.killer", ".gitignore"] {
        assert!(ws.path(&format!("my-app/{f}")).is_file(), "missing {f}");
    }
    let toml = std::fs::read_to_string(ws.path("my-app/killer.toml")).unwrap();
    assert!(toml.contains("name = \"my-app\"") && toml.contains("[dependencies]"), "{toml}");
    assert!(std::fs::read_to_string(ws.path("my-app/.gitignore")).unwrap().contains(".killer/"));
    assert!(fail(&ws.0, &["new", "my-app"]).contains("already exists"));
    assert!(fail(&ws.0, &["new", "../evil"]).contains("invalid package name"));
    // the scaffolded app runs
    assert_eq!(run_script(&ws.path("my-app/src/main.killer"), &ws.path("my-app")), "Hello from my-app");
    // init in an existing empty folder
    std::fs::create_dir_all(ws.path("existing")).unwrap();
    ok(&ws.path("existing"), &["init", "--name", "tool", "--lib"]);
    assert!(ws.path("existing/src/lib.killer").is_file());
    assert!(fail(&ws.path("existing"), &["init"]).contains("already exists"));
}

#[test]
fn publish_add_install_run_check_and_update_end_to_end() {
    let ws = Tmp::new("e2e");
    let reg_dir = ws.path("registry");
    let reg = file_url(&reg_dir);
    publish_lib(&ws, &reg, "libA", "1.0.0", "", LIB_A_1_0);
    publish_lib(&ws, &reg, "libA", "1.1.0", "", LIB_A_1_1);
    assert!(reg_dir.join("libA/1.1.0/package.pack").is_file() && reg_dir.join("libA/1.1.0/sha256").is_file() && reg_dir.join("libA/1.0.0/killer.toml").is_file());
    // releases are immutable
    assert!(fail(&ws.0, &["-C", "libA", "publish", "--registry", &reg]).contains("already published"));

    ok(&ws.0, &["new", "app"]);
    let out = ok(&ws.0, &["-C", "app", "add", "libA@^1.0", "--registry", &reg]);
    assert!(out.contains("libA"), "{out}");
    assert!(std::fs::read_to_string(ws.path("app/killer.toml")).unwrap().contains("libA = \"^1.0\""));
    let out = ok(&ws.0, &["-C", "app", "install", "--registry", &reg]);
    assert!(out.contains("libA 1.1.0"), "{out}");

    let lock = std::fs::read_to_string(ws.path("app/killer.lock")).unwrap();
    let hash = std::fs::read_to_string(reg_dir.join("libA/1.1.0/sha256")).unwrap();
    assert!(lock.contains("name = \"libA\"") && lock.contains("version = \"1.1.0\"") && lock.contains("registry+file:///"), "{lock}");
    assert!(lock.contains(hash.trim()), "lock must record the sha256 {hash}: {lock}");
    assert!(ws.path("app/.killer/packages/libA/src/lib.killer").is_file());

    // run the app and call the package function by name
    ws.write("app/src/main.killer", "import libA\nprintln(lib_a_mul(6, 7))\nprintln(lib_a_add(1, 2))\n");
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.path("app")), "42\n3");
    // running from a different directory works too
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.0), "42\n3");

    assert!(ok(&ws.path("app"), &["check"]).contains("ok"));
    let listing = ok(&ws.path("app"), &["list"]);
    assert!(listing.contains("libA") && listing.contains("1.1.0"), "{listing}");

    // install keeps the locked version; update moves to the newest allowed one
    publish_lib(&ws, &reg, "libA", "1.2.0", "", LIB_A_1_1);
    let out = ok(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("libA 1.1.0"), "{out}");
    let out = ok(&ws.path("app"), &["update", "--registry", &reg]);
    assert!(out.contains("libA 1.2.0"), "{out}");
    publish_lib(&ws, &reg, "libA", "2.0.0", "", LIB_A_1_1);
    let out = ok(&ws.path("app"), &["update", "--registry", &reg]);
    assert!(out.contains("libA 1.2.0"), "^1.0 must not jump to 2.0.0: {out}");

    // tampering with the installed file is caught by check
    ws.write("app/.killer/packages/libA/src/lib.killer", "fn lib_a_mul(a, b) {\n  return 0\n}\n");
    let out = fail(&ws.path("app"), &["check"]);
    assert!(out.contains("checksum mismatch") && out.contains("libA"), "{out}");
    // reinstall repairs it
    ok(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(ok(&ws.path("app"), &["check"]).contains("ok"));

    // remove drops the dependency and the installed copy
    ok(&ws.path("app"), &["remove", "libA"]);
    ok(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(!ws.path("app/.killer/packages/libA").exists());
    assert!(fail(&ws.path("app"), &["remove", "libA"]).contains("not a dependency"));
}

#[test]
fn registry_from_the_manifest_and_environment() {
    let ws = Tmp::new("regcfg");
    let reg = file_url(&ws.path("registry"));
    publish_lib(&ws, &reg, "libA", "1.0.0", "", LIB_A_1_0);
    ok(&ws.0, &["new", "app"]);
    ws.write("app/killer.toml", &format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[registry]\nurl = \"{reg}\"\n\n[dependencies]\n"));
    ok(&ws.path("app"), &["add", "libA"]); // newest version, caret requirement
    assert!(std::fs::read_to_string(ws.path("app/killer.toml")).unwrap().contains("libA = \"^1.0.0\""));
    ok(&ws.path("app"), &["install"]);
    assert!(ws.path("app/.killer/packages/libA").is_dir());
    // no registry configured anywhere
    ok(&ws.0, &["new", "bare"]);
    ws.write("bare/killer.toml", "[package]\nname = \"bare\"\nversion = \"0.1.0\"\n[dependencies]\nlibA = \"^1\"\n");
    let out = fail(&ws.path("bare"), &["install"]);
    assert!(out.contains("no registry configured"), "{out}");
}

#[test]
fn conflicting_requirements_give_a_readable_error() {
    let ws = Tmp::new("conflict");
    let reg = file_url(&ws.path("registry"));
    publish_lib(&ws, &reg, "libA", "1.0.0", "", LIB_A_1_0);
    publish_lib(&ws, &reg, "libA", "2.0.0", "", LIB_A_1_1);
    publish_lib(&ws, &reg, "libB", "1.0.0", "libA = \"^2.0\"\n", "fn lib_b() {\n  return 1\n}\n");
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "libA@^1.0", "--registry", &reg]);
    ok(&ws.0, &["-C", "app", "add", "libB@^1", "--registry", &reg]);
    let out = fail(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("version conflict on `libA`"), "{out}");
    assert!(out.contains("app requires libA ^1.0") && out.contains("libB 1.0.0 requires libA ^2.0"), "{out}");
    assert!(out.contains("available versions: 1.0.0, 2.0.0"), "{out}");
    assert!(!ws.path("app/killer.lock").exists());
    // a requirement nothing satisfies
    ok(&ws.0, &["-C", "app", "remove", "libB"]);
    ok(&ws.0, &["-C", "app", "add", "libA@^3.0", "--registry", &reg]);
    let out = fail(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("app requires libA ^3.0"), "{out}");
    // a package that does not exist
    ok(&ws.0, &["-C", "app", "add", "ghost@*", "--registry", &reg]);
    let out = fail(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("ghost") && out.contains("not found"), "{out}");
    // a bad requirement is rejected up front
    assert!(fail(&ws.path("app"), &["add", "libA@^banana", "--registry", &reg]).contains("invalid version requirement"));
}

#[test]
fn transitive_dependencies_are_installed_and_importable() {
    let ws = Tmp::new("trans");
    let reg = file_url(&ws.path("registry"));
    publish_lib(&ws, &reg, "libA", "1.0.0", "", LIB_A_1_0);
    publish_lib(&ws, &reg, "libB", "1.0.0", "libA = \"^1.0\"\n", "import libA\nfn lib_b(x) {\n  return lib_a_add(x, 100)\n}\n");
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "libB@^1", "--registry", &reg]);
    let out = ok(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("libA 1.0.0") && out.contains("libB 1.0.0"), "{out}");
    let lock = std::fs::read_to_string(ws.path("app/killer.lock")).unwrap();
    assert!(lock.contains("\"libA 1.0.0\""), "libB's dependency is recorded: {lock}");
    ws.write("app/src/main.killer", "import libB\nprintln(lib_b(5))\n");
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.path("app")), "105");
    assert!(ok(&ws.path("app"), &["list"]).contains("transitive"));
}

#[test]
fn workspace_path_dependencies_are_locked_and_checked() {
    let ws = Tmp::new("pathlock");
    ok(&ws.0, &["new", "shared", "--lib"]);
    ws.write("shared/src/lib.killer", "fn shared_one() {\n  return 1\n}\n");
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "shared", "--path", "../shared"]);
    assert!(std::fs::read_to_string(ws.path("app/killer.toml")).unwrap().contains("shared = { path = \"../shared\" }"));
    ok(&ws.path("app"), &["install"]);
    let lock = std::fs::read_to_string(ws.path("app/killer.lock")).unwrap();
    assert!(lock.contains("path+../shared"), "{lock}");
    ws.write("app/src/main.killer", "import shared\nprintln(shared_one())\n");
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.path("app")), "1");
    assert!(ok(&ws.path("app"), &["check"]).contains("ok"));
    // editing the path dependency after locking is reported (re-run install to accept)
    ws.write("shared/src/lib.killer", "fn shared_one() {\n  return 2\n}\n");
    assert!(fail(&ws.path("app"), &["check"]).contains("checksum mismatch"));
    ok(&ws.path("app"), &["install"]);
    assert!(ok(&ws.path("app"), &["check"]).contains("ok"));
}

#[test]
fn a_tampered_registry_release_fails_checksum_verification_on_install() {
    let ws = Tmp::new("tamper");
    let reg_dir = ws.path("registry");
    let reg = file_url(&reg_dir);
    publish_lib(&ws, &reg, "libA", "1.0.0", "", LIB_A_1_0);
    let pack = reg_dir.join("libA/1.0.0/package.pack");
    let bytes = std::fs::read(&pack).unwrap();
    let text = String::from_utf8_lossy(&bytes).replace("lib_a_add", "lib_a_xxx");
    std::fs::write(&pack, text.as_bytes()).unwrap();
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "libA@^1", "--registry", &reg]);
    let out = fail(&ws.path("app"), &["install", "--registry", &reg]);
    assert!(out.contains("checksum mismatch") && out.contains("libA 1.0.0"), "{out}");
    assert!(!ws.path("app/.killer/packages/libA").exists());
}

#[test]
fn an_http_registry_needs_a_token_to_publish_and_serves_installs() {
    let ws = Tmp::new("http");
    let srv = killer_native::pkg_registry::spawn_server(ws.path("registry"), vec!["tok-123".to_string()], "127.0.0.1:0").unwrap();
    let url = format!("http://127.0.0.1:{}", srv.addr.port());
    ok(&ws.0, &["new", "libH", "--lib"]);
    ws.write("libH/src/lib.killer", "fn lib_h() {\n  return \"over http\"\n}\n");
    let out = fail(&ws.0, &["-C", "libH", "publish", "--registry", &url]);
    assert!(out.contains("token"), "{out}");
    let out = fail(&ws.0, &["-C", "libH", "publish", "--registry", &url, "--token", "wrong"]);
    assert!(out.contains("401") || out.contains("invalid token"), "{out}");
    ok(&ws.0, &["-C", "libH", "publish", "--registry", &url, "--token", "tok-123"]);
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "libH", "--registry", &url]);
    let out = ok(&ws.path("app"), &["install", "--registry", &url]);
    assert!(out.contains("libH 0.1.0"), "{out}");
    ws.write("app/src/main.killer", "import libH\nprintln(lib_h())\n");
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.path("app")), "over http");
    srv.stop();
}

#[test]
fn publish_refuses_path_dependencies_and_supports_dry_run() {
    let ws = Tmp::new("pubchecks");
    let reg = file_url(&ws.path("registry"));
    ok(&ws.0, &["new", "shared", "--lib"]);
    ok(&ws.0, &["new", "pubby", "--lib"]);
    ok(&ws.0, &["-C", "pubby", "add", "shared", "--path", "../shared"]);
    assert!(fail(&ws.0, &["-C", "pubby", "publish", "--registry", &reg]).contains("path dependencies"));
    let out = ok(&ws.0, &["-C", "shared", "publish", "--dry-run"]);
    assert!(out.contains("Would publish shared 0.1.0") && out.contains("src/lib.killer"), "{out}");
    assert!(!ws.path("registry").exists());
}

#[test]
fn git_dependencies_are_pinned_to_a_commit() {
    let git = |cwd: &Path, args: &[&str]| -> Option<String> {
        let out = Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@example.com"]).args(args).current_dir(cwd).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let ws = Tmp::new("git");
    ok(&ws.0, &["new", "gitlib", "--lib"]);
    ws.write("gitlib/src/lib.killer", "fn from_git() {\n  return \"git works\"\n}\n");
    let repo = ws.path("gitlib");
    if git(&repo, &["init", "-q"]).is_none() {
        eprintln!("git is not available; skipping");
        return;
    }
    git(&repo, &["add", "."]).unwrap();
    git(&repo, &["commit", "-q", "-m", "init"]).unwrap();
    let commit = git(&repo, &["rev-parse", "HEAD"]).unwrap();
    ok(&ws.0, &["new", "app"]);
    ok(&ws.0, &["-C", "app", "add", "gitlib", "--git", &repo.to_string_lossy().replace('\\', "/")]);
    ok(&ws.path("app"), &["install"]);
    let lock = std::fs::read_to_string(ws.path("app/killer.lock")).unwrap();
    assert!(lock.contains(&format!("#{}", commit)), "{lock}");
    ws.write("app/src/main.killer", "import gitlib\nprintln(from_git())\n");
    assert_eq!(run_script(&ws.path("app/src/main.killer"), &ws.path("app")), "git works");
    assert!(ok(&ws.path("app"), &["check"]).contains("ok"));
}
