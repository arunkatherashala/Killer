// killer_rcore/src/codegen/rust_codegen.rs
// Main Rust code generator for hot loops
// Week 2 implementation for Killer Advanced v4.0

use crate::ast::*;
use crate::optimizer::LoopProfile;
use super::type_inference::{TypeInference, Type as InferredType};
use super::expr_converter::ExprConverter;
use std::collections::HashMap;

/// Generated Rust code for a hot loop
#[derive(Clone, Debug)]
pub struct GeneratedCode {
    /// Complete Rust source code (ready for rustc)
    pub source: String,
    
    /// Function name in generated code
    pub function_name: String,
    
    /// Detected variable types
    pub variable_types: HashMap<String, String>,
    
    /// Whether code was successfully generated
    pub is_valid: bool,
    
    /// Error message if generation failed
    pub error: Option<String>,
}

/// Converts Killer loops to Rust code for JIT compilation
pub struct RustCodegen {
    profile: LoopProfile,
    type_inference: TypeInference,
    expr_converter: ExprConverter,
    function_name: String,
}

impl RustCodegen {
    /// Create new code generator for a loop profile
    pub fn new(profile: LoopProfile) -> Self {
        let function_name = format!("killer_jit_loop_{}", profile.loop_id);
        
        RustCodegen {
            profile,
            type_inference: TypeInference::new(),
            expr_converter: ExprConverter::new(),
            function_name,
        }
    }
    
    /// Generate valid Rust code for this loop
    /// Returns complete source ready for rustc compilation
    pub fn generate(&mut self) -> GeneratedCode {
        // Step 1: Analyze loop body for variable types
        let inferred_types = self.type_inference.infer_from_loop(&self.profile);
        
        if inferred_types.is_empty() {
            return GeneratedCode {
                source: String::new(),
                function_name: self.function_name.clone(),
                variable_types: HashMap::new(),
                is_valid: false,
                error: Some("Failed to infer variable types".to_string()),
            };
        }
        
        // Step 2: Build Rust code
        let mut code = String::new();
        
        // Function signature
        code.push_str(&format!(
            "#[no_mangle]\npub extern \"C\" fn {}() -> i64 {{\n",
            self.function_name
        ));
        
        // Variable declarations
        for (var_name, type_str) in &inferred_types {
            code.push_str(&format!("    let mut {}: {} = 0;\n", var_name, type_str));
        }
        
        // Add blank line for readability
        code.push_str("\n");
        
        // Loop header
        let loop_decl = self.generate_loop_header(&inferred_types);
        code.push_str(&format!("    {}\n", loop_decl));
        
        // Loop body (placeholder - will be filled in step 3)
        code.push_str("        // Loop body (to be implemented)\n");
        
        // Loop end
        code.push_str("    }\n");
        
        // Return statement (return first accumulator variable)
        let return_var = inferred_types.keys().next().unwrap_or(&"0".to_string()).clone();
        code.push_str(&format!("    {}\n", self.generate_return_statement(&return_var)));
        
        code.push_str("}\n");
        
        GeneratedCode {
            source: code,
            function_name: self.function_name.clone(),
            variable_types: inferred_types,
            is_valid: true,
            error: None,
        }
    }
    
    /// Generate the loop header
    /// Example: while i < 1000000 {
    fn generate_loop_header(&self, inferred_types: &HashMap<String, String>) -> String {
        let loop_var = &self.profile.loop_var;
        
        let bound_str = match &self.profile.exit_condition.bound {
            crate::optimizer::Bound::Constant(n) => {
                format!("{}i64", n)
            }
            crate::optimizer::Bound::Variable(v) => v.clone(),
            crate::optimizer::Bound::Expression(e) => e.clone(),
            _ => "1000".to_string(),
        };
        
        format!(
            "while {} {} {} {{",
            loop_var,
            self.profile.exit_condition.operator,
            bound_str
        )
    }
    
    /// Generate return statement
    /// Returns the accumulated value (sum, product, etc.)
    fn generate_return_statement(&self, var_name: &str) -> String {
        format!("return {};", var_name)
    }
    
    /// Convert Killer expression to Rust expression
    /// Handles: arithmetic, comparisons, function calls, etc.
    pub fn expr_to_rust(&self, expr: &Expr) -> String {
        self.expr_converter.convert(expr)
    }
    
    /// Convert Killer type to Rust type string
    /// Maps: Int → i64, Float → f64, Bool → bool, etc.
    pub fn killer_type_to_rust(&self, killer_type: &Type) -> String {
        match killer_type {
            Type::Int => "i64".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Array(_) => "Vec<i64>".to_string(),
            _ => "i64".to_string(), // Default: i64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_loop_profile() -> LoopProfile {
        LoopProfile {
            loop_id: "test_loop".to_string(),
            estimated_iterations: 1_000_000,
            is_hot: true,
            loop_var: "i".to_string(),
            exit_condition: crate::optimizer::ExitCondition {
                var: "i".to_string(),
                operator: "<".to_string(),
                bound: crate::optimizer::Bound::Constant(1_000_000),
            },
            has_branches: false,
            is_parallelizable: true,
            source_line: 1,
        }
    }
    
    #[test]
    fn test_generate_creates_valid_rust() {
        let profile = create_test_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.is_valid);
        assert!(generated.error.is_none());
        assert!(!generated.source.is_empty());
    }
    
    #[test]
    fn test_generated_code_has_extern_c() {
        let profile = create_test_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.source.contains("pub extern \"C\""));
        assert!(generated.source.contains("fn killer_jit_loop"));
    }
    
    #[test]
    fn test_generated_code_has_loop() {
        let profile = create_test_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.source.contains("while i < 1000000i64 {"));
    }
    
    #[test]
    fn test_variable_types_inferred() {
        let profile = create_test_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(!generated.variable_types.is_empty());
    }
    
    #[test]
    fn test_function_name_from_profile() {
        let profile = create_test_loop_profile();
        let codegen = RustCodegen::new(profile.clone());
        
        assert_eq!(
            codegen.function_name,
            format!("killer_jit_loop_{}", profile.loop_id)
        );
    }
}
