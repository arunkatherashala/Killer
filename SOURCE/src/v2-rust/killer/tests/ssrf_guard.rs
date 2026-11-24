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

/// Bind a listener in 20000-32000 so a bypass would be visible as an accepted connection.
fn listener() -> (std::net::TcpListener, u16) {
    let base = std::process::id() as u16 % 12000;
    for i in 0..2000u16 {
        let port = 20000 + (base + i * 7) % 12000;
        if let Ok(l) = std::net::TcpListener::bind(("127.0.0.1", port)) {
            l.set_nonblocking(true).unwrap();
            return (l, port);
        }
    }
    panic!("no free port");
}

fn refused(out: &str) -> bool {
    out.contains("SSRF")
}

fn request(url: &str, allow_private: bool) -> String {
    let url = url.replace('\\', "\\\\");
    let opt = if allow_private { ", \"allow_private\": true" } else { "" };
    run(&format!("println(http_request({{\"url\": \"{}\", \"timeout\": 3{}}}))", url, opt), false)
}

#[test]
fn encoded_and_obfuscated_hosts_are_refused_before_any_connection() {
    let (l, port) = listener();
    // forms that mean loopback: refused by default
    let loopback = [
        format!("https://127.0.0.1.:{}/", port),
        format!("https://LOCALHOST.:{}/", port),
        format!("https://[::ffff:127.0.0.1]:{}/", port),
        format!("https://[::FFFF:7f00:1]:{}/", port),
        format!("https://evil.example@127.0.0.1:{}/", port),
        format!("https://2130706433:{}/", port),
        format!("https://0x7f.1:{}/", port),
    ];
    for url in &loopback {
        let out = request(url, false);
        assert!(refused(&out), "{} not refused: {}", url, out);
    }
    // percent-encoded, control-character and backslash hosts, and zone ids: refused even with allow_private
    let malformed = [
        format!("https://%31%32%37.0.0.1:{}/", port),
        format!("https://127%2e0.0.1:{}/", port),
        format!("https://127%2E0%2e0%2e1:{}/", port),
        format!("https://%6c%6f%63%61%6c%68%6f%73%74:{}/", port),
        format!("https://%4C%6fcalhost:{}/", port),
        format!("https://user:pw@%31%32%37.0.0.1:{}/", port),
        format!("https://127.0.0.1%00.example.com:{}/", port),
        format!("https://127.0.0.1\\@example.com:{}/", port),
        format!("https://%31%36%39.254.169.254:{}/", port),
        format!("https://169%2e254.169.254:{}/", port),
        format!("https://[::1%25lo]:{}/", port),
        format!("https://[::1%lo]:{}/", port),
        format!("https://[fe80::1%25eth0]:{}/", port),
        format!("https://[::ffff:169.254.169.254%25eth0]:{}/", port),
    ];
    for url in &malformed {
        for allow in [false, true] {
            let out = request(url, allow);
            assert!(refused(&out), "{} (allow_private={}) not refused: {}", url, allow, out);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(l.accept().is_err(), "a connection reached the loopback listener");
}

#[test]
fn zone_ids_never_unlock_blocked_addresses_even_with_allow_private() {
    for url in ["https://[fe80::1%25eth0]:9/", "https://[fe80::1%eth0]:9/", "https://[::ffff:169.254.169.254%25x]:9/", "https://[fd00:ec2::254%25x]:9/"] {
        let out = request(url, true);
        assert!(refused(&out), "{}: {}", url, out);
    }
}

#[test]
fn unresolvable_hosts_fail_closed() {
    let out = request("https://no-such-host.invalid:9/", false);
    assert!(refused(&out), "{}", out);
}
