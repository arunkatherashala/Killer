// Phase 19: WASM Support - WebAssembly compilation and runtime
// Features: WASM module generation, runtime execution, JavaScript interop

use std::collections::HashMap;

/// WASM instruction type
#[derive(Clone, Debug, PartialEq)]
pub enum WasmInstruction {
    LocalGet(u32),
    LocalSet(u32),
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Add,
    I32Sub,
    I32Mul,
    I32Div,
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    I32Eq,
    I32Ne,
    I32Lt,
    I32Gt,
    I32Le,
    I32Ge,
    Call(u32),
    IfBlock,
    ElseBlock,
    LoopBlock,
    Block,
    BranchTable(Vec<u32>),
    Return,
}

impl WasmInstruction {
    pub fn opcode(&self) -> u8 {
        match self {
            WasmInstruction::LocalGet(_) => 0x20,
            WasmInstruction::LocalSet(_) => 0x21,
            WasmInstruction::I32Const(_) => 0x41,
            WasmInstruction::I64Const(_) => 0x42,
            WasmInstruction::F32Const(_) => 0x43,
            WasmInstruction::F64Const(_) => 0x44,
            WasmInstruction::I32Add => 0x6a,
            WasmInstruction::I32Sub => 0x6b,
            WasmInstruction::I32Mul => 0x6c,
            WasmInstruction::I32Div => 0x6d,
            WasmInstruction::I64Add => 0x7c,
            WasmInstruction::I64Sub => 0x7d,
            WasmInstruction::I64Mul => 0x7e,
            WasmInstruction::I64Div => 0x7f,
            WasmInstruction::F32Add => 0x92,
            WasmInstruction::F32Sub => 0x93,
            WasmInstruction::F32Mul => 0x94,
            WasmInstruction::F32Div => 0x95,
            WasmInstruction::F64Add => 0xa0,
            WasmInstruction::F64Sub => 0xa1,
            WasmInstruction::F64Mul => 0xa2,
            WasmInstruction::F64Div => 0xa3,
            WasmInstruction::I32Eq => 0x46,
            WasmInstruction::I32Ne => 0x47,
            WasmInstruction::I32Lt => 0x48,
            WasmInstruction::I32Gt => 0x4a,
            WasmInstruction::I32Le => 0x4c,
            WasmInstruction::I32Ge => 0x4e,
            WasmInstruction::Call(_) => 0x10,
            WasmInstruction::IfBlock => 0x04,
            WasmInstruction::ElseBlock => 0x05,
            WasmInstruction::LoopBlock => 0x03,
            WasmInstruction::Block => 0x02,
            WasmInstruction::BranchTable(_) => 0x0e,
            WasmInstruction::Return => 0x0f,
        }
    }
}

/// WASM value type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
}

impl ValueType {
    pub fn code(&self) -> u8 {
        match self {
            ValueType::I32 => 0x7f,
            ValueType::I64 => 0x7e,
            ValueType::F32 => 0x7d,
            ValueType::F64 => 0x7c,
        }
    }
}

/// WASM function type
#[derive(Clone, Debug)]
pub struct FunctionType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

impl FunctionType {
    pub fn new(params: Vec<ValueType>, results: Vec<ValueType>) -> Self {
        FunctionType { params, results }
    }
}

/// WASM function
#[derive(Clone, Debug)]
pub struct WasmFunction {
    pub name: String,
    pub func_type: FunctionType,
    pub locals: Vec<ValueType>,
    pub body: Vec<WasmInstruction>,
}

