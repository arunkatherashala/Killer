#![allow(unsafe_code)]

// MEMORY OPTIMIZATION MODULE - Phase 2: Smart Allocation & Cache Optimization
// Target: +20-30% memory throughput improvement
// Goal: Optimize DDR4 latency and reduce GC overhead

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::alloc::{GlobalAlloc, Layout};

/// Memory Pool pre-allocator for hot allocations
#[derive(Clone)]
pub struct MemoryPool {
    name: String,
    pool_size: usize,
    object_size: usize,
    available: Arc<Mutex<Vec<*mut u8>>>,
    allocated: Arc<Mutex<usize>>,
}

impl MemoryPool {
    pub fn new(name: &str, object_size: usize, pool_size: usize) -> Self {
        let mut pool = Vec::with_capacity(pool_size);

        // Pre-allocate memory with cache-line alignment (64 bytes)
        for _ in 0..pool_size {
            let layout =
                Layout::from_size_align(object_size, 64).unwrap_or_else(|_| Layout::new::<u8>());
            unsafe {
                let ptr = std::alloc::alloc(layout);
                if !ptr.is_null() {
                    pool.push(ptr);
                }
            }
        }

        MemoryPool {
            name: name.to_string(),
            pool_size,
            object_size,
            available: Arc::new(Mutex::new(pool)),
            allocated: Arc::new(Mutex::new(0)),
        }
    }

    /// Allocate from pool (reuse pre-allocated memory)
    pub fn allocate(&self) -> Option<*mut u8> {
        if let Ok(mut available) = self.available.lock() {
            if let Some(ptr) = available.pop() {
                if let Ok(mut allocated) = self.allocated.lock() {
                    *allocated += 1;
                }
                return Some(ptr);
            }
        }
        None
    }

    /// Return memory to pool
    pub fn deallocate(&self, ptr: *mut u8) {
        if !ptr.is_null() {
            if let Ok(mut available) = self.available.lock() {
                if available.len() < self.pool_size {
                    available.push(ptr);
                    if let Ok(mut allocated) = self.allocated.lock() {
                        *allocated = allocated.saturating_sub(1);
                    }
                }
            }
        }
    }

    pub fn get_stats(&self) -> MemoryPoolStats {
        let available_count = self
            .available
            .lock()
            .map(|v| v.len())
            .unwrap_or(0);

        let allocated = self
            .allocated
            .lock()
            .map(|a| *a)
            .unwrap_or(0);

        MemoryPoolStats {
            name: self.name.clone(),
            total_size: self.pool_size,
            object_size: self.object_size,
            allocated,
            available: available_count,
            utilization: if self.pool_size > 0 {
                (allocated as f32) / (self.pool_size as f32)
            } else {
                0.0
            },
        }
    }

    pub fn print_stats(&self) {
        let stats = self.get_stats();
        println!(
            "  {} Pool: {}/{} objects ({:.1}% utilization)",
            stats.name, stats.allocated, stats.total_size, stats.utilization * 100.0
        );
    }
}

#[derive(Clone, Debug)]
pub struct MemoryPoolStats {
    pub name: String,
    pub total_size: usize,
    pub object_size: usize,
    pub allocated: usize,
    pub available: usize,
    pub utilization: f32,
}

/// Cache-aware memory layout optimizer
pub struct CacheLayout {
    cache_line_size: usize,
    false_sharing_threshold: usize,
}

impl CacheLayout {
    pub fn new() -> Self {
        CacheLayout {
            cache_line_size: 64, // Standard x86-64 cache line
            false_sharing_threshold: 128, // 2x cache line for safety
        }
    }

    /// Calculate aligned size for cache efficiency
    pub fn align_size(&self, size: usize) -> usize {
        if size == 0 {
            return 0;
        }

        let remainder = size % self.cache_line_size;
        if remainder == 0 {
            size
        } else {
            size + (self.cache_line_size - remainder)
        }
    }

    /// Pad structure to prevent false sharing
    pub fn pad_for_false_sharing_prevention(&self, size: usize) -> usize {
        self.align_size(size + self.cache_line_size)
    }

