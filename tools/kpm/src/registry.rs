use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::io::Read;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Archive;
use tar::Builder;
use std::fs::File as StdFile;
use std::path::Path;

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

/// Fetch a package from a remote URL and verify its SHA256 checksum from a manifest.
pub fn install_from_remote(url: &str, expected_sha256_hex: &str, dest_dir: &str) -> Result<()> {
    let client = Client::new();
    let mut resp = client.get(url).send().with_context(|| format!("GET {}", url))?;
    if !resp.status().is_success() { anyhow::bail!("failed download: {}", resp.status()); }
    let mut buf: Vec<u8> = Vec::new();
    resp.read_to_end(&mut buf)?;
    // compute sha256
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let got = hasher.finalize();
    let got_hex = hex::encode(got);
    if got_hex != expected_sha256_hex {
        anyhow::bail!("sha256 mismatch: expected {} got {}", expected_sha256_hex, got_hex);
    }
    // write to temp file and attempt to unpack if it's an archive, otherwise treat as manifest
    let tmp = PathBuf::from(dest_dir).join(".kpm_tmp");
    fs::create_dir_all(&tmp)?;
    let out = tmp.join("payload.bin");
    fs::write(&out, &buf)?;
    println!("Downloaded and verified {} bytes to {}", buf.len(), out.display());
    // Attempt to unpack tar.gz into dest_dir
    let file = std::fs::File::open(&out)?;
    let gz = GzDecoder::new(file);
    let mut ar = Archive::new(gz);
    ar.unpack(dest_dir).with_context(|| "unpack archive")?;
    Ok(())
}

/// Create a tar.gz archive from manifest files and return (path, sha256_hex).
pub fn publish_package(manifest_path: &str, output_dir: &str) -> Result<(String, String)> {
    let manifest_file = std::fs::read_to_string(manifest_path).with_context(|| "read manifest")?;
    let manifest: PackageManifest = serde_json::from_str(&manifest_file)?;
    let base_dir = Path::new(manifest_path).parent().unwrap_or(&PathBuf::from("."));
    let tar_path = PathBuf::from(output_dir).join(format!("{}-{}.tar.gz", manifest.name, manifest.version));
    let tar_file = StdFile::create(&tar_path)?;
    let gz = GzEncoder::new(tar_file, Compression::default());
    let mut ar = Builder::new(gz);
    for f in manifest.files.iter() {
        let src = base_dir.join(f);
        let mut file = StdFile::open(&src).with_context(|| format!("open {}", f))?;
        ar.append_file(f, &mut file).with_context(|| format!("add {} to archive", f))?;
    }
    ar.finish()?;
    let tar_bytes = std::fs::read(&tar_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&tar_bytes);
    let sha = hex::encode(hasher.finalize());
    println!("Published {} {} -> {} (sha256: {})", manifest.name, manifest.version, tar_path.display(), sha);
    Ok((tar_path.to_string_lossy().to_string(), sha))
}

/// Simple version resolver: exact match or wildcard.
pub fn resolve_deps(pkg_name: &str, required_version: &str, available: &[(String, String)]) -> Result<String> {
    for (name, ver) in available {
        if name == pkg_name {
            if ver == required_version || required_version.contains('*') {
                return Ok(ver.clone());
            }
        }
    }
    anyhow::bail!("no matching version for {}@{}", pkg_name, required_version)
}
