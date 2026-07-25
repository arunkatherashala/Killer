// killer_rcore/src/jit/loader.rs
// Runtime dynamic loader for compiled JIT binaries
// Week 3 part 4

use std::path::Path;
use std::collections::HashMap;
use std::sync::Mutex;

/// Type signature for JIT loop functions
/// All generated loops export: extern "C" fn killer_jit_loop_xxx() -> i64
pub type JITLoopFn = extern "C" fn() -> i64;

/// Type signature for parameterized functions
pub type JITLoopWithParamFn = extern "C" fn(i64) -> i64;

/// Runtime loader for compiled JIT binaries
pub struct JITLoader {
    /// Cached loaded libraries (path -> library metadata)
    /// Actual implementation requires dynamic linking support
    libraries: Mutex<HashMap<String, bool>>,
    
    /// Function symbol cache (fn_name -> function info)  
    symbols: Mutex<HashMap<String, bool>>,
    
    /// Whether to enable symbol caching
    #[allow(dead_code)]
    cache_symbols: bool,
}

/// Error type for loader operations
#[derive(Debug, Clone)]
pub enum LoadError {
    /// Library not found or cannot be loaded
    LibraryLoadFailed(String),
    
    /// Symbol not found in library
    SymbolNotFound(String),
    
    /// Invalid function signature
    InvalidSignature(String),
    
    /// Path validation failed
    InvalidPath(String),
    
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::LibraryLoadFailed(e) => write!(f, "Failed to load library: {}", e),
            LoadError::SymbolNotFound(e) => write!(f, "Symbol not found: {}", e),
            LoadError::InvalidSignature(e) => write!(f, "Invalid signature: {}", e),
            LoadError::InvalidPath(e) => write!(f, "Invalid path: {}", e),
            LoadError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for LoadError {}

impl JITLoader {
    /// Create new loader with default settings
    pub fn new() -> Self {
        JITLoader {
            libraries: Mutex::new(HashMap::new()),
            symbols: Mutex::new(HashMap::new()),
            cache_symbols: true,
        }
    }
    
    /// Create loader with symbol caching disabled
    pub fn without_caching() -> Self {
        JITLoader {
            libraries: Mutex::new(HashMap::new()),
            symbols: Mutex::new(HashMap::new()),
            cache_symbols: false,
        }
    }
    
    /// Load a compiled JIT binary and get a function pointer
    pub fn load_function<T>(
        &self,
        binary_path: &Path,
        _function_name: &str,
    ) -> Result<T, LoadError>
    where
        T: Copy,
    {
        // Validate path
        if !binary_path.exists() {
            return Err(LoadError::InvalidPath(
                format!("Binary not found: {}", binary_path.display()),
            ));
        }
        
        // Note: Full implementation requires libloading crate
        // For now, return informative error
        Err(LoadError::LibraryLoadFailed(
            "Dynamic library loading requires libloading crate (network dependency pending)".to_string()
        ))
    }
    
    /// Load and execute a parameterless JIT loop (common case)
    pub fn execute_loop_function(
        &self,
        binary_path: &Path,
        function_name: &str,
    ) -> Result<i64, LoadError> {
        let fn_ptr: JITLoopFn = self.load_function(binary_path, function_name)?;
        Ok(fn_ptr())
    }
    
    /// Load and execute a JIT function with one parameter
    pub fn execute_loop_function_with_param(
        &self,
        binary_path: &Path,
        function_name: &str,
        param: i64,
    ) -> Result<i64, LoadError> {
        let fn_ptr: JITLoopWithParamFn = self.load_function(binary_path, function_name)?;
        Ok(fn_ptr(param))
    }
    
    /// Verify a binary can be loaded before execution
    pub fn verify_binary(&self, binary_path: &Path) -> Result<(), LoadError> {
        if !binary_path.exists() {
            return Err(LoadError::InvalidPath(
                format!("Binary not found: {}", binary_path.display()),
            ));
        }
        
        // File exists - that's sufficient verification for now
        // Full implementation would require attempting to load the binary
        Ok(())
    }
    
    /// Clear all cached symbols
    pub fn clear_symbol_cache(&self) {
        if let Ok(mut symbols) = self.symbols.lock() {
            symbols.clear();
        }
    }
    
    /// Clear all loaded libraries
    pub fn clear_library_cache(&self) {
        if let Ok(mut libraries) = self.libraries.lock() {
            libraries.clear();
        }
    }
    
    /// Clear both caches
    pub fn clear_all(&self) {
        self.clear_symbol_cache();
        self.clear_library_cache();
    }
    
    /// Get number of cached symbols
    pub fn cached_symbols_count(&self) -> usize {
        self.symbols.lock().ok().map(|s| s.len()).unwrap_or(0)
    }
    
    /// Get number of cached libraries
    pub fn cached_libraries_count(&self) -> usize {
        self.libraries.lock().ok().map(|l| l.len()).unwrap_or(0)
    }
}

impl Default for JITLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_loader_creation() {
        let loader = JITLoader::new();
        assert!(loader.cache_symbols);
        assert_eq!(loader.cached_symbols_count(), 0);
        assert_eq!(loader.cached_libraries_count(), 0);
    }
    
    #[test]
    fn test_loader_without_caching() {
        let loader = JITLoader::without_caching();
        assert!(!loader.cache_symbols);
    }
    
    #[test]
    fn test_verify_nonexistent_binary() {
        let loader = JITLoader::new();
        let result = loader.verify_binary(Path::new("/nonexistent/binary.so"));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_load_function_nonexistent_binary() {
        let loader = JITLoader::new();
        let result: Result<JITLoopFn, _> = loader.load_function(
            Path::new("/nonexistent/binary.so"),
            "killer_jit_loop_test",
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_clear_symbol_cache() {
        let loader = JITLoader::new();
        // Simulate having cached symbols (can't actually cache without a real library)
        loader.clear_symbol_cache();
        assert_eq!(loader.cached_symbols_count(), 0);
    }
    
    #[test]
    fn test_clear_library_cache() {
        let loader = JITLoader::new();
        loader.clear_library_cache();
        assert_eq!(loader.cached_libraries_count(), 0);
    }
    
    #[test]
    fn test_clear_all_caches() {
        let loader = JITLoader::new();
        loader.clear_all();
        assert_eq!(loader.cached_symbols_count(), 0);
        assert_eq!(loader.cached_libraries_count(), 0);
    }
    
    #[test]
    fn test_load_error_display() {
        let err = LoadError::LibraryLoadFailed("test error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Failed to load library"));
    }
    
    #[test]
    fn test_symbol_not_found_error() {
        let err = LoadError::SymbolNotFound("my_function".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Symbol not found"));
        assert!(msg.contains("my_function"));
    }
    
    #[test]
    fn test_invalid_signature_error() {
        let err = LoadError::InvalidSignature("wrong type".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid signature"));
    }
    
    #[test]
    fn test_default_loader() {
        let loader = JITLoader::default();
        assert!(loader.cache_symbols);
    }
    
    #[test]
    fn test_error_is_send_sync() {
        // Verify LoadError can be sent across threads
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LoadError>();
    }
}
