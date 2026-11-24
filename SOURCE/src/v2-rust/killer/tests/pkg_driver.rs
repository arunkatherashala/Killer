//! The package manager through the unified `killer` driver (`killer new/add/install/run`), against a local
//! file registry in a temp directory, including lockfile checksum verification.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Tmp {
        let d = std::env::temp_dir().join(format!("killer_drv_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
    fn p(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn killer(cwd: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).args(args).current_dir(cwd).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n");
    (out.status.success(), text.trim().to_string())
}

fn ok(cwd: &Path, args: &[&str]) -> String {
    let (good, out) = killer(cwd, args);
    assert!(good, "killer {args:?} failed: {out}");
    out
}

fn file_url(p: &Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/'))
}

fn setup(ws: &Tmp) -> String {
    let reg = file_url(&ws.p("registry"));
    ok(&ws.0, &["new", "greet", "--lib"]);
    std::fs::write(ws.p("greet/killer.toml"), "[package]\nname = \"greet\"\nversion = \"1.0.0\"\nentry = \"src/lib.killer\"\n\n[dependencies]\n").unwrap();
    std::fs::write(ws.p("greet/src/lib.killer"), "fn greet_hello(n) {\n  return \"hello \" + n\n}\n").unwrap();
    ok(&ws.p("greet"), &["publish", "--registry", &reg]);
    ok(&ws.0, &["new", "app"]);
    ok(&ws.p("app"), &["add", "greet@^1", "--registry", &reg]);
    reg
}

/// Replace the published pack and the registry's own checksum for it, consistently, as a compromised registry
/// would (the checksum is the content hash of the files, not of the pack bytes).
fn tamper_registry(ws: &Tmp) -> String {
    let pack = ws.p("registry/greet/1.0.0/package.pack");
    let mut files = killer_native::pkg_files::unpack(&std::fs::read(&pack).unwrap()).unwrap();
    for (_, data) in files.iter_mut() {
        *data = String::from_utf8_lossy(data).replace("hello ", "pwned ").into_bytes();
    }
    std::fs::write(&pack, killer_native::pkg_files::pack(&files)).unwrap();
    let new_sha = killer_native::pkg_files::content_hash(&files);
    std::fs::write(ws.p("registry/greet/1.0.0/sha256"), format!("{new_sha}\n")).unwrap();
    new_sha
}

#[test]
fn driver_new_add_install_then_import_by_name_and_missing_import_is_an_error() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    let out = ok(&ws.p("app"), &["install", "--registry", &reg]);
    assert!(out.contains("greet 1.0.0"), "{out}");
    let lock = std::fs::read_to_string(ws.p("app/killer.lock")).unwrap();
    let sha = std::fs::read_to_string(ws.p("registry/greet/1.0.0/sha256")).unwrap();
    assert!(lock.contains(sha.trim()), "lockfile must record the checksum: {lock}");

    std::fs::write(ws.p("app/src/main.killer"), "import greet\nprintln(greet_hello(\"world\"))\n").unwrap();
    assert_eq!(ok(&ws.p("app"), &["run", "src/main.killer"]), "hello world");

    std::fs::write(ws.p("app/src/bad.killer"), "import nosuchpkg\nprintln(1)\n").unwrap();
    let (good, out) = killer(&ws.p("app"), &["run", "src/bad.killer"]);
    assert!(!good, "a missing import must fail hard: {out}");
    assert!(out.contains("nosuchpkg"), "{out}");
}

#[test]
fn install_verifies_against_the_lockfile_even_if_registry_and_its_checksum_are_both_rewritten() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    ok(&ws.p("app"), &["install", "--registry", &reg]);

    // an attacker (or a broken mirror) replaces the pack and also its own sha256 file, consistently
    tamper_registry(&ws);

    // a fresh checkout: lockfile present, no installed packages
    std::fs::remove_dir_all(ws.p("app/.killer")).unwrap();
    let (good, out) = killer(&ws.p("app"), &["install", "--registry", &reg]);
    assert!(!good, "install must refuse a release that differs from the lockfile: {out}");
    assert!(out.contains("checksum mismatch") && out.contains("killer.lock records"), "it must be the lockfile that catches it: {out}");
    assert!(!ws.p("app/.killer/packages/greet").exists());
}

