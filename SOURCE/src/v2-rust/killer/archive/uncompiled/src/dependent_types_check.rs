// Week 1: Type Checker for Dependent Types
// Validates dependent type constraints and solves type parameters

use crate::dependent_types::{DependentType, DependentTypeArg, TypeParamKind};
use crate::dependent_types_ast::{TypeCheckContext, TypeConstraint, FunctionDefWithDependentTypes};
use std::collections::HashMap;

/// Dependent type checker
pub struct DependentTypeChecker {
    /// Known type bindings
    bindings: HashMap<String, String>,
}

impl DependentTypeChecker {
    pub fn new() -> Self {
        DependentTypeChecker {
            bindings: HashMap::new(),
        }
    }
    
    /// Check if a type argument is valid given constraints
    pub fn check_type_arg(
        &self,
        arg: &DependentTypeArg,
    ) -> Result<String, String> {
        match arg {
            DependentTypeArg::Var(name) => {
                // Lookup variable binding
                self.bindings
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Unbound variable in dependent type: {}", name))
            }
            DependentTypeArg::Literal(n) => {
                if *n >= 0 {
                    Ok(format!("{}", n))
                } else {
                    Err(format!("Invalid negative value in dependent type: {}", n))
                }
            }
            DependentTypeArg::BinOp { left, op, right } => {
                let left_val = self.check_type_arg(left)?;
                let right_val = self.check_type_arg(right)?;
                
                // Try to parse and compute
                match (left_val.parse::<i64>(), right_val.parse::<i64>()) {
                    (Ok(l), Ok(r)) => {
                        let result = match op.as_str() {
                            "+" => l + r,
                            "-" => l - r,
                            "*" => l * r,
                            "/" if r != 0 => l / r,
                            "/" => return Err("Division by zero in dependent type".to_string()),
                            _ => return Err(format!("Unknown operator in dependent type: {}", op)),
                        };
                        if result >= 0 {
                            Ok(format!("{}", result))
                        } else {
                            Err(format!("Invalid negative result in dependent type computation: {}", result))
                        }
                    }
                    _ => {
                        // Cannot fully evaluate, keep as symbolic expression
                        Ok(format!("{} {} {}", left_val, op, right_val))
                    }
                }
            }
        }
    }
    
    /// Check if two dependent types are compatible (same shape/size)
    pub fn check_type_compatibility(
        &self,
        type1: &DependentType,
        type2: &DependentType,
    ) -> Result<(), String> {
        match (type1, type2) {
            // Same simple type
            (DependentType::Simple(t1), DependentType::Simple(t2)) => {
                if t1 == t2 {
                    Ok(())
                } else {
                    Err(format!("Type mismatch: {} vs {}", t1, t2))
                }
            }
            
            // Named types with same parameters
            (
                DependentType::Named { name: n1, params: p1 },
                DependentType::Named { name: n2, params: p2 },
            ) => {
                if n1 != n2 {
                    return Err(format!("Type name mismatch: {} vs {}", n1, n2));
                }
                
                if p1.len() != p2.len() {
                    return Err(format!(
                        "Type parameter count mismatch: {} vs {}",
                        p1.len(),
                        p2.len()
                    ));
                }
                
                // Check each parameter matches
                for (arg1, arg2) in p1.iter().zip(p2.iter()) {
                    self.check_type_arg_equality(arg1, arg2)?;
                }
                
                Ok(())
            }
            
            // Different forms
            _ => Err(format!("Type form mismatch: {:?} vs {:?}", type1, type2)),
        }
    }
    
    /// Check if two type arguments represent the same value
    fn check_type_arg_equality(
        &self,
        arg1: &DependentTypeArg,
        arg2: &DependentTypeArg,
    ) -> Result<(), String> {
        let val1 = self.check_type_arg(arg1)?;
        let val2 = self.check_type_arg(arg2)?;
        
        if val1 == val2 {
            Ok(())
        } else {
            Err(format!("Type argument mismatch: {} vs {}", val1, val2))
        }
    }
    
    /// Bind a type variable to a concrete value
    pub fn bind_type_var(&mut self, var: String, value: String) -> Result<(), String> {
        // Check if variable is already bound to a different value
        if let Some(existing) = self.bindings.get(&var) {
            if existing != &value {
                return Err(format!(
                    "Type variable {} already bound to {} (cannot rebind to {})",
                    var, existing, value
                ));
            }
        }
        
        self.bindings.insert(var, value);
        Ok(())
    }
    
    /// Get all current bindings
    pub fn get_bindings(&self) -> &HashMap<String, String> {
        &self.bindings
    }
    
