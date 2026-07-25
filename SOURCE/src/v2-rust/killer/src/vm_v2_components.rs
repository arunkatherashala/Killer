/// Virtual Machine Refactoring - Component Extraction - v4.3
/// Purpose: Extract god object pattern into focused components
/// Status: Production-ready modular architecture

use std::collections::HashMap;
use crate::value::{Value, ClassDef, ObjectInstance, Method};
use std::sync::{Arc, Mutex};

/// Execution context for VM operations
/// Contains stack, scopes, and call management
#[derive(Default)]
pub struct ExecutionContext {
    /// Value stack for computations
    pub stack: Vec<Value>,
    /// Variable scopes (local, block, etc.)
    pub scopes: Vec<HashMap<String, Value>>,
    /// Function call stack for debugging and recovery
    pub call_stack: Vec<usize>,
    /// Instruction pointer
    pub ip: usize,
    /// Current object instance (for method calls)
    pub current_object: Option<ObjectInstance>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        ExecutionContext::default()
    }

    /// Push a new scope for local variables
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Store a variable in the current scope
    pub fn store_variable(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Load a variable, searching from current scope upwards
    pub fn load_variable(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Push value onto stack
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop value from stack
    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// Peek at top of stack without removing
    pub fn peek(&self) -> Option<&Value> {
        self.stack.last()
    }

    /// Get stack depth
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
}

/// Class registry for object-oriented programming
/// Manages class definitions and instantiation
pub struct ClassRegistry {
    /// Registered classes and their methods
    classes: Arc<Mutex<HashMap<String, ClassInfo>>>,
}

#[derive(Debug, Clone)]
struct ClassInfo {
    name: String,
    parent: Option<String>,
    methods: HashMap<String, (Vec<String>, Vec<crate::ast::Stmt>)>,
}

impl ClassRegistry {
    pub fn new() -> Self {
        ClassRegistry {
            classes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new class
    pub fn register_class(
        &self,
        name: String,
        parent: Option<String>,
        methods: HashMap<String, (Vec<String>, Vec<crate::ast::Stmt>)>,
    ) -> Result<(), String> {
        let mut classes = self
            .classes
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        if classes.contains_key(&name) {
            return Err(format!("Class already defined: {}", name));
        }

        // Validate parent class exists if specified
        if let Some(ref parent_name) = parent {
            if !classes.contains_key(parent_name) {
                return Err(format!("Parent class not found: {}", parent_name));
            }
        }

        classes.insert(
            name.clone(),
            ClassInfo {
                name,
                parent,
                methods,
            },
        );

        Ok(())
    }

    /// Get class definition
    pub fn get_class(&self, name: &str) -> Result<Option<String>, String> {
        let classes = self
            .classes
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        Ok(classes.get(name).map(|c| c.name.clone()))
    }

    /// Check if class exists
    pub fn class_exists(&self, name: &str) -> Result<bool, String> {
        let classes = self
            .classes
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        Ok(classes.contains_key(name))
    }

    /// Get all classes
    pub fn list_classes(&self) -> Result<Vec<String>, String> {
        let classes = self
            .classes
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        Ok(classes.keys().cloned().collect())
    }

    /// Clear all classes
    pub fn clear(&self) -> Result<(), String> {
        let mut classes = self
            .classes
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        classes.clear();
        Ok(())
    }
}

impl Default for ClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization context for performance management
/// Consolidates all optimization modules (v4.3)
pub struct OptimizationContext {
    /// Call site caching statistics
    pub call_hits: u64,
    pub call_misses: u64,
    /// JIT compilation statistics
    pub jit_compilations: u64,
    /// Hot path detections
    pub hot_paths_detected: u64,
    /// Optimization level
    pub optimization_level: u32,
}

impl OptimizationContext {
    pub fn new() -> Self {
        OptimizationContext {
            call_hits: 0,
            call_misses: 0,
            jit_compilations: 0,
            hot_paths_detected: 0,
            optimization_level: 2,
        }
    }

    pub fn record_cache_hit(&mut self) {
        self.call_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.call_misses += 1;
    }

    pub fn get_hit_rate(&self) -> f64 {
        if self.call_hits + self.call_misses == 0 {
            0.0
        } else {
            (self.call_hits as f64) / ((self.call_hits + self.call_misses) as f64)
        }
    }
}

impl Default for OptimizationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Refactored VirtualMachine using component composition
/// Breaks god object pattern into focused components
pub struct VirtualMachineV2 {
    pub execution: ExecutionContext,
    pub classes: ClassRegistry,
    pub optimization: OptimizationContext,
}

impl VirtualMachineV2 {
    pub fn new() -> Self {
        VirtualMachineV2 {
            execution: ExecutionContext::new(),
            classes: ClassRegistry::new(),
            optimization: OptimizationContext::new(),
        }
    }

    pub fn reset(&mut self) {
        self.execution = ExecutionContext::new();
        let _ = self.classes.clear();
        self.optimization = OptimizationContext::new();
    }
}

impl Default for VirtualMachineV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_context_variable_storage() {
        let mut ctx = ExecutionContext::new();
        ctx.push_scope();
        ctx.store_variable("x".to_string(), Value::Number(42.0));

        let val = ctx.load_variable("x");
        assert!(val.is_some());
    }

    #[test]
    fn execution_context_stack() {
        let mut ctx = ExecutionContext::new();
        ctx.push(Value::Number(10.0));
        ctx.push(Value::Number(20.0));

        assert_eq!(ctx.stack_depth(), 2);
        assert_eq!(ctx.pop(), Some(Value::Number(20.0)));
    }

    #[test]
    fn class_registry_register() {
        let registry = ClassRegistry::new();
        let methods = HashMap::new();

        let result = registry.register_class("MyClass".to_string(), None, methods);
        assert!(result.is_ok());

        let exists = registry.class_exists("MyClass").unwrap();
        assert!(exists);
    }

    #[test]
    fn optimization_context_cache_tracking() {
        let mut opt = OptimizationContext::new();

        opt.record_cache_hit();
        opt.record_cache_hit();
        opt.record_cache_miss();

        let hit_rate = opt.get_hit_rate();
        assert!((hit_rate - 0.667).abs() < 0.01);
    }
}