#[test]
fn pinned_checksums_in_the_manifest_protect_a_first_install_that_has_no_lockfile() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    ok(&ws.p("app"), &["install", "--registry", &reg]);
    let good = std::fs::read_to_string(ws.p("registry/greet/1.0.0/sha256")).unwrap();
    let out = ok(&ws.p("app"), &["pin"]);
    assert!(out.contains("Pinned 1"), "{out}");
    let manifest = std::fs::read_to_string(ws.p("app/killer.toml")).unwrap();
    assert!(manifest.contains("[checksums]") && manifest.contains(&format!("\"greet@1.0.0\" = \"{}\"", good.trim())), "{manifest}");
    // pinning is idempotent and the manifest still parses
    ok(&ws.p("app"), &["pin"]);
    ok(&ws.p("app"), &["check"]);

    // the registry is compromised AFTER the pin, and this is a fresh clone: no lockfile, no installed packages.
    // Without the pin nothing local could notice: the registry's own checksum is rewritten to match.
    tamper_registry(&ws);
    std::fs::remove_file(ws.p("app/killer.lock")).unwrap();
    std::fs::remove_dir_all(ws.p("app/.killer")).unwrap();
    let (good_exit, out) = killer(&ws.p("app"), &["install", "--registry", &reg]);
    assert!(!good_exit, "a release that differs from the pin must be refused: {out}");
    assert!(out.contains("checksum mismatch") && out.contains("[checksums]"), "{out}");
    assert!(!ws.p("app/.killer/packages/greet").exists(), "nothing may be installed from a mismatching release");
    assert!(!ws.p("app/killer.lock").exists());
}

#[test]
fn without_a_pin_or_lockfile_a_rewritten_registry_is_accepted_which_is_the_documented_limit() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    tamper_registry(&ws);
    // trust on first use: the only checksum available came from the same server
    ok(&ws.p("app"), &["install", "--registry", &reg]);
}

#[test]
fn require_pins_refuses_unpinned_registry_releases() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    let (good, out) = killer(&ws.p("app"), &["install", "--registry", &reg, "--require-pins"]);
    assert!(!good, "{out}");
    assert!(out.contains("no pin") && out.contains("greet"), "{out}");
    assert!(!ws.p("app/.killer/packages/greet").exists());
    // after a normal install and a pin, the same flag passes
    ok(&ws.p("app"), &["install", "--registry", &reg]);
    ok(&ws.p("app"), &["pin"]);
    std::fs::remove_dir_all(ws.p("app/.killer")).unwrap();
    ok(&ws.p("app"), &["install", "--registry", &reg, "--require-pins"]);
}

#[test]
fn check_reports_a_lockfile_that_disagrees_with_the_pin_and_bad_pins_are_rejected() {
    let ws = Tmp::new();
    let reg = setup(&ws);
    ok(&ws.p("app"), &["install", "--registry", &reg]);
    ok(&ws.p("app"), &["pin"]);
    let lock = std::fs::read_to_string(ws.p("app/killer.lock")).unwrap();
    let sha = std::fs::read_to_string(ws.p("registry/greet/1.0.0/sha256")).unwrap();
    std::fs::write(ws.p("app/killer.lock"), lock.replace(sha.trim(), &"0".repeat(64))).unwrap();
    let (good, out) = killer(&ws.p("app"), &["check"]);
    assert!(!good && out.contains("pins"), "{out}");
    // a malformed pin is a manifest error, not silently ignored
    let m = std::fs::read_to_string(ws.p("app/killer.toml")).unwrap();
    std::fs::write(ws.p("app/killer.toml"), m.replace(sha.trim(), "abc")).unwrap();
    let (good, out) = killer(&ws.p("app"), &["check"]);
    assert!(!good && out.contains("[checksums]") && out.contains("64 hex"), "{out}");
}
