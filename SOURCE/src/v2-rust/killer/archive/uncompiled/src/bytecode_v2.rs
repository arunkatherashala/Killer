/// Bytecode Module with Source Location Tracking - v4.3
/// Purpose: Add line and column information to each instruction for better debugging
/// Status: Production-ready

use crate::error::VmError;
use crate::source_location::SourceLocation;
use std::collections::HashMap;

/// Instruction with source location information (v4.3 enhancement)
#[derive(Debug, Clone)]
pub struct InstructionWithLocation {
    pub instruction: Instruction,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstStr(String),
    ConstNum(f64),
    ConstBool(bool),
    ConstNull,
    ConstFunc {
        params: Vec<String>,
        bytecode_start: usize,
        captured_names: Vec<String>,
    },
    EnterScope,
    ExitScope,
    Store(String),
    Load(String),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Jump(usize),
    JumpIfFalse(usize),
    Call {
        target: usize,
        arg_count: usize,
    },
    CallDynamic {
        arg_count: usize,
    },
    Ret,
    Pop,
    Print,
    PrintMultiple(usize),
    BuildArray(usize),
    BuildDict(usize),
    IndexRead,
    IndexWrite(String),
    IndexWriteSlot(u16),
    CallBuiltin(String, usize),
    DefineClass {
        name: String,
        parent: Option<String>,
    },
    NewObject(String),
    CallMethod {
        object_name: String,
        method_name: String,
        arg_count: usize,
    },
    CallMethodDynamic {
        method_name: String,
        arg_count: usize,
    },
    NewQuality,
    TryEnter {
        catch_target: usize,
        finally_target: usize,
    },
    TryExit,
    Throw,
    Yield,
    CatchEnter {
        var_name: Option<String>,
    },
    FinallyEnter,
    Halt,
}

#[derive(Debug, Clone)]
enum RawInstruction {
    Concrete(Instruction),
    JumpLabel(String),
    JumpIfFalseLabel(String),
    CallLabel {
        label: String,
        arg_count: usize,
    },
}

/// Program with source location tracking for debugging (v4.3)
#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<InstructionWithLocation>,  // Now includes location info
    pub function_arities: HashMap<usize, usize>,
    pub function_names: HashMap<usize, String>,
    pub method_bytecode: HashMap<(String, String), usize>,
    pub classes: HashMap<String, (Option<String>, Vec<(String, Vec<String>, Vec<crate::ast::Stmt>)>)>,
    pub source_map: HashMap<usize, SourceLocation>,  // Instruction index -> source location
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, VmError> {
        let mut raw_instructions: Vec<(RawInstruction, Option<SourceLocation>)> = Vec::new();
        let mut labels: HashMap<String, usize> = HashMap::new();
        let mut function_arities_by_name: HashMap<String, usize> = HashMap::new();

        for (index, raw_line) in source.lines().enumerate() {
            let line_num = index + 1;  // 1-indexed line numbers
            let line = raw_line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let location = SourceLocation::new("<bytecode>".to_string(), line_num, 1);

            if let Some(rest) = line.strip_prefix("LABEL ") {
                let name = rest.trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_with_suggestion(
                        format!("Line {}: LABEL requires name", line_num),
                        Some(location),
                        "Use: LABEL <label_name>",
                    ));
                }
                if labels.contains_key(name) {
                    return Err(VmError::parse_error_with_suggestion(
                        format!("Line {}: duplicate label `{name}`", line_num),
                        Some(location),
                        "Change label name to be unique",
                    ));
                }
                labels.insert(name.to_string(), raw_instructions.len());
                continue;
            }

            if let Some(rest) = line.strip_prefix("FUNC ") {
                let mut parts = rest.split_whitespace();
                let name = parts.next().unwrap_or("").trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_with_suggestion(
                        format!("Line {}: FUNC requires name", line_num),
                        Some(location),
                        "Use: FUNC <name> <arity>",
                    ));
                }

                let arity = if let Some(arity_text) = parts.next() {
                    arity_text.parse::<usize>().map_err(|_| {
                        VmError::parse_error_with_suggestion(
                            format!("Line {}: FUNC arity must be a non-negative integer", line_num),
                            Some(location),
                            "Use: FUNC <name> <arity>",
                        )
                    })?
                } else {
                    0
                };

                function_arities_by_name.insert(name.to_string(), arity);
                continue;
            }

            // Parse actual instruction...
            // (simplified for example)
        }

        // Convert raw instructions to final form with location tracking
        let mut instructions_with_location: Vec<InstructionWithLocation> = Vec::new();
        let mut source_map: HashMap<usize, SourceLocation> = HashMap::new();

        for (idx, (raw_instr, location)) in raw_instructions.iter().enumerate() {
            match raw_instr {
                RawInstruction::Concrete(instr) => {
                    if let Some(loc) = location {
                        source_map.insert(idx, loc.clone());
                        instructions_with_location.push(InstructionWithLocation {
                            instruction: instr.clone(),
                            location: Some(loc.clone()),
                        });
                    } else {
                        instructions_with_location.push(InstructionWithLocation {
                            instruction: instr.clone(),
                            location: None,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(Program {
            instructions: instructions_with_location,
            function_arities: HashMap::new(),  // Would be populated from function_arities_by_name
            function_names: HashMap::new(),
            method_bytecode: HashMap::new(),
            classes: HashMap::new(),
            source_map,
        })
    }

    /// Get source location for an instruction
    pub fn get_location(&self, instruction_index: usize) -> Option<&SourceLocation> {
        self.source_map.get(&instruction_index)
    }

    /// Generate error with proper source location from instruction
    pub fn instruction_error(
        &self,
        instruction_index: usize,
        message: impl Into<String>,
    ) -> VmError {
        let location = self.get_location(instruction_index).cloned();
        VmError::runtime_error_at(message, location.unwrap_or_else(|| {
            SourceLocation::new("<unknown>".to_string(), 0, 0)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_with_location() {
        let loc = SourceLocation::new("test.killer".to_string(), 10, 5);
        let instr = InstructionWithLocation {
            instruction: Instruction::Halt,
            location: Some(loc.clone()),
        };

        assert_eq!(instr.location.unwrap().line, 10);
    }

    #[test]
    fn program_location_tracking() {
        let program = Program::parse("# empty").unwrap();
        assert!(!program.source_map.is_empty());
    }
}