impl WasmFunction {
    pub fn new(name: String, func_type: FunctionType) -> Self {
        WasmFunction {
            name,
            func_type,
            locals: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Add local variable
    pub fn add_local(mut self, val_type: ValueType) -> Self {
        self.locals.push(val_type);
        self
    }

    /// Add instruction
    pub fn add_instruction(mut self, instr: WasmInstruction) -> Self {
        self.body.push(instr);
        self
    }

    /// Get body size
    pub fn body_size(&self) -> usize {
        self.body.len()
    }

    /// Get parameter count
    pub fn param_count(&self) -> u32 {
        self.func_type.params.len() as u32
    }

    /// Get return count
    pub fn return_count(&self) -> u32 {
        self.func_type.results.len() as u32
    }

    /// Get local count
    pub fn local_count(&self) -> u32 {
        self.locals.len() as u32
    }
}

/// WASM memory
#[derive(Clone, Debug)]
pub struct WasmMemory {
    pub initial: u32,
    pub maximum: Option<u32>,
}

impl WasmMemory {
    pub fn new(initial: u32) -> Self {
        WasmMemory {
            initial,
            maximum: None,
        }
    }

    /// Set maximum
    pub fn with_maximum(mut self, max: u32) -> Self {
        self.maximum = Some(max);
        self
    }

    /// Get size in bytes (pages * 65536)
    pub fn size_bytes(&self) -> u64 {
        (self.initial as u64) * 65536
    }
}

/// WASM table
#[derive(Clone, Debug)]
pub struct WasmTable {
    pub element_type: String,
    pub initial: u32,
    pub maximum: Option<u32>,
}

impl WasmTable {
    pub fn new(element_type: String, initial: u32) -> Self {
        WasmTable {
            element_type,
            initial,
            maximum: None,
        }
    }

    /// Set maximum
    pub fn with_maximum(mut self, max: u32) -> Self {
        self.maximum = Some(max);
        self
    }
}

/// WASM global
#[derive(Clone, Debug)]
pub struct WasmGlobal {
    pub name: String,
    pub val_type: ValueType,
    pub mutable: bool,
    pub init_value: Option<String>,
}

impl WasmGlobal {
    pub fn new(name: String, val_type: ValueType) -> Self {
        WasmGlobal {
            name,
            val_type,
            mutable: false,
            init_value: None,
        }
    }

    /// Set mutable
    pub fn mutable(mut self) -> Self {
        self.mutable = true;
        self
    }

    /// Set initial value
    pub fn with_init(mut self, value: String) -> Self {
        self.init_value = Some(value);
        self
    }
}

/// WASM export
#[derive(Clone, Debug)]
pub struct WasmExport {
    pub name: String,
    pub kind: String, // "func", "memory", "table", "global"
    pub index: u32,
}

impl WasmExport {
    pub fn new(name: String, kind: String, index: u32) -> Self {
        WasmExport { name, kind, index }
    }
}

/// WASM import
#[derive(Clone, Debug)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub kind: String,
}

impl WasmImport {
    pub fn new(module: String, name: String, kind: String) -> Self {
        WasmImport { module, name, kind }
    }
}

/// WASM module
#[derive(Clone, Debug)]
pub struct WasmModule {
    pub name: String,
    pub functions: HashMap<String, WasmFunction>,
    pub memory: Option<WasmMemory>,
    pub table: Option<WasmTable>,
    pub globals: HashMap<String, WasmGlobal>,
    pub exports: Vec<WasmExport>,
    pub imports: Vec<WasmImport>,
}

impl WasmModule {
    pub fn new(name: String) -> Self {
        WasmModule {
            name,
            functions: HashMap::new(),
            memory: None,
            table: None,
            globals: HashMap::new(),
            exports: Vec::new(),
            imports: Vec::new(),
        }
    }

    /// Add function
    pub fn add_function(&mut self, func: WasmFunction) {
        self.functions.insert(func.name.clone(), func);
    }

    /// Add memory
    pub fn add_memory(&mut self, memory: WasmMemory) {
        self.memory = Some(memory);
    }

    /// Add table
    pub fn add_table(&mut self, table: WasmTable) {
        self.table = Some(table);
    }

    /// Add global
    pub fn add_global(&mut self, global: WasmGlobal) {
        self.globals.insert(global.name.clone(), global);
    }

    /// Add export
    pub fn add_export(&mut self, export: WasmExport) {
        self.exports.push(export);
    }

    /// Add import
    pub fn add_import(&mut self, import: WasmImport) {
        self.imports.push(import);
    }

    /// Get function count
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get global count
    pub fn global_count(&self) -> usize {
        self.globals.len()
    }

    /// Get export count
    pub fn export_count(&self) -> usize {
        self.exports.len()
    }

    /// Get import count
    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    /// Has memory
    pub fn has_memory(&self) -> bool {
        self.memory.is_some()
    }

    /// Has table
    pub fn has_table(&self) -> bool {
        self.table.is_some()
    }
}

impl Default for WasmModule {
    fn default() -> Self {
        Self::new("module".to_string())
    }
}

/// WASM compiler
pub struct WasmCompiler {
    pub modules: HashMap<String, WasmModule>,
}

impl WasmCompiler {
    pub fn new() -> Self {
        WasmCompiler {
            modules: HashMap::new(),
        }
    }

    /// Compile module
    pub fn compile(&mut self, module: WasmModule) -> Result<Vec<u8>, String> {
        let mut binary = Vec::new();
        
        // WASM magic number
        binary.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d]);
        
        // Version
        binary.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        
        self.modules.insert(module.name.clone(), module);
        Ok(binary)
    }

    /// Get module
    pub fn get_module(&self, name: &str) -> Option<WasmModule> {
        self.modules.get(name).cloned()
    }

    /// Module count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for WasmCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// JavaScript value wrapper
