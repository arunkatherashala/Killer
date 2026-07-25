// killer_rcore/src/jit/cache.rs
// Cache management for compiled JIT binaries
// Week 3 part 2

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use super::signature::LoopSignature;

/// Manages cached compiled JIT binaries
pub struct JITCache {
    /// Root cache directory (typically ~/.killer/jit_cache/)
    cache_dir: PathBuf,
    
    /// In-memory index of cached binaries
    index: HashMap<String, PathBuf>,
    
    /// Maximum cache size in bytes (default 100MB)
    max_size: u64,
    
    /// Current cache size in bytes
    current_size: u64,
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, String>;

impl JITCache {
    /// Create new cache manager with default directory
    pub fn new() -> CacheResult<Self> {
        let cache_dir = Self::default_cache_dir()?;
        Self::with_dir(cache_dir)
    }
    
    /// Create new cache manager with custom directory
    pub fn with_dir(cache_dir: PathBuf) -> CacheResult<Self> {
        // Create directory if missing
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        
        // Initialize index
        let index = Self::index_cache(&cache_dir)?;
        
        // Calculate current size
        let current_size = Self::calculate_size(&cache_dir)?;
        
        Ok(JITCache {
            cache_dir,
            index,
            max_size: 100 * 1024 * 1024, // 100MB default
            current_size,
        })
    }
    
    /// Get default cache directory (~/.killer/jit_cache/)
    fn default_cache_dir() -> CacheResult<PathBuf> {
        // Try HOME environment variable first (Linux/macOS)
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home).join(".killer").join("jit_cache"));
        }
        
        // Try USERPROFILE environment variable (Windows)
        if let Ok(home) = std::env::var("USERPROFILE") {
            return Ok(PathBuf::from(home).join(".killer").join("jit_cache"));
        }
        
        // Try HOMEDRIVE + HOMEPATH (Windows fallback)
        if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
            return Ok(PathBuf::from(format!("{}{}", drive, path)).join(".killer").join("jit_cache"));
        }
        
        // Use temp directory as absolute fallback
        let temp = std::env::temp_dir();
        Ok(temp.join("killer_jit_cache"))
    }
    
    /// Index all binaries in cache directory
    fn index_cache(cache_dir: &Path) -> CacheResult<HashMap<String, PathBuf>> {
        let mut index = HashMap::new();
        
        if !cache_dir.exists() {
            return Ok(index);
        }
        
        let entries = fs::read_dir(cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?;
        
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("Failed to read cache entry: {}", e))?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name() {
                if let Some(name_str) = filename.to_str() {
                    // Index by filename
                    index.insert(name_str.to_string(), path);
                }
            }
        }
        
        Ok(index)
    }
    
    /// Calculate total cache size
    fn calculate_size(cache_dir: &Path) -> CacheResult<u64> {
        let mut total = 0u64;
        
        if !cache_dir.exists() {
            return Ok(0);
        }
        
        for entry in fs::read_dir(cache_dir)
            .map_err(|e| format!("Failed to read cache: {}", e))?
        {
            let entry = entry
                .map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    total += metadata.len();
                }
            }
        }
        
        Ok(total)
    }
    
    /// Check if binary is cached for this signature
    pub fn has_cached(&self, sig: &LoopSignature) -> bool {
        let filename = sig.cache_filename();
        self.index.contains_key(&filename)
    }
    
    /// Get path to cached binary (if exists)
    pub fn get_cached(&self, sig: &LoopSignature) -> Option<PathBuf> {
        let filename = sig.cache_filename();
        self.index.get(&filename).cloned()
    }
    
    /// Store compiled binary in cache
    pub fn store(&mut self, sig: &LoopSignature, binary: &[u8]) -> CacheResult<PathBuf> {
        // Check if we need to make space
        let binary_size = binary.len() as u64;
        if self.current_size + binary_size > self.max_size {
            self.evict_lru(binary_size)?;
        }
        
        // Write binary to cache
        let filename = sig.cache_filename();
        let path = self.cache_dir.join(&filename);
        
        fs::write(&path, binary)
            .map_err(|e| format!("Failed to write cache: {}", e))?;
        
        // Update index and size
        self.index.insert(filename, path.clone());
        self.current_size += binary_size;
        
        Ok(path)
    }
    
    /// Load binary from cache
    pub fn load(&self, sig: &LoopSignature) -> CacheResult<Vec<u8>> {
        let path = self.get_cached(sig)
            .ok_or(format!("Binary not in cache: {}", sig.hash))?;
        
        fs::read(&path)
            .map_err(|e| format!("Failed to read cached binary: {}", e))
    }
    
    /// Remove binary from cache (LRU eviction)
    fn evict_lru(&mut self, needed_space: u64) -> CacheResult<()> {
        // Simple strategy: remove oldest files until space available
        let mut entries: Vec<_> = self.index
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        // Sort by modification time (oldest first)
        entries.sort_by(|a, b| {
            let time_a = fs::metadata(&a.1)
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            
            let time_b = fs::metadata(&b.1)
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            
            time_a.cmp(&time_b)
        });
        
        let mut freed = 0u64;
        for (filename, path) in entries {
            if freed >= needed_space {
                break;
            }
            
            if let Ok(metadata) = fs::metadata(&path) {
                let size = metadata.len();
                if fs::remove_file(&path).is_ok() {
                    freed += size;
                    self.current_size -= size;
                    self.index.remove(&filename);
                }
            }
        }
        
        if freed < needed_space {
            return Err("Insufficient cache space after LRU eviction".to_string());
        }
        
        Ok(())
    }
    
    /// Clear entire cache
    pub fn clear(&mut self) -> CacheResult<()> {
        for (_, path) in self.index.iter() {
            fs::remove_file(path)
                .map_err(|e| format!("Failed to delete cache file: {}", e))?;
        }
        
        self.index.clear();
        self.current_size = 0;
        
        Ok(())
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            num_cached: self.index.len(),
            total_size: self.current_size,
            max_size: self.max_size,
            utilization: (self.current_size as f64 / self.max_size as f64) * 100.0,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub num_cached: usize,
    pub total_size: u64,
    pub max_size: u64,
    pub utilization: f64,
}

impl Default for JITCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            JITCache {
                cache_dir: PathBuf::from("/tmp/killer_jit_cache"),
                index: HashMap::new(),
                max_size: 100 * 1024 * 1024,
                current_size: 0,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_signature() -> LoopSignature {
        LoopSignature::from_id_and_bounds("test_loop", 1_000_000)
    }
    
    #[test]
    fn test_cache_filename() {
        let sig = create_test_signature();
        let filename = sig.cache_filename();
        
        assert!(filename.starts_with("killer_jit_"));
        assert!(filename.ends_with(".so"));
    }
    
    #[test]
    fn test_signature_equality() {
        let sig1 = create_test_signature();
        let sig2 = LoopSignature::from_id_and_bounds("test_loop", 1_000_000);
        
        assert_eq!(sig1, sig2);
    }
    
    #[test]
    fn test_loop_signature_creation() {
        let sig = LoopSignature::from_id_and_bounds("my_loop", 5_000_000);
        assert_eq!(sig.loop_id, "my_loop");
        assert!(!sig.hash.is_empty());
        assert_eq!(sig.location, "unknown");
    }
}
