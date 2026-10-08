//! Package file handling: which files make up a package, the content hash recorded in
//! `killer.lock`, and the plain archive format used by registries.
//!
//! Content hash: SHA-256 over every package file in sorted path order, each contributing
//! `path NUL length NUL bytes`. It depends only on file names and contents, so a package
//! directory, its archive and its installed copy all hash the same.
//!
//! Archive (`package.pack`): `KPK1\n`, then per file `<byte length> <relative path>\n<bytes>\n`.

use std::path::{Path, PathBuf};

pub fn sha256_hex(data: &[u8]) -> String {
    crate::builtin::sha256_digest(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Directory names that are never part of a package.
const SKIP_DIRS: &[&str] = &["packages", "target", "node_modules"];
const SKIP_FILES: &[&str] = &["killer.lock"];

/// Files of the package rooted at `dir` as (relative path with `/`, absolute path), sorted.
pub fn collect_files(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut out = Vec::new();
    walk(dir, "", &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {}", dir.display(), e))?;
    for entry in rd {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // .killer, .git, .gitignore, ...
        }
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_symlink() {
            continue;
        }
        let rel = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        if ft.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(&entry.path(), &rel, out)?;
        } else if ft.is_file() && !SKIP_FILES.contains(&name.as_str()) {
            out.push((rel, entry.path()));
        }
    }
    Ok(())
}

pub fn read_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    collect_files(dir)?
        .into_iter()
        .map(|(rel, abs)| std::fs::read(&abs).map(|b| (rel, b)).map_err(|e| format!("cannot read {}: {}", abs.display(), e)))
        .collect()
}

pub fn content_hash(files: &[(String, Vec<u8>)]) -> String {
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut buf = Vec::new();
    for (p, b) in sorted {
        buf.extend_from_slice(p.as_bytes());
        buf.push(0);
        buf.extend_from_slice(b.len().to_string().as_bytes());
        buf.push(0);
        buf.extend_from_slice(b);
    }
    sha256_hex(&buf)
}

pub fn hash_dir(dir: &Path) -> Result<String, String> {
    Ok(content_hash(&read_files(dir)?))
}

pub fn pack(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"KPK1\n".to_vec();
    for (p, b) in files {
        out.extend_from_slice(format!("{} {}\n", b.len(), p).as_bytes());
        out.extend_from_slice(b);
        out.push(b'\n');
    }
    out
}

fn safe_rel_path(p: &str) -> bool {
    !p.is_empty()
        && !p.contains('\\')
        && !p.contains(':')
        && !p.starts_with('/')
        && p.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

pub fn unpack(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bad = |m: &str| format!("corrupt package archive: {}", m);
    let rest = bytes.strip_prefix(b"KPK1\n").ok_or_else(|| bad("missing header"))?;
    let mut pos = 0usize;
    let mut out = Vec::new();
    while pos < rest.len() {
        let nl = rest[pos..].iter().position(|&b| b == b'\n').ok_or_else(|| bad("truncated header"))? + pos;
        let head = std::str::from_utf8(&rest[pos..nl]).map_err(|_| bad("bad header"))?;
        let (len, path) = head.split_once(' ').ok_or_else(|| bad("bad header"))?;
        let len: usize = len.parse().map_err(|_| bad("bad length"))?;
        if !safe_rel_path(path) {
            return Err(format!("unsafe path in package archive: {}", path));
        }
        let start = nl + 1;
        if start + len + 1 > rest.len() || rest[start + len] != b'\n' {
            return Err(bad("truncated file"));
        }
        out.push((path.to_string(), rest[start..start + len].to_vec()));
        pos = start + len + 1;
    }
    Ok(out)
}

/// Write files below `dest` (created fresh: an existing `dest` is removed first).
pub fn write_files(dest: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| format!("cannot replace {}: {}", dest.display(), e))?;
    }
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for (rel, bytes) in files {
        if !safe_rel_path(rel) {
            return Err(format!("unsafe path in package: {}", rel));
        }
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&target, bytes).map_err(|e| format!("cannot write {}: {}", target.display(), e))?;
    }
    Ok(())
}

/// Lexically normalize a path (resolve `.` and `..` without touching the disk).
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_order_independent_and_content_sensitive() {
        let a = vec![("b.killer".to_string(), b"2".to_vec()), ("a.killer".to_string(), b"1".to_vec())];
        let b = vec![("a.killer".to_string(), b"1".to_vec()), ("b.killer".to_string(), b"2".to_vec())];
        assert_eq!(content_hash(&a), content_hash(&b));
        let c = vec![("a.killer".to_string(), b"1".to_vec()), ("b.killer".to_string(), b"3".to_vec())];
        assert_ne!(content_hash(&a), content_hash(&c));
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn pack_round_trips_and_rejects_traversal() {
        let files = vec![("src/a.killer".to_string(), b"line\nline2".to_vec()), ("killer.toml".to_string(), Vec::new())];
        assert_eq!(unpack(&pack(&files)).unwrap(), files);
        let evil = pack(&[("../x".to_string(), b"x".to_vec())]);
        assert!(unpack(&evil).unwrap_err().contains("unsafe"));
        assert!(unpack(b"junk").is_err());
    }
}
