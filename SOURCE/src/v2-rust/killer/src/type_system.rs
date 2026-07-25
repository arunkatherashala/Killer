// killer_rcore/src/type_system.rs
// Type system for Killer - type checking, inference, and validation
// Supports: number, string, boolean, void, any, and custom types

use std::collections::HashMap;
use std::fmt;

/// Represents a Killer type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// Numeric type (int, float)
    Number,
    /// String type
    String,
    /// Boolean type
    Boolean,
    /// Function returns nothing
    Void,
    /// Dynamic typing - no validation
    Any,
    /// Custom type/class name
    Custom(String),
    /// Array of elements of a type
    Array(Box<TypeKind>),
    /// Function type with params and return type
    Function {
        params: Vec<TypeKind>,
        returns: Box<TypeKind>,
    },
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeKind::Number => write!(f, "number"),
            TypeKind::String => write!(f, "string"),
            TypeKind::Boolean => write!(f, "boolean"),
            TypeKind::Void => write!(f, "void"),
            TypeKind::Any => write!(f, "any"),
            TypeKind::Custom(name) => write!(f, "{}", name),
            TypeKind::Array(inner) => write!(f, "{}[]", inner),
            TypeKind::Function { params, returns } => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", returns)
            }
        }
    }
}

/// Type annotation in source code
#[derive(Clone, Debug)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub optional: bool,  // For nullable types in future
}

impl TypeAnnotation {
    /// Create a type annotation from a type name string
    pub fn from_name(name: &str) -> Option<Self> {
        let kind = match name {
            "number" => TypeKind::Number,
            "string" => TypeKind::String,
            "boolean" => TypeKind::Boolean,
            "void" => TypeKind::Void,
            "any" => TypeKind::Any,
            _ => TypeKind::Custom(name.to_string()),
        };
        Some(TypeAnnotation {
            kind,
            optional: false,
        })
    }

    /// Create from TypeKind
    pub fn new(kind: TypeKind) -> Self {
        TypeAnnotation {
            kind,
            optional: false,
        }
    }

    /// Mark as optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// Function signature with type information
#[derive(Clone, Debug)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, TypeAnnotation)>,
    pub return_type: TypeAnnotation,
}

/// Type environment for a scope
#[derive(Clone, Debug)]
pub struct TypeEnvironment {
    /// Variable name -> type mapping
    variables: HashMap<String, TypeAnnotation>,
    /// Function name -> signature mapping
    functions: HashMap<String, FunctionSignature>,
    /// Class name -> field types mapping
    classes: HashMap<String, HashMap<String, TypeAnnotation>>,
    /// Parent scope (for nested scopes)
    parent: Option<Box<TypeEnvironment>>,
}

impl TypeEnvironment {
    /// Create a new empty environment
    pub fn new() -> Self {
        TypeEnvironment {
            variables: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            parent: None,
        }
    }

