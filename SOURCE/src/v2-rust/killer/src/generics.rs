/// Killer Generics System
/// Generic type parameters with constraints
///
/// Supports:
/// - Generic functions: `fn map<T, U>(f: (T) -> U, items: List<T>) -> List<U>`
/// - Generic structs: `struct Box<T> { value: T }`
/// - Generic traits: `trait Iterator<T> { fn next() -> T; }`
/// - Type constraints: `fn process<T: Clone>(item: T) -> T`
/// - Higher-kinded types: `Map<K, V>`

use std::collections::HashMap;
use std::fmt;

/// Generic type parameter
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeParameter {
    pub name: String,
    pub constraints: Vec<String>,
}

impl TypeParameter {
    pub fn new(name: impl Into<String>) -> Self {
        TypeParameter {
            name: name.into(),
            constraints: Vec::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }
}

impl fmt::Display for TypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.constraints.is_empty() {
            write!(f, ": {}", self.constraints.join(" + "))?;
        }
        Ok(())
    }
}

/// Generic type instantiation
#[derive(Clone, Debug)]
pub struct GenericType {
    pub name: String,
    pub type_args: Vec<String>,
}

impl GenericType {
    pub fn new(name: impl Into<String>, type_args: Vec<String>) -> Self {
        GenericType {
            name: name.into(),
            type_args,
        }
    }

    pub fn arity(&self) -> usize {
        self.type_args.len()
    }
}

impl fmt::Display for GenericType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.type_args.is_empty() {
            write!(f, "<{}>", self.type_args.join(", "))?;
        }
        Ok(())
    }
}

/// Generic function signature
#[derive(Clone, Debug)]
pub struct GenericFunction {
    pub name: String,
    pub type_params: Vec<TypeParameter>,
    pub params: Vec<(String, String)>, // (name, type)
    pub return_type: String,
}

impl GenericFunction {
    pub fn new(
        name: impl Into<String>,
        type_params: Vec<TypeParameter>,
        params: Vec<(String, String)>,
        return_type: impl Into<String>,
    ) -> Self {
        GenericFunction {
            name: name.into(),
            type_params,
            params,
            return_type: return_type.into(),
        }
    }

    /// Check if function can be instantiated with given types
    pub fn can_instantiate(&self, type_args: &[String]) -> bool {
        type_args.len() == self.type_params.len()
    }

    /// Instantiate function with concrete types
    pub fn instantiate(&self, type_args: &[String]) -> Option<ConcreteFunction> {
        if !self.can_instantiate(type_args) {
            return None;
        }

        let mut type_map: HashMap<String, String> = HashMap::new();
        for (param, arg) in self.type_params.iter().zip(type_args.iter()) {
            type_map.insert(param.name.clone(), arg.clone());
        }

        let concrete_params = self
            .params
            .iter()
            .map(|(name, ty)| {
                let concrete_ty = type_map.get(ty).unwrap_or(ty).clone();
                (name.clone(), concrete_ty)
            })
            .collect();

        let concrete_return = type_map
            .get(&self.return_type)
            .unwrap_or(&self.return_type)
            .clone();

        Some(ConcreteFunction {
            name: self.name.clone(),
            params: concrete_params,
            return_type: concrete_return,
        })
    }
}

impl fmt::Display for GenericFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "fn {}<{}>({}): {}",
            self.name,
            self.type_params
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", "),
            self.return_type
        )
    }
}

/// Concrete (non-generic) function
#[derive(Clone, Debug)]
pub struct ConcreteFunction {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub return_type: String,
}

impl fmt::Display for ConcreteFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "fn {}({}): {}",
            self.name,
            self.params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", "),
            self.return_type
        )
    }
}

/// Generic struct definition
#[derive(Clone, Debug)]
pub struct GenericStruct {
    pub name: String,
    pub type_params: Vec<TypeParameter>,
    pub fields: Vec<(String, String)>, // (name, type)
}

impl GenericStruct {
    pub fn new(
        name: impl Into<String>,
        type_params: Vec<TypeParameter>,
        fields: Vec<(String, String)>,
    ) -> Self {
        GenericStruct {
            name: name.into(),
            type_params,
            fields,
        }
    }

