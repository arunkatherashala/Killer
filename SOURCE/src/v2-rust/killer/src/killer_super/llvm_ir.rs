// LLVM IR Code Generation Module
// Converts Killer bytecode to LLVM IR

#[derive(Debug, Clone)]
pub struct LLVMFunction {
    pub name: String,
    pub return_type: String,
    pub params: Vec<(String, String)>,
    pub instructions: Vec<LLVMInstruction>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum LLVMInstruction {
    // Memory operations
    Alloca { var: String, ty: String },
    Load { dest: String, src: String, ty: String },
    Store { value: String, dest: String, ty: String },
    
    // Arithmetic
    Add { dest: String, left: String, right: String, ty: String },
    Sub { dest: String, left: String, right: String, ty: String },
    Mul { dest: String, left: String, right: String, ty: String },
    Div { dest: String, left: String, right: String, ty: String },
    Rem { dest: String, left: String, right: String, ty: String },
    
    // Bitwise
    And { dest: String, left: String, right: String, ty: String },
    Or { dest: String, left: String, right: String, ty: String },
    Xor { dest: String, left: String, right: String, ty: String },
    Shl { dest: String, left: String, right: String, ty: String },
    LShr { dest: String, left: String, right: String, ty: String },
    
    // Comparison
    ICmp { dest: String, pred: String, left: String, right: String },
    FCmp { dest: String, pred: String, left: String, right: String },
    
    // Control flow
    Branch { target: String },
    ConditionalBranch { cond: String, true_target: String, false_target: String },
    Switch { value: String, cases: Vec<(String, String)>, default: String },
    Return { value: Option<String> },
    Unreachable,
    
    // Function calls
    Call { dest: Option<String>, func: String, args: Vec<String>, ty: String },
    
    // Labels
    Label { name: String },
    
    // Optimization hints
    MayAlias { ptrs: Vec<String> },
    NoAlias { ptrs: Vec<String> },
    Readonly { func: String },
    Writeonly { func: String },
}

#[derive(Debug, Clone)]
pub struct LLVMModule {
    pub target_triple: String,
    pub data_layout: String,
    pub functions: Vec<LLVMFunction>,
    pub global_vars: Vec<(String, String, Option<String>)>,
    pub attributes: Vec<String>,
}

impl LLVMModule {
    pub fn new(target: &str) -> Self {
        LLVMModule {
            target_triple: target.to_string(),
            data_layout: Self::data_layout_for_target(target),
            functions: Vec::new(),
            global_vars: Vec::new(),
            attributes: vec![
                "attributes #0 = { nounwind }".to_string(),
                "attributes #1 = { nofree noreturn nounwind }".to_string(),
                "attributes #2 = { optsize }".to_string(),
            ],
        }
    }

    fn data_layout_for_target(target: &str) -> String {
        match target {
            "x86-64" => "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128".to_string(),
            "arm64" => "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128".to_string(),
            "wasm32" => "e-m:e-p:32:32-i64:64-n32:64-S128".to_string(),
            "riscv64" => "e-m:e-p:64:64-i64:64-i128:128-n32:64-S128".to_string(),
            _ => "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128".to_string(),
        }
    }

    pub fn add_function(&mut self, func: LLVMFunction) {
        self.functions.push(func);
    }

    pub fn to_ir_string(&self) -> String {
        let mut ir = String::new();

        // Module header
        ir.push_str(&format!("; ModuleID = 'killer.module'\n"));
        ir.push_str(&format!("target datalayout = \"{}\"\n", self.data_layout));
        ir.push_str(&format!("target triple = \"{}\"\n\n", self.target_triple));

        // Global variables
        if !self.global_vars.is_empty() {
            for (name, ty, init) in &self.global_vars {
                if let Some(val) = init {
                    ir.push_str(&format!("@{} = global {} {}\n", name, ty, val));
                } else {
                    ir.push_str(&format!("@{} = global {} zeroinitializer\n", name, ty));
                }
            }
            ir.push('\n');
        }

        // Functions
        for func in &self.functions {
            ir.push_str(&func.to_ir_string());
            ir.push('\n');
        }

        // Attributes
        if !self.attributes.is_empty() {
            ir.push('\n');
            for attr in &self.attributes {
                ir.push_str(attr);
                ir.push('\n');
            }
        }

        ir
    }
}

impl LLVMFunction {
    pub fn new(name: String, return_type: String) -> Self {
        LLVMFunction {
            name,
            return_type,
            params: Vec::new(),
            instructions: Vec::new(),
            attributes: Vec::new(),
        }
    }

