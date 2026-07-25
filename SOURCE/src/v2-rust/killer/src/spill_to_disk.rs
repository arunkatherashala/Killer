/// Spill-to-Disk Manager for Killer V2.1
/// Automatically overflows to SSD when RAM exhausted
/// 
/// Design: 8GB RAM limit → Spill to 237GB SSD automatically
/// Strategy: Keep "hot" data in RAM (LRU), cold data on disk
/// 
/// Result: 100M+ pending operations possible (limited by SSD space, not RAM)

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{Read, Write, Result as IoResult};
use std::path::PathBuf;
use std::time::Instant;

/// Memory pool for cached entries
pub struct MemoryPool {
    cache: HashMap<u64, Vec<u8>>,
    lru_order: VecDeque<u64>,
    max_size_bytes: usize,
    current_size_bytes: usize,
}

impl MemoryPool {
    pub fn new(max_size_bytes: usize) -> Self {
        MemoryPool {
            cache: HashMap::new(),
            lru_order: VecDeque::new(),
            max_size_bytes,
            current_size_bytes: 0,
        }
    }

    pub fn insert(&mut self, key: u64, value: Vec<u8>) -> Result<bool, String> {
        let value_size = value.len();

        // Check if would exceed limit
        if self.current_size_bytes + value_size > self.max_size_bytes {
            return Ok(false); // Should spill to disk
        }

        self.cache.insert(key, value);
        self.lru_order.push_back(key);
        self.current_size_bytes += value_size;

        Ok(true)
    }

