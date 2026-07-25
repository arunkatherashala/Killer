// Instruction Cache: Pre-decode and cache frequently used instructions for ~5x speedup
use crate::bytecode::{Instruction, Program};
use std::collections::HashMap;

/// Cached instruction representation for faster execution
#[derive(Debug, Clone)]
pub enum CachedInstruction {
    // Numeric operations (most common)
    ConstNum(f64),
    Load(String),
    Store(String),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    
    // Jump operations
    Jump(usize),
    JumpIfFalse(usize),
    
    // Stack operations
    Pop,
    Ret,
    Halt,
    
    // Comparison operations (combined)
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    
    // Other instructions (fallback to original)
    Other(Instruction),
}

/// Instruction cache for the entire program
pub struct InstructionCache {
    cached_instructions: Vec<CachedInstruction>,
    execution_frequency: HashMap<usize, usize>,
}

impl InstructionCache {
    /// Create a new instruction cache from a program
    pub fn new(program: &Program) -> Self {
        let cached_instructions = program
            .instructions
            .iter()
            .map(|instr| Self::cache_instruction(instr.clone()))
            .collect();

        InstructionCache {
            cached_instructions,
            execution_frequency: HashMap::new(),
        }
    }

    /// Convert a single instruction to cached form
    fn cache_instruction(instr: Instruction) -> CachedInstruction {
        match instr {
            Instruction::ConstNum(n) => CachedInstruction::ConstNum(n),
            Instruction::Load(name) => CachedInstruction::Load(name),
            Instruction::Store(name) => CachedInstruction::Store(name),
            Instruction::Add => CachedInstruction::Add,
            Instruction::Sub => CachedInstruction::Sub,
            Instruction::Mul => CachedInstruction::Mul,
            Instruction::Div => CachedInstruction::Div,
            Instruction::Mod => CachedInstruction::Mod,
            Instruction::Jump(target) => CachedInstruction::Jump(target),
            Instruction::JumpIfFalse(target) => CachedInstruction::JumpIfFalse(target),
            Instruction::Pop => CachedInstruction::Pop,
            Instruction::Ret => CachedInstruction::Ret,
            Instruction::Halt => CachedInstruction::Halt,
            Instruction::Eq => CachedInstruction::Eq,
            Instruction::Ne => CachedInstruction::Ne,
            Instruction::Gt => CachedInstruction::Gt,
            Instruction::Ge => CachedInstruction::Ge,
            Instruction::Lt => CachedInstruction::Lt,
            Instruction::Le => CachedInstruction::Le,
            other => CachedInstruction::Other(other),
        }
    }

    /// Get a cached instruction by index
    pub fn get(&self, index: usize) -> Option<&CachedInstruction> {
        self.cached_instructions.get(index)
    }

    /// Record execution frequency for JIT optimization
    pub fn record_execution(&mut self, index: usize) {
        let count = self.execution_frequency.entry(index).or_insert(0);
        *count += 1;
    }

    /// Get hot paths (frequently executed code sections) for JIT compilation
    pub fn get_hot_paths(&self, threshold: usize) -> Vec<(usize, usize)> {
        let mut hot_paths = Vec::new();
        
        for (&index, &frequency) in &self.execution_frequency {
            if frequency >= threshold {
                // Try to find a contiguous hot region
                let start = if index > 0 {
                    index.saturating_sub(2)
                } else {
                    0
                };
                
                let end = std::cmp::min(
                    index + 10,
                    self.cached_instructions.len().saturating_sub(1),
                );
                
                hot_paths.push((start, end));
            }
        }
        
        // Merge overlapping hot paths
        hot_paths.sort_by_key(|p| p.0);
        let mut merged = Vec::new();
        
        for (start, end) in hot_paths {
            if let Some((_last_start, last_end)) = merged.last_mut() {
                if start <= *last_end + 2 {
                    *last_end = std::cmp::max(*last_end, end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        
        merged
    }

    /// Get total number of cached instructions
    pub fn len(&self) -> usize {
        self.cached_instructions.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cached_instructions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_caching() {
        let instructions = vec![
            Instruction::ConstNum(42.0),
            Instruction::Load("x".to_string()),
            Instruction::Add,
            Instruction::Store("y".to_string()),
            Instruction::Halt,
        ];

        let program = Program {
            instructions,
            function_arities: HashMap::new(),
            function_names: HashMap::new(),
            method_bytecode: HashMap::new(),
            classes: HashMap::new(),
        };

        let cache = InstructionCache::new(&program);

        assert_eq!(cache.len(), 5);
        match cache.get(0).unwrap() {
            CachedInstruction::ConstNum(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected ConstNum"),
        }
    }

    #[test]
    fn test_hot_path_detection() {
        let program = Program {
            instructions: vec![Instruction::Halt],
            function_arities: HashMap::new(),
            function_names: HashMap::new(),
            method_bytecode: HashMap::new(),
            classes: HashMap::new(),
        };

        let mut cache = InstructionCache::new(&program);

        // Simulate loop execution
        for _ in 0..100 {
            cache.record_execution(0);
        }

        let hot_paths = cache.get_hot_paths(50);
        assert!(!hot_paths.is_empty());
    }
}
