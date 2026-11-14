// File I/O Module for Killer Language
// Comprehensive file operations
// Version: 2.1.0

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// File I/O module providing 15+ file operations
/// Includes: read, write, exists, delete, list, and more
pub struct FileModule;

#[derive(Debug, Clone)]
pub enum FileError {
    NotFound,
    PermissionDenied,
    InvalidPath,
    IOError(String),
    InvalidEncoding,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FileError::NotFound => write!(f, "File not found"),
            FileError::PermissionDenied => write!(f, "Permission denied"),
            FileError::InvalidPath => write!(f, "Invalid path"),
            FileError::IOError(msg) => write!(f, "IO error: {}", msg),
            FileError::InvalidEncoding => write!(f, "Invalid UTF-8 encoding"),
        }
    }
}

pub type FileResult<T> = Result<T, FileError>;

impl FileModule {
    // ==================== Reading ====================
    
    /// Read entire file as string
    /// read_file("path/to/file.txt") => "contents..."
    pub fn read_file(path: &str) -> FileResult<String> {
        fs::read_to_string(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    io::ErrorKind::InvalidData => FileError::InvalidEncoding,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Read file as bytes
    /// read_bytes("path/to/file.bin") => [255, 0, 127, ...]
    pub fn read_bytes(path: &str) -> FileResult<Vec<u8>> {
        fs::read(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Read file as lines
    /// read_lines("path/to/file.txt") => ["line1", "line2", ...]
    pub fn read_lines(path: &str) -> FileResult<Vec<String>> {
        let content = Self::read_file(path)?;
        Ok(content
            .lines()
            .map(|line| line.to_string())
            .collect())
    }
    
    /// Read file in chunks (return first N lines)
    /// read_lines_chunked("path/to/file.txt", 10) => first 10 lines
    pub fn read_lines_chunked(path: &str, chunk_size: usize) -> FileResult<Vec<String>> {
        let content = Self::read_file(path)?;
        Ok(content
            .lines()
            .take(chunk_size)
            .map(|line| line.to_string())
            .collect())
    }
    
    // ==================== Writing ====================
    
    /// Write string to file (overwrite)
    /// write_file("path/to/file.txt", "content") => Ok(())
    pub fn write_file(path: &str, content: &str) -> FileResult<()> {
        fs::write(path, content)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Write bytes to file
    /// write_bytes("path/to/file.bin", bytes) => Ok(())
    pub fn write_bytes(path: &str, data: &[u8]) -> FileResult<()> {
        fs::write(path, data)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Append to file (create if not exists)
    /// append_file("path/to/file.txt", "content") => Ok(())
    pub fn append_file(path: &str, content: &str) -> FileResult<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })?;
        
        file.write_all(content.as_bytes())
            .map_err(|e| FileError::IOError(e.to_string()))
    }
    
    /// Write lines to file
    /// write_lines("path/to/file.txt", ["line1", "line2"]) => Ok(())
    pub fn write_lines(path: &str, lines: &[&str]) -> FileResult<()> {
        let content = lines.join("\n");
        Self::write_file(path, &content)
    }
    
    // ==================== File Metadata ====================
    
    /// Check if file exists
    /// exists("path/to/file.txt") => true/false
    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }
    
    /// Check if path is a file
    /// is_file("path/to/file.txt") => true/false
    pub fn is_file(path: &str) -> bool {
        Path::new(path).is_file()
    }
    
    /// Check if path is a directory
    /// is_directory("path/to/folder") => true/false
    pub fn is_directory(path: &str) -> bool {
        Path::new(path).is_dir()
    }
    
    /// Get file size in bytes
    /// file_size("path/to/file.txt") => 1024
    pub fn file_size(path: &str) -> FileResult<u64> {
        fs::metadata(path)
            .map(|m| m.len())
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Get file extension
    /// extension("path/to/file.txt") => "txt"
    pub fn extension(path: &str) -> Option<String> {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string())
    }
    
    /// Get file name without path
    /// file_name("path/to/file.txt") => "file.txt"
    pub fn file_name(path: &str) -> Option<String> {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }
    
    /// Get directory name
    /// dir_name("path/to/folder") => "folder"
    pub fn dir_name(path: &str) -> Option<String> {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }
    
    /// Get absolute path
    /// absolute_path("./file.txt") => "/full/path/to/file.txt"
    pub fn absolute_path(path: &str) -> FileResult<String> {
        std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    // ==================== Directory Operations ====================
    
    /// List files in directory
    /// list_dir("path/to/folder") => ["file1.txt", "file2.txt", ...]
    pub fn list_dir(path: &str) -> FileResult<Vec<String>> {
        let files = fs::read_dir(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    e.file_name().into_string().ok()
                })
            })
            .collect::<Vec<_>>();
        Ok(files)
    }
    