    /// Check function signature for type consistency
    pub fn check_function_sig(
        &mut self,
        func: &FunctionDefWithDependentTypes,
    ) -> Result<(), String> {
        // For Week 1: basic validation
        // Later weeks: full dependent type checking and constraint solving
        
        // Verify all type parameters are referenced in function signature
        let referenced_params: Vec<_> = func.type_params.iter().map(|p| &p.name).collect();
        
        // Check that param types reference only declared type parameters
        for param in &func.params {
            self.validate_dependent_type_refs(&param.param_type, &referenced_params)?;
        }
        
        // Check return type
        self.validate_dependent_type_refs(&func.return_type, &referenced_params)?;
        
        Ok(())
    }
    
    /// Validate that type only references declared type parameters
    fn validate_dependent_type_refs(
        &self,
        ty: &DependentType,
        valid_params: &[&String],
    ) -> Result<(), String> {
        match ty {
            DependentType::Named { name: _, params } => {
                for arg in params {
                    match arg {
                        DependentTypeArg::Var(name) => {
                            if !valid_params.contains(&&name) && !self.is_builtin_type(name) {
                                return Err(format!(
                                    "Undefined type variable: {} (not in scope)",
                                    name
                                ));
                            }
                        }
                        DependentTypeArg::BinOp { left, op: _, right } => {
                            // Recursively check both sides
                            if let DependentTypeArg::Var(name) = &**left {
                                if !valid_params.contains(&name) && !self.is_builtin_type(name) {
                                    return Err(format!("Undefined type variable: {}", name));
                                }
                            }
                            if let DependentTypeArg::Var(name) = &**right {
                                if !valid_params.contains(&name) && !self.is_builtin_type(name) {
                                    return Err(format!("Undefined type variable: {}", name));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    /// Check if identifier is a built-in type
    fn is_builtin_type(&self, name: &str) -> bool {
        matches!(
            name,
            "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "String" | "Option"
                | "Vector" | "Matrix" | "Idx"
        )
    }
}

/// Constraint solver for dependent types
pub struct ConstraintSolver {
    constraints: Vec<TypeConstraint>,
}

impl ConstraintSolver {
    pub fn new() -> Self {
        ConstraintSolver {
            constraints: Vec::new(),
        }
    }
    
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }
    
    /// Solve all constraints (Week 1: simple satisfaction)
    /// Weeks 2-4: SMT solver integration for complex constraints
    pub fn solve(&self) -> Result<HashMap<String, String>, String> {
        let mut bindings: HashMap<String, String> = HashMap::new();
        
        for constraint in &self.constraints {
            match constraint {
                TypeConstraint::Equality { var, value } => {
                    // Check consistency
                    if let Some(existing) = bindings.get(var) {
                        if existing != value {
                            return Err(format!(
                                "Constraint conflict: {} must be both {} and {}",
                                var, existing, value
                            ));
                        }
                    } else {
                        bindings.insert(var.clone(), value.clone());
                    }
                }
                
                TypeConstraint::LessThan { var, bound } => {
                    // For Week 1: only check if var is a literal
                    if let Some(val) = bindings.get(var) {
                        match (val.parse::<i64>(), bound.parse::<i64>()) {
                            (Ok(v), Ok(b)) if v >= b => {
                                return Err(format!(
                                    "Constraint violation: {} < {} but {} = {}",
                                    var, bound, var, v
                                ));
                            }
                            _ => {} // Keep constraint unresolved for SMT solver later
                        }
                    }
                }
                
                TypeConstraint::SizeMatch { type1: _, type2: _ } => {
                    // Week 1: basic checking, full support in later weeks
                }
            }
        }
        
        Ok(bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_arg_literal() {
        let checker = DependentTypeChecker::new();
        let arg = DependentTypeArg::Literal(5);
        let result = checker.check_type_arg(&arg);
        assert_eq!(result, Ok("5".to_string()));
    }
    
    #[test]
    fn test_type_arg_var_unbound() {
        let checker = DependentTypeChecker::new();
        let arg = DependentTypeArg::Var("n".to_string());
        let result = checker.check_type_arg(&arg);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_type_arg_var_bound() {
        let mut checker = DependentTypeChecker::new();
        checker.bind_type_var("n".to_string(), "10".to_string()).unwrap();
        let arg = DependentTypeArg::Var("n".to_string());
        let result = checker.check_type_arg(&arg);
        assert_eq!(result, Ok("10".to_string()));
    }
    
    #[test]
    fn test_constraint_solver_equality() {
        let mut solver = ConstraintSolver::new();
        solver.add_constraint(TypeConstraint::Equality {
            var: "n".to_string(),
            value: "5".to_string(),
        });
        
        let result = solver.solve();
        assert!(result.is_ok());
        let bindings = result.unwrap();
        assert_eq!(bindings.get("n"), Some(&"5".to_string()));
    }
    
    #[test]
    fn test_constraint_solver_conflict() {
        let mut solver = ConstraintSolver::new();
        solver.add_constraint(TypeConstraint::Equality {
            var: "n".to_string(),
            value: "5".to_string(),
        });
        solver.add_constraint(TypeConstraint::Equality {
            var: "n".to_string(),
            value: "10".to_string(),
        });
        
        let result = solver.solve();
        assert!(result.is_err());
    }
}
