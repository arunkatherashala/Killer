// killer_rcore/src/codegen/type_inference.rs
// Type inference engine for loop variables
// Analyzes loop body to determine variable types

use crate::ast::*;
use crate::optimizer::LoopProfile;
use std::collections::HashMap;

/// Inferred type of a variable
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unknown,
}

/// Map of variable names to their inferred types
pub type InferredTypes = HashMap<String, String>;

/// Type inference engine for analyzing Killer code
pub struct TypeInference {
    types: InferredTypes,
}

impl TypeInference {
    /// Create new type inference engine
    pub fn new() -> Self {
        TypeInference {
            types: HashMap::new(),
        }
    }
    
    /// Infer variable types from loop profile
    /// Analyzes loop body and return conditions
    pub fn infer_from_loop(&mut self, profile: &LoopProfile) -> InferredTypes {
        // Start with loop variable
        self.types.insert(profile.loop_var.clone(), "i64".to_string());
        
        // Add accumulator variables (common in loops)
        // Conservative approach: assume i64 unless proven otherwise
        self.types.insert("sum".to_string(), "i64".to_string());
        self.types.insert("product".to_string(), "i64".to_string());
        self.types.insert("count".to_string(), "i64".to_string());
        self.types.insert("result".to_string(), "i64".to_string());
        
        // In real implementation, would analyze loop body AST
        // For now: conservative defaults
        
        self.types.clone()
    }
    
    /// Infer type from an expression
    pub fn infer_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(Literal::Integer(_)) => Type::Int,
            Expr::Literal(Literal::Float(_)) => Type::Float,
            Expr::Literal(Literal::Bool(_)) => Type::Bool,
            Expr::Literal(Literal::String(_)) => Type::String,
            Expr::Var(_) => Type::Unknown, // Would need symbol table
            Expr::BinaryOp { left, op, right } => {
                let left_type = self.infer_expr_type(left);
                let right_type = self.infer_expr_type(right);
                Self::combine_types(left_type, op.as_str(), right_type)
            }
            _ => Type::Unknown,
        }
    }
    
    /// Combine types from binary operation
    fn combine_types(left: Type, op: &str, right: Type) -> Type {
        match (left, right, op) {
            // Int + Int = Int
            (Type::Int, Type::Int, "+") => Type::Int,
            (Type::Int, Type::Int, "-") => Type::Int,
            (Type::Int, Type::Int, "*") => Type::Int,
            (Type::Int, Type::Int, "/") => Type::Int,
            
            // Float operations
            (Type::Float, Type::Float, "+") => Type::Float,
            (Type::Float, Type::Float, "-") => Type::Float,
            (Type::Float, Type::Float, "*") => Type::Float,
            (Type::Float, Type::Float, "/") => Type::Float,
            
            // Comparisons return bool
            (_, _, "<") => Type::Bool,
            (_, _, "<=") => Type::Bool,
            (_, _, ">") => Type::Bool,
            (_, _, ">=") => Type::Bool,
            (_, _, "==") => Type::Bool,
            (_, _, "!=") => Type::Bool,
            
            // Mixed types: promote to float
            (Type::Int, Type::Float, _) => Type::Float,
            (Type::Float, Type::Int, _) => Type::Float,
            
            // Default
            _ => Type::Unknown,
        }
    }
    
    /// Convert inferred type to Rust string
    pub fn type_to_rust_string(&self, t: &Type) -> String {
        match t {
            Type::Int => "i64".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Unknown => "i64".to_string(),
        }
    }
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_infer_integer_literal() {
        let engine = TypeInference::new();
        let expr = Expr::Literal(Literal::Integer(42));
        
        let t = engine.infer_expr_type(&expr);
        assert_eq!(t, Type::Int);
    }
    
    #[test]
    fn test_infer_float_literal() {
        let engine = TypeInference::new();
        let expr = Expr::Literal(Literal::Float(3.14));
        
        let t = engine.infer_expr_type(&expr);
        assert_eq!(t, Type::Float);
    }
    
    #[test]
    fn test_infer_bool_literal() {
        let engine = TypeInference::new();
        let expr = Expr::Literal(Literal::Bool(true));
        
        let t = engine.infer_expr_type(&expr);
        assert_eq!(t, Type::Bool);
    }
    
    #[test]
    fn test_combine_int_types() {
        let t = TypeInference::combine_types(Type::Int, "+", Type::Int);
        assert_eq!(t, Type::Int);
    }
    
    #[test]
    fn test_combine_comparison_types() {
        let t = TypeInference::combine_types(Type::Int, "<", Type::Int);
        assert_eq!(t, Type::Bool);
    }
    
    #[test]
    fn test_combine_mixed_numeric() {
        let t = TypeInference::combine_types(Type::Int, "+", Type::Float);
        assert_eq!(t, Type::Float);
    }
    
    #[test]
    fn test_type_to_rust_mapping() {
        let engine = TypeInference::new();
        
        assert_eq!(engine.type_to_rust_string(&Type::Int), "i64");
        assert_eq!(engine.type_to_rust_string(&Type::Float), "f64");
        assert_eq!(engine.type_to_rust_string(&Type::Bool), "bool");
        assert_eq!(engine.type_to_rust_string(&Type::String), "String");
    }
}
