// Week 1: AST Extensions for Dependent Types
// Extends the existing AST to support dependent type annotations

use crate::dependent_types::{DependentType, TypeParam, FunctionSignature};
use std::fmt;

/// Extended function definition with dependent types support
#[derive(Clone, Debug)]
pub struct FunctionDefWithDependentTypes {
    pub name: String,
    /// Type parameters: [n: nat, m: nat]
    pub type_params: Vec<TypeParam>,
    /// Function parameters with dependent types
    pub params: Vec<FunctionParam>,
    pub return_type: DependentType,
    /// Function body (expressions)
    pub body: Vec<String>, // Simplified for Week 1
}

/// Function parameter with dependent type
#[derive(Clone, Debug)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: DependentType,
}

/// Struct definition with dependent types
#[derive(Clone, Debug)]
pub struct StructDefWithDependentTypes {
    pub name: String,
    /// Type parameters: [n: nat]
    pub type_params: Vec<TypeParam>,
    /// Fields with dependent types
    pub fields: Vec<StructField>,
    /// Invariants: conditions that must always hold
    pub invariants: Vec<String>, // Simplified for Week 1
}

/// Struct field with dependent type
#[derive(Clone, Debug)]
pub struct StructField {
    pub name: String,
    pub field_type: DependentType,
}

/// Type variable binding (for inference)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeVarBinding {
    pub var_name: String,
    pub concrete_type: String, // e.g., "5" for n=5
}

/// Type environment for tracking bindings
#[derive(Clone, Debug)]
pub struct TypeEnvironment {
    /// Variable -> concrete value mappings
    pub bindings: std::collections::HashMap<String, String>,
    /// Type -> concrete value mappings
    pub type_bindings: std::collections::HashMap<String, String>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        TypeEnvironment {
            bindings: std::collections::HashMap::new(),
            type_bindings: std::collections::HashMap::new(),
        }
    }
    
    pub fn bind(&mut self, var: String, value: String) {
        self.bindings.insert(var, value);
    }
    
    pub fn lookup(&self, var: &str) -> Option<&String> {
        self.bindings.get(var)
    }
}

impl fmt::Display for FunctionDefWithDependentTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "fn {}", self.name)?;
        
        // Type parameters
        if !self.type_params.is_empty() {
            write!(f, "[")?;
            for (i, param) in self.type_params.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}: {}", param.name, param.kind)?;
            }
            write!(f, "]")?;
        }
        
        // Function parameters
        write!(f, "(")?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}: {}", param.name, param.param_type)?;
        }
        write!(f, ")")?;
        
        // Return type
        write!(f, " -> {}", self.return_type)?;
        
        // Body
        write!(f, " {{ ... }}")
    }
}

impl fmt::Display for StructDefWithDependentTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "struct {}", self.name)?;
        
        // Type parameters
        if !self.type_params.is_empty() {
            write!(f, "[")?;
            for (i, param) in self.type_params.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}: {}", param.name, param.kind)?;
            }
            write!(f, "]")?;
        }
        
        write!(f, " {{ ")?;
        
        // Fields
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}: {}", field.name, field.field_type)?;
        }
        
        write!(f, " }}")
    }
}

/// Type checking context for dependent types
#[derive(Clone, Debug)]
pub struct TypeCheckContext {
    /// Current type environment
    pub env: TypeEnvironment,
    /// Accumulated constraints (for later solving)
    pub constraints: Vec<TypeConstraint>,
}

/// Type constraint to be verified
#[derive(Clone, Debug)]
pub enum TypeConstraint {
    /// Variable must equal value: n == 5
    Equality {
        var: String,
        value: String,
    },
    /// Variable must be less than value: i < n
    LessThan {
        var: String,
        bound: String,
    },
    /// Type sizes must match
    SizeMatch {
        type1: DependentType,
        type2: DependentType,
    },
}

impl TypeCheckContext {
    pub fn new() -> Self {
        TypeCheckContext {
            env: TypeEnvironment::new(),
            constraints: Vec::new(),
        }
    }
    
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }
    
    pub fn solve(&self) -> Result<(), String> {
        // Simple constraint solving for Week 1
        // More sophisticated solving in later weeks
        for constraint in &self.constraints {
            match constraint {
                TypeConstraint::Equality { var, value } => {
                    // Check if variable is already bound to different value
                    if let Some(existing) = self.env.lookup(var) {
                        if existing != value {
                            return Err(format!(
                                "Type constraint violation: {} must equal both {} and {}",
                                var, existing, value
                            ));
                        }
                    }
                }
                TypeConstraint::LessThan { var, bound } => {
                    // Check if variable binding satisfies bound
                    // Simplified for Week 1
                }
                TypeConstraint::SizeMatch { type1, type2 } => {
                    // Check if both types have equal sizes
                    // Simplified for Week 1
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependent_types::{TypeParamKind, DependentTypeArg};
    
    #[test]
    fn test_function_def_display() {
        let func = FunctionDefWithDependentTypes {
            name: "process".to_string(),
            type_params: vec![TypeParam {
                name: "n".to_string(),
                kind: TypeParamKind::Nat,
            }],
            params: vec![FunctionParam {
                name: "v".to_string(),
                param_type: DependentType::Named {
                    name: "Vector".to_string(),
                    params: vec![DependentTypeArg::Var("n".to_string())],
                },
            }],
            return_type: DependentType::Simple("i32".to_string()),
            body: vec!["// implementation".to_string()],
        };
        
        let s = func.to_string();
        assert!(s.contains("fn process[n: nat]"));
        assert!(s.contains("Vector[n]"));
    }
    
    #[test]
    fn test_struct_def_display() {
        let struct_def = StructDefWithDependentTypes {
            name: "SafeVec".to_string(),
            type_params: vec![TypeParam {
                name: "n".to_string(),
                kind: TypeParamKind::Nat,
            }],
            fields: vec![
                StructField {
                    name: "data".to_string(),
                    field_type: DependentType::Named {
                        name: "Vector".to_string(),
                        params: vec![DependentTypeArg::Var("n".to_string())],
                    },
                },
                StructField {
                    name: "len".to_string(),
                    field_type: DependentType::Simple("i32".to_string()),
                },
            ],
            invariants: vec!["data.len() == n".to_string()],
        };
        
        let s = struct_def.to_string();
        assert!(s.contains("struct SafeVec[n: nat]"));
        assert!(s.contains("Vector[n]"));
    }
    
    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        env.bind("n".to_string(), "5".to_string());
        
        assert_eq!(env.lookup("n"), Some(&"5".to_string()));
        assert_eq!(env.lookup("m"), None);
    }
    
    #[test]
    fn test_type_check_constraint() {
        let mut ctx = TypeCheckContext::new();
        ctx.env.bind("n".to_string(), "5".to_string());
        ctx.add_constraint(TypeConstraint::Equality {
            var: "n".to_string(),
            value: "5".to_string(),
        });
        
        let result = ctx.solve();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_type_check_constraint_violation() {
        let mut ctx = TypeCheckContext::new();
        ctx.env.bind("n".to_string(), "5".to_string());
        ctx.add_constraint(TypeConstraint::Equality {
            var: "n".to_string(),
            value: "10".to_string(),
        });
        
        let result = ctx.solve();
        assert!(result.is_err());
    }
}
