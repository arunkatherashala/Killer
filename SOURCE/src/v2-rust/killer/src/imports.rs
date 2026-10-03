//! `import "other.killer"` — compile-time inclusion of another Killer file.
//!
//! The imported file's text replaces the `import` line, so its functions, classes and globals
//! are visible to the importing file exactly as if they had been written there. Paths are
//! resolved relative to the importing file (the driver sets the base directory with
//! [`set_base_dir`]), then `packages/`. A file is included once (repeated or circular imports
//! are skipped). `import name` without quotes is left alone for the runtime package loader.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

thread_local! {
    static BASE_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

const MAX_DEPTH: usize = 32;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

/// Directory that relative imports are resolved against (normally the main script's folder).
pub fn set_base_dir(dir: impl Into<PathBuf>) {
    BASE_DIR.with(|b| *b.borrow_mut() = Some(dir.into()));
}

fn base_dir() -> PathBuf {
    BASE_DIR
        .with(|b| b.borrow().clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// The quoted path of an `import "path"` line, if it is one.
fn quoted_import(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("import ")?.trim();
    let rest = rest.strip_suffix(';').unwrap_or(rest).trim();
    let inner = rest.strip_prefix('"')?.strip_suffix('"')?;
    if inner.is_empty() || inner.contains('"') {
        None
    } else {
        Some(inner)
    }
}

fn locate(path: &str, from: &Path) -> Option<PathBuf> {
    let candidates = [
        from.join(path),
        from.join(format!("{path}.killer")),
        from.join("packages").join(path),
        from.join("packages").join(format!("{path}.killer")),
    ];
    candidates.into_iter().find(|c| c.is_file())
}

fn expand(
    source: &str,
    dir: &Path,
    seen: &mut HashSet<PathBuf>,
    depth: usize,
    total: &mut usize,
) -> Result<String, String> {
    if !source.lines().any(|l| quoted_import(l).is_some()) {
        return Ok(source.to_string());
    }
    if depth > MAX_DEPTH {
        return Err("imports are nested too deeply".to_string());
    }
    let mut out = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let Some(path) = quoted_import(line) else {
            out.push_str(line);
            continue;
        };
        crate::security::require_file_read().map_err(|e| e.to_string())?;
        // Not found here: leave the line for the runtime package loader (optional imports are common).
        let Some(file) = locate(path, dir) else {
            out.push_str(line);
            continue;
        };
        let canonical = file.canonicalize().unwrap_or_else(|_| file.clone());
        if !seen.insert(canonical.clone()) {
            out.push('\n'); // already included: keep the line count
            continue;
        }
        let text = std::fs::read_to_string(&file).map_err(|e| format!("import \"{}\": {}", path, e))?;
        *total += text.len();
        if *total > MAX_TOTAL_BYTES {
            return Err("imported files are too large".to_string());
        }
        let parent = canonical.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf());
        let body = expand(&text, &parent, seen, depth + 1, total)?;
        out.push_str(&body);
        if !body.ends_with('\n') {
            out.push('\n');
        }
    }
    Ok(out)
}

/// Replace every `import "file"` line in `source` with the file's (recursively expanded) text.
pub fn resolve(source: &str) -> Result<String, String> {
    if !source.contains("import") {
        return Ok(source.to_string());
    }
    let dir = base_dir();
    let mut seen = HashSet::new();
    let mut total = 0usize;
    expand(source, &dir, &mut seen, 0, &mut total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("killer_imp_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn source_without_imports_is_untouched() {
        assert_eq!(resolve("x = 1\nprintln(x)\n").unwrap(), "x = 1\nprintln(x)\n");
    }

    #[test]
    fn imports_are_inlined_relative_to_the_importing_file() {
        let d = temp_dir("inline");
        std::fs::create_dir_all(d.join("lib")).unwrap();
        std::fs::write(d.join("lib/a.killer"), "import \"b.killer\"\nfn a() {\n  return b() + 1\n}\n").unwrap();
        std::fs::write(d.join("lib/b.killer"), "fn b() {\n  return 1\n}\n").unwrap();
        set_base_dir(&d);
        let out = resolve("import \"lib/a.killer\"\nprintln(a())\n").unwrap();
        assert!(out.contains("fn b()"), "{out}");
        assert!(out.contains("fn a()"), "{out}");
        assert!(out.ends_with("println(a())\n"), "{out}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn repeated_and_circular_imports_are_included_once() {
        let d = temp_dir("cycle");
        std::fs::write(d.join("x.killer"), "import \"y.killer\"\nfn x() {\n  return 1\n}\n").unwrap();
        std::fs::write(d.join("y.killer"), "import \"x.killer\"\nfn y() {\n  return 2\n}\n").unwrap();
        set_base_dir(&d);
        let out = resolve("import \"x.killer\"\nimport \"y.killer\"\n").unwrap();
        assert_eq!(out.matches("fn x()").count(), 1, "{out}");
        assert_eq!(out.matches("fn y()").count(), 1, "{out}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_files_and_bare_names_are_left_for_the_runtime() {
        let d = temp_dir("missing");
        set_base_dir(&d);
        assert_eq!(resolve("import \"nope.killer\"\n").unwrap(), "import \"nope.killer\"\n");
        assert_eq!(resolve("import somepkg\n").unwrap(), "import somepkg\n");
        let _ = std::fs::remove_dir_all(&d);
    }
}
