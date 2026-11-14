// Phase 3.3: Polymorphic Dispatch Optimizer
// Optimizes runtime dispatch between specialized and generic code paths
// Uses inline caches and fast-path specialization

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum DispatchStrategy {
    /// Always use interpreter (default fallback)
    AlwaysPolymorphic,
    /// Inline cache: cache last seen type, check fast path
    InlineCache,
    /// Polymorphic inline cache: stash multiple type specializations
    PolymorphicInlineCache(usize), // max cache entries
    /// Virtual method table with dispatch
    VirtualMethodTable,
}

#[derive(Debug, Clone)]
pub struct DispatchCacheEntry {
    pub type_signature: String,
    pub specialization_id: usize,
    pub hits: u32,
    pub misses: u32,
}

#[derive(Debug)]
pub struct DispatchOptimizer {
    /// Selected dispatch strategy
    strategy: DispatchStrategy,
    /// Inline caches per function
    inline_caches: HashMap<String, DispatchCacheEntry>,
    /// Polymorphic inline caches (per function, multiple entries)
    pic_caches: HashMap<String, Vec<DispatchCacheEntry>>,
    /// Total dispatch decisions
    total_dispatches: u32,
    /// Cache hits (successful dispatch)
    cache_hits: u32,
    /// Cache misses (fallback to polymorphic)
    cache_misses: u32,
}

impl DispatchOptimizer {
    pub fn new(strategy: DispatchStrategy) -> Self {
        DispatchOptimizer {
            strategy,
            inline_caches: HashMap::new(),
            pic_caches: HashMap::new(),
            total_dispatches: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Record dispatch attempt and update cache
    pub fn dispatch(
        &mut self,
        function_name: &str,
        type_signature: &str,
        specialization_id: usize,
    ) -> DispatchResult {
        self.total_dispatches += 1;

        match &self.strategy {
            DispatchStrategy::AlwaysPolymorphic => {
                // Always fall back to polymorphic dispatch
                self.cache_misses += 1;
                DispatchResult::FallbackToPolymorphic
            }

            DispatchStrategy::InlineCache => {
                // Single-entry inline cache
                let cache = self
                    .inline_caches
                    .entry(function_name.to_string())
                    .or_insert_with(|| DispatchCacheEntry {
                        type_signature: type_signature.to_string(),
                        specialization_id,
                        hits: 0,
                        misses: 0,
                    });

                if cache.type_signature == type_signature {
                    // Cache hit
                    cache.hits += 1;
                    self.cache_hits += 1;
                    DispatchResult::UseSpecialization(specialization_id)
                } else {
                    // Cache miss - update for next time
                    cache.misses += 1;
                    self.cache_misses += 1;
                    cache.type_signature = type_signature.to_string();
                    cache.specialization_id = specialization_id;
                    DispatchResult::FallbackToPolymorphic
                }
            }

            DispatchStrategy::PolymorphicInlineCache(max_entries) => {
                // Multi-entry polymorphic inline cache
                let caches = self
                    .pic_caches
                    .entry(function_name.to_string())
                    .or_insert_with(Vec::new);

                // Check existing cache entries
                if let Some(entry) = caches.iter_mut().find(|e| e.type_signature == type_signature)
                {
                    // Hit in PIC
                    entry.hits += 1;
                    self.cache_hits += 1;
                    return DispatchResult::UseSpecialization(entry.specialization_id);
                }

                // Miss in PIC - add new entry if space
                if caches.len() < *max_entries {
                    caches.push(DispatchCacheEntry {
                        type_signature: type_signature.to_string(),
                        specialization_id,
                        hits: 1,
                        misses: 0,
                    });
                    self.cache_misses += 1;
                    DispatchResult::FallbackToPolymorphic
                } else {
                    // PIC full - evict least recently used
                    if let Some(min_entry) = caches.iter_mut().min_by_key(|e| e.hits) {
                        min_entry.type_signature = type_signature.to_string();
                        min_entry.specialization_id = specialization_id;
                        min_entry.hits = 1;
                        min_entry.misses = 0;
                    }
                    self.cache_misses += 1;
                    DispatchResult::FallbackToPolymorphic
                }
            }

            DispatchStrategy::VirtualMethodTable => {
                // Virtual table dispatch - always successful
                let cache = self
                    .inline_caches
                    .entry(function_name.to_string())
                    .or_insert_with(|| DispatchCacheEntry {
                        type_signature: type_signature.to_string(),
                        specialization_id,
                        hits: 0,
                        misses: 0,
                    });

                cache.hits += 1;
                self.cache_hits += 1;
                DispatchResult::UseSpecialization(specialization_id)
            }
        }
    }

    /// Get dispatch statistics
    pub fn get_statistics(&self) -> DispatchStatistics {
        let hit_rate = if self.total_dispatches > 0 {
            (self.cache_hits * 100) / self.total_dispatches
        } else {
            0
        };

        DispatchStatistics {
            total_dispatches: self.total_dispatches,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            hit_rate_percent: hit_rate,
            strategy_name: format!("{:?}", self.strategy),
            cached_functions: self.inline_caches.len() + self.pic_caches.len(),
        }
    }

    /// Estimate speedup from dispatch optimization
    pub fn estimate_dispatch_speedup(&self) -> f32 {
        // Hit rate * 2. speedup (specialized is ~2x faster than polymorphic)
        let hit_rate = self.cache_hits as f32 / self.total_dispatches.max(1) as f32;
        1.0 + (hit_rate * 1.5) // 1.0 + (hit_rate * speedup_factor - 1)
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.inline_caches.clear();
        self.pic_caches.clear();
    }

    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.total_dispatches = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
}

#[derive(Debug, Clone)]
pub enum DispatchResult {
    /// Use specialized code path with given ID
    UseSpecialization(usize),
    /// Fall back to polymorphic dispatch
    FallbackToPolymorphic,
}

#[derive(Debug, Clone)]
pub struct DispatchStatistics {
    pub total_dispatches: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub hit_rate_percent: u32,
    pub strategy_name: String,
    pub cached_functions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_polymorphic() {
        let mut optimizer = DispatchOptimizer::new(DispatchStrategy::AlwaysPolymorphic);

        let result =
            optimizer.dispatch("add", "add_i64_i64", 0);
        assert!(matches!(result, DispatchResult::FallbackToPolymorphic));
    }

    #[test]
    fn test_inline_cache_hit() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        // First dispatch
        optimizer.dispatch("add", "add_i64_i64", 0);
        // Second dispatch with same type - should hit
        let result = optimizer.dispatch("add", "add_i64_i64", 0);

        assert!(matches!(result, DispatchResult::UseSpecialization(0)));
        assert_eq!(optimizer.cache_hits, 1);
    }

    #[test]
    fn test_inline_cache_miss() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        optimizer.dispatch("add", "add_i64_i64", 0);
        // Different type - cache miss
        let result = optimizer.dispatch("add", "add_f64_f64", 1);

        assert!(matches!(result, DispatchResult::FallbackToPolymorphic));
        assert_eq!(optimizer.cache_misses, 2); // First miss from cache, second from type change
    }

