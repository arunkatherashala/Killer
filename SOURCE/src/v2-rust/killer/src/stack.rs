use crate::error::VmError;
use crate::value::Value;
use std::collections::HashMap;

/// Stack frame representing a scope with local variables
pub struct StackFrame {
    pub variables: HashMap<String, Value>,
}

/// Stack management utilities for the Virtual Machine
pub struct StackManager {
    pub stack: Vec<Value>,
    pub scopes: Vec<HashMap<String, Value>>,
}

impl StackManager {
    pub fn new() -> Self {
        let mut manager = Self {
            stack: Vec::new(),
            scopes: Vec::new(),
        };
        manager.push_scope();
        manager
    }

    /// Push a new scope onto the scope stack
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop a scope off the scope stack (cannot pop root scope)
    pub fn pop_scope(&mut self) -> Result<(), VmError> {
        if self.scopes.len() <= 1 {
            return Err(VmError::runtime_error(
                "Cannot exit root scope".to_string(),
            ));
        }
        self.scopes.pop();
        Ok(())
    }

    /// Store a variable in the current scope
    pub fn store_var(&mut self, name: &str, value: Value) -> Result<(), VmError> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            VmError::runtime_error("No active scope available".to_string())
        })?;
        scope.insert(name.to_string(), value);
        Ok(())
    }

    /// Load a variable from the current scope (walks up scope chain if needed)
    pub fn load_var(&self, name: &str) -> Result<Value, VmError> {
        // Handle special global objects
        if name == "Math" {
            let mut math_obj = std::collections::HashMap::new();
            math_obj.insert("PI".to_string(), Value::Number(std::f64::consts::PI));
            math_obj.insert("E".to_string(), Value::Number(std::f64::consts::E));
            return Ok(Value::Dict(Box::new(math_obj)));
        }
        
        if name == "Physics" {
            let mut physics_obj = std::collections::HashMap::new();
            physics_obj.insert("G".to_string(), Value::Number(9.81));  // Gravitational constant
            physics_obj.insert("PI".to_string(), Value::Number(std::f64::consts::PI));
            return Ok(Value::Dict(Box::new(physics_obj)));
        }

        if name == "Array" {
            // Marker object for static helpers like Array.isArray(...)
            return Ok(Value::Dict(Box::new(std::collections::HashMap::new())));
        }

        // Walk up scope chain looking for the variable
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }

        Err(VmError::runtime_error(format!(
            "Undefined variable `{name}`"
        )))
    }

    /// Pop a value from the stack
    pub fn pop_value(&mut self) -> Result<Value, VmError> {
        self.stack
            .pop()
            .ok_or_else(|| VmError::runtime_error("Stack underflow".to_string()))
    }

    /// Pop a number from the stack (type-checked)
    pub fn pop_number(&mut self) -> Result<f64, VmError> {
        match self.stack.pop() {
            Some(Value::Number(n)) => Ok(n),
            Some(other) => Err(VmError::runtime_error(format!(
                "Expected number on stack, found {other}"
            ))),
            None => Err(VmError::runtime_error("Stack underflow".to_string())),
        }
    }

    /// Push a value onto the stack
    pub fn push_value(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Check if a value is truthy
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Bool(v) => *v,
            Value::Null => false,
            Value::Number(v) => *v != 0.0,
            Value::Str(v) => !v.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Dict(d) => !d.is_empty(),
            Value::Object(_) => true,  // Objects are always truthy
            Value::Class(_) => true,   // Classes are always truthy
            Value::Function { .. } => true,  // Functions are always truthy
            Value::Generator(_) => true,  // Generators are always truthy
            Value::QualityWrapped(_) => true,  // Quality objects are always truthy
            Value::Trit(t) => *t > 0,  // T_POS is truthy
            Value::Signal { value, .. } => Self::is_truthy(value),  // Delegate to inner value
            Value::Qubit { alpha, .. } => alpha * alpha >= 0.5,  // P(|0⟩) >= 50%
            Value::Tryte(ts) => ts.iter().any(|&t| t > 0),  // truthy if any T_POS
            Value::Future(_) => true,  // a future handle is truthy
            Value::Integer(n) => *n != 0,
            Value::Bytes(b) => !b.is_empty(),
            Value::Pointer(p) => *p != 0,
        }
    }

    /// Get the current stack depth
    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }

    /// Truncate the stack to a specific length
    pub fn truncate_stack(&mut self, len: usize) {
        self.stack.truncate(len);
    }

    /// Clear all scopes except root
    pub fn clear_scopes(&mut self) {
        while self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Reset the entire stack manager (used at program start)
    pub fn reset(&mut self) {
        self.stack.clear();
        self.clear_scopes();
    }
}

impl Default for StackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_scope() {
        let mut manager = StackManager::new();
        manager.push_scope();
        assert_eq!(manager.scopes.len(), 2);
        manager.pop_scope().unwrap();
        assert_eq!(manager.scopes.len(), 1);
    }

    #[test]
    fn test_store_load_var() {
        let mut manager = StackManager::new();
        manager.store_var("x", Value::Number(42.0)).unwrap();
        let val = manager.load_var("x").unwrap();
        assert_eq!(val, Value::Number(42.0));
    }

    #[test]
    fn test_stack_operations() {
        let mut manager = StackManager::new();
        manager.push_value(Value::Number(1.0));
        manager.push_value(Value::Number(2.0));
        assert_eq!(manager.stack_len(), 2);
        let val = manager.pop_value().unwrap();
        assert_eq!(val, Value::Number(2.0));
    }
}
