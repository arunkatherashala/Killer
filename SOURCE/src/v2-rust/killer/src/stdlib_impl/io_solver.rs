// ================================================================
// FILE I/O & STREAM SOLVER - Phase 21.5
// File operations, streams, buffering, serialization
// ================================================================

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write, BufRead, BufReader, BufWriter, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub type FileHandle = File;
pub type Result<T> = io::Result<T>;

/// File I/O and Stream Operations Solver
pub struct IoSolver;

impl IoSolver {
    // ================================================================
    // FILE OPERATIONS (1-20)
    // ================================================================

    /// Problem 1: Read entire file into string
    pub fn read_file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
        fs::read_to_string(path)
    }

    /// Problem 2: Write string to file
    pub fn write_string_to_file<P: AsRef<Path>>(path: P, contents: &str) -> Result<()> {
        fs::write(path, contents)
    }

    /// Problem 3: Read file as bytes
    pub fn read_file_to_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        fs::read(path)
    }

    /// Problem 4: Write bytes to file
    pub fn write_bytes_to_file<P: AsRef<Path>>(path: P, bytes: &[u8]) -> Result<()> {
        fs::write(path, bytes)
    }

    /// Problem 5: Append to file
    pub fn append_to_file<P: AsRef<Path>>(path: P, contents: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    /// Problem 6: Create directory
    pub fn create_directory<P: AsRef<Path>>(path: P) -> Result<()> {
        fs::create_dir_all(path)
    }

    /// Problem 7: Check if file exists
    pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    /// Problem 8: Delete file
    pub fn delete_file<P: AsRef<Path>>(path: P) -> Result<()> {
        fs::remove_file(path)
    }

    /// Problem 9: Delete directory (recursive)
    pub fn delete_directory<P: AsRef<Path>>(path: P) -> Result<()> {
        fs::remove_dir_all(path)
    }

    /// Problem 10: List directory contents
    pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(path)?;
        entries
            .map(|e| e.map(|entry| entry.path()))
            .collect()
    }

    /// Problem 11: Copy file
    pub fn copy_file<P: AsRef<Path>>(src: P, dst: P) -> Result<u64> {
        fs::copy(src, dst)
    }

    /// Problem 12: Rename/Move file
    pub fn rename_file<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
        fs::rename(src, dst)
    }

    /// Problem 13: Get file size
    pub fn file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
        Ok(fs::metadata(path)?.len())
    }

    /// Problem 14: Check if path is directory
    pub fn is_directory<P: AsRef<Path>>(path: P) -> bool {
        fs::metadata(path)
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Problem 15: Check if path is file
    pub fn is_file<P: AsRef<Path>>(path: P) -> bool {
        fs::metadata(path)
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    // ================================================================
    // BUFFERED STREAMS (16-30)
    // ================================================================

    /// Problem 16: Read file lines into vector
    pub fn read_lines<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        
        for line_result in reader.lines() {
            let line = line_result?;
            lines.push(line);
        }
        Ok(lines)
    }

    /// Problem 17: Count lines in file
    pub fn line_count<P: AsRef<Path>>(path: P) -> Result<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(reader.lines().count())
    }

    /// Problem 18: Write lines to file
    pub fn write_lines<P: AsRef<Path>>(path: P, lines: &[&str]) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        for line in lines {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    /// Problem 19: Buffered read with custom size
    pub fn buffered_read<P: AsRef<Path>>(path: P, buffer_size: usize) -> Result<Vec<Vec<u8>>> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(buffer_size, file);
        let mut buffers = Vec::new();
        let mut buf = vec![0; buffer_size];
        
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 { break; }
            buffers.push(buf[..n].to_vec());
        }
        Ok(buffers)
    }

    /// Problem 20: Buffered write with custom size
    pub fn buffered_write<P: AsRef<Path>>(path: P, data: &[u8], buffer_size: usize) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::with_capacity(buffer_size, file);
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    // ================================================================
    // SERIALIZATION (21-35)
    // ================================================================

    /// Problem 21: JSON-like serialization (simple)
    pub fn serialize_map(map: &std::collections::HashMap<String, String>) -> String {
        let mut result = String::from("{");
        for (k, v) in map {
            result.push_str(&format!("\"{}\":\"{}\",", k, v));
        }
        if result.len() > 1 {
            result.pop(); // Remove trailing comma
        }
        result.push('}');
        result
    }

    /// Problem 22: CSV-like formatting
    pub fn format_csv(records: &[Vec<&str>]) -> String {
        records
            .iter()
            .map(|r| r.join(","))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Problem 23: Parse CSV-like data
    pub fn parse_csv(data: &str) -> Vec<Vec<String>> {
        data.lines()
            .map(|line| {
                line.split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .collect()
    }

    /// Problem 24: Escape string for file path
    pub fn escape_path(s: &str) -> String {
        s.replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
    }

    /// Problem 25: Unescape file path
    pub fn unescape_path(s: &str) -> String {
        s.replace("\\\\", "\\")
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\r", "\r")
    }

    // ================================================================
    // SEEKING & POSITIONING (26-35)
    // ================================================================

    /// Problem 26: Seek to position in file
    pub fn seek_to_position<P: AsRef<Path>>(path: P, pos: u64) -> Result<u64> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(pos))
    }

    /// Problem 27: Get current file position
    pub fn file_position(file: &mut File) -> Result<u64> {
        file.stream_position()
    }

    /// Problem 28: Read file chunk at offset
    pub fn read_chunk<P: AsRef<Path>>(path: P, offset: u64, size: usize) -> Result<Vec<u8>> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        
        let mut buf = vec![0; size];
        let n = file.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Problem 29: Split file into chunks
    pub fn split_file<P: AsRef<Path>>(path: P, chunk_size: usize) -> Result<Vec<Vec<u8>>> {
        let mut file = File::open(path)?;
        let mut chunks = Vec::new();
        let mut buf = vec![0; chunk_size];
        
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            chunks.push(buf[..n].to_vec());
        }
        Ok(chunks)
    }

    /// Problem 30: Merge file chunks
    pub fn merge_chunks<P: AsRef<Path>>(dest: P, chunks: &[Vec<u8>]) -> Result<()> {
        let mut file = File::create(dest)?;
        for chunk in chunks {
            file.write_all(chunk)?;
        }
        Ok(())
    }

    // ================================================================
    // BINARY I/O (31-42)
    // ================================================================

    /// Problem 31: Write u32 in little-endian
    pub fn write_u32_le<P: AsRef<Path>>(path: P, value: u32) -> Result<()> {
        let bytes = value.to_le_bytes();
        fs::write(path, &bytes)
    }

    /// Problem 32: Read u32 in little-endian
    pub fn read_u32_le<P: AsRef<Path>>(path: P) -> Result<u32> {
        let bytes = fs::read(path)?;
        if bytes.len() < 4 { return Ok(0); }
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Problem 33: Write f64 in little-endian
    pub fn write_f64_le<P: AsRef<Path>>(path: P, value: f64) -> Result<()> {
        let bytes = value.to_le_bytes();
        fs::write(path, &bytes)
    }

    /// Problem 34: Read f64 in little-endian
    pub fn read_f64_le<P: AsRef<Path>>(path: P) -> Result<f64> {
        let bytes = fs::read(path)?;
        if bytes.len() < 8 { return Ok(0.0); }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[..8]);
        Ok(f64::from_le_bytes(arr))
    }

    /// Problem 35: Write byte array with length prefix
    pub fn write_prefixed_bytes<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
        let mut file = File::create(path)?;
        let len = data.len() as u32;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(data)?;
        Ok(())
    }

    /// Problem 36: Read prefixed bytes
    pub fn read_prefixed_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        let bytes = fs::read(path)?;
        if bytes.len() < 4 { return Ok(Vec::new()); }
        
        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if bytes.len() < 4 + len { return Ok(Vec::new()); }
        
        Ok(bytes[4..4 + len].to_vec())
    }

    /// Problem 37: Hex dump of file
    pub fn hex_dump<P: AsRef<Path>>(path: P, max_bytes: usize) -> Result<String> {
        let bytes = fs::read(path)?;
        let mut result = String::new();
        
        for (i, chunk) in bytes.iter().take(max_bytes).collect::<Vec<_>>().chunks(16).enumerate() {
            result.push_str(&format!("{:04x}: ", i * 16));
            for &b in chunk {
                result.push_str(&format!("{:02x} ", b));
            }
            result.push('\n');
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_read_string() {
        let path = "test_io_1.txt";
        let content = "Hello, World!";
        
        let _ = IoSolver::write_string_to_file(path, content);
        let read_content = IoSolver::read_file_to_string(path).unwrap();
        
        assert_eq!(content, read_content);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_file_operations() {
        let path = "test_dir";
        let _ = IoSolver::create_directory(path);
        assert!(IoSolver::file_exists(path));
        
        let _ = IoSolver::delete_directory(path);
        assert!(!IoSolver::file_exists(path));
    }

    #[test]
    fn test_csv_parsing() {
        let csv = "a,b,c\n1,2,3\n4,5,6";
        let records = IoSolver::parse_csv(csv);
        
        assert_eq!(records.len(), 3);
        assert_eq!(records[0], vec!["a", "b", "c"]);
    }

    #[test]
    fn test_binary_io() {
        let path = "test_bin.dat";
        let value = 0x12345678u32;
        
        let _ = IoSolver::write_u32_le(path, value);
        let read_value = IoSolver::read_u32_le(path).unwrap();
        
        assert_eq!(value, read_value);
        let _ = fs::remove_file(path);
    }
}
