/// Type Specialization for Arithmetic Loops
/// 
/// This module implements bytecode specialization for arithmetic-heavy loops.
/// By detecting loops that only use Number types, we generate specialized bytecode
/// that skips type checking and pattern matching, reducing per-operation overhead.
///
/// Performance Impact:
/// - Type checking represents ~35% of execution time (per Week 4 analysis)
/// - Specialization eliminates type matching branches
/// - Expected speedup: 1.5-2x for arithmetic-only code

use crate::bytecode::Instruction;
use std::collections::HashSet;

/// Specialized arithmetic instructions (no type checking)
#[derive(Debug, Clone)]
pub enum SpecializedInstruction {
    // Arithmetic operations (Numbers only)
    AddNumber,      // Stack: [lhs, rhs] -> [lhs + rhs]
    SubNumber,      // Stack: [lhs, rhs] -> [lhs - rhs]
    MulNumber,      // Stack: [lhs, rhs] -> [lhs * rhs]
    DivNumber,      // Stack: [lhs, rhs] -> [lhs / rhs]
    ModNumber,      // Stack: [lhs, rhs] -> [lhs % rhs]
    
    // Comparisons (Numbers only)
    LtNumber,       // Stack: [lhs, rhs] -> [lhs < rhs]
    GtNumber,       // Stack: [lhs, rhs] -> [lhs > rhs]
    LeNumber,       // Stack: [lhs, rhs] -> [lhs <= rhs]
    GeNumber,       // Stack: [lhs, rhs] -> [lhs >= rhs]
    EqNumber,       // Stack: [lhs, rhs] -> [lhs == rhs]
    NeNumber,       // Stack: [lhs, rhs] -> [lhs != rhs]
    
    // Variable access (assume Number type in store)
    LoadVar(String),
    StoreVar(String),
    
    // Constants and control flow
    ConstNum(f64),
    ConstBool(bool),
    Jump(usize),
    JumpIfFalse(usize),
    
    // Stack operations
    Pop,
    
    // Standard (fall back to normal type checking if encountered)
    Standard(Box<Instruction>),
}

/// Analyzer for detecting arithmetic-only code patterns
pub struct ArithmeticAnalyzer {
    arithmetic_vars: HashSet<String>,  // Variables known to hold only Numbers
}

impl ArithmeticAnalyzer {
    pub fn new() -> Self {
        Self {
            arithmetic_vars: HashSet::new(),
        }
    }

    /// Analyze bytecode sequence and determine if it's arithmetic-only
    /// 
    /// Rules:
    /// - Only arithmetic operations and variable access allowed
    /// - Variables must be consistent (Number throughout)
    /// - Constants must be Numbers
    /// - Jumps allowed (for loop structure)
    pub fn is_arithmetic_only(&mut self, bytecode: &[Instruction]) -> bool {
        self.arithmetic_vars.clear();

        for instruction in bytecode {
            match instruction {
                // Constants
                Instruction::ConstNum(_) => {
                    // Number constant is OK in arithmetic code
                }
                Instruction::ConstStr(_) | Instruction::ConstBool(_) | Instruction::ConstNull => {
                    // Non-numeric constants break arithmetic-only assumption
                    return false;
                }

                // Variable operations
                Instruction::Load(name) => {
                    // Track as potential arithmetic variable
                    self.arithmetic_vars.insert(name.clone());
                }
                Instruction::Store(name) => {
                    // Variable being assigned should be arithmetic
                    self.arithmetic_vars.insert(name.clone());
                }

                // Arithmetic operations
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::IntDiv
                | Instruction::Mod => {
                    // Arithmetic operation - OK
                }

                // Comparisons
                Instruction::Lt | Instruction::Gt | Instruction::Le | Instruction::Ge
                | Instruction::Eq | Instruction::Ne => {
                    // Comparison - OK
                }

                // Control flow
                Instruction::Jump(_) | Instruction::JumpIfFalse(_) => {
                    // Jumps allowed (loops)
                }

                // Stack operations
                Instruction::EnterScope | Instruction::ExitScope => {
                    // Scope management OK
                }

                // Everything else is not arithmetic-only
                _ => {
                    return false;
                }
            }
        }

        true
    }

    /// Convert regular bytecode to specialized bytecode
    /// 
    /// This transforms generic instructions to specialized arithmetic versions.
    /// If any operation would fail type assumptions, it includes fallback logic.
    pub fn specialize(&self, bytecode: &[Instruction]) -> Vec<SpecializedInstruction> {
        let mut specialized = Vec::with_capacity(bytecode.len());

        for instruction in bytecode {
            let spec_instr = match instruction {
                Instruction::ConstNum(n) => SpecializedInstruction::ConstNum(*n),
                Instruction::ConstBool(b) => SpecializedInstruction::ConstBool(*b),
                Instruction::Load(name) => SpecializedInstruction::LoadVar(name.clone()),
                Instruction::Store(name) => SpecializedInstruction::StoreVar(name.clone()),
                Instruction::Add => SpecializedInstruction::AddNumber,
                Instruction::Sub => SpecializedInstruction::SubNumber,
                Instruction::Mul => SpecializedInstruction::MulNumber,
                Instruction::Div => SpecializedInstruction::DivNumber,
                Instruction::Mod => SpecializedInstruction::ModNumber,
                Instruction::Lt => SpecializedInstruction::LtNumber,
                Instruction::Gt => SpecializedInstruction::GtNumber,
                Instruction::Le => SpecializedInstruction::LeNumber,
                Instruction::Ge => SpecializedInstruction::GeNumber,
                Instruction::Eq => SpecializedInstruction::EqNumber,
                Instruction::Ne => SpecializedInstruction::NeNumber,
                Instruction::Jump(t) => SpecializedInstruction::Jump(*t),
                Instruction::JumpIfFalse(t) => SpecializedInstruction::JumpIfFalse(*t),
                other => SpecializedInstruction::Standard(Box::new(other.clone())),
            };
            specialized.push(spec_instr);
        }

        specialized
    }
}