    pub fn get_alignment(&self) -> usize {
        self.cache_line_size
    }
}

impl Default for CacheLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive memory sizing based on system configuration
#[derive(Clone, Debug)]
pub struct AdaptiveMemorySizer {
    available_ram_mb: usize,
    num_cores: usize,
    policy: MemorySizePolicy,
}

#[derive(Clone, Debug, Copy)]
pub enum MemorySizePolicy {
    /// Use 25% of available RAM
    Conservative,
    /// Use 50% of available RAM
    Balanced,
    /// Use 75% of available RAM
    Aggressive,
}

impl AdaptiveMemorySizer {
    pub fn new(available_ram_mb: usize, num_cores: usize) -> Self {
        AdaptiveMemorySizer {
            available_ram_mb,
            num_cores,
            policy: MemorySizePolicy::Balanced,
        }
    }

    pub fn set_policy(&mut self, policy: MemorySizePolicy) {
        self.policy = policy;
    }

    /// Get recommended heap size
    pub fn get_heap_size_mb(&self) -> usize {
        let percentage = match self.policy {
            MemorySizePolicy::Conservative => 0.25,
            MemorySizePolicy::Balanced => 0.50,
            MemorySizePolicy::Aggressive => 0.75,
        };

        ((self.available_ram_mb as f32) * percentage) as usize
    }

    /// Get recommended garbage collection threshold
    pub fn get_gc_threshold_mb(&self) -> usize {
        let heap_size = self.get_heap_size_mb();
        (heap_size as f32 * 0.7) as usize
    }

    /// Get pool size for object allocation
    pub fn get_pool_size(&self, object_size: usize) -> usize {
        let heap_size = self.get_heap_size_mb();
        let bytes = heap_size * 1024 * 1024;
        bytes / object_size
    }

    /// Get recommended memory per core
    pub fn get_memory_per_core_mb(&self) -> usize {
        self.get_heap_size_mb() / self.num_cores.max(1)
    }

    pub fn print_recommendations(&self) {
        println!("\n=== ADAPTIVE MEMORY SIZING ===");
        println!("Available RAM: {} MB", self.available_ram_mb);
        println!("Number of cores: {}", self.num_cores);
        println!("Policy: {:?}", self.policy);
        println!("Recommended heap: {} MB", self.get_heap_size_mb());
        println!("GC threshold: {} MB", self.get_gc_threshold_mb());
        println!("Per core: {} MB", self.get_memory_per_core_mb());
        println!("=====================================\n");
    }
}

/// DDR4-optimized memory access pattern
pub struct DDR4Optimizer {
    row_buffer_size: usize, // 2KB for typical DDR4
    timing_cl: u32,         // CAS latency
    timing_rcd: u32,        // RAS to CAS delay
}

impl DDR4Optimizer {
    pub fn new() -> Self {
        DDR4Optimizer {
            row_buffer_size: 2048,  // 2KB row buffer
            timing_cl: 16,          // CAS latency in cycles
            timing_rcd: 16,         // RAS-to-CAS in cycles
        }
    }

    /// Calculate sequential access cost
    pub fn estimate_sequential_latency_ns(&self) -> f32 {
        // Assume 2.6 GHz = ~0.38ns per cycle
        let ns_per_cycle = 1.0 / 2600.0 * 1000.0;
        (self.timing_cl as f32) * ns_per_cycle
    }

    /// Calculate random access cost (row buffer miss)
    pub fn estimate_random_latency_ns(&self) -> f32 {
        // Row miss: RCD + CL + precharge time
        let ns_per_cycle = 1.0 / 2600.0 * 1000.0;
        ((self.timing_rcd + self.timing_cl) as f32) * ns_per_cycle
    }

    /// Recommend access pattern
    pub fn recommend_access_pattern(&self, data_size: usize) -> AccessPattern {
        if data_size <= self.row_buffer_size {
            AccessPattern::Sequential
        } else {
            AccessPattern::CacheOptimized
        }
    }
}

impl Default for DDR4Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccessPattern {
    Sequential,      // Full row buffer hits
    CacheOptimized,  // L3 cache optimized
    Strided,         // Regular stride pattern
}

