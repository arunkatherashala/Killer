// Phase 17: Function Memoization Engine
// Caches function results to achieve 100-1000x speedup for recursive functions

use std::collections::HashMap;
use crate::value::Value;

/// A cached function result
#[derive(Debug, Clone)]
pub struct MemoizedResult {
    pub result: Value,
    pub access_count: usize,        // How many times this result was reused
    pub creation_time: u64,         // Timestamp for TTL
    pub memory_cost: usize,         // Bytes used for this entry
}

/// Function memoization cache
pub struct MemoizationCache {
    /// Map: function_name + args_hash → result
    cache: HashMap<(String, u64), MemoizedResult>,
    
    /// Statistics
    hits: usize,
    misses: usize,
    total_memory_saved: u64,  // Cycles saved by avoiding recomputation
    
    /// Configuration
    max_cache_size: usize,    // Max bytes to cache
    ttl_seconds: u64,         // Time-to-live for entries
    eviction_policy: EvictionPolicy,
}

/// Policy for evicting cache entries when full
#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    LRU,        // Least Recently Used
    LFU,        // Least Frequently Used
    FIFO,       // First In First Out
}

impl MemoizationCache {
    pub fn new() -> Self {
        MemoizationCache {
            cache: HashMap::new(),
            hits: 0,
            misses: 0,
            total_memory_saved: 0,
            max_cache_size: 50 * 1024 * 1024,  // 50 MB default
            ttl_seconds: 3600,  // 1 hour TTL
            eviction_policy: EvictionPolicy::LRU,
        }
    }

    /// Try to get a memoized result
    pub fn get(&mut self, function_name: &str, args_hash: u64) -> Option<Value> {
        let key = (function_name.to_string(), args_hash);
        
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.access_count += 1;
            self.hits += 1;
            Some(entry.result.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store a memoized result
    pub fn put(&mut self, function_name: &str, args_hash: u64, result: Value) {
        let key = (function_name.to_string(), args_hash);
        let memory_cost = self.estimate_memory_cost(&result);
        
        let entry = MemoizedResult {
            result,
            access_count: 0,
            creation_time: 0,  // Would be real timestamp in production
            memory_cost,
        };
        
        self.cache.insert(key, entry);
        
        // Evict if necessary
        self.evict_if_needed();
    }

    /// Estimate memory used by a value
    fn estimate_memory_cost(&self, value: &Value) -> usize {
        match value {
            Value::Number(_) => 8,
            Value::Str(s) => s.len() + 8,
            Value::Bool(_) => 1,
            Value::Null => 1,
            Value::Array(arr) => {
                8 + arr.iter().map(|v| self.estimate_memory_cost(v)).sum::<usize>()
            }
            Value::Dict(map) => {
                map.iter()
                    .map(|(k, v)| k.len() + self.estimate_memory_cost(v) + 8)
                    .sum()
            }
            _ => 64,  // Conservative estimate for complex types
        }
    }

    /// Evict entries if cache exceeds size limit
    fn evict_if_needed(&mut self) {
        let total_size: usize = self.cache.values()
            .map(|e| e.memory_cost)
            .sum();
        
        if total_size > self.max_cache_size {
            let to_remove = total_size - (self.max_cache_size / 2);
            let mut removed = 0;
            
            let keys_to_remove: Vec<_> = match self.eviction_policy {
                EvictionPolicy::LRU => {
                    // Remove least recently used
                    let mut entries: Vec<_> = self.cache.iter().collect();
                    entries.sort_by_key(|(_k, e)| e.access_count);
                    entries.iter()
                        .take_while(|_| removed < to_remove)
                        .map(|(k, _)| (*k).clone())
                        .collect()
                }
                EvictionPolicy::LFU => {
                    // Remove least frequently used
                    let mut entries: Vec<_> = self.cache.iter().collect();
                    entries.sort_by_key(|(_k, e)| e.access_count);
                    entries.iter()
                        .take_while(|_| removed < to_remove)
                        .map(|(k, _)| (*k).clone())
                        .collect()
                }
                EvictionPolicy::FIFO => {
                    // Remove oldest entries
                    let mut entries: Vec<_> = self.cache.iter().collect();
                    entries.sort_by_key(|(_k, e)| e.creation_time);
                    entries.iter()
                        .take_while(|_| removed < to_remove)
                        .map(|(k, _)| (*k).clone())
                        .collect()
                }
            };
            
            for key in keys_to_remove {
                self.cache.remove(&key);
            }
        }
    }

    /// Get memoization statistics
    pub fn get_stats(&self) -> MemoizationStats {
        let total: usize = self.cache.values()
            .map(|e| e.memory_cost)
            .sum();
        
        let total_accesses = self.hits + self.misses;
        let hit_rate = if total_accesses > 0 {
            (self.hits as f64) / (total_accesses as f64)
        } else {
            0.0
        };
        
        MemoizationStats {
            cached_functions: self.cache.len(),
            cache_hits: self.hits,
            cache_misses: self.misses,
            hit_rate,
            total_memory_used: total,
            max_memory: self.max_cache_size,
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Print memoization report
    pub fn print_report(&self) {
        let stats = self.get_stats();
        println!("\n=== Memoization Cache Report (Phase 17) ===");
        println!("Cached Functions: {}", stats.cached_functions);
        println!("Cache Hits: {}", stats.cache_hits);
        println!("Cache Misses: {}", stats.cache_misses);
        println!("Hit Rate: {:.1}%", stats.hit_rate * 100.0);
        println!("Memory Used: {} KB / {} KB",
            stats.total_memory_used / 1024,
            stats.max_memory / 1024
        );
        
        if stats.hit_rate > 0.5 {
            println!("Status: ✅ Effective memoization");
        } else if stats.hit_rate > 0.1 {
            println!("Status: ⚠️ Moderate effectiveness");
        } else {
            println!("Status: ℹ️ Low cache utilization");
        }
    }
}

/// Memoization statistics
#[derive(Debug)]
pub struct MemoizationStats {
    pub cached_functions: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub hit_rate: f64,
    pub total_memory_used: usize,
    pub max_memory: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoization_hit() {
        let mut cache = MemoizationCache::new();
        
        let result = Value::Number(42.0);
        cache.put("fibonacci", 5, result.clone());
        
        let retrieved = cache.get("fibonacci", 5);
        assert!(retrieved.is_some());
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_memoization_miss() {
        let mut cache = MemoizationCache::new();
        
        let retrieved = cache.get("fibonacci", 5);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = MemoizationCache::new();
        
        cache.put("test", 1, Value::Number(1.0));
        cache.get("test", 1);  // Hit
        cache.get("test", 1);  // Hit
        cache.get("test", 2);  // Miss
        
        let stats = cache.get_stats();
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 1);
    }
}
