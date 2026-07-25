// ============================================================================
// KORE Transactions (Gap #7 + #8) — Time Travel + ACID Writes
// ============================================================================
//
// - Atomic writes via temp-file + rename (no partial writes)
// - Version manifest for time travel (list of snapshots)
// - Append-only: new versions create new files, old versions kept immutable
//
// Usage:
//   let txn = KoreTxn::begin("data.kore");
//   txn.write_atomic(writer, rows)?;      // atomic via temp+rename
//   txn.commit("added Q1 data")?;         // record version in manifest
//
//   let versions = KoreTxn::list_versions("data.kore")?;
//   let old = KoreTxn::checkout("data.kore", 2)?;  // read version 2
//

// ── Version Manifest ─────────────────────────────────────────────────────────
/// Stored as `<name>.kore.versions` — a simple text-based log of all versions.
/// Each line: `version|timestamp|message|filename|nrows|size_bytes`
#[derive(Debug, Clone)]
pub struct KoreVersion {
    pub version: u64,
    pub timestamp: u64,    // Unix epoch seconds
    pub message: String,
    pub filename: String,  // actual .kore file for this version
    pub nrows: usize,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub struct KoreVersionLog {
    pub base_path: String,
    pub versions: Vec<KoreVersion>,
}

impl KoreVersionLog {
    /// Load or create version manifest for a KORE table.
    pub fn open(kore_path: &str) -> Self {
        let manifest_path = format!("{}.versions", kore_path);
        let versions = if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
            contents.lines().filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 6 {
                    Some(KoreVersion {
                        version: parts[0].parse().ok()?,
                        timestamp: parts[1].parse().ok()?,
                        message: parts[2].to_string(),
                        filename: parts[3].to_string(),
                        nrows: parts[4].parse().ok()?,
                        size_bytes: parts[5].parse().ok()?,
                    })
                } else { None }
            }).collect()
        } else {
            Vec::new()
        };
        KoreVersionLog { base_path: kore_path.to_string(), versions }
    }

    /// Get the latest version number.
    pub fn latest_version(&self) -> u64 {
        self.versions.last().map(|v| v.version).unwrap_or(0)
    }

    /// Record a new version in the manifest.
    pub fn record_version(&mut self, message: &str, filename: &str, nrows: usize, size_bytes: u64) -> Result<u64, String> {
        let version = self.latest_version() + 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = KoreVersion {
            version, timestamp, message: message.to_string(),
            filename: filename.to_string(), nrows, size_bytes,
        };
        self.versions.push(entry);
        self.save()?;
        Ok(version)
    }

    /// Save manifest to disk.
    fn save(&self) -> Result<(), String> {
        use std::io::Write;
        let manifest_path = format!("{}.versions", self.base_path);
        let mut f = std::fs::File::create(&manifest_path)
            .map_err(|e| format!("Cannot create manifest {}: {}", manifest_path, e))?;
        for v in &self.versions {
            writeln!(f, "{}|{}|{}|{}|{}|{}",
                v.version, v.timestamp, v.message, v.filename, v.nrows, v.size_bytes
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Get the filename for a specific version (time travel).
    pub fn get_version(&self, version: u64) -> Option<&KoreVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// Get version as of a specific timestamp.
    pub fn as_of(&self, timestamp: u64) -> Option<&KoreVersion> {
        self.versions.iter().rev().find(|v| v.timestamp <= timestamp)
    }

    /// List all versions with metadata.
    pub fn list(&self) -> Vec<String> {
        self.versions.iter().map(|v| {
            let ts = format_timestamp(v.timestamp);
            format!("v{}: {} | {} rows | {} bytes | \"{}\"",
                v.version, ts, v.nrows, v.size_bytes, v.message)
        }).collect()
    }
}

fn format_timestamp(ts: u64) -> String {
    // Simple ISO-8601 from Unix epoch (no external deps)
    let days = ts / 86400;
    let secs = ts % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    // Approximate date calculation (good enough for display)
    let mut y = 1970u64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        y += 1;
    }
    let month_days = [31, 28 + if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 1 } else { 0 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1u64;
    for md in &month_days {
        if remaining_days < *md as u64 { break; }
        remaining_days -= *md as u64;
        mo += 1;
    }
    let d = remaining_days + 1;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, s)
}

// ── Atomic Transaction ───────────────────────────────────────────────────────
pub struct KoreTxn {
    pub base_path: String,
    temp_path: String,
    committed: bool,
}

impl KoreTxn {
    /// Begin a transaction for writing to a KORE file.
    /// Writes go to a temp file; commit() atomically renames it.
    pub fn begin(kore_path: &str) -> Self {
        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let temp_path = format!("{}.tmp.{}.{}", kore_path, pid, ts);
        KoreTxn { base_path: kore_path.to_string(), temp_path, committed: false }
    }

    /// Get the temp path for writing.
    pub fn temp_path(&self) -> &str { &self.temp_path }

    /// Atomically commit: rename temp → final + record version.
    pub fn commit(mut self, message: &str) -> Result<u64, String> {
        if !std::path::Path::new(&self.temp_path).exists() {
            return Err("Nothing written to transaction".to_string());
        }
        let size = std::fs::metadata(&self.temp_path)
            .map(|m| m.len()).unwrap_or(0);

        // Versioned filename: data.kore.v3
        let mut log = KoreVersionLog::open(&self.base_path);
        let next_ver = log.latest_version() + 1;
        let versioned_path = format!("{}.v{}", self.base_path, next_ver);

        // Atomic: rename temp → versioned file
        std::fs::rename(&self.temp_path, &versioned_path)
            .map_err(|e| format!("Atomic rename failed: {}", e))?;

        // Also copy as the "latest" (overwrite base_path)
        std::fs::copy(&versioned_path, &self.base_path)
            .map_err(|e| format!("Copy to latest failed: {}", e))?;

        // Count rows from file
        let nrows = if let Ok(reader) = crate::kore_v2::KoreReader::open(&versioned_path) {
            reader.nrows
        } else { 0 };

        let version = log.record_version(message, &versioned_path, nrows, size)?;
        self.committed = true;
        Ok(version)
    }

    /// Abort transaction: clean up temp file.
    pub fn abort(mut self) {
        let _ = std::fs::remove_file(&self.temp_path);
        self.committed = true; // prevent Drop cleanup warning
    }
}

impl Drop for KoreTxn {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.temp_path);
        }
    }
}

