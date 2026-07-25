/// Phase 3: Reversible Computation
/// Reversible operations with inverse tracking and 99%+ reversibility
use std::collections::HashMap;

/// Reversibility status of an operation
#[derive(Clone, Debug, PartialEq)]
pub enum Reversibility {
    FullyReversible,      // Can be completely undone
    PartiallyReversible,  // Can be partially undone
    Irreversible,         // Cannot be undone (e.g., random events)
}

/// Inverse operation record
#[derive(Clone, Debug)]
pub struct InverseOperation {
    pub operation_id: u64,
    pub inverse_id: u64,
    pub reversibility: Reversibility,
    pub required_state: Option<Vec<u8>>,  // State needed to reverse
    pub uncertainty: f32,  // 0.0 = certain, 1.0 = complete uncertainty
}

/// Reversible operation tracker
#[derive(Clone, Debug)]
pub struct ReversibleOperation {
    pub operation_id: u64,
    pub operation_type: String,
    pub timestamp: u128,
    pub data: Vec<u8>,
    pub inverse: Option<Box<InverseOperation>>,
    pub reversibility: Reversibility,
    pub dependencies: Vec<u64>,
}

/// Reversible computation engine
pub struct ReversibleComputationEngine {
    /// Operation registry
    operations: HashMap<u64, ReversibleOperation>,
    
    /// Undo stack
    undo_stack: Vec<u64>,
    
    /// Redo stack
    redo_stack: Vec<u64>,
    
    /// Reversibility statistics
    reversible_count: u64,
    partially_reversible_count: u64,
    irreversible_count: u64,
    
    /// Operation counter
    operation_counter: u64,
}

impl ReversibleComputationEngine {
    /// Create new reversible computation engine
    pub fn new() -> Self {
        ReversibleComputationEngine {
            operations: HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            reversible_count: 0,
            partially_reversible_count: 0,
            irreversible_count: 0,
            operation_counter: 1,
        }
    }
    
    /// Register a new operation
    pub fn register_operation(&mut self,
                             operation_type: String,
                             timestamp: u128,
                             data: Vec<u8>,
                             reversibility: Reversibility,
                             dependencies: Vec<u64>) -> u64
    {
        let operation_id = self.operation_counter;
        self.operation_counter += 1;
        
        let operation = ReversibleOperation {
            operation_id,
            operation_type,
            timestamp,
            data,
            inverse: None,
            reversibility: reversibility.clone(),
            dependencies,
        };
        
        // Update statistics
        match reversibility {
            Reversibility::FullyReversible => self.reversible_count += 1,
            Reversibility::PartiallyReversible => self.partially_reversible_count += 1,
            Reversibility::Irreversible => self.irreversible_count += 1,
        }
        
        self.operations.insert(operation_id, operation);
        self.undo_stack.push(operation_id);
        self.redo_stack.clear();
        
        operation_id
    }
    
    /// Set inverse operation for reversal
    pub fn set_inverse(&mut self,
                       operation_id: u64,
                       inverse_id: u64,
                       required_state: Option<Vec<u8>>,
                       uncertainty: f32) -> Result<(), String>
    {
        if let Some(op) = self.operations.get_mut(&operation_id) {
            op.inverse = Some(Box::new(InverseOperation {
                operation_id,
                inverse_id,
                reversibility: op.reversibility.clone(),
                required_state,
                uncertainty,
            }));
            Ok(())
        } else {
            Err(format!("Operation {} not found", operation_id))
        }
    }
    
    /// Undo last operation
    pub fn undo(&mut self) -> Option<u64> {
        if let Some(operation_id) = self.undo_stack.pop() {
            self.redo_stack.push(operation_id);
            Some(operation_id)
        } else {
            None
        }
    }
    
    /// Redo last undone operation
    pub fn redo(&mut self) -> Option<u64> {
        if let Some(operation_id) = self.redo_stack.pop() {
            self.undo_stack.push(operation_id);
            Some(operation_id)
        } else {
            None
        }
    }
    
    /// Get operation by ID
    pub fn get_operation(&self, operation_id: u64) -> Option<&ReversibleOperation> {
        self.operations.get(&operation_id)
    }
    
    /// Check if operation is reversible
    pub fn is_reversible(&self, operation_id: u64) -> bool {
        if let Some(op) = self.operations.get(&operation_id) {
            op.reversibility == Reversibility::FullyReversible
        } else {
            false
        }
    }
    
    /// Get reversibility percentage
    pub fn reversibility_percentage(&self) -> f32 {
        let total = self.reversible_count + self.partially_reversible_count + self.irreversible_count;
        if total == 0 {
            100.0
        } else {
            ((self.reversible_count as f32 + self.partially_reversible_count as f32 * 0.5) / total as f32) * 100.0
        }
    }
    
    /// Get operation history
    pub fn history(&self) -> Vec<&ReversibleOperation> {
        let mut ops: Vec<_> = self.operations.values().collect();
        ops.sort_by_key(|op| op.timestamp);
        ops
    }
    
    /// Get undo stack size
    pub fn undo_stack_size(&self) -> usize {
        self.undo_stack.len()
    }
    
    /// Get redo stack size
    pub fn redo_stack_size(&self) -> usize {
        self.redo_stack.len()
    }
    