    #[test]
    fn test_pic_multiple_entries() {
        let mut optimizer = DispatchOptimizer::new(DispatchStrategy::PolymorphicInlineCache(3));

        optimizer.dispatch("add", "add_i64_i64", 0);
        optimizer.dispatch("add", "add_f64_f64", 1);
        optimizer.dispatch("add", "add_string_string", 2);

        // All should be in PIC now, next access should hit
        let result = optimizer.dispatch("add", "add_i64_i64", 0);
        assert_eq!(optimizer.cache_hits, 1);
    }

    #[test]
    fn test_vmt_always_hits() {
        let mut optimizer = DispatchOptimizer::new(DispatchStrategy::VirtualMethodTable);

        optimizer.dispatch("add", "add_i64_i64", 0);
        optimizer.dispatch("add", "add_f64_f64", 1);

        let result = optimizer.dispatch("add", "add_string_string", 2);
        assert!(matches!(result, DispatchResult::UseSpecialization(_)));
        assert!(optimizer.cache_hits >= 1);
    }

    #[test]
    fn test_dispatch_statistics() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        for _ in 0..8 {
            optimizer.dispatch("add", "add_i64_i64", 0);
        }
        for _ in 0..2 {
            optimizer.dispatch("add", "add_f64_f64", 1);
        }

        let stats = optimizer.get_statistics();
        assert_eq!(stats.total_dispatches, 10);
        assert_eq!(stats.cache_misses, 2);
    }

    #[test]
    fn test_speedup_estimation() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        for _ in 0..9 {
            optimizer.dispatch("add", "add_i64_i64", 0);
        }
        for _ in 0..1 {
            optimizer.dispatch("add", "add_f64_f64", 1);
        }

        let speedup = optimizer.estimate_dispatch_speedup();
        // 90% hit rate * 1.5 speedup factor ≈ 1.35x faster
        assert!(speedup > 1.3 && speedup < 1.4);
    }

    #[test]
    fn test_clear_caches() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        optimizer.dispatch("add", "add_i64_i64", 0);
        assert!(optimizer.inline_caches.len() > 0);

        optimizer.clear_caches();
        assert_eq!(optimizer.inline_caches.len(), 0);
    }

    #[test]
    fn test_reset_statistics() {
        let mut optimizer =
            DispatchOptimizer::new(DispatchStrategy::InlineCache);

        optimizer.dispatch("add", "add_i64_i64", 0);
        assert!(optimizer.total_dispatches > 0);

        optimizer.reset_statistics();
        assert_eq!(optimizer.total_dispatches, 0);
        assert_eq!(optimizer.cache_hits, 0);
    }
}
