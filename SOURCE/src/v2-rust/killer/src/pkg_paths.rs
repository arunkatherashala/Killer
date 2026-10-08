//! Where `import name` / `import "name/sub.killer"` find packages at compile time.
//!
//! For a package `name`, after the project's own files, the search order is, for each directory
//! from the script's folder up to the project root (the nearest `killer.toml`):
//!   1. `<dir>/packages/<name>/`
//!   2. `<dir>/.killer/packages/<name>/`
//! and then the workspace path dependencies declared in the root `killer.toml` and recorded as
//! `path+...` sources in `killer.lock`.

use crate::pkg_files::normalize;
use crate::pkg_manifest::{DepSource, Lockfile, Manifest};
use std::path::{Path, PathBuf};

/// Directories (in search order) that could hold the package `name`, as (description, dir).
pub fn package_dirs(name: &str, base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root = crate::pkg_manifest::find_root(base);
    let mut cur = Some(base.to_path_buf());
    while let Some(d) = cur {
        out.push(d.join("packages").join(name));
        out.push(d.join(".killer").join("packages").join(name));
        if root.as_deref() == Some(d.as_path()) || root.is_none() {
            break;
        }
        cur = d.parent().map(Path::to_path_buf);
    }
    if let Some(root) = &root {
        if let Ok(m) = Manifest::load(root) {
            for d in m.dependencies.iter().chain(m.dev_dependencies.iter()) {
                if d.name == name {
                    if let DepSource::Path(p) = &d.source {
                        out.push(normalize(&root.join(p)));
                    }
                }
            }
        }
        if let Ok(Some(lock)) = Lockfile::load(root) {
            if let Some(p) = lock.get(name) {
                if let Some(rel) = p.source.strip_prefix("path+") {
                    out.push(normalize(&root.join(rel)));
                }
            }
        }
    }
    out
}

/// The entry file of the package in `dir`: `entry` from its killer.toml, else conventional names.
pub fn entry_of(dir: &Path, name: &str) -> Option<PathBuf> {
    if let Ok(m) = Manifest::load(dir) {
        if let Some(e) = m.entry_file(dir) {
            return Some(e);
        }
    }
    ["src/lib.killer", "src/main.killer", "lib.killer", "main.killer"]
        .iter()
        .map(|c| dir.join(c))
        .chain(std::iter::once(dir.join(format!("{}.killer", name))))
        .find(|p| p.is_file())
}

fn file_in(dir: &Path, sub: &str) -> Option<PathBuf> {
    [dir.join(sub), dir.join(format!("{}.killer", sub)), dir.join("src").join(sub), dir.join("src").join(format!("{}.killer", sub))]
        .into_iter()
        .find(|p| p.is_file())
}

/// Find `spec` (`name` or `name/sub/path.killer`) in a package. On failure returns the places
/// that were searched.
pub fn find(spec: &str, base: &Path) -> Result<PathBuf, Vec<String>> {
    let (name, sub) = match spec.split_once('/') {
        Some((n, s)) => (n, s),
        None => (spec, ""),
    };
    if name.is_empty() || name == "." || name == ".." {
        return Err(vec![]);
    }
    let mut tried = Vec::new();
    for dir in package_dirs(name, base) {
        let hit = if dir.is_dir() {
            if sub.is_empty() { entry_of(&dir, name) } else { file_in(&dir, sub) }
        } else {
            None
        };
        match hit {
            Some(f) => return Ok(f),
            None => tried.push(dir.to_string_lossy().replace('\\', "/")),
        }
    }
    Err(tried)
}
