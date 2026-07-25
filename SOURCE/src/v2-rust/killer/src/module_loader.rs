/// Killer Module Loader and Registry
/// Handles module resolution, loading, caching, and selective imports

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

pub struct ModuleRegistry {
    /// Cached compiled modules: path -> bytecode
    cache: HashMap<String, Vec<u8>>,
    /// Module search paths (stdlib, packages, current dir)
    search_paths: Vec<PathBuf>,
    /// Loaded module globals for each module
    module_globals: HashMap<String, HashMap<String, crate::value::Value>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from("./stdlib"),
            PathBuf::from("./packages"),
            PathBuf::from("."),
        ];
        
        // Add platform-specific stdlib location
        #[cfg(not(target_os = "windows"))]
        search_paths.push(PathBuf::from("/usr/local/lib/killer/stdlib"));
        
        #[cfg(target_os = "windows")]
        search_paths.push(PathBuf::from("C:\\Program Files\\Killer\\stdlib"));
        
        Self {
            cache: HashMap::new(),
            search_paths,
            module_globals: HashMap::new(),
        }
    }
    
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.insert(0, path);
    }
    
    /// Resolve a module path to actual file
    pub fn resolve_module(&self, module_name: &str) -> Option<PathBuf> {
        let candidates = [
            module_name.to_string(),
            format!("{}.killer", module_name),
            format!("{}/index.killer", module_name),
        ];
        
        for search_path in &self.search_paths {
            for candidate in &candidates {
                let full_path = search_path.join(candidate);
                if full_path.exists() && full_path.is_file() {
                    return Some(full_path);
                }
            }
        }
        None
    }
    
    /// Load a module source (cached or fresh)
    pub fn load_module_source(&mut self, module_name: &str) -> Result<String, String> {
        if let Some(path) = self.resolve_module(module_name) {
            fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read module '{}': {}", module_name, e))
        } else {
            Err(format!("Module '{}' not found in search paths", module_name))
        }
    }
    
    /// Selectively import symbols from a module
    /// Returns a HashMap of symbol -> value for the requested symbols
    pub fn selective_import(
        &self,
        module_name: &str,
        symbols: &[String],
    ) -> Result<HashMap<String, crate::value::Value>, String> {
        if let Some(globals) = self.module_globals.get(module_name) {
            let mut imported = HashMap::new();
            for symbol in symbols {
                if let Some(value) = globals.get(symbol) {
                    imported.insert(symbol.clone(), value.clone());
                } else {
                    return Err(format!("Symbol '{}' not exported from module '{}'", symbol, module_name));
                }
            }
            Ok(imported)
        } else {
            Err(format!("Module '{}' not loaded", module_name))
        }
    }
    
    /// Import all exports from a module
    pub fn import_all(
        &self,
        module_name: &str,
    ) -> Result<HashMap<String, crate::value::Value>, String> {
        if let Some(globals) = self.module_globals.get(module_name) {
            Ok(globals.clone())
        } else {
            Err(format!("Module '{}' not loaded", module_name))
        }
    }
    
    /// Store module globals after execution
    pub fn store_module_globals(
        &mut self,
        module_name: String,
        globals: HashMap<String, crate::value::Value>,
    ) {
        self.module_globals.insert(module_name, globals);
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Stdlib module loader
pub mod stdlib {
    use super::*;
    
    /// Get the path to a stdlib module (json, math, string, etc)
    pub fn get_stdlib_path(module_name: &str) -> Option<PathBuf> {
        let stdlib_dirs = [
            PathBuf::from("./stdlib"),
            PathBuf::from("stdlib"),
        ];
        
        #[cfg(not(target_os = "windows"))]
        let stdlib_dirs = [
            PathBuf::from("/usr/local/lib/killer/stdlib"),
            PathBuf::from("/usr/lib/killer/stdlib"),
            PathBuf::from("./stdlib"),
        ];
        
        for base in &stdlib_dirs {
            let module_path = base.join(format!("{}.killer", module_name));
            if module_path.exists() {
                return Some(module_path);
            }
        }
        None
    }
    
    /// List available stdlib modules
    pub fn list_modules() -> Vec<String> {
        vec![
            "io".to_string(),
            "json".to_string(),
            "collections".to_string(),
            "math".to_string(),
            "string".to_string(),
        ]
    }
    
    /// Check if a module is a stdlib module
    pub fn is_stdlib_module(name: &str) -> bool {
        matches!(name,
            "io" | "json" | "collections" | "math" | "string" | 
            "crypto" | "http" | "async" | "debug" | "path" | "fs" |
            "datetime" | "encoding"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stdlib_detection() {
        assert!(stdlib::is_stdlib_module("json"));
        assert!(stdlib::is_stdlib_module("math"));
        assert!(!stdlib::is_stdlib_module("my_package"));
    }
    
    #[test]
    fn test_module_registry() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.search_paths.len(), 3);
    }
}
