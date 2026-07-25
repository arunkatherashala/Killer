// Custom Optimization Strategy Examples
// Ready-to-extend implementations for users to customize

use std::collections::HashMap;

// ============================================================================
// Example 1: Memory-Optimized Strategy
// ============================================================================

#[derive(Debug)]
pub struct MemoryOptimizationStrategy {
    /// Enable arena allocation
    pub arenas_enabled: bool,
    /// Max arena size (MB)
    pub max_arena_size: usize,
    /// Enable object pooling
    pub pooling_enabled: bool,
    /// Pool sizes per type
    pub pool_sizes: HashMap<String, usize>,
    /// Estimated memory reduction
    pub estimated_reduction_percent: f32,
}

impl MemoryOptimizationStrategy {
    pub fn new() -> Self {
        MemoryOptimizationStrategy {
            arenas_enabled: true,
            max_arena_size: 256, // 256MB
            pooling_enabled: true,
            pool_sizes: {
                let mut m = HashMap::new();
                m.insert("String".to_string(), 1000);
                m.insert("Vec".to_string(), 500);
                m.insert("HashMap".to_string(), 100);
                m
            },
            estimated_reduction_percent: 30.0,
        }
    }

    /// Customize arena size
    pub fn set_arena_size(&mut self, size_mb: usize) -> &mut Self {
        self.max_arena_size = size_mb;
        self
    }

    /// Customize pool size for type
    pub fn set_pool_size(&mut self, type_name: String, size: usize) -> &mut Self {
        self.pool_sizes.insert(type_name, size);
        self
    }

    /// Build optimized configuration
    pub fn build(self) -> String {
        format!(
            "Memory Optimization: {} arena allocation ({}MB), {} object pooling with {} types",
            if self.arenas_enabled { "Enabled" } else { "Disabled" },
            self.max_arena_size,
            if self.pooling_enabled { "Enabled" } else { "Disabled" },
            self.pool_sizes.len()
        )
    }
}

impl Default for MemoryOptimizationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Example 2: Concurrency-Optimized Strategy
// ============================================================================

#[derive(Debug, Clone)]
pub enum ConcurrencyModel {
    ThreadPool,
    ActorModel,
    Coroutines,
    AsyncAwait,
}

#[derive(Debug)]
pub struct ConcurrencyOptimizationStrategy {
    /// Concurrency model
    pub model: ConcurrencyModel,
    /// Number of worker threads (for pool)
    pub worker_count: usize,
    /// Enable work stealing
    pub work_stealing: bool,
    /// Lock-free data structures
    pub lockfree_enabled: bool,
    /// Channel buffer size
    pub channel_buffer_size: usize,
}

impl ConcurrencyOptimizationStrategy {
    pub fn new(model: ConcurrencyModel) -> Self {
        let worker_count = num_cpus();

        ConcurrencyOptimizationStrategy {
            model,
            worker_count,
            work_stealing: true,
            lockfree_enabled: true,
            channel_buffer_size: 1000,
        }
    }

    /// Set worker count
    pub fn set_workers(&mut self, count: usize) -> &mut Self {
        self.worker_count = count;
        self
    }

    /// Enable/disable work stealing
    pub fn set_work_stealing(&mut self, enabled: bool) -> &mut Self {
        self.work_stealing = enabled;
        self
    }

