/// Call Site Caching - Inline method/function call optimization
/// 
/// This module implements inline caching for method and function calls,
/// eliminating repeated HashMap lookups for the same call sites.
/// 
/// Expected improvement: 3-5% speedup for call-heavy workloads
/// 
/// How it works:
/// 1. First call to method: Do full lookup, record in cache
/// 2. Subsequent calls: Check cache first (O(1) instead of O(log n))
/// 3. Cache invalidation: On class redefinition (rare)

use std::collections::HashMap;

/// Cached method information
#[derive(Clone, Debug)]
pub struct CachedMethod {
    pub class_name: String,
    pub method_name: String,
    pub argument_count: usize,
    pub bytecode_start: Option<usize>,
    /// Number of parameters on the resolved method (redundant with `param_names` when present).
    pub param_count: usize,
    pub param_names: Option<Vec<String>>,
    pub hits: usize,
}

/// Cached function information
#[derive(Clone, Debug)]
pub struct CachedFunction {
    pub function_name: String,
    pub argument_count: usize,
    /// Resolved entry point in bytecode when known (`None` until resolved).
    pub target: Option<usize>,
    /// Arity of the resolved function (parameter count).
    pub arity: usize,
    pub hits: usize,
}

/// Call site cache for method and function dispatch
#[derive(Debug)]
pub struct CallSiteCache {
    method_cache: HashMap<(String, String), CachedMethod>,
    function_cache: HashMap<String, CachedFunction>,
    total_method_calls: usize,
    total_function_calls: usize,
    method_cache_hits: usize,
    function_cache_hits: usize,
}

impl CallSiteCache {
    /// Create a new call site cache
    pub fn new() -> Self {
        CallSiteCache {
            method_cache: HashMap::new(),
            function_cache: HashMap::new(),
            total_method_calls: 0,
            total_function_calls: 0,
            method_cache_hits: 0,
            function_cache_hits: 0,
        }
    }

    /// Look up a cached method resolution.
    /// Returns `Some((bytecode_start, param_names))` on a full cache hit,
    /// or `None` on miss (the VM should do the full walk and call `store_method_resolution`).
    pub fn lookup_method(&mut self, class_name: &str, method_name: &str, arg_count: usize) -> Option<(usize, &[String])> {
        self.total_method_calls += 1;

        let key = (class_name.to_string(), method_name.to_string());

        if let Some(cached) = self.method_cache.get_mut(&key) {
            if cached.argument_count == arg_count {
                cached.hits += 1;
                self.method_cache_hits += 1;
            }
        }

        self.method_cache.get(&key).and_then(|cached| {
            if cached.argument_count != arg_count {
                return None;
            }
            let start = cached.bytecode_start?;
            match cached.param_names.as_ref() {
                Some(params) => Some((start, params.as_slice())),
                None => Some((start, &[] as &[String])),
            }
        })
    }

    /// Record a method call site (without full resolution data).
    /// For full inline caching, use `store_method_resolution` instead.
    pub fn record_method(&mut self, class_name: String, method_name: String, arg_count: usize) {
        let key = (class_name.clone(), method_name.clone());
        self.method_cache.insert(key, CachedMethod {
            class_name,
            method_name,
            argument_count: arg_count,
            bytecode_start: None,
            param_count: 0,
            param_names: None,
            hits: 1,
        });
    }

    /// Store a fully resolved method so subsequent lookups skip the method walk.
    pub fn store_method_resolution(
        &mut self,
        _call_site: String,
        class_name: &str,
        method_name: &str,
        bytecode_start: usize,
        params: Vec<String>,
    ) {
        let key = (class_name.to_string(), method_name.to_string());
        let arg_count = params.len();
        self.method_cache.insert(key, CachedMethod {
            class_name: class_name.to_string(),
            method_name: method_name.to_string(),
            argument_count: arg_count,
            bytecode_start: Some(bytecode_start),
            param_count: arg_count,
            param_names: Some(params),
            hits: 1,
        });
    }

    /// Store a resolved method lookup (bytecode entry and parameter count only).
    /// Use when the VM does not need cached parameter names for dispatch.
    pub fn cache_method_resolution(
        &mut self,
        class_name: &str,
        method_name: &str,
        bytecode_start: usize,
        param_count: usize,
    ) {
        let key = (class_name.to_string(), method_name.to_string());
        self.method_cache.insert(key, CachedMethod {
            class_name: class_name.to_string(),
            method_name: method_name.to_string(),
            argument_count: param_count,
            bytecode_start: Some(bytecode_start),
            param_count,
            param_names: None,
            hits: 1,
        });
    }

