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
