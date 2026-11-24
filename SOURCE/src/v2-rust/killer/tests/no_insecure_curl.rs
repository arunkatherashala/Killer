//! Certificate verification must stay on: the only place allowed to pass `-k` to curl is the explicit
//! `insecure: true` option of `http_request` (src/http_req.rs). Any other `-k`, `--insecure` or a combined flag
//! such as `-sLk` in the source fails this test.

use std::path::Path;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().map_or(false, |e| e == "rs") {
            out.push(path);
        }
    }
}

/// A curl flag string that turns certificate checks off: "-k", "--insecure", or a short-flag bundle holding k.
fn is_insecure_flag(token: &str) -> bool {
    if token == "--insecure" {
        return true;
    }
    token.len() >= 2
        && token.starts_with('-')
        && !token.starts_with("--")
        && token[1..].chars().all(|c| c.is_ascii_alphabetic())
        && token[1..].contains('k')
}

#[test]
fn no_source_file_disables_certificate_checks_except_the_explicit_option() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() > 100, "found only {} source files", files.len());
    let mut found = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        if name.ends_with("/src/http_req.rs") || name.contains("/archive/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            // only curl arguments: openssl has flags such as "-key" too. The args may sit a few lines under `Command::new(curl)`.
            let near_curl = lines[n.saturating_sub(4)..=n].iter().any(|l| l.contains("curl"));
            if !near_curl {
                continue;
            }
            // every double-quoted token on the line
            for (i, part) in line.split('"').enumerate() {
                if i % 2 == 1 && is_insecure_flag(part) {
                    found.push(format!("{}:{}: {}", name, n + 1, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "curl certificate checks disabled outside http_request(insecure): \n{}", found.join("\n"));
}

#[test]
fn the_flag_detector_catches_the_known_spellings() {
    for bad in ["-k", "-sk", "-sLk", "-ks", "--insecure"] {
        assert!(is_insecure_flag(bad), "{bad}");
    }
    for good in ["-s", "-sL", "-S", "--ssl-no-revoke", "--max-time", "-o", "-X", "-H", "-A", "-d"] {
        assert!(!is_insecure_flag(good), "{good}");
    }
}

/// Run a script with the killer binary; returns (stdout, stderr).
fn run_script(name: &str, src: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("killer_insecure_{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (String::from_utf8_lossy(&out.stdout).to_string(), String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn insecure_true_prints_a_warning_naming_the_host_only() {
    let src = r#"try {
  http_request({"url": "https://example.invalid/path?key=TOPSECRET", "insecure": true, "timeout": 3})
} catch e {
  println("failed")
}
"#;
    let (_, err) = run_script("warn", src);
    assert!(err.contains("warning: TLS certificate verification is DISABLED for example.invalid"), "{err}");
    assert!(!err.contains("TOPSECRET"), "the warning must not echo the URL query: {err}");
    assert_eq!(err.matches("DISABLED").count(), 1, "one warning per request: {err}");
}

#[test]
fn normal_requests_print_no_warning() {
    let src = r#"try {
  http_request({"url": "https://example.invalid/", "timeout": 3})
} catch e {
  println("failed")
}
"#;
    let (_, err) = run_script("quiet", src);
    assert!(!err.contains("DISABLED"), "{err}");
}

/// A bearer key must never be written to a raw socket: no source file may format an `Authorization: Bearer`
/// header while also holding a `TcpStream`, except the two audited ones (llm.rs hands its headers to http_req;
/// pkg_registry.rs talks to a user-chosen registry and is tracked as an open decision in SECURITY.md).
#[test]
fn no_source_file_writes_a_bearer_key_to_a_raw_socket() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut found = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        if name.ends_with("/src/llm.rs") || name.ends_with("/src/pkg_registry.rs") || name.contains("/archive/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let prod = text.split("#[cfg(test)]").next().unwrap_or("");
        let has_sock = prod.lines().any(|l| !l.trim_start().starts_with("//") && l.contains("TcpStream"));
        let has_bearer = prod.lines().any(|l| !l.trim_start().starts_with("//") && l.contains("Bearer {"));
        if has_sock && has_bearer {
            found.push(name);
        }
    }
    assert!(found.is_empty(), "bearer key near a raw socket in: {:?}", found);
}