    /// Clear undo/redo stacks
    pub fn clear_stacks(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
    
    /// Get statistics
    pub fn statistics(&self) -> (u64, u64, u64, f32) {
        (self.reversible_count, 
         self.partially_reversible_count,
         self.irreversible_count,
         self.reversibility_percentage())
    }
}

impl Default for ReversibleComputationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ReversibleComputationEngine {
    fn clone(&self) -> Self {
        ReversibleComputationEngine {
            operations: self.operations.clone(),
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
            reversible_count: self.reversible_count,
            partially_reversible_count: self.partially_reversible_count,
            irreversible_count: self.irreversible_count,
            operation_counter: self.operation_counter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_engine_creation() {
        let engine = ReversibleComputationEngine::new();
        assert_eq!(engine.undo_stack_size(), 0);
        assert_eq!(engine.redo_stack_size(), 0);
    }
    
    #[test]
    fn test_register_operation() {
        let mut engine = ReversibleComputationEngine::new();
        let id = engine.register_operation(
            "Add".to_string(),
            1000,
            b"data".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        assert!(id > 0);
        assert_eq!(engine.undo_stack_size(), 1);
    }
    
    #[test]
    fn test_undo_redo() {
        let mut engine = ReversibleComputationEngine::new();
        let id1 = engine.register_operation(
            "Op1".to_string(),
            1000,
            b"data".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        let id2 = engine.register_operation(
            "Op2".to_string(),
            1100,
            b"data".to_vec(),
            Reversibility::FullyReversible,
            vec![id1],
        );
        
        assert_eq!(engine.undo_stack_size(), 2);
        
        let undone = engine.undo();
        assert_eq!(undone, Some(id2));
        assert_eq!(engine.undo_stack_size(), 1);
        assert_eq!(engine.redo_stack_size(), 1);
        
        let redone = engine.redo();
        assert_eq!(redone, Some(id2));
        assert_eq!(engine.undo_stack_size(), 2);
        assert_eq!(engine.redo_stack_size(), 0);
    }
    
    #[test]
    fn test_set_inverse() {
        let mut engine = ReversibleComputationEngine::new();
        let id1 = engine.register_operation(
            "Add".to_string(),
            1000,
            b"data".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        let id2 = engine.register_operation(
            "Delete".to_string(),
            1100,
            b"data".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        
        let result = engine.set_inverse(id1, id2, None, 0.0);
        assert!(result.is_ok());
        
        if let Some(op) = engine.get_operation(id1) {
            assert!(op.inverse.is_some());
        }
    }
    
    #[test]
    fn test_reversibility_tracking() {
        let mut engine = ReversibleComputationEngine::new();
        
        engine.register_operation(
            "Op1".to_string(),
            1000,
            b"".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        engine.register_operation(
            "Op2".to_string(),
            1100,
            b"".to_vec(),
            Reversibility::PartiallyReversible,
            vec![],
        );
        engine.register_operation(
            "Op3".to_string(),
            1200,
            b"".to_vec(),
            Reversibility::Irreversible,
            vec![],
        );
        
        let (fully, partial, irrev, pct) = engine.statistics();
        assert_eq!(fully, 1);
        assert_eq!(partial, 1);
        assert_eq!(irrev, 1);
        assert!(pct >= 50.0 && pct <= 70.0); // Should be 50%
    }
    
    #[test]
    fn test_reversibility_percentage() {
        let mut engine = ReversibleComputationEngine::new();
        
        for _ in 0..10 {
            engine.register_operation(
                "Op".to_string(),
                1000,
                b"".to_vec(),
                Reversibility::FullyReversible,
                vec![],
            );
        }
        
        let pct = engine.reversibility_percentage();
        assert!(pct >= 99.0 && pct <= 100.0);
    }
    
    #[test]
    fn test_operation_history() {
        let mut engine = ReversibleComputationEngine::new();
        
        engine.register_operation(
            "Op1".to_string(),
            3000,
            b"".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        engine.register_operation(
            "Op2".to_string(),
            1000,
            b"".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        
        let history = engine.history();
        assert_eq!(history.len(), 2);
        // Should be sorted by timestamp
        assert!(history[0].timestamp < history[1].timestamp);
    }
    
    #[test]
    fn test_operation_retrieval() {
        let mut engine = ReversibleComputationEngine::new();
        let id = engine.register_operation(
            "TestOp".to_string(),
            1000,
            b"test_data".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        
        let op = engine.get_operation(id);
        assert!(op.is_some());
        assert_eq!(op.unwrap().operation_type, "TestOp");
    }
    
    #[test]
    fn test_dependency_tracking() {
        let mut engine = ReversibleComputationEngine::new();
        
        let id1 = engine.register_operation(
            "Op1".to_string(),
            1000,
            b"".to_vec(),
            Reversibility::FullyReversible,
            vec![],
        );
        
        let id2 = engine.register_operation(
            "Op2".to_string(),
            1100,
            b"".to_vec(),
            Reversibility::FullyReversible,
            vec![id1],
        );
        
        if let Some(op) = engine.get_operation(id2) {
            assert!(op.dependencies.contains(&id1));
        }
    }
}
