// AI Response Cache
// src/ai/cache.rs
//
// LRU cache for AI responses to reduce API calls and improve performance

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub value: String,
    pub created_at: u64,
    pub ttl_secs: Option<u64>,
}

impl CacheEntry {
    /// Check if this entry is still valid
    pub fn is_valid(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now < self.created_at + ttl
        } else {
            true
        }
    }
}

/// Simple LRU cache for AI responses
pub struct AICache {
    cache: HashMap<String, CacheEntry>,
    max_size: usize,
    access_order: Vec<String>,
}

impl AICache {
    /// Create new cache with specified maximum size
    pub fn new(max_size: usize) -> Self {
        AICache {
            cache: HashMap::new(),
            max_size,
            access_order: Vec::new(),
        }
    }

    /// Generate cache key from operation and parameters
    pub fn key(operation: &str, params: &[(&str, &str)]) -> String {
        let mut key = operation.to_string();
        for (k, v) in params {
            key.push('|');
            key.push_str(k);
            key.push(':');
            key.push_str(v);
        }
        key
    }

    /// Get value from cache
    pub fn get(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.cache.get(key) {
            if entry.is_valid() {
                // Update access order
                self.access_order.retain(|k| k != key);
                self.access_order.push(key.to_string());
                return Some(entry.value.clone());
            } else {
                self.cache.remove(key);
                self.access_order.retain(|k| k != key);
            }
        }
        None
    }

    /// Put value in cache
    pub fn put(&mut self, key: String, value: String, ttl_secs: Option<u64>) {
        if self.access_order.len() >= self.max_size && !self.cache.contains_key(&key) {
            // Remove least recently used
            if let Some(lru_key) = self.access_order.first() {
                self.cache.remove(lru_key);
                self.access_order.remove(0);
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.cache.insert(
            key.clone(),
            CacheEntry {
                value,
                created_at: now,
                ttl_secs,
            },
        );

        self.access_order.retain(|k| k != &key);
        self.access_order.push(key);
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            max_size: self.max_size,
            utilization: if self.max_size > 0 {
                (self.cache.len() as f64 / self.max_size as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Remove specific key
    pub fn remove(&mut self, key: &str) -> bool {
        if self.cache.remove(key).is_some() {
            self.access_order.retain(|k| k != key);
            true
        } else {
            false
        }
    }

    /// Check if key exists and is valid
    pub fn contains(&self, key: &str) -> bool {
        if let Some(entry) = self.cache.get(key) {
            entry.is_valid()
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_size: usize,
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = AICache::new(10);
        cache.put("key1".to_string(), "value1".to_string(), None);
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn test_cache_lru() {
        let mut cache = AICache::new(2);
        cache.put("key1".to_string(), "value1".to_string(), None);
        cache.put("key2".to_string(), "value2".to_string(), None);
        cache.put("key3".to_string(), "value3".to_string(), None);

        // key1 should be evicted (LRU)
        assert_eq!(cache.get("key1"), None);
        // key2 and key3 should exist
        assert_eq!(cache.get("key2"), Some("value2".to_string()));
        assert_eq!(cache.get("key3"), Some("value3".to_string()));
    }

    #[test]
    fn test_cache_key_generation() {
        let key = AICache::key("generate", &[("model", "gpt-3.5"), ("temp", "0.7")]);
        assert!(key.contains("generate"));
        assert!(key.contains("gpt-3.5"));
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = AICache::new(10);
        cache.put("key1".to_string(), "value1".to_string(), None);
        cache.put("key2".to_string(), "value2".to_string(), None);

        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.max_size, 10);
        assert!(stats.utilization > 0.0);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = AICache::new(10);
        cache.put("key1".to_string(), "value1".to_string(), None);
        assert!(cache.remove("key1"));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = AICache::new(10);
        cache.put("key1".to_string(), "value1".to_string(), None);
        cache.put("key2".to_string(), "value2".to_string(), None);
        cache.clear();
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_cache_contains() {
        let mut cache = AICache::new(10);
        cache.put("key1".to_string(), "value1".to_string(), None);
        assert!(cache.contains("key1"));
        assert!(!cache.contains("key2"));
    }
}
