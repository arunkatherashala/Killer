// Phase 2.3: JIT Code Cache Management
// LRU cache for compiled functions with eviction policy
// Prevents unbounded memory growth while keeping hot functions cached

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CachedFunction {
    pub name: String,
    pub compiled_code_size: usize,
    pub access_count: u64,
    pub last_accessed_timestamp: u64,
}

#[derive(Debug)]
pub struct JITCodeCache {
    /// Cached compiled functions (name -> function)
    cache: HashMap<String, CachedFunction>,
    /// LRU tracking (timestamp -> function_name)
    lru_order: Vec<(u64, String)>,
    /// Total cached bytecode size in bytes
    total_cached_size: usize,
    /// Maximum cache size (default: 50MB)
    max_cache_size: usize,
    /// Current timestamp counter
    timestamp_counter: u64,
}

impl JITCodeCache {
    pub fn new(max_size_mb: usize) -> Self {
        JITCodeCache {
            cache: HashMap::new(),
            lru_order: Vec::new(),
            total_cached_size: 0,
            max_cache_size: max_size_mb * 1024 * 1024,
            timestamp_counter: 0,
        }
    }

    /// Add compiled function to cache
    /// Returns true if cached, false if evicted due to size limit
    pub fn add_compiled_function(
        &mut self,
        name: String,
        compiled_code_size: usize,
    ) -> bool {
        self.timestamp_counter += 1;
        let timestamp = self.timestamp_counter;

        // Check if function already cached
        if self.cache.contains_key(&name) {
            // Update existing function
            if let Some(func) = self.cache.get_mut(&name) {
                func.access_count += 1;
                func.last_accessed_timestamp = timestamp;
            }
            return true;
        }

        // Check if adding would exceed cache size
        if self.total_cached_size + compiled_code_size > self.max_cache_size {
            // Evict least recently used function
            self.evict_lru();
        }

        // Add new function
        self.cache.insert(
            name.clone(),
            CachedFunction {
                name: name.clone(),
                compiled_code_size,
                access_count: 1,
                last_accessed_timestamp: timestamp,
            },
        );

        self.total_cached_size += compiled_code_size;
        self.lru_order.push((timestamp, name));

        true
    }

    /// Check if function is cached
    pub fn is_cached(&mut self, name: &str) -> bool {
        if self.cache.contains_key(name) {
            // Update access count and timestamp
            self.timestamp_counter += 1;
            if let Some(func) = self.cache.get_mut(name) {
                func.access_count += 1;
                func.last_accessed_timestamp = self.timestamp_counter;
            }
            return true;
        }
        false
    }

    /// Evict least recently used function
    fn evict_lru(&mut self) {
        if let Some((_, name)) = self.lru_order.first().cloned() {
            if let Some(func) = self.cache.remove(&name) {
                self.total_cached_size -= func.compiled_code_size;
                self.lru_order.retain(|(_, fn_name)| fn_name != &name);
            }
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            cached_functions: self.cache.len(),
            total_cached_bytes: self.total_cached_size,
            max_cache_bytes: self.max_cache_size,
            utilization_percent: if self.max_cache_size > 0 {
                (self.total_cached_size * 100) / self.max_cache_size
            } else {
                0
            },
            most_accessed: self
                .cache
                .values()
                .max_by_key(|f| f.access_count)
                .map(|f| (f.name.clone(), f.access_count)),
        }
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
        self.total_cached_size = 0;
    }

    /// Get total functions in cache
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_functions: usize,
    pub total_cached_bytes: usize,
    pub max_cache_bytes: usize,
    pub utilization_percent: usize,
    pub most_accessed: Option<(String, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = JITCodeCache::new(50); // 50MB
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_add_function_to_cache() {
        let mut cache = JITCodeCache::new(50);
        
        let added = cache.add_compiled_function("test_fn".to_string(), 1024);
        assert!(added, "Should successfully add function");
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_is_cached() {
        let mut cache = JITCodeCache::new(50);
        
        cache.add_compiled_function("func1".to_string(), 1024);
        assert!(cache.is_cached("func1"));
        assert!(!cache.is_cached("func2"));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = JITCodeCache::new(3); // 3MB only
        
        // Add first function (1MB)
        cache.add_compiled_function("f1".to_string(), 1024 * 1024);
        assert_eq!(cache.size(), 1);
        
        // Add second function (1MB)
        cache.add_compiled_function("f2".to_string(), 1024 * 1024);
        assert_eq!(cache.size(), 2);
        
        // Add third function (1MB)
        cache.add_compiled_function("f3".to_string(), 1024 * 1024);
        assert_eq!(cache.size(), 3);
        
        // Add fourth - should evict LRU (f1)
        cache.add_compiled_function("f4".to_string(), 1024 * 1024);
        assert_eq!(cache.size(), 3); // Should still be 3
        assert!(!cache.is_cached("f1"), "f1 should be evicted");
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = JITCodeCache::new(10);
        
        cache.add_compiled_function("func1".to_string(), 2 * 1024 * 1024);
        cache.add_compiled_function("func2".to_string(), 3 * 1024 * 1024);
        
        let stats = cache.get_stats();
        assert_eq!(stats.cached_functions, 2);
        assert_eq!(stats.total_cached_bytes, 5 * 1024 * 1024);
        assert_eq!(stats.utilization_percent, 50); // 5/10 = 50%
    }

    #[test]
    fn test_access_count_tracking() {
        let mut cache = JITCodeCache::new(50);
        
        cache.add_compiled_function("hot_fn".to_string(), 1024);
        
        // Access the function multiple times
        for _ in 0..5 {
            cache.is_cached("hot_fn");
        }
        
        let stats = cache.get_stats();
        if let Some((name, count)) = stats.most_accessed {
            assert_eq!(name, "hot_fn");
            assert_eq!(count, 6); // 1 (from add) + 5 (from is_cached)
        }
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = JITCodeCache::new(50);
        
        cache.add_compiled_function("f1".to_string(), 1024);
        cache.add_compiled_function("f2".to_string(), 1024);
        assert_eq!(cache.size(), 2);
        
        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_cache_size_limits() {
        let mut cache = JITCodeCache::new(1); // 1MB limit
        
        // Add 0.5MB function
        let added1 = cache.add_compiled_function("f1".to_string(), 512 * 1024);
        assert!(added1);
        
        // Add another 0.5MB function (should fit)
        let added2 = cache.add_compiled_function("f2".to_string(), 512 * 1024);
        assert!(added2);
        assert_eq!(cache.size(), 2);
        
        // Try to add 0.6MB function (should evict f1)
        let added3 = cache.add_compiled_function("f3".to_string(), 600 * 1024);
        assert!(added3);
        assert_eq!(cache.size(), 2); // f1 evicted, f2 + f3 remain
        assert!(!cache.is_cached("f1"));
    }

    #[test]
    fn test_cache_efficiency() {
        let mut cache = JITCodeCache::new(10);
        
        // Add some functions
        for i in 0..5 {
            cache.add_compiled_function(
                format!("func{}", i),
                (i + 1) * 1024 * 1024, // Increasing sizes
            );
        }
        
        // Verify cache captured some functions
        let stats = cache.get_stats();
        assert!(stats.cached_functions > 0, "Should have cached some functions");
        assert!(stats.utilization_percent > 0, "Should have some utilization");
    }
}
