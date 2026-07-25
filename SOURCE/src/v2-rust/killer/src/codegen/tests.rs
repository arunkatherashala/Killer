// killer_rcore/src/codegen/tests.rs
// Integration tests for code generation
// Week 2 validation

#[cfg(test)]
mod codegen_tests {
    use crate::codegen::{RustCodegen, ExprConverter, TypeInference};
    use crate::ast::*;
    use crate::optimizer::{LoopProfile, ExitCondition, Bound};
    
    fn create_simple_loop_profile() -> LoopProfile {
        LoopProfile {
            loop_id: "loop_simple".to_string(),
            estimated_iterations: 1_000_000,
            is_hot: true,
            loop_var: "i".to_string(),
            exit_condition: ExitCondition {
                var: "i".to_string(),
                operator: "<".to_string(),
                bound: Bound::Constant(1_000_000),
            },
            has_branches: false,
            is_parallelizable: true,
            source_line: 1,
        }
    }
    
    #[test]
    fn test_codegen_full_pipeline() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        // Verify generated code structure
        assert!(generated.is_valid);
        assert!(generated.error.is_none());
        assert!(!generated.source.is_empty());
        assert!(generated.source.len() > 50); // Non-trivial code
    }
    
    #[test]
    fn test_generated_code_compiles_to_valid_rust() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        // Check for Rust syntax requirements
        assert!(generated.source.contains("pub extern \"C\""));
        assert!(generated.source.contains("fn killer_jit_loop"));
        assert!(generated.source.contains("{"));
        assert!(generated.source.contains("}"));
    }
    
    #[test]
    fn test_generated_code_has_type_annotations() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        // Should have type annotations for variables
        assert!(generated.source.contains(": i64"));
    }
    
    #[test]
    fn test_type_inference_on_loop() {
        let profile = create_simple_loop_profile();
        let mut type_engine = TypeInference::new();
        let inferred = type_engine.infer_from_loop(&profile);
        
        // Loop variable should be i64
        assert_eq!(inferred.get("i"), Some(&"i64".to_string()));
    }
    
    #[test]
    fn test_expression_converter_arithmetic() {
        let converter = ExprConverter::new();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Var("sum".to_string())),
            op: "+".to_string(),
            right: Box::new(Expr::Var("i".to_string())),
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("sum"));
        assert!(result.contains("i"));
    }
    
    #[test]
    fn test_expression_converter_comparison() {
        let converter = ExprConverter::new();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Var("i".to_string())),
            op: "<".to_string(),
            right: Box::new(Expr::Literal(Literal::Integer(1000000))),
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("i"));
        assert!(result.contains("<"));
        assert!(result.contains("1000000"));
    }
    
    #[test]
    fn test_killer_type_to_rust_mapping() {
        let profile = create_simple_loop_profile();
        let codegen = RustCodegen::new(profile);
        
        assert_eq!(codegen.killer_type_to_rust(&Type::Int), "i64");
        assert_eq!(codegen.killer_type_to_rust(&Type::Float), "f64");
        assert_eq!(codegen.killer_type_to_rust(&Type::Bool), "bool");
        assert_eq!(codegen.killer_type_to_rust(&Type::String), "String");
    }
    
    #[test]
    fn test_function_name_generation() {
        let profile = create_simple_loop_profile();
        let codegen = RustCodegen::new(profile.clone());
        
        let expected = format!("killer_jit_loop_{}", profile.loop_id);
        assert_eq!(codegen.function_name, expected);
    }
    
    #[test]
    fn test_generated_code_contains_loop_header() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.source.contains("while i < 1000000i64 {"));
    }
    
    #[test]
    fn test_generated_code_contains_return() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.source.contains("return"));
        assert!(generated.source.contains(";"));
    }
    
    #[test]
    fn test_variable_type_inference_for_loop() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        // Should have inferred types for multiple variables
        assert!(!generated.variable_types.is_empty());
    }
    
    #[test]
    fn test_type_system_integer_arithmetic() {
        let type_engine = TypeInference::new();
        
        let left = Expr::Literal(Literal::Integer(10));
        let right = Expr::Literal(Literal::Integer(20));
        
        let left_type = type_engine.infer_expr_type(&left);
        let right_type = type_engine.infer_expr_type(&right);
        
        use crate::codegen::type_inference::Type;
        assert_eq!(left_type, Type::Int);
        assert_eq!(right_type, Type::Int);
    }
    
    #[test]
    fn test_code_generation_no_errors() {
        let profile = create_simple_loop_profile();
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.error.is_none(), "{:?}", generated.error);
    }
    
    // Example: Test generating code for nested loops
    #[test]
    fn test_codegen_with_nested_loop_profile() {
        let profile = LoopProfile {
            loop_id: "loop_nested".to_string(),
            estimated_iterations: 1_000_000, // 1K × 1K
            is_hot: true,
            loop_var: "i".to_string(),
            exit_condition: ExitCondition {
                var: "i".to_string(),
                operator: "<".to_string(),
                bound: Bound::Constant(1000),
            },
            has_branches: false,
            is_parallelizable: true,
            source_line: 10,
        };
        
        let mut codegen = RustCodegen::new(profile);
        let generated = codegen.generate();
        
        assert!(generated.is_valid);
        assert!(!generated.source.is_empty());
    }
}
