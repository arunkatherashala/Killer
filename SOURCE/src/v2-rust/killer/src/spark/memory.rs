// Memory Management System
// Handles datasets larger than RAM via spilling with smart caching

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;

// ============================================================================
// Memory Pool - Allocates and tracks memory usage
// ============================================================================

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub id: usize,
    pub owner: String,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub id: usize,
    pub size: usize,
    pub owner: String,
    pub allocated_at: std::time::Instant,
}

pub struct MemoryPool {
    pub max_size: usize,
    pub current_usage: Arc<AtomicUsize>,
    pub allocations: Arc<Mutex<Vec<MemoryBlock>>>,
    next_id: Arc<AtomicUsize>,
}

impl MemoryPool {
    pub fn new(max_size: usize) -> Self {
        MemoryPool {
            max_size,
            current_usage: Arc::new(AtomicUsize::new(0)),
            allocations: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn allocate(&self, size: usize, owner: &str) -> Result<MemoryRegion, String> {
        let current = self.current_usage.load(Ordering::Relaxed);
        if current + size > self.max_size {
            return Err(format!(
                "Memory allocation failed: {} bytes requested, {} available",
                size,
                self.max_size - current
            ));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let block = MemoryBlock {
            id,
            size,
            owner: owner.to_string(),
            allocated_at: std::time::Instant::now(),
        };

        self.allocations.lock().unwrap().push(block);
        self.current_usage.fetch_add(size, Ordering::Relaxed);

        Ok(MemoryRegion {
            id,
            owner: owner.to_string(),
            size,
        })
    }

    pub fn deallocate(&self, region: &MemoryRegion) -> Result<(), String> {
        let mut allocs = self.allocations.lock().unwrap();
        if let Some(pos) = allocs.iter().position(|a| a.id == region.id) {
            let block = allocs.remove(pos);
            self.current_usage.fetch_sub(block.size, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!("Memory region {} not found", region.id))
        }
    }

    pub fn usage_percentage(&self) -> f64 {
        let current = self.current_usage.load(Ordering::Relaxed);
        (current as f64 / self.max_size as f64) * 100.0
    }

    pub fn available(&self) -> usize {
        let current = self.current_usage.load(Ordering::Relaxed);
        self.max_size.saturating_sub(current)
    }

    pub fn total_allocated(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }
}

// ============================================================================
// LRU Cache - Least Recently Used eviction policy
// ============================================================================

pub struct LruCache<K: Clone + Eq + std::hash::Hash, V> {
    cache: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K: Clone + Eq + std::hash::Hash, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            cache: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.cache.contains_key(key) {
            // Move to end (most recently used)
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            return self.cache.get(key);
        }
        None
    }

    pub fn put(&mut self, key: K, value: V) {
        if self.cache.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.cache.len() >= self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.cache.remove(&evicted);
            }
        }

        self.cache.insert(key.clone(), value);
        self.order.push_back(key);
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn is_full(&self) -> bool {
        self.cache.len() >= self.capacity
    }

    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    pub fn evict_lru(&mut self) -> Option<K> {
        self.order.pop_front().and_then(|k| {
            self.cache.remove(&k);
            Some(k)
        })
    }
}

// ============================================================================
// Disk Buffer - Spill-to-disk storage with compression
// ============================================================================

pub struct DiskBuffer {
    pub spill_dir: PathBuf,
    pub max_size: usize,
    pub current_size: Arc<AtomicUsize>,
    pub compression: bool,
}

impl DiskBuffer {
    pub fn new(spill_dir: &str, max_size: usize) -> Result<Self, String> {
        let path = PathBuf::from(spill_dir);
        fs::create_dir_all(&path).map_err(|e| format!("Failed to create spill dir: {}", e))?;

        Ok(DiskBuffer {
            spill_dir: path,
            max_size,
            current_size: Arc::new(AtomicUsize::new(0)),
            compression: true,
        })
    }

    pub fn write(&self, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.spill_dir.join(key);
        
        let compressed = if self.compression {
            self.compress_data(data)
        } else {
            data.to_vec()
        };

        fs::write(&path, &compressed)
            .map_err(|e| format!("Failed to write spill file: {}", e))?;

        self.current_size.fetch_add(compressed.len(), Ordering::Relaxed);

        if self.current_size.load(Ordering::Relaxed) > self.max_size {
            let _ = fs::remove_file(&path);
            return Err(format!(
                "Spill size exceeded: {} > {}",
                self.current_size.load(Ordering::Relaxed),
                self.max_size
            ));
        }

        Ok(())
    }