    /// Create a new environment with a parent
    pub fn with_parent(parent: TypeEnvironment) -> Self {
        TypeEnvironment {
            variables: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Register a variable with a type
    pub fn define_var(&mut self, name: String, type_ann: TypeAnnotation) {
        self.variables.insert(name, type_ann);
    }

    /// Register a function with its signature
    pub fn define_fn(&mut self, sig: FunctionSignature) {
        self.functions.insert(sig.name.clone(), sig);
    }

    /// Register a class with its fields
    pub fn define_class(&mut self, name: String, fields: HashMap<String, TypeAnnotation>) {
        self.classes.insert(name, fields);
    }

    /// Lookup a variable type
    pub fn lookup_var(&self, name: &str) -> Option<TypeAnnotation> {
        if let Some(ty) = self.variables.get(name) {
            return Some(ty.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_var(name);
        }
        None
    }

    /// Lookup a function signature
    pub fn lookup_fn(&self, name: &str) -> Option<FunctionSignature> {
        if let Some(sig) = self.functions.get(name) {
            return Some(sig.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_fn(name);
        }
        None
    }

    /// Lookup class fields
    pub fn lookup_class(&self, name: &str) -> Option<HashMap<String, TypeAnnotation>> {
        if let Some(fields) = self.classes.get(name) {
            return Some(fields.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_class(name);
        }
        None
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker for validating type correctness
pub struct TypeChecker {
    env: TypeEnvironment,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        TypeChecker {
            env: TypeEnvironment::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a type error
    pub fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    /// Add a type warning
    pub fn warning(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    /// Check if types are compatible (assignable)
    pub fn is_assignable(from: &TypeKind, to: &TypeKind) -> bool {
        match (from, to) {
            // Same types are always compatible
            _ if from == to => true,
            // Any type is compatible with everything
            (TypeKind::Any, _) | (_, TypeKind::Any) => true,
            // Number can be assigned to Number
            (TypeKind::Number, TypeKind::Number) => true,
            // String can be assigned to String
            (TypeKind::String, TypeKind::String) => true,
            // Boolean can be assigned to Boolean
            (TypeKind::Boolean, TypeKind::Boolean) => true,
            // Array types must match element types
            (TypeKind::Array(a), TypeKind::Array(b)) => {
                Self::is_assignable(a, b)
            }
            // Other combinations are not compatible
            _ => false,
        }
    }

    /// Validate a function call
    pub fn check_call(&mut self, fn_name: &str, arg_types: &[TypeKind]) -> bool {
        if let Some(sig) = self.env.lookup_fn(fn_name) {
            if sig.params.len() != arg_types.len() {
                self.error(format!(
                    "Function '{}' expects {} arguments, got {}",
                    fn_name,
                    sig.params.len(),
                    arg_types.len()
                ));
                return false;
            }

            for (i, (_, expected)) in sig.params.iter().enumerate() {
                if !Self::is_assignable(&arg_types[i], &expected.kind) {
                    self.error(format!(
                        "Argument {} to '{}': expected {}, got {}",
                        i + 1,
                        fn_name,
                        expected.kind,
                        arg_types[i]
                    ));
                    return false;
                }
            }
            return true;
        }

        // Unknown function
        self.error(format!("Unknown function: '{}'", fn_name));
        false
    }

    /// Validate variable assignment
    pub fn check_assign(&mut self, var_name: &str, value_type: &TypeKind) -> bool {
        if let Some(var_type) = self.env.lookup_var(var_name) {
            if !Self::is_assignable(value_type, &var_type.kind) {
                self.error(format!(
                    "Cannot assign {} to variable '{}' of type {}",
                    value_type, var_name, var_type.kind
                ));
                return false;
            }
            return true;
        }

        // Undefined variable - warning but allow (dynamic typing)
        self.warning(format!("Variable '{}' was not previously declared", var_name));
        true
    }

    /// Infer type from a value
    pub fn infer_type(value: &str) -> TypeKind {
        // Try parsing as number
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            return TypeKind::Number;
        }

        // Check for string literal
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return TypeKind::String;
        }

        // Check for boolean
        if value == "true" || value == "false" {
            return TypeKind::Boolean;
        }

        // Default to Any if can't infer
        TypeKind::Any
    }

    /// Get all errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Get all warnings
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Check if type checking passed
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_annotation_from_name() {
        let ty = TypeAnnotation::from_name("number").unwrap();
        assert_eq!(ty.kind, TypeKind::Number);

        let ty = TypeAnnotation::from_name("string").unwrap();
        assert_eq!(ty.kind, TypeKind::String);

        let ty = TypeAnnotation::from_name("boolean").unwrap();
        assert_eq!(ty.kind, TypeKind::Boolean);
    }

    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        env.define_var("x".to_string(), TypeAnnotation::new(TypeKind::Number));

        let x_type = env.lookup_var("x");
        assert!(x_type.is_some());
        assert_eq!(x_type.unwrap().kind, TypeKind::Number);
    }

    #[test]
    fn test_type_compatibility() {
        assert!(TypeChecker::is_assignable(
            &TypeKind::Number,
            &TypeKind::Number
        ));
        assert!(!TypeChecker::is_assignable(
            &TypeKind::Number,
            &TypeKind::String
        ));
        assert!(TypeChecker::is_assignable(
            &TypeKind::Any,
            &TypeKind::Number
        ));
    }

    #[test]
    fn test_type_inference() {
        assert_eq!(TypeChecker::infer_type("42"), TypeKind::Number);
        assert_eq!(TypeChecker::infer_type("3.14"), TypeKind::Number);
        assert_eq!(
            TypeChecker::infer_type("\"hello\""),
            TypeKind::String
        );
        assert_eq!(TypeChecker::infer_type("true"), TypeKind::Boolean);
        assert_eq!(TypeChecker::infer_type("false"), TypeKind::Boolean);
    }

    #[test]
    fn test_type_checker_function_call() {
        let mut checker = TypeChecker::new();
        let sig = FunctionSignature {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), TypeAnnotation::new(TypeKind::Number)),
                ("b".to_string(), TypeAnnotation::new(TypeKind::Number)),
            ],
            return_type: TypeAnnotation::new(TypeKind::Number),
        };
        checker.env.define_fn(sig);

        // Valid call
        assert!(checker.check_call("add", &[TypeKind::Number, TypeKind::Number]));
        assert!(checker.errors().is_empty());

        // Invalid argument count
        assert!(!checker.check_call("add", &[TypeKind::Number]));
        assert!(!checker.errors().is_empty());
    }

    #[test]
    fn test_type_display() {
        assert_eq!(TypeKind::Number.to_string(), "number");
        assert_eq!(TypeKind::String.to_string(), "string");
        assert_eq!(TypeKind::Boolean.to_string(), "boolean");
        assert_eq!(TypeKind::Void.to_string(), "void");
        assert_eq!(TypeKind::Any.to_string(), "any");
        assert_eq!(
            TypeKind::Array(Box::new(TypeKind::Number)).to_string(),
            "number[]"
        );
    }
}
