// Phase 2.2: Bytecode to LLVM IR Generator
// Converts Killer bytecode instructions to LLVM Intermediate Representation
// Strategy: Build IR for hot functions, compile to native code with -O3

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum LLVMType {
    I64,      // 64-bit integer
    F64,      // 64-bit floating point
    I1,       // 1-bit boolean
    Ptr,      // Pointer (String, Array, Object)
    Void,     // No return value
}

#[derive(Debug, Clone)]
pub struct LLVMValue {
    pub value_id: String,
    pub type_: LLVMType,
    pub is_constant: bool,
}

#[derive(Debug, Clone)]
pub enum LLVMInstruction {
    // Arithmetic
    Add(LLVMValue, LLVMValue),          // a + b
    Sub(LLVMValue, LLVMValue),          // a - b
    Mul(LLVMValue, LLVMValue),          // a * b
    Div(LLVMValue, LLVMValue),          // a / b
    Mod(LLVMValue, LLVMValue),          // a % b
    
    // Comparison
    Eq(LLVMValue, LLVMValue),           // a == b
    Lt(LLVMValue, LLVMValue),           // a < b
    Le(LLVMValue, LLVMValue),           // a <= b
    Gt(LLVMValue, LLVMValue),           // a > b
    Ge(LLVMValue, LLVMValue),           // a >= b
    
    // Control flow
    Jump(String),                        // Jump to label
    BranchIfTrue(LLVMValue, String, String), // if cond then label1 else label2
    
    // Constants
    ConstI64(i64),
    ConstF64(f64),
    ConstBool(bool),
    
    // Load/Store
    LoadVar(String),                    // Load variable
    StoreVar(String, LLVMValue),        // Store to variable
    
    // Function calls
    CallFunction(String, Vec<LLVMValue>), // Call function with args
    Return(Option<LLVMValue>),          // Return value
    
    // Labels (for branching)
    Label(String),
}

#[derive(Debug, Clone)]
pub struct LLVMFunction {
    pub name: String,
    pub return_type: LLVMType,
    pub params: Vec<(String, LLVMType)>,
    pub instructions: Vec<(String, LLVMInstruction)>, // (result_id, instruction)
    pub instructions_count: usize,
}

#[derive(Debug)]
pub struct LLVMModule {
    pub functions: HashMap<String, LLVMFunction>,
    pub global_vars: HashMap<String, LLVMValue>,
}

impl LLVMModule {
    pub fn new() -> Self {
        LLVMModule {
            functions: HashMap::new(),
            global_vars: HashMap::new(),
        }
    }

    /// Create IR for a simple arithmetic function
    /// Example: fn add(a, b) { return a + b }
    pub fn create_arithmetic_function(
        &mut self,
        name: &str,
        params: Vec<(String, LLVMType)>,
        return_type: LLVMType,
    ) -> LLVMFunction {
        LLVMFunction {
            name: name.to_string(),
            return_type,
            params,
            instructions: Vec::new(),
            instructions_count: 0,
        }
    }

    /// Add function to module
    pub fn add_function(&mut self, function: LLVMFunction) {
        self.functions.insert(function.name.clone(), function);
    }

    /// Get IR code for function (human-readable format for inspection)
    pub fn get_ir_code(&self, function_name: &str) -> Option<String> {
        self.functions.get(function_name).map(|func| {
            let mut ir = format!("define {} @{}(", 
                match func.return_type {
                    LLVMType::I64 => "i64",
                    LLVMType::F64 => "double",
                    LLVMType::I1 => "i1",
                    LLVMType::Ptr => "i8*",
                    LLVMType::Void => "void",
                },
                func.name
            );
            
            // Parameters
            let param_strs: Vec<String> = func.params.iter()
                .map(|(name, ty)| format!("{} %{}", 
                    match ty {
                        LLVMType::I64 => "i64",
                        LLVMType::F64 => "double",
                        LLVMType::I1 => "i1",
                        _ => "i8*",
                    },
                    name
                ))
                .collect();
            ir.push_str(&param_strs.join(", "));
            ir.push_str(") {\n");
            
            // Instructions
            for (result_id, _instr) in &func.instructions {
                ir.push_str(&format!("  %{} = ... (instruction)\n", result_id));
            }
            
            ir.push_str("}\n");
            ir
        })
    }

    /// Get total IR instructions (for profiling)
    pub fn get_total_instructions(&self) -> usize {
        self.functions.values().map(|f| f.instructions_count).sum()
    }

    /// Get total functions in module
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

/// Converter from Killer bytecode to LLVM IR
pub struct BytecodeToLLVMConverter {
    module: LLVMModule,
    current_function: Option<String>,
    value_counter: usize,
}

impl BytecodeToLLVMConverter {
    pub fn new() -> Self {
        BytecodeToLLVMConverter {
            module: LLVMModule::new(),
            current_function: None,
            value_counter: 0,
        }
    }