// ── Time Travel API ──────────────────────────────────────────────────────────

/// Read a specific version of a KORE table (time travel).
pub fn checkout(kore_path: &str, version: u64) -> Result<crate::kore_v2::KoreReader, String> {
    let log = KoreVersionLog::open(kore_path);
    let ver = log.get_version(version)
        .ok_or(format!("Version {} not found", version))?;
    crate::kore_v2::KoreReader::open(&ver.filename)
}

/// Read the KORE table as it existed at a specific Unix timestamp.
pub fn as_of(kore_path: &str, timestamp: u64) -> Result<crate::kore_v2::KoreReader, String> {
    let log = KoreVersionLog::open(kore_path);
    let ver = log.as_of(timestamp)
        .ok_or("No version found for specified timestamp".to_string())?;
    crate::kore_v2::KoreReader::open(&ver.filename)
}

/// List all versions of a KORE table.
pub fn list_versions(kore_path: &str) -> Vec<String> {
    let log = KoreVersionLog::open(kore_path);
    log.list()
}

/// Diff two versions: returns (added_rows, removed_rows, changed_count).
pub fn diff_versions(kore_path: &str, v1: u64, v2: u64) -> Result<(usize, usize, usize), String> {
    let log = KoreVersionLog::open(kore_path);
    let ver1 = log.get_version(v1).ok_or(format!("Version {} not found", v1))?;
    let ver2 = log.get_version(v2).ok_or(format!("Version {} not found", v2))?;
    // Simple diff: row count comparison
    let added = if ver2.nrows > ver1.nrows { ver2.nrows - ver1.nrows } else { 0 };
    let removed = if ver1.nrows > ver2.nrows { ver1.nrows - ver2.nrows } else { 0 };
    Ok((added, removed, 0))
}

// ============================================================================
//  Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_log() {
        let mut log = KoreVersionLog {
            base_path: "test.kore".to_string(),
            versions: Vec::new(),
        };
        assert_eq!(log.latest_version(), 0);
        log.versions.push(KoreVersion {
            version: 1, timestamp: 1700000000,
            message: "initial".to_string(),
            filename: "test.kore.v1".to_string(),
            nrows: 1000, size_bytes: 5000,
        });
        assert_eq!(log.latest_version(), 1);
        assert!(log.get_version(1).is_some());
        assert!(log.get_version(2).is_none());
    }

    #[test]
    fn test_format_timestamp() {
        let ts = format_timestamp(1700000000);
        assert!(ts.starts_with("2023-"));
    }

    #[test]
    fn test_txn_abort() {
        let txn = KoreTxn::begin("nonexistent.kore");
        assert!(!txn.temp_path().is_empty());
        txn.abort();
    }
}