    /// List files with full paths
    /// list_dir_recursive("path/to/folder") => ["path/file1", "path/file2", ...]
    pub fn list_dir_recursive(path: &str) -> FileResult<Vec<String>> {
        let mut files = Vec::new();
        Self::walk_dir(path, &mut files)?;
        Ok(files)
    }
    
    fn walk_dir(path: &str, files: &mut Vec<String>) -> FileResult<()> {
        for entry in fs::read_dir(path)
            .map_err(|e| FileError::IOError(e.to_string()))?
        {
            let entry = entry.map_err(|e| FileError::IOError(e.to_string()))?;
            let path = entry.path();
            
            if path.is_file() {
                files.push(path.to_string_lossy().to_string());
            } else if path.is_dir() {
                Self::walk_dir(path.to_str().unwrap_or(""), files)?;
            }
        }
        Ok(())
    }
    
    /// Create directory
    /// mkdir("path/to/new/folder") => Ok(())
    pub fn mkdir(path: &str) -> FileResult<()> {
        fs::create_dir_all(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    // ==================== Deletion & Movement ====================
    
    /// Delete file
    /// delete_file("path/to/file.txt") => Ok(())
    pub fn delete_file(path: &str) -> FileResult<()> {
        fs::remove_file(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Delete directory (must be empty)
    /// delete_dir("path/to/folder") => Ok(())
    pub fn delete_dir(path: &str) -> FileResult<()> {
        fs::remove_dir(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Delete directory recursively (with contents)
    /// delete_dir_recursive("path/to/folder") => Ok(())
    pub fn delete_dir_recursive(path: &str) -> FileResult<()> {
        fs::remove_dir_all(path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Rename/move file
    /// rename("old.txt", "new.txt") => Ok(())
    pub fn rename(old_path: &str, new_path: &str) -> FileResult<()> {
        fs::rename(old_path, new_path)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })
    }
    
    /// Copy file
    /// copy_file("source.txt", "dest.txt") => Ok(())
    pub fn copy_file(src: &str, dst: &str) -> FileResult<()> {
        fs::copy(src, dst)
            .map_err(|e| {
                match e.kind() {
                    io::ErrorKind::NotFound => FileError::NotFound,
                    io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
                    _ => FileError::IOError(e.to_string()),
                }
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_exists() {
        // Test with this source file itself
        assert!(FileModule::exists("src/file_io.rs") || FileModule::exists("src/lib.rs"));
    }
    
    #[test]
    fn test_extension() {
        assert_eq!(FileModule::extension("file.txt"), Some("txt".to_string()));
        assert_eq!(FileModule::extension("path/to/file.rs"), Some("rs".to_string()));
    }
    
    #[test]
    fn test_file_name() {
        assert_eq!(FileModule::file_name("path/to/file.txt"), Some("file.txt".to_string()));
        assert_eq!(FileModule::file_name("file.rs"), Some("file.rs".to_string()));
    }
    
    #[test]
    fn test_read_write() {
        let test_path = "/tmp/killer_test.txt";
        let content = "Hello, Killer!";
        
        // Write test
        assert!(FileModule::write_file(test_path, content).is_ok());
        
        // Verify exists
        assert!(FileModule::exists(test_path));
        assert!(FileModule::is_file(test_path));
        
        // Read test
        let read_result = FileModule::read_file(test_path);
        if let Ok(read_content) = read_result {
            assert_eq!(read_content, content);
        }
        
        // Cleanup
        _ = FileModule::delete_file(test_path);
    }
}