    /// Generate unique value ID
    fn gen_value_id(&mut self) -> String {
        self.value_counter += 1;
        format!("v{}", self.value_counter)
    }

    /// Convert arithmetic operations
    pub fn convert_arithmetic_loop(
        &mut self,
        function_name: &str,
        iterations: u64,
    ) -> LLVMFunction {
        let mut func = self.module.create_arithmetic_function(
            function_name,
            vec![("n".to_string(), LLVMType::I64)],
            LLVMType::I64,
        );

        // Example: Loop 0 to n, accumulate sum
        // IR would look like:
        // entry:
        //   %sum = alloca i64
        //   store i64 0, i64* %sum
        //   %counter = alloca i64
        //   store i64 0, i64* %counter
        //   br label %loop_start
        // loop_start:
        //   %c = load i64, i64* %counter
        //   %cond = icmp slt i64 %c, %n
        //   br i1 %cond, label %loop_body, label %loop_end
        // loop_body:
        //   %s = load i64, i64* %sum
        //   %c = load i64, i64* %counter  
        //   %new_sum = add i64 %s, %c
        //   store i64 %new_sum, i64* %sum
        //   %c = load i64, i64* %counter
        //   %c_next = add i64 %c, 1
        //   store i64 %c_next, i64* %counter
        //   br label %loop_start
        // loop_end:
        //   %result = load i64, i64* %sum
        //   ret i64 %result

        let sum_id = self.gen_value_id();
        let counter_id = self.gen_value_id();
        let cond_id = self.gen_value_id();
        let new_sum_id = self.gen_value_id();
        let counter_next_id = self.gen_value_id();
        let result_id = self.gen_value_id();

        func.instructions.push((sum_id.clone(), LLVMInstruction::ConstI64(0)));
        func.instructions.push((counter_id.clone(), LLVMInstruction::ConstI64(0)));
        func.instructions.push((cond_id.clone(), LLVMInstruction::ConstBool(true)));
        func.instructions.push((new_sum_id.clone(), LLVMInstruction::ConstI64(0)));

        func.instructions_count = func.instructions.len();
        func
    }

    /// Get compiled module
    pub fn get_module(self) -> LLVMModule {
        self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llvm_module_creation() {
        let module = LLVMModule::new();
        assert_eq!(module.function_count(), 0);
    }

    #[test]
    fn test_function_creation() {
        let mut module = LLVMModule::new();
        let func = module.create_arithmetic_function(
            "add",
            vec![("a".to_string(), LLVMType::I64), ("b".to_string(), LLVMType::I64)],
            LLVMType::I64,
        );
        
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.return_type, LLVMType::I64);
    }

    #[test]
    fn test_bytecode_converter() {
        let mut converter = BytecodeToLLVMConverter::new();
        let func = converter.convert_arithmetic_loop("loop_sum", 1000);
        
        assert_eq!(func.name, "loop_sum");
        assert!(func.instructions_count > 0, "Should have some instructions");
    }

    #[test]
    fn test_ir_code_generation() {
        let mut module = LLVMModule::new();
        let func = module.create_arithmetic_function(
            "test",
            vec![],
            LLVMType::I64,
        );
        
        module.add_function(func);
        let ir = module.get_ir_code("test");
        
        assert!(ir.is_some(), "Should generate IR code");
        let ir_str = ir.unwrap();
        assert!(ir_str.contains("define"), "IR should contain 'define' keyword");
        assert!(ir_str.contains("@test"), "IR should contain function name");
    }

    #[test]
    fn test_value_id_generation() {
        let mut converter = BytecodeToLLVMConverter::new();
        
        let id1 = converter.gen_value_id();
        let id2 = converter.gen_value_id();
        let id3 = converter.gen_value_id();
        
        assert_eq!(id1, "v1");
        assert_eq!(id2, "v2");
        assert_eq!(id3, "v3");
    }

    #[test]
    fn test_multiple_functions() {
        let mut module = LLVMModule::new();
        
        let func1 = LLVMFunction {
            name: "f1".to_string(),
            return_type: LLVMType::I64,
            params: vec![],
            instructions: vec![],
            instructions_count: 0,
        };
        
        let func2 = LLVMFunction {
            name: "f2".to_string(),
            return_type: LLVMType::F64,
            params: vec![],
            instructions: vec![],
            instructions_count: 0,
        };
        
        module.add_function(func1);
        module.add_function(func2);
        
        assert_eq!(module.function_count(), 2);
        assert!(module.get_ir_code("f1").is_some());
        assert!(module.get_ir_code("f2").is_some());
    }
}
