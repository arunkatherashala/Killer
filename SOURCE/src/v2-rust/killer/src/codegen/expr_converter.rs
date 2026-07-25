// killer_rcore/src/codegen/expr_converter.rs
// Converts Killer expressions to Rust expressions
// Handles: arithmetic, comparisons, function calls, etc.

use crate::ast::*;

/// Converts Killer AST expressions to Rust code strings
pub struct ExprConverter;

impl ExprConverter {
    /// Create new expression converter
    pub fn new() -> Self {
        ExprConverter
    }
    
    /// Convert a Killer expression to Rust code string
    pub fn convert(&self, expr: &Expr) -> String {
        match expr {
            // Literals: directly convert
            Expr::Literal(lit) => self.literal_to_rust(lit),
            
            // Variable references
            Expr::Var(name) => name.clone(),
            
            // Binary operations: a + b, a < b, etc.
            Expr::BinaryOp { left, op, right } => {
                self.binary_op_to_rust(left, op, right)
            }
            
            // Unary operations: -x, !x, etc.
            Expr::UnaryOp { op, operand } => {
                self.unary_op_to_rust(op, operand)
            }
            
            // Function call: len(array)
            Expr::FunctionCall { name, args } => {
                self.function_call_to_rust(name, args)
            }
            
            // Array indexing: arr[i]
            Expr::ArrayIndex { array, index } => {
                let array_str = self.convert(array);
                let index_str = self.convert(index);
                format!("{}[{}]", array_str, index_str)
            }
            
            // Conditional: if a then b else c
            Expr::Conditional { condition, then_expr, else_expr } => {
                let cond = self.convert(condition);
                let then_branch = self.convert(then_expr);
                let else_branch = self.convert(else_expr);
                format!("if {} {{ {} }} else {{ {} }}", cond, then_branch, else_branch)
            }
            
            // Default
            _ => "0".to_string(),
        }
    }
    
    /// Convert literal to Rust
    fn literal_to_rust(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer(n) => format!("{}i64", n),
            Literal::Float(f) => format!("{}f64", f),
            Literal::Bool(b) => b.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Null => "0".to_string(),
        }
    }
    
    /// Convert binary operation to Rust
    fn binary_op_to_rust(&self, left: &Expr, op: &str, right: &Expr) -> String {
        let left_str = self.convert(left);
        let right_str = self.convert(right);
        
        // Use wrapping arithmetic to match Killer semantics
        let rust_op = match op {
            "+" => ".wrapping_add(",
            "-" => ".wrapping_sub(",
            "*" => ".wrapping_mul(",
            "/" => ".wrapping_div(",
            "%" => ".wrapping_rem(",
            _ => &format!(" {} ", op),
        };
        
        // Some ops need special handling
        if rust_op.contains("wrapping") {
            format!("({}{}{})", left_str, rust_op, right_str)
        } else {
            format!("({} {} {})", left_str, op, right_str)
        }
    }
    
    /// Convert unary operation to Rust
    fn unary_op_to_rust(&self, op: &str, operand: &Expr) -> String {
        let operand_str = self.convert(operand);
        
        match op {
            "-" => format!("-({})", operand_str),
            "!" => format!("!({})", operand_str),
            "+" => format!("+({})", operand_str),
            _ => operand_str,
        }
    }
    
    /// Convert function call to Rust
    fn function_call_to_rust(&self, name: &str, args: &[Expr]) -> String {
        match name {
            // Common math functions
            "abs" => {
                if args.len() > 0 {
                    format!("({}).abs()", self.convert(&args[0]))
                } else {
                    "0".to_string()
                }
            }
            
            "max" => {
                if args.len() >= 2 {
                    format!(
                        "({}).max({})",
                        self.convert(&args[0]),
                        self.convert(&args[1])
                    )
                } else {
                    "0".to_string()
                }
            }
            
            "min" => {
                if args.len() >= 2 {
                    format!(
                        "({}).min({})",
                        self.convert(&args[0]),
                        self.convert(&args[1])
                    )
                } else {
                    "0".to_string()
                }
            }
            
            "sqrt" => {
                if args.len() > 0 {
                    format!("({} as f64).sqrt() as i64", self.convert(&args[0]))
                } else {
                    "0".to_string()
                }
            }
            
            "pow" => {
                if args.len() >= 2 {
                    format!(
                        "({}).pow({} as u32)",
                        self.convert(&args[0]),
                        self.convert(&args[1])
                    )
                } else {
                    "0".to_string()
                }
            }
            
            // Array operations
            "len" => {
                if args.len() > 0 {
                    format!("({}).len() as i64", self.convert(&args[0]))
                } else {
                    "0".to_string()
                }
            }
            
            // Default: assume standard function
            _ => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.convert(a)).collect();
                format!("{}({})", name, arg_strs.join(", "))
            }
        }
    }
}

