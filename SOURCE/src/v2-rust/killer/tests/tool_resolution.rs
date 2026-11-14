//! How Killer finds the helper programs behind TLS (`openssl`) and HTTP (`curl`): absolute paths only, never the
//! current directory or a relative PATH entry, with KILLER_OPENSSL / KILLER_CURL as an explicit override.
use killer_native::tool_path::resolve_in;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn exe_name(tool: &str) -> String {
    if cfg!(windows) { format!("{}.exe", tool) } else { tool.to_string() }
}

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!("killer_toolres_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&d).unwrap();
    d.canonicalize().unwrap()
}

fn plant(dir: &Path, tool: &str) -> PathBuf {
    let f = dir.join(exe_name(tool));
    std::fs::write(&f, b"not a real program").unwrap();
    f
}

fn join(parts: &[&Path]) -> OsString {
    std::env::join_paths(parts.iter().map(|p| p.as_os_str())).unwrap()
}

fn sep() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

#[test]
fn finds_the_tool_in_an_absolute_path_entry() {
    let d = scratch();
    let f = plant(&d, "openssl");
    let r = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&join(&[&d])), None).unwrap();
    assert_eq!(r, f);
    assert!(r.is_absolute());
}

#[test]
fn first_absolute_entry_wins() {
    let (a, b) = (scratch(), scratch());
    let fa = plant(&a, "curl");
    plant(&b, "curl");
    let r = resolve_in("curl", "KILLER_CURL", None, Some(&join(&[&a, &b])), None).unwrap();
    assert_eq!(r, fa);
}

#[test]
fn a_planted_tool_in_the_current_directory_is_ignored() {
    let cwd = scratch();
    plant(&cwd, "openssl");
    // the current directory written as an absolute PATH entry is skipped too
    let r = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&join(&[&cwd])), Some(&cwd));
    assert!(r.is_err(), "the cwd copy must not be used, got {:?}", r);
    // and a bare "." entry or an empty one
    let dot = OsString::from(format!(".{}", sep()));
    let r = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&dot), Some(&cwd));
    assert!(r.is_err());
}

#[test]
fn a_planted_tool_in_a_relative_path_entry_is_ignored() {
    let d = scratch();
    std::fs::create_dir_all(d.join("bin")).unwrap();
    plant(&d.join("bin"), "openssl");
    for rel in ["bin", "./bin", "../bin", ""] {
        let r = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&OsString::from(rel)), Some(&d));
        assert!(r.is_err(), "relative PATH entry '{}' must be skipped, got {:?}", rel, r);
    }
}

#[test]
fn a_relative_entry_before_a_good_absolute_one_is_skipped_not_fatal() {
    let good = scratch();
    let f = plant(&good, "openssl");
    let bad = scratch();
    plant(&bad, "openssl");
    let pv = std::env::join_paths([OsString::from("relbin"), good.clone().into_os_string()]).unwrap();
    let r = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&pv), Some(&bad)).unwrap();
    assert_eq!(r, f);
}

#[test]
fn override_must_be_absolute_and_exist() {
    let d = scratch();
    let f = plant(&d, "openssl");
    let ok = resolve_in("openssl", "KILLER_OPENSSL", Some(f.to_str().unwrap()), Some(&OsString::new()), None).unwrap();
    assert_eq!(ok, f);
    let rel = resolve_in("openssl", "KILLER_OPENSSL", Some("openssl"), None, None).unwrap_err();
    assert!(rel.contains("KILLER_OPENSSL") && rel.contains("absolute"), "{rel}");
    let rel2 = resolve_in("openssl", "KILLER_OPENSSL", Some("./openssl"), None, None).unwrap_err();
    assert!(rel2.contains("absolute"), "{rel2}");
    let missing = d.join("nothing_here");
    let m = resolve_in("openssl", "KILLER_OPENSSL", Some(missing.to_str().unwrap()), None, None).unwrap_err();
    assert!(m.contains("not a file"), "{m}");
}

#[test]
fn missing_tool_gives_a_clear_error() {
    let e = resolve_in("openssl", "KILLER_OPENSSL", None, Some(&OsString::new()), None).unwrap_err();
    assert!(e.contains("openssl") && e.contains("KILLER_OPENSSL") && e.contains("PATH"), "{e}");
    let e = resolve_in("curl", "KILLER_CURL", None, None, None).unwrap_err();
    assert!(e.contains("curl") && e.contains("KILLER_CURL"), "{e}");
}

