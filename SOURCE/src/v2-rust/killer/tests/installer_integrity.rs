//! `install.sh` integrity checks, run against a fake release served from a local directory through `file://`
//! (curl reads those). The archive's own `.sha256` comes from the same place as the archive, so these tests show
//! that an independent checksum (`--sha256`, or a pinned checksums file) is what catches a consistently rewritten
//! release. Skipped, with a message, when `sh`, `curl`, `tar` or a SHA-256 tool is missing (e.g. a bare Windows box).
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Tmp {
        let d = std::env::temp_dir().join(format!("killer_inst_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
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

fn sh_ok(script: &str) -> bool {
    Command::new("sh").arg("-c").arg(script).output().map(|o| o.status.success()).unwrap_or(false)
}

fn tools_available() -> bool {
    let ok = sh_ok("exit 0") && sh_ok("curl --version >/dev/null 2>&1") && sh_ok("tar --help >/dev/null 2>&1") && sh_ok("command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1");
    if !ok {
        eprintln!("sh, curl, tar or a sha256 tool is missing: skipping the install.sh checks");
    }
    ok
}

fn file_url(p: &Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/'))
}

fn install_sh() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).expect("repository root").join("install.sh")
}

const TAG: &str = "v9.9.9";
const NAME: &str = "killer_super-v9.9.9-linux-x64";

/// A fake release directory with the archive and its (consistent) `.sha256`; returns the archive's SHA-256.
fn make_release(ws: &Tmp, payload: &str) -> String {
    let rel = ws.p("release");
    let dir = rel.join(NAME);
    std::fs::create_dir_all(&dir).unwrap();
    for b in ["killer", "killer_super"] {
        std::fs::write(dir.join(b), format!("#!/bin/sh\necho {payload}\n")).unwrap();
    }
    let st = Command::new("tar").args(["-czf", &format!("{NAME}.tar.gz"), NAME]).current_dir(&rel).status().unwrap();
    assert!(st.success());
    let archive = std::fs::read(rel.join(format!("{NAME}.tar.gz"))).unwrap();
    let sha = killer_native::pkg_files::sha256_hex(&archive);
    std::fs::write(rel.join(format!("{NAME}.tar.gz.sha256")), format!("{sha}  {NAME}.tar.gz\n")).unwrap();
    sha
}

fn run(ws: &Tmp, extra: &[&str], env: &[(&str, String)]) -> (bool, i32, String) {
    let mut c = Command::new("sh");
    c.arg(install_sh()).args(["--version", TAG, "--prefix"]).arg(ws.p("prefix")).args(extra);
    c.env("KILLER_OS", "Linux").env("KILLER_ARCH", "x86_64").env("KILLER_BASE_URL", file_url(&ws.p("release")));
    c.env_remove("KILLER_SHA256").env_remove("KILLER_REQUIRE_PINNED").env_remove("KILLER_CHECKSUMS_URL");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n");
    (out.status.success(), out.status.code().unwrap_or(-1), text)
}

fn installed(ws: &Tmp) -> bool {
    ws.p("prefix/bin/killer").exists()
}

#[test]
fn a_matching_independent_checksum_installs() {
    if !tools_available() {
        return;
    }
    let ws = Tmp::new();
    let sha = make_release(&ws, "good");
    let (ok, _, out) = run(&ws, &["--sha256", &sha.to_uppercase()], &[]);
    assert!(ok, "{out}");
    assert!(out.contains("pinned checksum ok"), "{out}");
    assert!(installed(&ws));
}

#[test]
fn a_release_rewritten_consistently_is_caught_by_the_independent_checksum() {
    if !tools_available() {
        return;
    }
    let ws = Tmp::new();
    let honest = make_release(&ws, "good");
    // an attacker with write access to the release assets replaces the archive AND its .sha256
    let evil = make_release(&ws, "evil");
    assert_ne!(honest, evil);
    // the release's own .sha256 agrees with the evil archive, so without an independent source it installs
    let (ok, _, out) = run(&ws, &[], &[]);
    assert!(ok, "{out}");
    assert!(out.contains("pinned:   none"), "{out}");
    std::fs::remove_dir_all(ws.p("prefix")).unwrap();
    // with the hash published elsewhere (release notes, checksums/<tag>.sha256) it is refused and nothing is installed
    let (ok, _, out) = run(&ws, &["--sha256", &honest], &[]);
    assert!(!ok, "{out}");
    assert!(out.contains("checksum mismatch") && out.contains("pinned"), "{out}");
    assert!(!installed(&ws));
    let (ok, _, out) = run(&ws, &[], &[("KILLER_SHA256", honest.clone())]);
    assert!(!ok && !installed(&ws), "{out}");
}

#[test]
fn a_pinned_checksums_file_is_used_and_must_have_an_entry() {
    if !tools_available() {
        return;
    }
    let ws = Tmp::new();
    let sha = make_release(&ws, "good");
    let sums = ws.p("pins.sha256");
    let url = file_url(&sums);
    // sha256sum format, with another archive's line first and the binary-mode '*' marker
    std::fs::write(&sums, format!("{}  killer_super-v9.9.9-macos-arm64.tar.gz\n{} *{}.tar.gz\n", "1".repeat(64), sha, NAME)).unwrap();
    let (ok, _, out) = run(&ws, &[], &[("KILLER_CHECKSUMS_URL", url.clone())]);
    assert!(ok && out.contains("pinned checksum ok"), "{out}");
    std::fs::remove_dir_all(ws.p("prefix")).unwrap();
    // wrong hash in the pinned file
    std::fs::write(&sums, format!("{}  {}.tar.gz\n", "2".repeat(64), NAME)).unwrap();
    let (ok, _, out) = run(&ws, &[], &[("KILLER_CHECKSUMS_URL", url.clone())]);
    assert!(!ok && out.contains("pinned checksum is") && !installed(&ws), "{out}");
    // no entry for this archive
    std::fs::write(&sums, format!("{}  other.tar.gz\n", sha)).unwrap();
    let (ok, _, out) = run(&ws, &[], &[("KILLER_CHECKSUMS_URL", url)]);
    assert!(!ok && out.contains("no entry") && !installed(&ws), "{out}");
    // an explicit checksums file that cannot be fetched is an error, not a warning
    let (ok, _, out) = run(&ws, &[], &[("KILLER_CHECKSUMS_URL", file_url(&ws.p("missing.sha256")))]);
    assert!(!ok && out.contains("could not fetch") && !installed(&ws), "{out}");
}

#[test]
fn require_pinned_refuses_to_install_on_the_release_checksum_alone() {
    if !tools_available() {
        return;
    }
    let ws = Tmp::new();
    let sha = make_release(&ws, "good");
    let (ok, _, out) = run(&ws, &["--require-pinned"], &[]);
    assert!(!ok && out.contains("refusing to install") && !installed(&ws), "{out}");
    let (ok, _, out) = run(&ws, &[], &[("KILLER_REQUIRE_PINNED", "1".to_string())]);
    assert!(!ok && !installed(&ws), "{out}");
    let (ok, _, out) = run(&ws, &["--require-pinned", "--sha256", &sha], &[]);
    assert!(ok && installed(&ws), "{out}");
}

#[test]
fn a_malformed_sha256_option_is_rejected_before_anything_is_downloaded() {
    if !tools_available() {
        return;
    }
    let ws = Tmp::new();
    make_release(&ws, "good");
    for bad in ["abc", &"g".repeat(64), &"a".repeat(63), &"a".repeat(65)] {
        let (ok, code, out) = run(&ws, &["--sha256", bad], &[]);
        assert!(!ok && code == 2 && out.contains("64 hex digits"), "{bad}: {out}");
    }
    assert!(!installed(&ws));
}