    pub fn read(&self, key: &str) -> Result<Vec<u8>, String> {
        let path = self.spill_dir.join(key);
        let data = fs::read(&path)
            .map_err(|e| format!("Failed to read spill file: {}", e))?;

        if self.compression {
            self.decompress_data(&data)
        } else {
            Ok(data)
        }
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        let path = self.spill_dir.join(key);
        let metadata = fs::metadata(&path)
            .map_err(|e| format!("Failed to get metadata: {}", e))?;
        
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;

        self.current_size.fetch_sub(metadata.len() as usize, Ordering::Relaxed);
        Ok(())
    }

    pub fn cleanup(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.spill_dir)
            .map_err(|e| format!("Failed to cleanup spill dir: {}", e))?;
        fs::create_dir_all(&self.spill_dir)
            .map_err(|e| format!("Failed to recreate spill dir: {}", e))?;
        self.current_size.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn compress_data(&self, data: &[u8]) -> Vec<u8> {
        // Simple compression: just prefix with length for now
        // In production, use deflate/snappy/zstd
        let mut result = vec![];
        result.extend_from_slice(&(data.len() as u32).to_le_bytes());
        result.extend_from_slice(data);
        result
    }

    fn decompress_data(&self, compressed: &[u8]) -> Result<Vec<u8>, String> {
        if compressed.len() < 4 {
            return Err("Compressed data too small".to_string());
        }
        let size = u32::from_le_bytes([
            compressed[0],
            compressed[1],
            compressed[2],
            compressed[3],
        ]) as usize;

        if compressed.len() < 4 + size {
            return Err("Compressed data corrupt".to_string());
        }

        Ok(compressed[4..4 + size].to_vec())
    }

    pub fn usage_percentage(&self) -> f64 {
        let current = self.current_size.load(Ordering::Relaxed);
        (current as f64 / self.max_size as f64) * 100.0
    }
}

// ============================================================================
// Spillable Cache - Automatic spilling when memory full
// ============================================================================

pub struct SpillableCache {
    pub memory_cache: LruCache<String, Vec<u8>>,
    pub disk_buffer: DiskBuffer,
    pub max_memory: usize,
    pub spill_threshold: f64,
    stats: Arc<Mutex<MemoryStats>>,
}

impl SpillableCache {
    pub fn new(
        max_memory: usize,
        spill_dir: &str,
        max_disk: usize,
    ) -> Result<Self, String> {
        let memory_items = (max_memory / 100_000).max(100); // Assume 100KB average item
        
        Ok(SpillableCache {
            memory_cache: LruCache::new(memory_items),
            disk_buffer: DiskBuffer::new(spill_dir, max_disk)?,
            max_memory,
            spill_threshold: 0.8,
            stats: Arc::new(Mutex::new(MemoryStats::default())),
        })
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        let value_size = value.len();
        let current_usage = self.memory_cache.size() * 100_000; // Estimate

        if current_usage + value_size > (self.max_memory as f64 * self.spill_threshold) as usize {
            // Spill LRU item to disk
            // First get the value before evicting
            if let Some(evicted_key) = self.memory_cache.order.front().cloned() {
                if let Some(evicted_value) = self.memory_cache.cache.get(&evicted_key).cloned() {
                    // Now evict it
                    self.memory_cache.evict_lru();
                    self.disk_buffer.write(&evicted_key, &evicted_value)?;
                    self.stats.lock().unwrap().spill_count += 1;
                    self.stats.lock().unwrap().spill_bytes += evicted_value.len();
                }
            }
        }

        self.memory_cache.put(key, value);
        Ok(())
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, String> {
        // Try memory first
        if let Some(value) = self.memory_cache.get(&key.to_string()) {
            self.stats.lock().unwrap().cache_hits += 1;
            return Ok(Some(value.clone()));
        }

        // Try disk
        match self.disk_buffer.read(key) {
            Ok(value) => {
                self.stats.lock().unwrap().cache_hits += 1;
                // Move back to memory
                self.put(key.to_string(), value.clone())?;
                Ok(Some(value))
            }
            Err(_) => {
                self.stats.lock().unwrap().cache_misses += 1;
                Ok(None)
            }
        }
    }

    pub fn get_stats(&self) -> MemoryStats {
        self.stats.lock().unwrap().clone()
    }
}

// ============================================================================
// Memory Manager - Central memory management
// ============================================================================

pub struct MemoryManager {
    pub pool: MemoryPool,
    pub cache: Arc<Mutex<SpillableCache>>,
    stats: Arc<Mutex<MemoryStats>>,
}

impl MemoryManager {
    pub fn new(
        max_memory: usize,
        spill_dir: &str,
        max_disk: usize,
    ) -> Result<Self, String> {
        Ok(MemoryManager {
            pool: MemoryPool::new(max_memory),
            cache: Arc::new(Mutex::new(SpillableCache::new(
                max_memory,
                spill_dir,
                max_disk,
            )?)),
            stats: Arc::new(Mutex::new(MemoryStats::default())),
        })
    }

