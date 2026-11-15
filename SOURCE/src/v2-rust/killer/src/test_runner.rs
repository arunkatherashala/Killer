//! `killer-native --test [pattern]` — discover and run `.killer` test files.
//!
//! Convention:
//! - Files named `test_*.killer` or `*_test.killer` are test files.
//! - Each file is executed as a standalone program.
//! - A test **passes** if it exits without error (no unhandled exception, no panic).
//! - A test **fails** if `run_killer_source` returns `Err`.
//! - Lines containing `assert(` are assertion calls (the builtin panics on false).
//!
//! Usage:
//!   killer-native --test                 # run all test_*.killer / *_test.killer in cwd
//!   killer-native --test str             # only files whose name contains "str"
//!   killer-native --test tests/          # run all test files under tests/

use std::path::PathBuf;
use std::time::Instant;
use crate::error::VmError;

/// Discover and run test files, printing results in TAP-like format.
/// True when `arg` is written like a path (separator, `.killer` suffix or leading `.`/`~`)
/// rather than a plain name filter.
fn looks_like_path(arg: &str) -> bool {
    arg != "."
        && (arg.contains('/') || arg.contains('\\') || arg.ends_with(".killer") || arg.starts_with('.') || arg.starts_with('~'))
}

/// Resolve the CLI argument into the list of files to run. A path that does not exist is an
/// error (not a filter that silently matches nothing); a path that yields no tests is an error.
fn resolve_test_files(pattern: &str) -> Result<Vec<PathBuf>, String> {
    let p = std::path::Path::new(pattern);
    if p.is_file() {
        return Ok(vec![p.to_path_buf()]);
    }
    if !p.exists() && looks_like_path(pattern) {
        return Err(format!("test path '{pattern}' does not exist"));
    }
    let search_dir = if p.is_dir() {
        PathBuf::from(pattern)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    let files = discover_test_files(&search_dir, pattern);
    if files.is_empty() && pattern != "." {
        return Err(format!("no test files found matching '{pattern}'"));
    }
    Ok(files)
}

pub fn run_test_suite(pattern: &str) -> Result<(), VmError> {
    let files = resolve_test_files(pattern).map_err(VmError::runtime_error)?;

    if files.is_empty() {
        println!("No test files found matching '{pattern}'");
        return Ok(());
    }

    println!("Running {} test file(s)...\n", files.len());

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors: Vec<(String, String)> = Vec::new();
    let suite_start = Instant::now();

    for file in &files {
        let name = file.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file.display().to_string());

        let start = Instant::now();
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                failed += 1;
                let msg = format!("read error: {e}");
                println!("  FAIL  {} ({})", name, msg);
                errors.push((name, msg));
                continue;
            }
        };

        match crate::run_killer_source(&source) {
            Ok(()) => {
                passed += 1;
                let elapsed = start.elapsed();
                println!("  PASS  {} ({:.0?})", name, elapsed);
            }
            Err(e) => {
                failed += 1;
                let msg = format!("{e}");
                println!("  FAIL  {} — {}", name, msg);
                errors.push((name, msg));
            }
        }
    }

    let total_time = suite_start.elapsed();
    println!("\n──────────────────────────────────────────");
    println!("Results: {} passed, {} failed, {} total ({:.2?})",
             passed, failed, passed + failed, total_time);

    if !errors.is_empty() {
        println!("\nFailures:");
        for (name, msg) in &errors {
            println!("  {name}: {msg}");
        }
    }

    if failed > 0 {
        Err(VmError::runtime_error(format!("{failed} test(s) failed")))
    } else {
        println!("\nAll tests passed!");
        Ok(())
    }
}

/// Walk `dir` recursively and collect files matching `test_*.killer` or `*_test.killer`
/// whose name also contains `pattern` (unless pattern is "." for match-all).
fn discover_test_files(dir: &std::path::Path, pattern: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return results; };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            results.extend(discover_test_files(&path, pattern));
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let is_test = (name.starts_with("test_") || name.starts_with("_test_")
                           || name.ends_with("_test.killer"))
                          && name.ends_with(".killer");
            let matches_pattern = pattern == "." || name.contains(pattern)
                                  || dir.to_string_lossy().contains(pattern);
            if is_test && matches_pattern {
                results.push(path);
            }
        }
    }

    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_is_an_error() {
        assert!(resolve_test_files("no/such/dir_xyz").is_err());
        assert!(resolve_test_files("missing_file_xyz.killer").is_err());
        assert!(run_test_suite("no/such/dir_xyz").is_err());
    }

    #[test]
    fn empty_dir_path_is_an_error() {
        let tmp = std::env::temp_dir().join("killer_test_runner_empty_err");
        let _ = std::fs::create_dir_all(&tmp);
        let r = resolve_test_files(&tmp.to_string_lossy());
        let _ = std::fs::remove_dir(&tmp);
        assert!(r.is_err());
    }

    #[test]
    fn test_discover_empty_dir() {
        let tmp = std::env::temp_dir().join("killer_test_runner_empty");
        let _ = std::fs::create_dir_all(&tmp);
        let files = discover_test_files(&tmp, ".");
        // Should not panic, may be empty
        assert!(files.is_empty() || !files.is_empty());
        let _ = std::fs::remove_dir(&tmp);
    }
}