/// Global Memory Manager
pub struct MemoryManager {
    pools: Arc<Mutex<HashMap<String, MemoryPool>>>,
    sizer: AdaptiveMemorySizer,
    ddr4_optimizer: DDR4Optimizer,
    cache_layout: CacheLayout,
}

impl MemoryManager {
    pub fn new(available_ram_mb: usize, num_cores: usize) -> Self {
        let sizer = AdaptiveMemorySizer::new(available_ram_mb, num_cores);

        MemoryManager {
            pools: Arc::new(Mutex::new(HashMap::new())),
            sizer,
            ddr4_optimizer: DDR4Optimizer::new(),
            cache_layout: CacheLayout::new(),
        }
    }

    /// Create or get a memory pool
    pub fn get_pool(&self, name: &str, object_size: usize) -> Option<MemoryPool> {
        let pool_size = self.sizer.get_pool_size(object_size);

        if let Ok(mut pools) = self.pools.lock() {
            if pools.contains_key(name) {
                pools.get(name).cloned()
            } else {
                let pool = MemoryPool::new(name, object_size, pool_size);
                pools.insert(name.to_string(), pool.clone());
                Some(pool)
            }
        } else {
            None
        }
    }

    pub fn print_memory_report(&self) {
        println!("\n+========== MEMORY OPTIMIZATION REPORT ==========+");
        self.sizer.print_recommendations();

        println!("DDR4 Performance Estimates:");
        println!(
            "  Sequential latency: {:.2} ns (row hit)",
            self.ddr4_optimizer.estimate_sequential_latency_ns()
        );
        println!(
            "  Random latency: {:.2} ns (row miss)",
            self.ddr4_optimizer.estimate_random_latency_ns()
        );

        println!("\nMemory Pools:");
        if let Ok(pools) = self.pools.lock() {
            if pools.is_empty() {
                println!("  (No pools allocated yet)");
            } else {
                for (_, pool) in pools.iter() {
                    pool.print_stats();
                }
            }
        }

        println!("\nCache Optimization:");
        println!(
            "  Cache line size: {} bytes",
            self.cache_layout.get_alignment()
        );
        println!(
            "  False sharing prevention: {} bytes padding",
            self.cache_layout.cache_line_size
        );

        println!("+=================================================+\n");
    }
}

impl Clone for MemoryManager {
    fn clone(&self) -> Self {
        MemoryManager {
            pools: Arc::clone(&self.pools),
            sizer: self.sizer.clone(),
            ddr4_optimizer: DDR4Optimizer::default(),
            cache_layout: CacheLayout::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        let pool = MemoryPool::new("test", 256, 10);

        let ptr1 = pool.allocate();
        assert!(ptr1.is_some());

        let stats = pool.get_stats();
        assert_eq!(stats.allocated, 1);
    }

    #[test]
    fn test_cache_layout() {
        let layout = CacheLayout::new();

        assert_eq!(layout.align_size(0), 0);
        assert_eq!(layout.align_size(32), 64);
        assert_eq!(layout.align_size(64), 64);
        assert_eq!(layout.align_size(65), 128);
    }

    #[test]
    fn test_adaptive_memory_sizer() {
        let mut sizer = AdaptiveMemorySizer::new(16384, 4); // 16GB, 4 cores

        assert_eq!(sizer.get_heap_size_mb(), 8192); // 50% of 16GB

        sizer.set_policy(MemorySizePolicy::Conservative);
        assert_eq!(sizer.get_heap_size_mb(), 4096); // 25% of 16GB

        sizer.set_policy(MemorySizePolicy::Aggressive);
        assert_eq!(sizer.get_heap_size_mb(), 12288); // 75% of 16GB
    }

    #[test]
    fn test_ddr4_optimizer() {
        let optimizer = DDR4Optimizer::new();
        assert!(optimizer.estimate_sequential_latency_ns() > 0.0);
        assert!(optimizer.estimate_random_latency_ns() > optimizer.estimate_sequential_latency_ns());
    }
}