    pub fn cache_data(&self, key: String, data: Vec<u8>) -> Result<(), String> {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, data)
    }

    pub fn get_data(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(key)
    }

    pub fn get_stats(&self) -> MemoryStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn memory_usage(&self) -> f64 {
        self.pool.usage_percentage()
    }

    pub fn disk_usage(&self) -> f64 {
        self.cache.lock().unwrap().disk_buffer.usage_percentage()
    }
}

// ============================================================================
// Memory Statistics
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_freed: usize,
    pub spill_count: usize,
    pub spill_bytes: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub peak_memory: usize,
}

impl MemoryStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }

    pub fn spill_ratio(&self) -> f64 {
        if self.total_allocated == 0 { 0.0 } else {
            self.spill_bytes as f64 / self.total_allocated as f64
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_allocation() {
        let pool = MemoryPool::new(1_000_000);
        let region = pool.allocate(100_000, "test").unwrap();
        assert_eq!(region.size, 100_000);
        assert_eq!(pool.total_allocated(), 100_000);
    }

    #[test]
    fn test_memory_pool_deallocation() {
        let pool = MemoryPool::new(1_000_000);
        let region = pool.allocate(100_000, "test").unwrap();
        pool.deallocate(&region).unwrap();
        assert_eq!(pool.total_allocated(), 0);
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache: LruCache<String, Vec<u8>> = LruCache::new(3);
        cache.put("a".to_string(), vec![1, 2, 3]);
        cache.put("b".to_string(), vec![4, 5, 6]);
        cache.put("c".to_string(), vec![7, 8, 9]);

        // Add 4th item, should evict "a"
        cache.put("d".to_string(), vec![10, 11, 12]);

        assert!(!cache.contains(&"a".to_string()));
        assert!(cache.contains(&"b".to_string()));
    }

    #[test]
    fn test_disk_buffer_write_read() {
        let buffer = DiskBuffer::new("test_spill", 10_000_000).unwrap();
        let data = vec![1, 2, 3, 4, 5];

        buffer.write("test_key", &data).unwrap();
        let read_data = buffer.read("test_key").unwrap();

        assert_eq!(data, read_data);
        let _ = buffer.cleanup();
    }

    #[test]
    fn test_spillable_cache() {
        let mut cache = SpillableCache::new(100_000, "test_cache_spill", 1_000_000).unwrap();

        let data = vec![0u8; 50_000];
        cache.put("item1".to_string(), data.clone()).unwrap();
        cache.put("item2".to_string(), data.clone()).unwrap();

        let retrieved = cache.get("item1").unwrap();
        assert!(retrieved.is_some());

        let _ = cache.disk_buffer.cleanup();
    }

    #[test]
    fn test_memory_manager() {
        let manager = MemoryManager::new(1_000_000, "test_manager_spill", 1_000_000).unwrap();

        let data = vec![0u8; 10_000];
        manager.cache_data("key1".to_string(), data).unwrap();

        let retrieved = manager.get_data("key1").unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_memory_stats() {
        let mut stats = MemoryStats {
            cache_hits: 8,
            cache_misses: 2,
            spill_bytes: 500_000,
            total_allocated: 1_000_000,
            ..Default::default()
        };

        assert_eq!(stats.hit_rate(), 0.8);
        assert_eq!(stats.spill_ratio(), 0.5);
    }

    #[test]
    fn test_memory_pool_overflow() {
        let pool = MemoryPool::new(100_000);
        pool.allocate(80_000, "test1").unwrap();
        let result = pool.allocate(50_000, "test2");

        assert!(result.is_err());
    }

    #[test]
    fn test_lru_cache_get_updates_order() {
        let mut cache: LruCache<String, i32> = LruCache::new(2);
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);

        let _ = cache.get(&"a".to_string());
        cache.put("c".to_string(), 3);

        assert!(cache.contains(&"a".to_string()));
        assert!(!cache.contains(&"b".to_string()));
    }
}