// ---- end to end through the interpreter ---------------------------------------------------------------------

fn run_script(src: &str, cwd: &Path, envs: &[(&str, &str)]) -> (bool, String) {
    let file = cwd.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_killer"));
    c.arg("run").arg(&file).current_dir(cwd).env_remove("KILLER_OPENSSL").env_remove("KILLER_CURL");
    for (k, v) in envs {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

#[test]
fn tls_client_does_not_run_a_planted_openssl_from_cwd_or_a_relative_path() {
    let cwd = scratch();
    plant(&cwd, "openssl");
    std::fs::create_dir_all(cwd.join("bin")).unwrap();
    plant(&cwd.join("bin"), "openssl");
    let path = format!(".{0}bin{0}./bin", sep());
    let (ok, text) = run_script("tcp_connect_tls(\"127.0.0.1:9\")\n", &cwd, &[("PATH", &path)]);
    assert!(!ok, "{text}");
    assert!(text.contains("openssl") && text.contains("not found in any absolute PATH directory"), "{text}");
}

#[test]
fn http_client_does_not_run_a_planted_curl_and_override_must_be_absolute() {
    let cwd = scratch();
    plant(&cwd, "curl");
    let path = format!(".{}", sep());
    let src = "http_request({\"url\": \"https://example.invalid/\"})\n";
    let (ok, text) = run_script(src, &cwd, &[("PATH", &path)]);
    assert!(!ok && text.contains("curl") && text.contains("not found in any absolute PATH directory"), "{text}");
    let (ok, text) = run_script(src, &cwd, &[("PATH", &path), ("KILLER_CURL", "curl")]);
    assert!(!ok && text.contains("KILLER_CURL must be an absolute path"), "{text}");
}

#[test]
fn script_that_changes_path_with_env_set_cannot_redirect_the_tool() {
    let cwd = scratch();
    let evil = cwd.join("evil");
    std::fs::create_dir_all(&evil).unwrap();
    plant(&evil, "openssl");
    let evil_s = evil.to_string_lossy().replace('\\', "/");
    // PATH at start has no openssl; the script then sets PATH to an absolute directory holding a fake one
    let src = format!("env_set(\"PATH\", \"{}\")\ntcp_connect_tls(\"127.0.0.1:9\")\n", evil_s);
    let (ok, text) = run_script(&src, &cwd, &[("PATH", "")]);
    assert!(!ok, "{text}");
    assert!(text.contains("not found in any absolute PATH directory"), "the PATH edit must not be honoured: {text}");
}

#[cfg(unix)]
#[test]
fn override_runs_exactly_the_given_absolute_program() {
    use std::os::unix::fs::PermissionsExt;
    let cwd = scratch();
    let marker = cwd.join("ran");
    let fake = cwd.join("fake_openssl");
    std::fs::write(&fake, format!("#!/bin/sh\necho x > '{}'\nexit 1\n", marker.display())).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _ = run_script("tcp_connect_tls(\"127.0.0.1:9\")\n", &cwd, &[("KILLER_OPENSSL", fake.to_str().unwrap()), ("PATH", "")]);
    assert!(marker.exists(), "the override program was not started");
}

#[test]
fn the_network_capability_still_gates_tls_and_http() {
    use killer_native::builtin::BuiltinFunctions;
    use killer_native::security::{CapabilityScopeGuard, CapabilitySet};
    use killer_native::value::Value;
    let _g = CapabilityScopeGuard::install(CapabilitySet::restricted());
    let d = Value::Dict(killer_native::value::SharedDict::new(std::collections::HashMap::from([("url".to_string(), Value::Str("https://example.invalid/".into()))])));
    for (name, args) in [
        ("tcp_connect_tls", vec![Value::Str("127.0.0.1:9".into())]),
        ("tls_listen", vec![Value::Str("127.0.0.1:0".into()), Value::Str("c".into()), Value::Str("k".into())]),
        ("http_request", vec![d]),
    ] {
        let m = match BuiltinFunctions::call(name, &args) {
            Ok(v) => format!("{v:?}"),
            Err(e) => format!("{e}"),
        };
        assert!(m.contains("apabilit"), "{name}: {m}");
    }
}