    /// Instantiate struct with concrete types
    pub fn instantiate(&self, type_args: &[String]) -> Option<ConcreteStruct> {
        if type_args.len() != self.type_params.len() {
            return None;
        }

        let mut type_map: HashMap<String, String> = HashMap::new();
        for (param, arg) in self.type_params.iter().zip(type_args.iter()) {
            type_map.insert(param.name.clone(), arg.clone());
        }

        let concrete_fields = self
            .fields
            .iter()
            .map(|(name, ty)| {
                let concrete_ty = type_map.get(ty).unwrap_or(ty).clone();
                (name.clone(), concrete_ty)
            })
            .collect();

        Some(ConcreteStruct {
            name: self.name.clone(),
            fields: concrete_fields,
        })
    }
}

impl fmt::Display for GenericStruct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "struct {}<{}> {{ {} }}",
            self.name,
            self.type_params
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Concrete struct definition
#[derive(Clone, Debug)]
pub struct ConcreteStruct {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl fmt::Display for ConcreteStruct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "struct {} {{ {} }}",
            self.name,
            self.fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Generic trait definition
#[derive(Clone, Debug)]
pub struct GenericTrait {
    pub name: String,
    pub type_params: Vec<TypeParameter>,
    pub methods: Vec<(String, Vec<(String, String)>, String)>, // (name, params, return_type)
}

impl GenericTrait {
    pub fn new(
        name: impl Into<String>,
        type_params: Vec<TypeParameter>,
        methods: Vec<(String, Vec<(String, String)>, String)>,
    ) -> Self {
        GenericTrait {
            name: name.into(),
            type_params,
            methods,
        }
    }
}

impl fmt::Display for GenericTrait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "trait {}<{}> {{ ... }}",
            self.name,
            self.type_params
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Type substitution mapping
pub struct TypeSubstitution {
    mapping: HashMap<String, String>,
}

impl TypeSubstitution {
    pub fn new() -> Self {
        TypeSubstitution {
            mapping: HashMap::new(),
        }
    }

    pub fn insert(&mut self, param: impl Into<String>, concrete_type: impl Into<String>) {
        self.mapping.insert(param.into(), concrete_type.into());
    }

    pub fn resolve(&self, type_name: &str) -> String {
        self.mapping
            .get(type_name)
            .cloned()
            .unwrap_or_else(|| type_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_parameter() {
        let t = TypeParameter::new("T").with_constraint("Clone");
        assert_eq!(t.name, "T");
        assert!(t.constraints.contains(&"Clone".to_string()));
    }

    #[test]
    fn test_generic_type() {
        let gt = GenericType::new("List", vec!["i64".to_string()]);
        assert_eq!(gt.arity(), 1);
        assert_eq!(gt.to_string(), "List<i64>");
    }

    #[test]
    fn test_generic_function() {
        let f = GenericFunction::new(
            "map",
            vec![TypeParameter::new("T"), TypeParameter::new("U")],
            vec![("func".to_string(), "T".to_string())],
            "U".to_string(),
        );
        assert_eq!(f.type_params.len(), 2);
        assert!(f.can_instantiate(&["i64".to_string(), "String".to_string()]));
    }

    #[test]
    fn test_function_instantiation() {
        let f = GenericFunction::new(
            "identity",
            vec![TypeParameter::new("T")],
            vec![("x".to_string(), "T".to_string())],
            "T".to_string(),
        );
        let concrete = f.instantiate(&["i64".to_string()]).unwrap();
        assert_eq!(concrete.return_type, "i64");
    }

    #[test]
    fn test_generic_struct() {
        let s = GenericStruct::new(
            "Box",
            vec![TypeParameter::new("T")],
            vec![("value".to_string(), "T".to_string())],
        );
        let concrete = s.instantiate(&["String".to_string()]).unwrap();
        assert_eq!(concrete.fields[0].1, "String");
    }

    #[test]
    fn test_type_substitution() {
        let mut subst = TypeSubstitution::new();
        subst.insert("T", "i64");
        assert_eq!(subst.resolve("T"), "i64");
        assert_eq!(subst.resolve("U"), "U");
    }
}