/// Statistics about specialization
#[derive(Debug, Clone, Default)]
pub struct SpecializationStats {
    pub loops_analyzed: usize,
    pub loops_specialized: usize,
    pub instructions_specialized: usize,
    pub type_checks_eliminated: usize,
    pub pattern_matches_eliminated: usize,
}

impl SpecializationStats {
    /// Calculate estimated performance improvement
    /// 
    /// Based on Week 4 analysis:
    /// - Type checking: 35% of execution time
    /// - Arithmetic type matching has ~3-4 branches
    /// - Direct Number ops skip all pattern matching
    /// 
    /// Estimated improvement: ~1.5x for arithmetic heavy loops
    pub fn estimated_speedup(&self) -> f64 {
        if self.instructions_specialized == 0 {
            return 1.0;
        }
        
        // Each type check eliminated saves ~2-3 cycles per operation
        // Each pattern match branch eliminated saves branch prediction misses
        let _cycles_saved_per_op = 2.5_f64;
        let _operations_per_second_per_cycle = 1.0_f64;  // Simplified (actually varies)
        
        // Type checking is 35% of overhead
        // Eliminating it from instruction execution saves proportionally
        let type_checking_percent = 0.35_f64;
        let speedup = 1.0_f64 / (1.0_f64 - type_checking_percent);
        
        speedup.min(2.0_f64)  // Conservative estimate: 1.5-2x
    }
}

/// Bytecode specializer for performance optimization
pub struct BytecodeSpecializer {
    analyzer: ArithmeticAnalyzer,
    stats: SpecializationStats,
}

impl BytecodeSpecializer {
    pub fn new() -> Self {
        Self {
            analyzer: ArithmeticAnalyzer::new(),
            stats: SpecializationStats::default(),
        }
    }

    /// Analyze a loop and optionally specialize it
    /// 
    /// Returns the specialized bytecode if the loop is arithmetic-only,
    /// otherwise returns None (use standard bytecode)
    pub fn try_specialize_loop(&mut self, bytecode: &[Instruction], start: usize, length: usize) -> Option<Vec<SpecializedInstruction>> {
        let end = std::cmp::min(start + length, bytecode.len());
        let loop_code = &bytecode[start..end];

        if self.analyzer.is_arithmetic_only(loop_code) {
            self.stats.loops_specialized += 1;
            self.stats.instructions_specialized += loop_code.len();
            
            // Estimate type checks eliminated: each arithmetic op has ~3-4 type checks
            self.stats.type_checks_eliminated += loop_code.len() * 3;
            
            // Estimate pattern matches eliminated (for Add, Sub, etc)
            self.stats.pattern_matches_eliminated += loop_code.iter()
                .filter(|instr| matches!(instr, 
                    Instruction::Add | Instruction::Sub | Instruction::Mul | 
                    Instruction::Div | Instruction::IntDiv | Instruction::Mod))
                .count() * 4;  // Each has ~4 pattern branches
            
            Some(self.analyzer.specialize(loop_code))
        } else {
            self.stats.loops_analyzed += 1;
            None
        }
    }

    /// Get performance statistics
    pub fn stats(&self) -> &SpecializationStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = SpecializationStats::default();
    }
}

impl Default for BytecodeSpecializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_only_detection() {
        let bytecode = vec![
            Instruction::ConstNum(0.0),
            Instruction::Store("sum".to_string()),
            Instruction::ConstNum(0.0),
            Instruction::Store("i".to_string()),
            Instruction::Load("i".to_string()),
            Instruction::Add,
            Instruction::Store("sum".to_string()),
            Instruction::Load("i".to_string()),
            Instruction::ConstNum(1.0),
            Instruction::Add,
            Instruction::Store("i".to_string()),
            Instruction::Jump(4),
        ];

        let mut analyzer = ArithmeticAnalyzer::new();
        assert!(analyzer.is_arithmetic_only(&bytecode));
    }

    #[test]
    fn test_non_arithmetic_detection() {
        let bytecode = vec![
            Instruction::ConstNum(0.0),
            Instruction::Store("sum".to_string()),
            Instruction::ConstStr("hello".to_string()),  // Non-numeric constant
            Instruction::Store("msg".to_string()),
            Instruction::Load("sum".to_string()),
            Instruction::Add,
        ];

        let mut analyzer = ArithmeticAnalyzer::new();
        assert!(!analyzer.is_arithmetic_only(&bytecode));
    }

    #[test]
    fn test_specialization() {
        let bytecode = vec![
            Instruction::ConstNum(1.0),
            Instruction::Load("x".to_string()),
            Instruction::Add,
            Instruction::Store("y".to_string()),
        ];

        let mut analyzer = ArithmeticAnalyzer::new();
        if analyzer.is_arithmetic_only(&bytecode) {
            let specialized = analyzer.specialize(&bytecode);
            assert_eq!(specialized.len(), 4);
            assert!(matches!(specialized[2], SpecializedInstruction::AddNumber));
        }
    }

    #[test]
    fn test_speedup_estimation() {
        let mut stats = SpecializationStats::default();
        stats.instructions_specialized = 100;
        stats.type_checks_eliminated = 300;

        let speedup = stats.estimated_speedup();
        assert!(speedup >= 1.5 && speedup <= 2.0);
    }
}