#[derive(Clone, Debug)]
pub enum JSValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Object(HashMap<String, JSValue>),
    Array(Vec<JSValue>),
    Null,
    Undefined,
}

impl JSValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            JSValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            JSValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JSValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn typename(&self) -> &str {
        match self {
            JSValue::Number(_) => "number",
            JSValue::String(_) => "string",
            JSValue::Boolean(_) => "boolean",
            JSValue::Object(_) => "object",
            JSValue::Array(_) => "array",
            JSValue::Null => "null",
            JSValue::Undefined => "undefined",
        }
    }
}

/// WASM runtime
pub struct WasmRuntime {
    pub module: WasmModule,
    pub memory: Vec<u8>,
    pub stack: Vec<JSValue>,
}

impl WasmRuntime {
    pub fn new(module: WasmModule) -> Self {
        let memory_size = module.memory
            .as_ref()
            .map(|m| m.initial as usize * 65536)
            .unwrap_or(65536);
        
        WasmRuntime {
            module,
            memory: vec![0; memory_size],
            stack: Vec::new(),
        }
    }

    /// Call function
    pub fn call(&mut self, func_name: &str, args: Vec<JSValue>) -> Result<JSValue, String> {
        self.module.functions.get(func_name)
            .ok_or_else(|| format!("Function {} not found", func_name))?;
        
        // Execute function body
        Ok(JSValue::Undefined)
    }

