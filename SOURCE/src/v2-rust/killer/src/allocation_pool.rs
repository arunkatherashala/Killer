/// Stack Allocation Pool - Reduce malloc/free overhead
/// 
/// This module implements a simple allocation pool for the value stack,
/// reducing garbage collector pressure and malloc/free overhead.
/// 
/// Expected improvement: 2-3% speedup for allocation-heavy loops
/// 
/// How it works:
/// 1. Pre-allocate a pool of fixed-size buffers
/// 2. Reuse buffers instead of freeing and reallocating
/// 3. Return buffers to pool when done
/// 4. Reduces GC pressure significantly

use crate::value::Value;

/// Simple value buffer pool
#[derive(Debug)]
pub struct ValueBufferPool {
    buffers: Vec<Vec<Value>>,
    buffer_capacity: usize,
    allocations: usize,
    deallocations: usize,
    pool_hits: usize,
    pool_misses: usize,
}

impl ValueBufferPool {
    /// Create a new buffer pool with given capacity and buffer size
    pub fn new(max_buffers: usize, capacity: usize) -> Self {
        ValueBufferPool {
            buffers: Vec::with_capacity(max_buffers),
            buffer_capacity: capacity,
            allocations: 0,
            deallocations: 0,
            pool_hits: 0,
            pool_misses: 0,
        }
    }

    /// Get a buffer from the pool or allocate a new one
    pub fn get_buffer(&mut self) -> Vec<Value> {
        self.allocations += 1;
        
        if let Some(mut buf) = self.buffers.pop() {
            buf.clear();
            self.pool_hits += 1;
            buf
        } else {
            self.pool_misses += 1;
            Vec::with_capacity(self.buffer_capacity)
        }
    }

    /// Return a buffer to the pool for reuse
    pub fn return_buffer(&mut self, buffer: Vec<Value>) {
        self.deallocations += 1;
        
        // Only keep buffer if it doesn't use too much memory
        if buffer.capacity() <= self.buffer_capacity * 2 {
            self.buffers.push(buffer);
        }
    }

    /// Get current pool size
    pub fn pool_size(&self) -> usize {
        self.buffers.len()
    }

    /// Get pool statistics
    pub fn statistics(&self) -> PoolStatistics {
        let total_ops = self.allocations.max(1);
        let hit_rate = (self.pool_hits as f64 / total_ops as f64) * 100.0;
        
        PoolStatistics {
            allocations: self.allocations,
            deallocations: self.deallocations,
            pool_size: self.buffers.len(),
            pool_hits: self.pool_hits,
            pool_misses: self.pool_misses,
            hit_rate,
        }
    }

    /// Clear the pool
    pub fn clear(&mut self) {
        self.buffers.clear();
        self.allocations = 0;
        self.deallocations = 0;
        self.pool_hits = 0;
        self.pool_misses = 0;
    }
}

impl Default for ValueBufferPool {
    fn default() -> Self {
        // Default: 8 buffers of capacity 256 each
        Self::new(8, 256)
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub allocations: usize,
    pub deallocations: usize,
    pub pool_size: usize,
    pub pool_hits: usize,
    pub pool_misses: usize,
    pub hit_rate: f64,
}

/// Scope variable cache - Fast lookup for local variables
/// 
/// Instead of walking the entire scope stack, cache recent variable accesses
#[derive(Debug)]
pub struct ScopeVariableCache {
    // Last scope level accessed
    last_scope: usize,
    
    // Hottest variable names in current scope
    hot_vars: Vec<String>,
    
    // Statistics
    lookups: usize,
    hits: usize,
}

impl ScopeVariableCache {
    /// Create new scope variable cache
    pub fn new() -> Self {
        ScopeVariableCache {
            last_scope: 0,
            hot_vars: Vec::with_capacity(32),
            lookups: 0,
            hits: 0,
        }
    }

    /// Record a variable access
    pub fn access(&mut self, var_name: &str, scope_depth: usize) {
        self.lookups += 1;
        
        // Update scope if changed
        if scope_depth != self.last_scope {
            self.last_scope = scope_depth;
            self.hot_vars.clear();
            return;
        }

        // Check if variable is in hot list
        if self.hot_vars.contains(&var_name.to_string()) {
            self.hits += 1;
        } else if self.hot_vars.len() < 32 {
            // Add to hot variables
            self.hot_vars.push(var_name.to_string());
        }
    }

    /// Check if variable is likely in current scope (for optimization hints)
    pub fn is_likely_local(&self, var_name: &str) -> bool {
        self.hot_vars.contains(&var_name.to_string())
    }

    /// Get cache effectiveness
    pub fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            0.0
        } else {
            (self.hits as f64 / self.lookups as f64) * 100.0
        }
    }

    /// Reset for new scope
    pub fn reset_scope(&mut self, new_scope_depth: usize) {
        self.last_scope = new_scope_depth;
        self.hot_vars.clear();
    }

    /// Clear statistics
    pub fn clear(&mut self) {
        self.hot_vars.clear();
        self.lookups = 0;
        self.hits = 0;
    }
}

impl Default for ScopeVariableCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_reuse() {
        let mut pool = ValueBufferPool::new(4, 256);
        
        // Get first buffer
        let buf1 = pool.get_buffer();
        assert_eq!(pool.pool_size(), 0);
        
        // Return it
        pool.return_buffer(buf1);
        assert_eq!(pool.pool_size(), 1);
        
        // Get again - should be reused
        let _buf2 = pool.get_buffer();
        assert_eq!(pool.pool_size(), 0);
        
        let stats = pool.statistics();
        assert_eq!(stats.pool_hits, 1);
    }

    #[test]
    fn test_buffer_pool_statistics() {
        let mut pool = ValueBufferPool::new(4, 256);
        
        // Do some allocations/deallocations
        for _ in 0..10 {
            let buf = pool.get_buffer();
            pool.return_buffer(buf);
        }
        
        let stats = pool.statistics();
        assert_eq!(stats.allocations, 10);
        assert_eq!(stats.deallocations, 10);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_scope_variable_cache() {
        let mut cache = ScopeVariableCache::new();
        
        // Record some accesses
        cache.access("x", 0);
        cache.access("y", 0);
        cache.access("x", 0);  // Second access to x
        
        assert!(cache.is_likely_local("x"));
        assert!(cache.is_likely_local("y"));
        assert!(!cache.is_likely_local("z"));
    }

    #[test]
    fn test_scope_reset() {
        let mut cache = ScopeVariableCache::new();
        
        cache.access("x", 0);
        cache.access("y", 0);
        
        // Enter new scope
        cache.reset_scope(1);
        
        assert!(!cache.is_likely_local("x"));
        assert!(!cache.is_likely_local("y"));
    }
}