    /// Fast path: return `(bytecode_start, param_count)` if this call site was resolved.
    /// Does not update statistics; use `lookup_method` when profiling cache behavior.
    pub fn try_cached_method(&self, class_name: &str, method_name: &str) -> Option<(usize, usize)> {
        let key = (class_name.to_string(), method_name.to_string());
        self.method_cache.get(&key).and_then(|cached| {
            cached.bytecode_start.map(|start| (start, cached.param_count))
        })
    }

    /// Remove all cached entries for a given class (needed when classes are redefined).
    pub fn invalidate_class(&mut self, class_name: &str) {
        self.method_cache.retain(|_, cached| cached.class_name != class_name);
    }

    /// Look up a function, recording cache statistics
    pub fn lookup_function(&mut self, function_name: &str, arg_count: usize) -> bool {
        self.total_function_calls += 1;
        
        if let Some(cached) = self.function_cache.get_mut(function_name) {
            if cached.argument_count == arg_count {
                cached.hits += 1;
                self.function_cache_hits += 1;
                return true;  // Cache hit
            }
        }
        
        false  // Cache miss
    }

    /// Record a new function
    pub fn record_function(&mut self, function_name: String, arg_count: usize) {
        self.function_cache.insert(function_name.clone(), CachedFunction {
            function_name,
            argument_count: arg_count,
            target: None,
            arity: arg_count,
            hits: 1,
        });
    }

    /// Get method cache hit rate (percentage)
    pub fn method_hit_rate(&self) -> f64 {
        if self.total_method_calls == 0 {
            0.0
        } else {
            (self.method_cache_hits as f64 / self.total_method_calls as f64) * 100.0
        }
    }

    /// Get function cache hit rate (percentage)
    pub fn function_hit_rate(&self) -> f64 {
        if self.total_function_calls == 0 {
            0.0
        } else {
            (self.function_cache_hits as f64 / self.total_function_calls as f64) * 100.0
        }
    }

    /// Get overall cache hit rate
    pub fn overall_hit_rate(&self) -> f64 {
        let total = self.total_method_calls + self.total_function_calls;
        if total == 0 {
            0.0
        } else {
            let hits = self.method_cache_hits + self.function_cache_hits;
            (hits as f64 / total as f64) * 100.0
        }
    }

    /// Clear all caches (for class redefinition)
    pub fn clear(&mut self) {
        self.method_cache.clear();
        self.function_cache.clear();
        self.total_method_calls = 0;
        self.total_function_calls = 0;
        self.method_cache_hits = 0;
        self.function_cache_hits = 0;
    }

    /// Get cache statistics for profiling
    pub fn statistics(&self) -> CallSiteCacheStats {
        CallSiteCacheStats {
            method_cache_size: self.method_cache.len(),
            function_cache_size: self.function_cache.len(),
            total_method_calls: self.total_method_calls,
            total_function_calls: self.total_function_calls,
            method_cache_hits: self.method_cache_hits,
            function_cache_hits: self.function_cache_hits,
            method_hit_rate: self.method_hit_rate(),
            function_hit_rate: self.function_hit_rate(),
            overall_hit_rate: self.overall_hit_rate(),
        }
    }
}

impl Default for CallSiteCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Call site cache statistics
#[derive(Debug, Clone)]
pub struct CallSiteCacheStats {
    pub method_cache_size: usize,
    pub function_cache_size: usize,
    pub total_method_calls: usize,
    pub total_function_calls: usize,
    pub method_cache_hits: usize,
    pub function_cache_hits: usize,
    pub method_hit_rate: f64,
    pub function_hit_rate: f64,
    pub overall_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_cache_basic() {
        let mut cache = CallSiteCache::new();

        // First lookup misses
        assert!(cache.lookup_method("String", "length", 0).is_none());
        assert_eq!(cache.total_method_calls, 1);
        assert_eq!(cache.method_cache_hits, 0);

        // Store resolved method
        cache.store_method_resolution(
            "site_1".to_string(), "String", "length", 42, vec![],
        );

        // Next lookup should hit and return resolution
        let result = cache.lookup_method("String", "length", 0);
        assert!(result.is_some());
        let (start, params) = result.unwrap();
        assert_eq!(start, 42);
        assert!(params.is_empty());
        assert_eq!(cache.total_method_calls, 2);
        assert_eq!(cache.method_cache_hits, 1);
    }

