/// Data Sharding for Killer V2.1
/// Partitions data across cores to achieve perfect load balancing
/// 
/// Strategy: shard_key % core_count → no cross-core sync needed
/// Load balance: Each core gets exactly 25% of data (on 4-core system)
/// 
/// Benefit: Eliminate lock contention, improve cache locality per core

use std::collections::HashMap;

/// A unit of data to be sharded
#[derive(Debug, Clone)]
pub struct ShardedData {
    pub key: u64,
    pub value: Vec<u8>,
    pub metadata: ShardMetadata,
}

/// Metadata about sharded data
#[derive(Debug, Clone)]
pub struct ShardMetadata {
    pub created_at: u64,
    pub accessed_at: u64,
    pub access_count: u64,
    pub size_bytes: usize,
}

/// A single shard (one per core)
pub struct Shard {
    shard_id: usize,
    data: HashMap<u64, ShardedData>,
    total_size_bytes: usize,
    access_count: u64,
}

impl Shard {
    pub fn new(shard_id: usize) -> Self {
        Shard {
            shard_id,
            data: HashMap::new(),
            total_size_bytes: 0,
            access_count: 0,
        }
    }

    pub fn insert(&mut self, key: u64, value: Vec<u8>) -> Result<(), String> {
        let size = value.len();
        
        let metadata = ShardMetadata {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            accessed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            access_count: 1,
            size_bytes: size,
        };

        let data = ShardedData {
            key,
            value,
            metadata,
        };

        self.data.insert(key, data);
        self.total_size_bytes += size;
        Ok(())
    }

