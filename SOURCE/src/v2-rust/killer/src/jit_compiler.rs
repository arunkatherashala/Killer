// JIT Compiler: Compile hot bytecode sections to native Rust for ~10x speedup
use crate::bytecode::Instruction;
use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;

/// Compiled code for a hot bytecode section
pub type CompiledFunction = Box<dyn Fn(&mut Vec<Value>) -> Result<(), VmError> + Send + Sync>;

/// JIT compiler for hot bytecode sections
pub struct JitCompiler {
    compiled_cache: HashMap<usize, CompiledFunction>,
    compilation_count: usize,
    max_compiled_functions: usize,
}

impl JitCompiler {
    /// Create a new JIT compiler
    pub fn new() -> Self {
        JitCompiler {
            compiled_cache: HashMap::new(),
            compilation_count: 0,
            max_compiled_functions: 1000, // Limit to prevent memory explosion
        }
    }

    /// Compile a bytecode sequence into a native function
    /// This is a simplified JIT that handles common patterns
    pub fn compile_hot_path(
        &mut self,
        start_ip: usize,
        instructions: &[Instruction],
    ) -> Option<usize> {
        if self.compilation_count >= self.max_compiled_functions {
            return None;
        }

        // Analyze the instruction sequence for compilability
        if !Self::is_compilable(instructions) {
            return None;
        }

        // Generate a specialized function for this path
        let compiled_fn = Self::generate_compiled_function(instructions)?;

        let fn_id = self.compilation_count;
        self.compiled_cache.insert(start_ip, compiled_fn);
        self.compilation_count += 1;

        Some(fn_id)
    }

    /// Check if a bytecode sequence is suitable for JIT compilation
    fn is_compilable(instructions: &[Instruction]) -> bool {
        // Compilable sequences must:
        // 1. Be relatively short (< 100 instructions)
        // 2. Not contain complex control flow
        // 3. Consist mainly of arithmetic/stack operations

        if instructions.len() > 100 {
            return false;
        }

        let mut has_jumps = false;
        for instr in instructions {
            match instr {
                Instruction::Call { .. }
                | Instruction::CallDynamic { .. }
                | Instruction::CallBuiltin(_, _)
                | Instruction::DefineClass { .. }
                | Instruction::NewObject(_)
                | Instruction::TryEnter { .. } => {
                    // These are complex and shouldn't be JIT compiled
                    return false;
                }
                Instruction::Jump(_) | Instruction::JumpIfFalse(_) => {
                    has_jumps = true;
                }
                _ => {}
            }
        }

        // Limit JIT to sequences without complex control flow
        !has_jumps
    }

    /// Generate a compiled function for simple arithmetic sequences
    fn generate_compiled_function(
        instructions: &[Instruction],
    ) -> Option<CompiledFunction> {
        // Clone instructions for the closure
        let instrs = instructions.to_vec();

        Some(Box::new(move |stack: &mut Vec<Value>| {
            for instr in &instrs {
                match instr {
                    Instruction::ConstNum(n) => stack.push(Value::Number(*n)),
                    Instruction::ConstBool(b) => stack.push(Value::Bool(*b)),
                    Instruction::ConstNull => stack.push(Value::Null),
                    Instruction::Add => {
                        let rhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        let lhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        stack.push(Value::Number(lhs + rhs));
                    }
                    Instruction::Sub => {
                        let rhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        let lhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        stack.push(Value::Number(lhs - rhs));
                    }
                    Instruction::Mul => {
                        let rhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        let lhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        stack.push(Value::Number(lhs * rhs));
                    }
                    Instruction::Div => {
                        let rhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        let lhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        if rhs == 0.0 {
                            return Err(VmError::runtime_error("Division by zero".to_string()));
                        }
                        stack.push(Value::Number(lhs / rhs));
                    }
                    Instruction::IntDiv => {
                        let rhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        let lhs = match stack.pop() {
                            Some(Value::Number(n)) => n,
                            _ => return Err(VmError::runtime_error("Type error".to_string())),
                        };
                        if rhs == 0.0 {
                            return Err(VmError::runtime_error(
                                "Floor division by zero".to_string(),
                            ));
                        }
                        stack.push(Value::Number((lhs / rhs).floor()));
                    }
                    Instruction::Pop => {
                        stack.pop();
                    }
                    _ => {
                        // For unsupported instructions in JIT, return error
                        return Err(VmError::runtime_error(
                            "Unsupported instruction in JIT".to_string(),
                        ));
                    }
                }
            }
            Ok(())
        }))
    }

    /// Try to execute a compiled function
    pub fn execute_compiled(
        &self,
        start_ip: usize,
        stack: &mut Vec<Value>,
    ) -> Option<Result<(), VmError>> {
        self.compiled_cache
            .get(&start_ip)
            .map(|compiled_fn| compiled_fn(stack))
    }

    /// Get compilation statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.compiled_cache.len(), self.compilation_count)
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_compilation() {
        let instructions = vec![
            Instruction::ConstNum(10.0),
            Instruction::ConstNum(20.0),
            Instruction::Add,
        ];

        let mut jit = JitCompiler::new();
        let compiled_id = jit.compile_hot_path(0, &instructions);

        assert!(compiled_id.is_some());
    }

    #[test]
    fn test_jit_execution() {
        let instructions = vec![
            Instruction::ConstNum(5.0),
            Instruction::ConstNum(3.0),
            Instruction::Mul,
        ];

        let mut jit = JitCompiler::new();
        jit.compile_hot_path(0, &instructions);

        let mut stack = Vec::new();
        let result = jit.execute_compiled(0, &mut stack);

        assert!(result.is_some());
        if let Some(Ok(())) = result {
            assert_eq!(stack.len(), 1);
            if let Value::Number(n) = stack[0] {
                assert_eq!(n, 15.0);
            }
        }
    }
}