    /// Build configuration
    pub fn build(self) -> String {
        format!(
            "Concurrency: {:?} model with {} workers, {}, {}",
            self.model,
            self.worker_count,
            if self.work_stealing { "work-stealing enabled" } else { "work-stealing disabled" },
            if self.lockfree_enabled { "lock-free structures" } else { "locked structures" }
        )
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

impl Default for ConcurrencyOptimizationStrategy {
    fn default() -> Self {
        Self::new(ConcurrencyModel::AsyncAwait)
    }
}

// ============================================================================
// Example 3: Cache-Optimized Strategy  
// ============================================================================

#[derive(Debug)]
pub struct CacheOptimizationStrategy {
    /// L1 cache line size (typically 64 bytes)
    pub l1_cache_line: usize,
    /// Prefetch distance (how many iterations ahead)
    pub prefetch_distance: usize,
    /// Data layout optimization enabled
    pub layout_optimization: bool,
    /// NUMA awareness (for multi-socket systems)
    pub numa_aware: bool,
    /// Cache associativity assumptions
    pub assumed_associativity: u8,
}

impl CacheOptimizationStrategy {
    pub fn new() -> Self {
        CacheOptimizationStrategy {
            l1_cache_line: 64,
            prefetch_distance: 8,
            layout_optimization: true,
            numa_aware: false,
            assumed_associativity: 8,
        }
    }

    /// Enable NUMA awareness
    pub fn with_numa(&mut self) -> &mut Self {
        self.numa_aware = true;
        self
    }

    /// Set prefetch distance
    pub fn set_prefetch(&mut self, distance: usize) -> &mut Self {
        self.prefetch_distance = distance;
        self
    }

    /// Build configuration
    pub fn build(self) -> String {
        format!(
            "Cache Optimization: {} cache line, prefetch={}, {}, {}",
            self.l1_cache_line,
            self.prefetch_distance,
            if self.layout_optimization { "data layout optimized" } else { "default layout" },
            if self.numa_aware { "NUMA-aware" } else { "NUMA-unaware" }
        )
    }
}

impl Default for CacheOptimizationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Example 4: SIMD Vectorization Strategy
// ============================================================================

#[derive(Debug, Clone)]
pub enum VectorWidth {
    SSE,          // 128 bits
    AVX,          // 256 bits
    AVX512,       // 512 bits
}

#[derive(Debug)]
pub struct SIMDVectorizationStrategy {
    /// Vector width
    pub width: VectorWidth,
    /// Auto-vectorize loops
    pub autovectorize: bool,
    /// Fallback on unsupported operations
    pub fallback_enabled: bool,
    /// Enable packed operations
    pub packed_ops: bool,
}

impl SIMDVectorizationStrategy {
    pub fn new(width: VectorWidth) -> Self {
        SIMDVectorizationStrategy {
            width,
            autovectorize: true,
            fallback_enabled: true,
            packed_ops: true,
        }
    }

    /// Set vector width
    pub fn set_width(&mut self, width: VectorWidth) -> &mut Self {
        self.width = width;
        self
    }

    /// Build configuration
    pub fn build(self) -> String {
        format!(
            "SIMD Vectorization: {:?}, autovectorize={}, fallback={}",
            self.width, self.autovectorize, self.fallback_enabled
        )
    }
}

impl Default for SIMDVectorizationStrategy {
    fn default() -> Self {
        Self::new(VectorWidth::AVX)
    }
}

// ============================================================================
// Composite Strategy Builder (Fluent API)
// ============================================================================

#[derive(Debug)]
pub struct CompositeOptimizationBuilder {
    strategies: Vec<String>,
    descriptions: Vec<String>,
    total_effort: f32,
    expected_speedup: f32,
}

impl CompositeOptimizationBuilder {
    pub fn new() -> Self {
        CompositeOptimizationBuilder {
            strategies: Vec::new(),
            descriptions: Vec::new(),
            total_effort: 0.0,
            expected_speedup: 1.0,
        }
    }

    /// Add memory optimization
    pub fn with_memory_optimization(mut self, effort_hours: f32, speedup: f32) -> Self {
        self.strategies.push("Memory Optimization".to_string());
        self.descriptions
            .push("Arena allocation & object pooling".to_string());
        self.total_effort += effort_hours;
        self.expected_speedup *= speedup;
        self
    }

    /// Add concurrency optimization
    pub fn with_concurrency(mut self, effort_hours: f32, speedup: f32) -> Self {
        self.strategies.push("Concurrency".to_string());
        self.descriptions
            .push("Thread pools, async/await, lock-free".to_string());
        self.total_effort += effort_hours;
        self.expected_speedup *= speedup;
        self
    }

    /// Add cache optimization
    pub fn with_cache_optimization(mut self, effort_hours: f32, speedup: f32) -> Self {
        self.strategies.push("Cache Optimization".to_string());
        self.descriptions.push("L1/L2/L3 locality & prefetch".to_string());
        self.total_effort += effort_hours;
        self.expected_speedup *= speedup;
        self
    }

    /// Add SIMD vectorization
    pub fn with_simd_vectorization(mut self, effort_hours: f32, speedup: f32) -> Self {
        self.strategies.push("SIMD Vectorization".to_string());
        self.descriptions
            .push("Auto-vectorization & packed ops".to_string());
        self.total_effort += effort_hours;
        self.expected_speedup *= speedup;
        self
    }

    /// Build the composite strategy
    pub fn build(self) -> CompositeOptimizationPlugin {
        CompositeOptimizationPlugin {
            strategies: self.strategies,
            descriptions: self.descriptions,
            total_effort_hours: self.total_effort,
            expected_total_speedup: self.expected_speedup,
        }
    }
}

impl Default for CompositeOptimizationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct CompositeOptimizationPlugin {
    pub strategies: Vec<String>,
    pub descriptions: Vec<String>,
    pub total_effort_hours: f32,
    pub expected_total_speedup: f32,
}

impl CompositeOptimizationPlugin {
    /// Generate implementation report
    pub fn get_implementation_plan(&self) -> String {
        let mut plan = format!(
            "Composite Optimization Plugin\n\
            Strategies: {}\n\
            Total Effort: {:.1} hours\n\
            Expected Speedup: {:.2}x\n\n",
            self.strategies.join(", "),
            self.total_effort_hours,
            self.expected_total_speedup
        );

        plan.push_str("Implementation Plan:\n");
        for (i, (strategy, desc)) in self
            .strategies
            .iter()
            .zip(self.descriptions.iter())
            .enumerate()
        {
            plan.push_str(&format!("  {}. {} - {}\n", i + 1, strategy, desc));
        }

        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_strategy() {
        let mut strategy = MemoryOptimizationStrategy::new();
        strategy.set_arena_size(512);
        let config = strategy.build();
        assert!(config.contains("Memory Optimization"));
    }

    #[test]
    fn test_concurrency_strategy() {
        let strategy = ConcurrencyOptimizationStrategy::new(ConcurrencyModel::AsyncAwait);
        let config = strategy.build();
        assert!(config.contains("AsyncAwait"));
    }

    #[test]
    fn test_cache_strategy() {
        let strategy = CacheOptimizationStrategy::new();
        let config = strategy.build();
        assert!(config.contains("Cache Optimization"));
    }

    #[test]
    fn test_simd_strategy() {
        let strategy = SIMDVectorizationStrategy::new(VectorWidth::AVX);
        let config = strategy.build();
        assert!(config.contains("SIMD"));
    }

    #[test]
    fn test_composite_builder() {
        let plugin = CompositeOptimizationBuilder::new()
            .with_memory_optimization(20.0, 1.8)
            .with_cache_optimization(15.0, 1.4)
            .build();

        assert_eq!(plugin.strategies.len(), 2);
        assert!(plugin.total_effort_hours > 30.0);
    }

    #[test]
    fn test_implementation_plan() {
        let plugin = CompositeOptimizationBuilder::new()
            .with_simd_vectorization(30.0, 4.0)
            .build();

        let plan = plugin.get_implementation_plan();
        assert!(plan.contains("SIMD Vectorization"));
        assert!(plan.contains("4.00x"));
    }
}