impl Default for ExprConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_convert_integer_literal() {
        let converter = ExprConverter::new();
        let expr = Expr::Literal(Literal::Integer(42));
        
        let result = converter.convert(&expr);
        assert_eq!(result, "42i64");
    }
    
    #[test]
    fn test_convert_float_literal() {
        let converter = ExprConverter::new();
        let expr = Expr::Literal(Literal::Float(3.14));
        
        let result = converter.convert(&expr);
        assert!(result.contains("3.14"));
        assert!(result.contains("f64"));
    }
    
    #[test]
    fn test_convert_bool_literal() {
        let converter = ExprConverter::new();
        let expr = Expr::Literal(Literal::Bool(true));
        
        let result = converter.convert(&expr);
        assert_eq!(result, "true");
    }
    
    #[test]
    fn test_convert_variable() {
        let converter = ExprConverter::new();
        let expr = Expr::Var("count".to_string());
        
        let result = converter.convert(&expr);
        assert_eq!(result, "count");
    }
    
    #[test]
    fn test_convert_addition() {
        let converter = ExprConverter::new();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Literal(Literal::Integer(10))),
            op: "+".to_string(),
            right: Box::new(Expr::Literal(Literal::Integer(20))),
        };
        
        let result = converter.convert(&expr);
        // Should use wrapping arithmetic
        assert!(result.contains("wrapping_add") || result.contains("+"));
    }
    
    #[test]
    fn test_convert_comparison() {
        let converter = ExprConverter::new();
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Var("i".to_string())),
            op: "<".to_string(),
            right: Box::new(Expr::Literal(Literal::Integer(100))),
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("i"));
        assert!(result.contains("<"));
        assert!(result.contains("100"));
    }
    
    #[test]
    fn test_convert_array_index() {
        let converter = ExprConverter::new();
        let expr = Expr::ArrayIndex {
            array: Box::new(Expr::Var("arr".to_string())),
            index: Box::new(Expr::Var("i".to_string())),
        };
        
        let result = converter.convert(&expr);
        assert_eq!(result, "arr[i]");
    }
    
    #[test]
    fn test_convert_abs_function() {
        let converter = ExprConverter::new();
        let expr = Expr::FunctionCall {
            name: "abs".to_string(),
            args: vec![Expr::Literal(Literal::Integer(-42))],
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("abs"));
    }
    
    #[test]
    fn test_convert_max_function() {
        let converter = ExprConverter::new();
        let expr = Expr::FunctionCall {
            name: "max".to_string(),
            args: vec![
                Expr::Literal(Literal::Integer(10)),
                Expr::Literal(Literal::Integer(20)),
            ],
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("max"));
    }
    
    #[test]
    fn test_convert_len_function() {
        let converter = ExprConverter::new();
        let expr = Expr::FunctionCall {
            name: "len".to_string(),
            args: vec![Expr::Var("array".to_string())],
        };
        
        let result = converter.convert(&expr);
        assert!(result.contains("len"));
        assert!(result.contains("array"));
    }
}