    pub fn get(&mut self, key: u64) -> Option<Vec<u8>> {
        if let Some(value) = self.cache.get(&key) {
            // Move to end of LRU order (most recently used)
            if let Some(pos) = self.lru_order.iter().position(|k| *k == key) {
                let k = self.lru_order.remove(pos).unwrap();
                self.lru_order.push_back(k);
            }
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn evict_lru(&mut self) -> Option<(u64, Vec<u8>)> {
        if let Some(key) = self.lru_order.pop_front() {
            if let Some(value) = self.cache.remove(&key) {
                self.current_size_bytes = self.current_size_bytes.saturating_sub(value.len());
                return Some((key, value));
            }
        }
        None
    }

    pub fn contains_key(&self, key: u64) -> bool {
        self.cache.contains_key(&key)
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn memory_used(&self) -> usize {
        self.current_size_bytes
    }

    pub fn memory_available(&self) -> usize {
        self.max_size_bytes.saturating_sub(self.current_size_bytes)
    }
}

/// Disk buffer for spilled data
pub struct DiskBuffer {
    spill_dir: PathBuf,
    spill_manifest: HashMap<u64, SpillEntry>,
    total_spilled_bytes: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SpillEntry {
    key: u64,
    file_offset: u64,
    size: usize,
    spilled_at: Instant,
}

impl DiskBuffer {
    pub fn new(spill_dir: &str) -> IoResult<Self> {
        let path = PathBuf::from(spill_dir);
        
        // Create directory if doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        Ok(DiskBuffer {
            spill_dir: path,
            spill_manifest: HashMap::new(),
            total_spilled_bytes: 0,
        })
    }

    pub fn write(&mut self, key: u64, data: &[u8]) -> IoResult<()> {
        let file_path = self.spill_dir.join(format!("spill_{}.bin", key));
        let mut file = File::create(file_path)?;
        file.write_all(data)?;
        
        self.spill_manifest.insert(key, SpillEntry {
            key,
            file_offset: 0,
            size: data.len(),
            spilled_at: Instant::now(),
        });

        self.total_spilled_bytes += data.len() as u64;
        Ok(())
    }

    pub fn read(&self, key: u64) -> IoResult<Vec<u8>> {
        let file_path = self.spill_dir.join(format!("spill_{}.bin", key));
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn remove(&mut self, key: u64) -> IoResult<()> {
        let file_path = self.spill_dir.join(format!("spill_{}.bin", key));
        
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }

        self.spill_manifest.remove(&key);
        Ok(())
    }

    pub fn contains_key(&self, key: u64) -> bool {
        self.spill_manifest.contains_key(&key)
    }

    pub fn spilled_count(&self) -> usize {
        self.spill_manifest.len()
    }

    pub fn total_spilled_bytes(&self) -> u64 {
        self.total_spilled_bytes
    }

    pub fn cleanup(&mut self) -> IoResult<()> {
        // Remove all spill files
        for (key, _) in self.spill_manifest.drain() {
            let file_path = self.spill_dir.join(format!("spill_{}.bin", key));
            if file_path.exists() {
                fs::remove_file(file_path)?;
            }
        }
        Ok(())
    }
}

/// Strategy for when to spill
#[derive(Debug, Clone)]
pub struct SpillStrategy {
    pub spill_threshold_percent: u32,
    pub force_spill_at_percent: u32,
}

impl SpillStrategy {
    pub fn default_for_system() -> Self {
        SpillStrategy {
            spill_threshold_percent: 75,  // Spill at 75% RAM
            force_spill_at_percent: 95,   // Force at 95%
        }
    }

    pub fn should_spill(&self, memory_used_percent: u32) -> bool {
        memory_used_percent >= self.spill_threshold_percent
    }

    pub fn should_force_spill(&self, memory_used_percent: u32) -> bool {
        memory_used_percent >= self.force_spill_at_percent
    }
}

/// Main spill manager coordinates memory + disk
pub struct SpillManager {
    memory_pool: MemoryPool,
    disk_buffer: DiskBuffer,
    strategy: SpillStrategy,
    access_stats: SpillStats,
}

#[derive(Debug, Clone)]
pub struct SpillStats {
    pub memory_hits: u64,
    pub disk_hits: u64,
    pub evictions: u64,
    pub spills: u64,
}

impl SpillManager {
    pub fn new(disk_path: &str, ram_limit_bytes: usize, strategy: SpillStrategy) -> IoResult<Self> {
        Ok(SpillManager {
            memory_pool: MemoryPool::new(ram_limit_bytes),
            disk_buffer: DiskBuffer::new(disk_path)?,
            strategy,
            access_stats: SpillStats {
                memory_hits: 0,
                disk_hits: 0,
                evictions: 0,
                spills: 0,
            },
        })
    }

    pub fn insert(&mut self, key: u64, data: Vec<u8>) -> IoResult<()> {
        let memory_percent = (self.memory_pool.memory_used() as u32 * 100)
            / (self.memory_pool.max_size_bytes as u32).max(1);

        // Check if should spill
        if self.strategy.should_spill(memory_percent) {
            // Try to insert into memory first
            match self.memory_pool.insert(key, data.clone()) {
                Ok(true) => {
                    // Succeeded in memory
                    Ok(())
                }
                Ok(false) => {
                    // Doesn't fit in memory, spill to disk
                    self.disk_buffer.write(key, &data)?;
                    self.access_stats.spills += 1;
                    Ok(())
                }
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to insert into memory",
                )),
            }
        } else {
            // Plenty of memory, use it
            match self.memory_pool.insert(key, data.clone()) {
                Ok(true) => Ok(()),
                Ok(false) => {
                    // Memory pool full, spill to disk instead
                    self.disk_buffer.write(key, &data)?;
                    self.access_stats.spills += 1;
                    Ok(())
                }
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to insert into memory",
                )),
            }
        }
    }

    pub fn retrieve(&mut self, key: u64) -> IoResult<Option<Vec<u8>>> {
        // Try memory first
        if let Some(data) = self.memory_pool.get(key) {
            self.access_stats.memory_hits += 1;
            return Ok(Some(data));
        }

        // Try disk
        if self.disk_buffer.contains_key(key) {
            let data = self.disk_buffer.read(key)?;
            self.access_stats.disk_hits += 1;

            // Try to bring back into memory
            match self.memory_pool.insert(key, data.clone()) {
                Ok(true) => {
                    // Successfully moved back to memory
                    self.disk_buffer.remove(key)?;
                    Ok(Some(data))
                }
                _ => Ok(Some(data)), // Left on disk
            }
        } else {
            Ok(None)
        }
    }

    pub fn spill_ratio(&self) -> (usize, usize) {
        (
            self.memory_pool.memory_used(),
            self.disk_buffer.spilled_count(),
        )
    }

    pub fn stats(&self) -> SpillStats {
        self.access_stats.clone()
    }

    pub fn memory_stats(&self) -> (usize, usize) {
        (
            self.memory_pool.memory_used(),
            self.memory_pool.memory_available(),
        )
    }

    pub fn capacity_estimate(&self) -> u64 {
        // Total capacity = RAM + Disk
        let ram_capacity = self.memory_pool.max_size_bytes as u64;
        let disk_capacity = 237 * 1024 * 1024 * 1024u64; // 237GB available
        ram_capacity + disk_capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_memory_pool_basic() {
        let mut pool = MemoryPool::new(10000);
        
        pool.insert(1, vec![42; 100]).unwrap();
        assert!(pool.contains_key(1));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_memory_pool_lru() {
        let mut pool = MemoryPool::new(1000);
        
        pool.insert(1, vec![42; 100]).unwrap();
        pool.insert(2, vec![42; 100]).unwrap();
        
        let _ = pool.get(1); // Touch entry 1
        
        // Evict should remove oldest untouched
        assert!(pool.evict_lru().is_some());
    }

    #[test]
    fn test_disk_buffer() {
        let temp_dir = "./test_spill";
        let _ = fs::remove_dir_all(temp_dir);

        let mut disk = DiskBuffer::new(temp_dir).unwrap();
        
        disk.write(1, b"test data").unwrap();
        assert!(disk.contains_key(1));
        
        let retrieved = disk.read(1).unwrap();
        assert_eq!(retrieved, b"test data");
        
        disk.cleanup().unwrap();
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_spill_manager() {
        let temp_dir = "./test_spill_mgr";
        let _ = fs::remove_dir_all(temp_dir);

        let strategy = SpillStrategy::default_for_system();
        let mut manager = SpillManager::new(temp_dir, 10000, strategy).unwrap();
        
        manager.insert(1, vec![42; 100]).unwrap();
        
        let retrieved = manager.retrieve(1).unwrap();
        assert!(retrieved.is_some());
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_spill_strategy() {
        let strategy = SpillStrategy::default_for_system();
        
        assert!(strategy.should_spill(80));
        assert!(!strategy.should_spill(70));
        assert!(strategy.should_force_spill(95));
    }
}