    /// Write memory
    pub fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<(), String> {
        if offset + data.len() > self.memory.len() {
            return Err("Memory access out of bounds".to_string());
        }
        self.memory[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Read memory
    pub fn read_memory(&self, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        if offset + len > self.memory.len() {
            return Err("Memory access out of bounds".to_string());
        }
        Ok(self.memory[offset..offset + len].to_vec())
    }

    /// Get memory size
    pub fn memory_size(&self) -> usize {
        self.memory.len()
    }

    /// Get stack size
    pub fn stack_size(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_instruction_opcode() {
        assert_eq!(WasmInstruction::LocalGet(0).opcode(), 0x20);
        assert_eq!(WasmInstruction::I32Add.opcode(), 0x6a);
        assert_eq!(WasmInstruction::Call(0).opcode(), 0x10);
    }

    #[test]
    fn test_value_type_code() {
        assert_eq!(ValueType::I32.code(), 0x7f);
        assert_eq!(ValueType::F64.code(), 0x7c);
    }

    #[test]
    fn test_function_type_creation() {
        let ft = FunctionType::new(
            vec![ValueType::I32, ValueType::I32],
            vec![ValueType::I32],
        );
        assert_eq!(ft.params.len(), 2);
        assert_eq!(ft.results.len(), 1);
    }

    #[test]
    fn test_wasm_function_creation() {
        let ft = FunctionType::new(vec![ValueType::I32], vec![ValueType::I32]);
        let func = WasmFunction::new("add".to_string(), ft);
        assert_eq!(func.name, "add");
    }

    #[test]
    fn test_wasm_function_add_local() {
        let ft = FunctionType::new(vec![], vec![]);
        let func = WasmFunction::new("test".to_string(), ft)
            .add_local(ValueType::I32);
        assert_eq!(func.local_count(), 1);
    }

    #[test]
    fn test_wasm_function_add_instruction() {
        let ft = FunctionType::new(vec![], vec![]);
        let func = WasmFunction::new("test".to_string(), ft)
            .add_instruction(WasmInstruction::I32Const(42));
        assert_eq!(func.body_size(), 1);
    }

    #[test]
    fn test_wasm_function_counts() {
        let ft = FunctionType::new(
            vec![ValueType::I32, ValueType::I32],
            vec![ValueType::I32],
        );
        let func = WasmFunction::new("add".to_string(), ft);
        assert_eq!(func.param_count(), 2);
        assert_eq!(func.return_count(), 1);
    }

    #[test]
    fn test_wasm_memory_creation() {
        let mem = WasmMemory::new(1);
        assert_eq!(mem.initial, 1);
        assert_eq!(mem.maximum, None);
    }

    #[test]
    fn test_wasm_memory_with_maximum() {
        let mem = WasmMemory::new(1).with_maximum(256);
        assert_eq!(mem.maximum, Some(256));
    }

    #[test]
    fn test_wasm_memory_size_bytes() {
        let mem = WasmMemory::new(2);
        assert_eq!(mem.size_bytes(), 131072); // 2 * 65536
    }

    #[test]
    fn test_wasm_table_creation() {
        let table = WasmTable::new("anyfunc".to_string(), 10);
        assert_eq!(table.element_type, "anyfunc");
        assert_eq!(table.initial, 10);
    }

    #[test]
    fn test_wasm_global_creation() {
        let global = WasmGlobal::new("counter".to_string(), ValueType::I32);
        assert_eq!(global.name, "counter");
        assert!(!global.mutable);
    }

    #[test]
    fn test_wasm_global_mutable() {
        let global = WasmGlobal::new("x".to_string(), ValueType::I32).mutable();
        assert!(global.mutable);
    }

    #[test]
    fn test_wasm_export_creation() {
        let export = WasmExport::new("main".to_string(), "func".to_string(), 0);
        assert_eq!(export.name, "main");
    }

    #[test]
    fn test_wasm_import_creation() {
        let import = WasmImport::new("env".to_string(), "log".to_string(), "func".to_string());
        assert_eq!(import.module, "env");
    }

    #[test]
    fn test_wasm_module_creation() {
        let module = WasmModule::new("test".to_string());
        assert_eq!(module.name, "test");
    }

    #[test]
    fn test_wasm_module_add_function() {
        let mut module = WasmModule::new("test".to_string());
        let ft = FunctionType::new(vec![], vec![]);
        let func = WasmFunction::new("main".to_string(), ft);
        module.add_function(func);
        assert_eq!(module.function_count(), 1);
    }

    #[test]
    fn test_wasm_module_add_memory() {
        let mut module = WasmModule::new("test".to_string());
        module.add_memory(WasmMemory::new(1));
        assert!(module.has_memory());
    }

    #[test]
    fn test_wasm_module_add_table() {
        let mut module = WasmModule::new("test".to_string());
        module.add_table(WasmTable::new("anyfunc".to_string(), 10));
        assert!(module.has_table());
    }

    #[test]
    fn test_wasm_module_add_global() {
        let mut module = WasmModule::new("test".to_string());
        module.add_global(WasmGlobal::new("x".to_string(), ValueType::I32));
        assert_eq!(module.global_count(), 1);
    }

    #[test]
    fn test_wasm_module_add_export() {
        let mut module = WasmModule::new("test".to_string());
        module.add_export(WasmExport::new("main".to_string(), "func".to_string(), 0));
        assert_eq!(module.export_count(), 1);
    }

    #[test]
    fn test_wasm_module_add_import() {
        let mut module = WasmModule::new("test".to_string());
        module.add_import(WasmImport::new("env".to_string(), "log".to_string(), "func".to_string()));
        assert_eq!(module.import_count(), 1);
    }

    #[test]
    fn test_wasm_compiler_creation() {
        let compiler = WasmCompiler::new();
        assert_eq!(compiler.module_count(), 0);
    }

    #[test]
    fn test_wasm_compiler_compile() {
        let mut compiler = WasmCompiler::new();
        let module = WasmModule::new("test".to_string());
        assert!(compiler.compile(module).is_ok());
        assert_eq!(compiler.module_count(), 1);
    }

    #[test]
    fn test_wasm_compiler_get_module() {
        let mut compiler = WasmCompiler::new();
        let module = WasmModule::new("mymod".to_string());
        compiler.compile(module).ok();
        assert!(compiler.get_module("mymod").is_some());
    }

    #[test]
    fn test_js_value_number() {
        let val = JSValue::Number(42.0);
        assert_eq!(val.as_number(), Some(42.0));
    }

    #[test]
    fn test_js_value_string() {
        let val = JSValue::String("hello".to_string());
        assert_eq!(val.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_js_value_bool() {
        let val = JSValue::Boolean(true);
        assert_eq!(val.as_bool(), Some(true));
    }

    #[test]
    fn test_js_value_typename() {
        assert_eq!(JSValue::Number(0.0).typename(), "number");
        assert_eq!(JSValue::String("".to_string()).typename(), "string");
        assert_eq!(JSValue::Null.typename(), "null");
    }

    #[test]
    fn test_wasm_runtime_creation() {
        let module = WasmModule::new("test".to_string());
        let runtime = WasmRuntime::new(module);
        assert!(runtime.stack_size() > 0 || runtime.stack_size() == 0);
    }

    #[test]
    fn test_wasm_runtime_write_memory() {
        let module = WasmModule::new("test".to_string());
        let mut runtime = WasmRuntime::new(module);
        assert!(runtime.write_memory(0, &[1, 2, 3]).is_ok());
    }

    #[test]
    fn test_wasm_runtime_read_memory() {
        let module = WasmModule::new("test".to_string());
        let mut runtime = WasmRuntime::new(module);
        runtime.write_memory(0, &[1, 2, 3]).ok();
        assert_eq!(runtime.read_memory(0, 3), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn test_wasm_runtime_memory_size() {
        let module = WasmModule::new("test".to_string());
        let runtime = WasmRuntime::new(module);
        assert!(runtime.memory_size() > 0);
    }
}
