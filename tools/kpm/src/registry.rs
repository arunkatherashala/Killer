use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub files: Vec<String>,
}

pub fn install_from_local(path: &str, dest_dir: &str) -> Result<()> {
    let p = PathBuf::from(path);
    let data = fs::read(&p).with_context(|| format!("reading package {}", path))?;
    // For the stub, expect JSON manifest file
    let manifest: PackageManifest = serde_json::from_slice(&data).with_context(|| "parse manifest")?;
    let dest = PathBuf::from(dest_dir).join(format!("{}-{}", manifest.name, manifest.version));
    fs::create_dir_all(&dest)?;
    for f in manifest.files.iter() {
        let src = p.parent().unwrap_or(&PathBuf::from(".")).join(f);
        let dst = dest.join(f);
        if let Some(parent) = dst.parent() { fs::create_dir_all(parent)?; }
        fs::copy(&src, &dst).with_context(|| format!("copy {}", f))?;
    }
    println!("Installed {} {} -> {}", manifest.name, manifest.version, dest.display());
    Ok(())
}
