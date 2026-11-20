//! SSRF guard of the http_* builtins, end to end through the real binary: loopback/private targets need an explicit
//! opt-in; link-local and cloud-metadata addresses stay blocked even with `allow_private`.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str, allow_env: bool) -> String {
    let path = std::env::temp_dir().join(format!("killer_ssrf_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_killer_super"));
    cmd.arg(&path).arg("--run").env_remove("KILLER_ALLOW_PRIVATE_NET").env_remove("KILLER_ALLOW_METADATA_NET");
    if allow_env {
        cmd.env("KILLER_ALLOW_PRIVATE_NET", "1");
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn loopback_and_private_targets_are_refused_by_default() {
    for url in ["http://127.0.0.1:9/", "http://[::1]:9/", "http://localhost:9/", "https://10.0.0.1/", "https://[::ffff:127.0.0.1]:9/"] {
        let out = run(&format!("println(http_request({{\"url\": \"{}\", \"timeout\": 3}}))", url), false);
        assert!(out.contains("SSRF"), "{} not refused: {}", url, out);
    }
    let out = run("println(http_get(\"http://127.0.0.1:9/\"))", false);
    assert!(out.contains("SSRF"), "{}", out);
}

#[test]
fn allow_private_option_and_env_opt_in_reach_the_network_layer() {
    let out = run("println(http_request({\"url\": \"http://127.0.0.1:9/\", \"timeout\": 3, \"allow_private\": true}))", false);
    assert!(!out.contains("SSRF") && out.contains("failed"), "{}", out);
    let out = run("println(http_get(\"http://127.0.0.1:9/\", 3))", true);
    assert!(!out.contains("SSRF"), "{}", out);
}

#[test]
fn metadata_and_link_local_stay_blocked_even_with_allow_private() {
    for url in ["https://169.254.169.254/latest/meta-data/", "https://[fd00:ec2::254]/", "https://169.254.10.10/", "https://metadata.google.internal/", "https://[::ffff:169.254.169.254]/"] {
        let src = format!("println(http_request({{\"url\": \"{}\", \"timeout\": 3, \"allow_private\": true}}))", url);
        for env in [false, true] {
            let out = run(&src, env);
            assert!(out.contains("metadata"), "{} (env={}): {}", url, env, out);
        }
    }
}