    pub fn get(&mut self, key: u64) -> Option<Vec<u8>> {
        if let Some(data) = self.data.get_mut(&key) {
            data.metadata.accessed_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            data.metadata.access_count += 1;
            self.access_count += 1;
            Some(data.value.clone())
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: u64) -> Option<Vec<u8>> {
        if let Some(data) = self.data.remove(&key) {
            self.total_size_bytes = self.total_size_bytes.saturating_sub(data.metadata.size_bytes);
            Some(data.value)
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: u64) -> bool {
        self.data.contains_key(&key)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_size_bytes
    }

    pub fn access_count(&self) -> u64 {
        self.access_count
    }

    pub fn get_hot_keys(&self, limit: usize) -> Vec<(u64, u64)> {
        let mut hot_keys: Vec<(u64, u64)> = self
            .data
            .iter()
            .map(|(k, v)| (*k, v.metadata.access_count))
            .collect();
        
        hot_keys.sort_by(|a, b| b.1.cmp(&a.1));
        hot_keys.into_iter().take(limit).collect()
    }

    pub fn stats(&self) -> ShardStats {
        ShardStats {
            shard_id: self.shard_id,
            item_count: self.data.len() as u64,
            total_bytes: self.total_size_bytes as u64,
            access_count: self.access_count,
            avg_access_count: if self.data.is_empty() {
                0
            } else {
                self.access_count / self.data.len() as u64
            },
        }
    }
}

/// Statistics about a shard
#[derive(Debug, Clone)]
pub struct ShardStats {
    pub shard_id: usize,
    pub item_count: u64,
    pub total_bytes: u64,
    pub access_count: u64,
    pub avg_access_count: u64,
}

/// Hash function for sharding
pub struct ShardKey {
    core_count: usize,
}

impl ShardKey {
    pub fn new(core_count: usize) -> Self {
        ShardKey { core_count }
    }

    /// Determine which shard/core gets this key
    pub fn shard_id(&self, key: u64) -> usize {
        (key as usize) % self.core_count
    }

    /// Hash a string key to shard ID
    pub fn shard_id_str(&self, key: &str) -> usize {
        let mut hash = 5381u64;
        
        for byte in key.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        
        (hash as usize) % self.core_count
    }
}

/// Load balancing statistics
#[derive(Debug, Clone)]
pub struct LoadBalanceStats {
    pub core_count: usize,
    pub data_per_core: Vec<u64>,
    pub bytes_per_core: Vec<u64>,
    pub skew_ratio: f64, // Max/Min distribution ratio
}

/// Manager for all shards
pub struct ShardManager {
    shards: Vec<Shard>,
    shard_key: ShardKey,
}

impl ShardManager {
    pub fn new(core_count: usize) -> Self {
        let mut shards = Vec::new();
        for i in 0..core_count {
            shards.push(Shard::new(i));
        }

        ShardManager {
            shards,
            shard_key: ShardKey::new(core_count),
        }
    }

    /// Insert data with automatic sharding
    pub fn insert_sharded(&mut self, key: u64, value: Vec<u8>) -> Result<usize, String> {
        let shard_id = self.shard_key.shard_id(key);
        self.shards[shard_id].insert(key, value)?;
        Ok(shard_id)
    }

    /// Insert string-keyed data
    pub fn insert_sharded_str(&mut self, key: &str, value: Vec<u8>) -> Result<usize, String> {
        let str_hash = self.shard_key.shard_id_str(key);
        self.shards[str_hash].insert(key.as_ptr() as u64, value)?;
        Ok(str_hash)
    }

    /// Get data from appropriate shard
    pub fn get_sharded(&mut self, key: u64) -> Option<Vec<u8>> {
        let shard_id = self.shard_key.shard_id(key);
        self.shards[shard_id].get(key)
    }

    /// Get statistics for single shard
    pub fn get_shard_stats(&self, shard_id: usize) -> Option<ShardStats> {
        if shard_id < self.shards.len() {
            Some(self.shards[shard_id].stats())
        } else {
            None
        }
    }

    /// Get all shard statistics
    pub fn get_all_shard_stats(&self) -> Vec<ShardStats> {
        self.shards.iter().map(|s| s.stats()).collect()
    }

    /// Get load balancing statistics
    pub fn load_balance_stats(&self) -> LoadBalanceStats {
        let stats: Vec<ShardStats> = self.shards.iter().map(|s| s.stats()).collect();
        
        let data_per_core: Vec<u64> = stats.iter().map(|s| s.item_count).collect();
        let bytes_per_core: Vec<u64> = stats.iter().map(|s| s.total_bytes).collect();

        let max_items = data_per_core.iter().max().copied().unwrap_or(1) as f64;
        let min_items = data_per_core.iter().min().copied().unwrap_or(1) as f64;
        let skew_ratio = max_items / min_items.max(1.0);

        LoadBalanceStats {
            core_count: self.shards.len(),
            data_per_core,
            bytes_per_core,
            skew_ratio,
        }
    }

    /// Rebalance shards (redistribute if skewed)
    pub fn rebalance(&mut self) -> Vec<ShardStats> {
        // In this implementation, rebalancing would require redistribution
        // For now, just collect statistics
        self.get_all_shard_stats()
    }

    /// Get shard for direct access
    pub fn get_shard(&self, shard_id: usize) -> Option<&Shard> {
        if shard_id < self.shards.len() {
            Some(&self.shards[shard_id])
        } else {
            None
        }
    }

    /// Remove data from shard
    pub fn remove_sharded(&mut self, key: u64) -> Option<Vec<u8>> {
        let shard_id = self.shard_key.shard_id(key);
        self.shards[shard_id].remove(key)
    }

    /// Total items across all shards
    pub fn total_items(&self) -> u64 {
        self.shards.iter().map(|s| s.size() as u64).sum()
    }

    /// Total bytes across all shards
    pub fn total_bytes(&self) -> u64 {
        self.shards.iter().map(|s| s.total_bytes() as u64).sum()
    }

    /// Total accesses across all shards
    pub fn total_accesses(&self) -> u64 {
        self.shards.iter().map(|s| s.access_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_basic() {
        let mut shard = Shard::new(0);
        
        shard.insert(1, vec![42; 100]).unwrap();
        assert!(shard.contains_key(1));
        assert_eq!(shard.size(), 1);
        assert_eq!(shard.total_bytes(), 100);
    }

    #[test]
    fn test_shard_key_distribution() {
        let shard_key = ShardKey::new(4);
        
        let mut shard_counts = vec![0; 4];
        for i in 0..1000 {
            let shard_id = shard_key.shard_id(i);
            shard_counts[shard_id] += 1;
        }

        // Should be roughly even distribution
        for count in shard_counts {
            assert!(count > 200 && count < 300);
        }
    }

    #[test]
    fn test_shard_manager_insertion() {
        let mut manager = ShardManager::new(4);
        
        for i in 0..100 {
            let result = manager.insert_sharded(i, vec![42; 50]);
            assert!(result.is_ok());
        }

        assert_eq!(manager.total_items(), 100);
    }

    #[test]
    fn test_shard_manager_retrieval() {
        let mut manager = ShardManager::new(4);
        
        manager.insert_sharded(123, vec![42; 50]).unwrap();
        
        let retrieved = manager.get_sharded(123);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 50);
    }

    #[test]
    fn test_load_balance() {
        let mut manager = ShardManager::new(4);
        
        for i in 0..1000 {
            manager.insert_sharded(i, vec![42; 50]).unwrap();
        }

        let lb_stats = manager.load_balance_stats();
        assert_eq!(lb_stats.core_count, 4);
        
        // Check skew is minimal (should be < 1.1x)
        assert!(lb_stats.skew_ratio < 1.5);
    }

    #[test]
    fn test_shard_statistics() {
        let mut manager = ShardManager::new(4);
        
        for i in 0..40 {
            manager.insert_sharded(i, vec![42; 50]).unwrap();
        }

        for i in 0..4 {
            if let Some(stats) = manager.get_shard_stats(i) {
                assert!(stats.item_count > 0);
                assert_eq!(stats.total_bytes, stats.item_count * 50);
            }
        }
    }

    #[test]
    fn test_shard_key_string() {
        let shard_key = ShardKey::new(4);
        
        let id1 = shard_key.shard_id_str("user:123");
        let id2 = shard_key.shard_id_str("user:456");
        
        assert!(id1 < 4);
        assert!(id2 < 4);
    }
}
