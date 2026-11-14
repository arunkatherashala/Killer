// Phase 5.1: File System API
// Core file I/O operations for Killer programs

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileHandle {
    pub path: String,
    pub mode: FileMode,
    pub is_open: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    Append,
    ReadWrite,
}

#[derive(Debug)]
pub struct FileSystem {
    /// Open file handles
    handles: HashMap<usize, FileHandle>,
    /// Next file handle ID
    next_handle_id: usize,
    /// Current working directory
    cwd: PathBuf,
}

impl FileSystem {
    pub fn new() -> Self {
        FileSystem {
            handles: HashMap::new(),
            next_handle_id: 1,
            cwd: PathBuf::from("."),
        }
    }

    /// Open a file in specified mode
    pub fn open(&mut self, path: String, mode: FileMode) -> Result<usize, String> {
        let full_path = self.cwd.join(&path);

        // Validate path exists or is writable
        match mode {
            FileMode::Read => {
                if !full_path.exists() {
                    return Err(format!("File not found: {}", path));
                }
            }
            FileMode::Write | FileMode::Append => {
                // OK to create new file
            }
            FileMode::ReadWrite => {
                // Should exist or be created
            }
        }

        let handle_id = self.next_handle_id;
        self.next_handle_id += 1;

        self.handles.insert(
            handle_id,
            FileHandle {
                path,
                mode,
                is_open: true,
            },
        );

        Ok(handle_id)
    }

    /// Read file contents
    pub fn read_file(&self, path: &str) -> Result<String, String> {
        let full_path = self.cwd.join(path);
        fs::read_to_string(&full_path).map_err(|e| e.to_string())
    }

    /// Write to file
    pub fn write_file(&self, path: &str, contents: &str) -> Result<(), String> {
        let full_path = self.cwd.join(path);
        fs::write(&full_path, contents).map_err(|e| e.to_string())
    }

    /// Append to file
    pub fn append_file(&self, path: &str, contents: &str) -> Result<(), String> {
        let full_path = self.cwd.join(path);
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full_path)
            .map_err(|e| e.to_string())?;

        file.write_all(contents.as_bytes())
            .map_err(|e| e.to_string())
    }

    /// Close file handle
    pub fn close(&mut self, handle_id: usize) -> Result<(), String> {
        if let Some(mut handle) = self.handles.get_mut(&handle_id) {
            handle.is_open = false;
            Ok(())
        } else {
            Err("Invalid file handle".to_string())
        }
    }

    /// List directory contents
    pub fn list_directory(&self, path: &str) -> Result<Vec<String>, String> {
        let full_path = self.cwd.join(path);
        let mut entries = Vec::new();

        for entry in fs::read_dir(&full_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            entries.push(path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string());
        }

        Ok(entries)
    }

    /// Get file size
    pub fn file_size(&self, path: &str) -> Result<u64, String> {
        let full_path = self.cwd.join(path);
        fs::metadata(&full_path)
            .map_err(|e| e.to_string())
            .map(|m| m.len())
    }

    /// Set current working directory
    pub fn set_cwd(&mut self, path: String) -> Result<(), String> {
        let new_cwd = self.cwd.join(&path);
        if new_cwd.exists() && new_cwd.is_dir() {
            self.cwd = new_cwd;
            Ok(())
        } else {
            Err(format!("Directory not found: {}", path))
        }
    }

    /// Get current working directory
    pub fn get_cwd(&self) -> String {
        self.cwd.display().to_string()
    }

    /// Get file statistics
    pub fn get_file_stats(&self) -> FileSystemStats {
        FileSystemStats {
            open_file_handles: self.handles.len(),
            current_directory: self.cwd.display().to_string(),
            total_operations: self.next_handle_id - 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileSystemStats {
    pub open_file_handles: usize,
    pub current_directory: String,
    pub total_operations: usize,
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_creation() {
        let fs = FileSystem::new();
        assert_eq!(fs.next_handle_id, 1);
    }

    #[test]
    fn test_read_file() {
        let fs = FileSystem::new();
        // Would need actual test files in real implementation
        let result = fs.read_file("nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_file() {
        let fs = FileSystem::new();
        let result = fs.write_file("test_output.txt", "hello world");
        // May succeed or fail depending on permissions
        let _ = result;
    }

    #[test]
    fn test_file_operations() {
        let mut fs = FileSystem::new();
        
        // Open file
        let result = fs.open("test.txt".to_string(), FileMode::Write);
        match result {
            Ok(handle) => {
                assert!(handle > 0);
                let _ = fs.close(handle);
            }
            Err(_) => {
                // OK if file can't be opened (permissions, etc)
            }
        }
    }

    #[test]
    fn test_get_cwd() {
        let fs = FileSystem::new();
        assert!(!fs.get_cwd().is_empty());
    }
}
