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

use crate::sourcemap::{Loc, SourceMap};

thread_local! {
    static BASE_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static MAIN_FILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Name shown in diagnostics for the script being compiled (normally its path as typed).
pub fn set_main_file(name: impl Into<String>) {
    MAIN_FILE.with(|b| *b.borrow_mut() = Some(name.into()));
}

/// Name of the main script for diagnostics (`<script>` when the driver never set one).
pub fn main_file() -> String {
    MAIN_FILE.with(|b| b.borrow().clone()).unwrap_or_else(|| "<script>".to_string())
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

struct Expander {
    seen: HashSet<PathBuf>,
    total: usize,
    base: PathBuf,
    files: Vec<String>,
    sources: Vec<String>,
    locs: Vec<Loc>,
}

impl Expander {
    /// Name shown for an imported file: relative to the main script's folder when possible.
    fn display_name(&self, file: &Path) -> String {
        let rel = file.strip_prefix(&self.base).unwrap_or(file);
        rel.to_string_lossy().replace('\\', "/")
    }

    /// Expand `source` (the file with index `file_idx`) into `out`, recording the origin of every
    /// line that ends in a newline. An unterminated last line is left for the caller to finish.
    fn expand(&mut self, source: &str, file_idx: u32, dir: &Path, depth: usize, out: &mut String) -> Result<(), String> {
        for (n, line) in source.split_inclusive('\n').enumerate() {
            let here = Loc { file: file_idx, line: n as u32 + 1 };
            let terminated = line.ends_with('\n');
            let Some(path) = quoted_import(line) else {
                out.push_str(line);
                if terminated {
                    self.locs.push(here);
                }
                continue;
            };
            if depth > MAX_DEPTH {
                return Err("imports are nested too deeply".to_string());
            }
            crate::security::require_file_read().map_err(|e| e.to_string())?;
            // Not found here: leave the line for the runtime package loader (optional imports are common).
            let Some(file) = locate(path, dir) else {
                out.push_str(line);
                if terminated {
                    self.locs.push(here);
                }
                continue;
            };
            let canonical = file.canonicalize().unwrap_or_else(|_| file.clone());
            if !self.seen.insert(canonical.clone()) {
                out.push('\n'); // already included: keep the line count
                self.locs.push(here);
                continue;
            }
            let text = std::fs::read_to_string(&file).map_err(|e| format!("import \"{}\": {}", path, e))?;
            self.total += text.len();
            if self.total > MAX_TOTAL_BYTES {
                return Err("imported files are too large".to_string());
            }
            let parent = canonical.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf());
            let shown = if canonical.starts_with(&self.base) {
                self.display_name(&canonical)
            } else {
                self.display_name(&file)
            };
            self.files.push(shown);
            self.sources.push(text.clone());
            let inner_idx = (self.files.len() - 1) as u32;
            let before = out.len();
            self.expand(&text, inner_idx, &parent, depth + 1, out)?;
            if !out[before..].ends_with('\n') {
                out.push('\n');
                self.locs.push(Loc { file: inner_idx, line: text.split('\n').count() as u32 });
            }
        }
        Ok(())
    }
}

/// Replace every `import "file"` line in `source` with the file's (recursively expanded) text.
pub fn resolve(source: &str) -> Result<String, String> {
    resolve_mapped(source).map(|(text, _)| text)
}

/// Like [`resolve`], also returning the origin (file, line) of every line of the result.
pub fn resolve_mapped(source: &str) -> Result<(String, SourceMap), String> {
    let main = main_file();
    if !source.contains("import") {
        return Ok((source.to_string(), SourceMap::identity(&main, source)));
    }
    let dir = base_dir();
    let canonical_base = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    let mut ex = Expander { seen: HashSet::new(), total: 0, base: canonical_base, files: vec![main], sources: vec![String::new()], locs: Vec::new() };
    let mut out = String::with_capacity(source.len());
    ex.expand(source, 0, &dir, 0, &mut out)?;
    // the (possibly empty) text after the last newline is the last line
    ex.locs.push(Loc { file: 0, line: source.split('\n').count() as u32 });
    ex.sources[0] = source.to_string();
    Ok((out, SourceMap { files: ex.files, locs: ex.locs, sources: std::rc::Rc::new(ex.sources) }))
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
    fn mapped_import_records_the_origin_of_every_line() {
        let d = temp_dir("mapped");
        std::fs::create_dir_all(d.join("lib")).unwrap();
        std::fs::write(d.join("lib/a.killer"), "import \"b.killer\"\nfn a() {\n  return b() + 1\n}").unwrap();
        std::fs::write(d.join("lib/b.killer"), "fn b() {\n  return 1\n}\n").unwrap();
        set_base_dir(&d);
        set_main_file("main.killer");
        let (out, map) = resolve_mapped("x = 1\nimport \"lib/a.killer\"\nprintln(a())\n").unwrap();
        assert_eq!(map.locs.len(), crate::sourcemap::line_count(&out), "{out}");
        let line_of = |needle: &str| out.split('\n').position(|l| l.contains(needle)).unwrap() + 1;
        assert_eq!(map.resolve(line_of("x = 1")), Some(("main.killer", 1)));
        assert_eq!(map.resolve(line_of("return 1")), Some(("lib/b.killer", 2)));
        assert_eq!(map.resolve(line_of("return b() + 1")), Some(("lib/a.killer", 3)));
        assert_eq!(map.resolve(line_of("println(a())")), Some(("main.killer", 3)));
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