    pub fn add_param(&mut self, name: String, ty: String) {
        self.params.push((name, ty));
    }

    pub fn add_instruction(&mut self, inst: LLVMInstruction) {
        self.instructions.push(inst);
    }

    pub fn to_ir_string(&self) -> String {
        let mut ir = String::new();

        // Function signature
        let params = self
            .params
            .iter()
            .map(|(name, ty)| format!("{} %{}", ty, name))
            .collect::<Vec<_>>()
            .join(", ");

        ir.push_str(&format!(
            "define {} @{}({}) {{\n",
            self.return_type, self.name, params
        ));
        ir.push_str("entry:\n");

        // Instructions
        for inst in &self.instructions {
            ir.push_str(&format!("  {}\n", inst.to_ir_string()));
        }

        ir.push_str("}\n");
        ir
    }
}

impl LLVMInstruction {
    pub fn to_ir_string(&self) -> String {
        match self {
            LLVMInstruction::Alloca { var, ty } => {
                format!("%{} = alloca {}", var, ty)
            }
            LLVMInstruction::Load { dest, src, ty } => {
                format!("%{} = load {}, {}* %{}", dest, ty, ty, src)
            }
            LLVMInstruction::Store { value, dest, ty } => {
                format!("store {} {}, {}* %{}", ty, value, ty, dest)
            }
            LLVMInstruction::Add { dest, left, right, ty } => {
                format!("%{} = add {} %{}, %{}", dest, ty, left, right)
            }
            LLVMInstruction::Sub { dest, left, right, ty } => {
                format!("%{} = sub {} %{}, %{}", dest, ty, left, right)
            }
            LLVMInstruction::Mul { dest, left, right, ty } => {
                format!("%{} = mul {} %{}, %{}", dest, ty, left, right)
            }
            LLVMInstruction::Div { dest, left, right, ty } => {
                format!("%{} = sdiv {} %{}, %{}", dest, ty, left, right)
            }
            LLVMInstruction::Return { value } => {
                if let Some(val) = value {
                    format!("ret {}", val)
                } else {
                    "ret void".to_string()
                }
            }
            LLVMInstruction::Label { name } => {
                format!("{}:\n", name)
            }
            LLVMInstruction::Branch { target } => {
                format!("br label %{}", target)
            }
            LLVMInstruction::ConditionalBranch { cond, true_target, false_target } => {
                format!("br i1 %{}, label %{}, label %{}", cond, true_target, false_target)
            }
            LLVMInstruction::Call { dest, func, args, ty } => {
                let arg_str = args.join(", ");
                if let Some(d) = dest {
                    format!("%{} = call {} @{}({})", d, ty, func, arg_str)
                } else {
                    format!("call {} @{}({})", ty, func, arg_str)
                }
            }
            LLVMInstruction::Readonly { func } => {
                format!("declare dereferenceable(4) i8* @{}(...) readonly", func)
            }
            LLVMInstruction::ICmp { dest, pred, left, right } => {
                format!("%{} = icmp {} i64 %{}, %{}", dest, pred, left, right)
            }
            _ => "/* unsupported instruction */".to_string(),
        }
    }
}

/// Convert Killer bytecode to LLVM IR
pub fn bytecode_to_llvm(_bytecode: &str, target: &str) -> LLVMModule {
    let mut module = LLVMModule::new(target);

    // Parse bytecode and generate LLVM functions
    // This is a simplified example - real implementation would parse actual bytecode
    let mut func = LLVMFunction::new("killer_main".to_string(), "i32".to_string());

    // Example: simple function that returns 42
    func.add_instruction(LLVMInstruction::Return {
        value: Some("i32 42".to_string()),
    });

    module.add_function(func);
    module
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llvm_module_creation() {
        let mut module = LLVMModule::new("x86-64");
        let func = LLVMFunction::new("test".to_string(), "i32".to_string());
        module.add_function(func);
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_llvm_ir_generation() {
        let module = LLVMModule::new("x86-64");
        let ir = module.to_ir_string();
        assert!(ir.contains("target datalayout"));
        assert!(ir.contains("target triple"));
    }

    #[test]
    fn test_llvm_instruction_ir() {
        let inst = LLVMInstruction::Add {
            dest: "result".to_string(),
            left: "a".to_string(),
            right: "b".to_string(),
            ty: "i64".to_string(),
        };
        let ir = inst.to_ir_string();
        assert!(ir.contains("add i64"));
    }
}