    #[test]
    fn test_function_cache_basic() {
        let mut cache = CallSiteCache::new();
        
        // First lookup misses
        assert!(!cache.lookup_function("print", 1));
        cache.record_function("print".to_string(), 1);
        
        // Subsequent lookups hit
        assert!(cache.lookup_function("print", 1));
        assert!(cache.lookup_function("print", 1));
        
        let stats = cache.statistics();
        assert_eq!(stats.function_cache_hits, 2);
        assert!(stats.function_hit_rate > 66.0);
    }

    #[test]
    fn test_cache_hit_rate_calculation() {
        let mut cache = CallSiteCache::new();

        cache.store_method_resolution("s1".to_string(), "String", "length", 10, vec![]);
        cache.store_method_resolution("s2".to_string(), "Array", "push", 20, vec!["item".to_string()]);

        for _ in 0..10 {
            cache.lookup_method("String", "length", 0);
        }

        cache.lookup_method("Array", "push", 1);

        let hit_rate = cache.method_hit_rate();
        assert!(hit_rate > 80.0);
    }

    #[test]
    fn test_cache_wrong_arg_count() {
        let mut cache = CallSiteCache::new();

        cache.store_method_resolution("s1".to_string(), "Array", "push", 20, vec!["item".to_string()]);

        // Same method but different arg count -> cache miss
        assert!(cache.lookup_method("Array", "push", 2).is_none());
        assert_eq!(cache.method_cache_hits, 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = CallSiteCache::new();
        
        cache.record_function("print".to_string(), 1);
        assert!(cache.lookup_function("print", 1));
        
        cache.clear();
        
        assert!(!cache.lookup_function("print", 1));
        assert_eq!(cache.total_function_calls, 1);
        assert_eq!(cache.function_cache_hits, 0);
    }

    #[test]
    fn test_store_and_lookup_resolution() {
        let mut cache = CallSiteCache::new();

        cache.store_method_resolution(
            "call_site_0".to_string(),
            "MyClass",
            "greet",
            100,
            vec!["name".to_string(), "greeting".to_string()],
        );

        let result = cache.lookup_method("MyClass", "greet", 2);
        assert!(result.is_some());
        let (start, params) = result.unwrap();
        assert_eq!(start, 100);
        assert_eq!(params, &["name", "greeting"]);
    }

    #[test]
    fn test_record_method_without_resolution() {
        let mut cache = CallSiteCache::new();

        // record_method stores an entry without resolution data
        cache.record_method("String".to_string(), "length".to_string(), 0);

        // lookup_method returns None (no bytecode_start) but still tracks the hit
        assert!(cache.lookup_method("String", "length", 0).is_none());
        assert_eq!(cache.method_cache_hits, 1);
    }

    #[test]
    fn test_invalidate_class() {
        let mut cache = CallSiteCache::new();

        cache.store_method_resolution("s1".to_string(), "Foo", "bar", 10, vec![]);
        cache.store_method_resolution("s2".to_string(), "Foo", "baz", 20, vec!["x".to_string()]);
        cache.store_method_resolution("s3".to_string(), "Other", "run", 30, vec![]);

        cache.invalidate_class("Foo");

        assert!(cache.lookup_method("Foo", "bar", 0).is_none());
        assert!(cache.lookup_method("Foo", "baz", 1).is_none());
        assert!(cache.lookup_method("Other", "run", 0).is_some());
    }

    #[test]
    fn test_invalidate_class_preserves_stats() {
        let mut cache = CallSiteCache::new();

        cache.store_method_resolution("s1".to_string(), "Foo", "bar", 10, vec![]);
        cache.lookup_method("Foo", "bar", 0);
        cache.lookup_method("Foo", "bar", 0);

        let hits_before = cache.method_cache_hits;
        cache.invalidate_class("Foo");

        // Stats are preserved across invalidation
        assert_eq!(cache.method_cache_hits, hits_before);
        assert_eq!(cache.method_cache.len(), 0);
    }

    #[test]
    fn test_cache_method_resolution_and_try_cached_method() {
        let mut cache = CallSiteCache::new();

        assert!(cache.try_cached_method("A", "m").is_none());

        cache.cache_method_resolution("A", "m", 99, 2);

        assert_eq!(cache.try_cached_method("A", "m"), Some((99, 2)));

        let resolved = cache.lookup_method("A", "m", 2);
        assert!(resolved.is_some());
        let (start, names) = resolved.unwrap();
        assert_eq!(start, 99);
        assert!(names.is_empty());
    }

    #[test]
    fn test_cached_function_has_target_and_arity() {
        let mut cache = CallSiteCache::new();
        cache.record_function("f".to_string(), 3);
        let cf = cache.function_cache.get("f").unwrap();
        assert!(cf.target.is_none());
        assert_eq!(cf.arity, 3);
    }
}
