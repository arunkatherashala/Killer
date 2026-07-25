// src/v2-rust/killer_vm/src/trait_system.rs
// Trait system for Killer language - v3.0 simplified implementation
// Provides trait definitions, implementations, and method resolution

use std::collections::HashMap;

/// Trait definition
#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethod>,
    pub doc: Option<String>,
}

/// Method signature in a trait
#[derive(Clone, Debug)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>, // Optional type hint for v3.0
}

/// Implementation of a trait for a specific type
#[derive(Clone, Debug)]
pub struct TraitImpl {
    pub trait_name: String,
    pub for_type: String, // Type or class name
    pub methods: HashMap<String, TraitImplMethod>,
}

/// Implemented method in a trait implementation
#[derive(Clone, Debug)]
pub struct TraitImplMethod {
    pub name: String,
    pub params: Vec<String>,
    pub body_hash: u64, // Reference to compiled body in compiler
}

/// Global trait registry
pub struct TraitRegistry {
    pub traits: HashMap<String, TraitDef>,
    pub implementations: Vec<TraitImpl>,
    pub type_traits: HashMap<String, Vec<String>>, // type_name -> [list of trait names]
}

impl TraitRegistry {
    /// Create a new trait registry
    pub fn new() -> Self {
        TraitRegistry {
            traits: HashMap::new(),
            implementations: Vec::new(),
            type_traits: HashMap::new(),
        }
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, trait_def: TraitDef) -> Result<(), String> {
        if self.traits.contains_key(&trait_def.name) {
            return Err(format!("Trait '{}' already defined", trait_def.name));
        }
        self.traits.insert(trait_def.name.clone(), trait_def);
        Ok(())
    }

    /// Register a trait implementation
    pub fn register_impl(&mut self, trait_impl: TraitImpl) -> Result<(), String> {
        // Verify trait exists
        if !self.traits.contains_key(&trait_impl.trait_name) {
            return Err(format!("Trait '{}' not found", trait_impl.trait_name));
        }

        // Verify all required methods are implemented
        let trait_def = &self.traits[&trait_impl.trait_name];
        for method in &trait_def.methods {
            if !trait_impl.methods.contains_key(&method.name) {
                return Err(format!(
                    "Method '{}' required by trait '{}' not implemented for type '{}'",
                    method.name, trait_impl.trait_name, trait_impl.for_type
                ));
            }
        }

        // Register type-to-trait mapping
        self.type_traits
            .entry(trait_impl.for_type.clone())
            .or_insert_with(Vec::new)
            .push(trait_impl.trait_name.clone());

        self.implementations.push(trait_impl);
        Ok(())
    }

    /// Check if a type implements a trait
    pub fn implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        self.type_traits
            .get(type_name)
            .map(|traits| traits.contains(&trait_name.to_string()))
            .unwrap_or(false)
    }

    /// Get all traits for a type
    pub fn get_traits_for_type(&self, type_name: &str) -> Vec<String> {
        self.type_traits
            .get(type_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get implementation of a trait for a type
    pub fn get_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&TraitImpl> {
        self.implementations
            .iter()
            .find(|imp| imp.trait_name == trait_name && imp.for_type == for_type)
    }

    /// Resolve a method call using traits
    pub fn resolve_method(
        &self,
        object_type: &str,
        method_name: &str,
    ) -> Option<(String, String)> {
        // First check direct implementation
        for impl_block in &self.implementations {
            if impl_block.for_type == object_type {
                if impl_block.methods.contains_key(method_name) {
                    return Some((impl_block.trait_name.clone(), method_name.to_string()));
                }
            }
        }

        // Check trait methods
        if let Some(traits) = self.type_traits.get(object_type) {
            for trait_name in traits {
                if let Some(trait_def) = self.traits.get(trait_name) {
                    if trait_def.methods.iter().any(|m| m.name == method_name) {
                        return Some((trait_name.clone(), method_name.to_string()));
                    }
                }
            }
        }

        None
    }
}

/// Built-in default trait implementations
pub fn create_default_traits() -> TraitRegistry {
    let mut registry = TraitRegistry::new();

    // Trait: Display
    let display_trait = TraitDef {
        name: "Display".to_string(),
        methods: vec![TraitMethod {
            name: "to_string".to_string(),
            params: vec![],
            return_type: Some("String".to_string()),
        }],
        doc: Some("Trait for types that can be displayed as strings".to_string()),
    };

    // Trait: Comparable
    let comparable_trait = TraitDef {
        name: "Comparable".to_string(),
        methods: vec![
            TraitMethod {
                name: "compare_to".to_string(),
                params: vec!["other".to_string()],
                return_type: Some("Number".to_string()),
            },
            TraitMethod {
                name: "equals".to_string(),
                params: vec!["other".to_string()],
                return_type: Some("Bool".to_string()),
            },
        ],
        doc: Some("Trait for types that can be compared".to_string()),
    };

    // Trait: Cloneable
    let cloneable_trait = TraitDef {
        name: "Cloneable".to_string(),
        methods: vec![TraitMethod {
            name: "clone".to_string(),
            params: vec![],
            return_type: None,
        }],
        doc: Some("Trait for types that can be cloned".to_string()),
    };

    // Trait: Iterable
    let iterable_trait = TraitDef {
        name: "Iterable".to_string(),
        methods: vec![
            TraitMethod {
                name: "iterator".to_string(),
                params: vec![],
                return_type: None,
            },
            TraitMethod {
                name: "has_next".to_string(),
                params: vec![],
                return_type: Some("Bool".to_string()),
            },
        ],
        doc: Some("Trait for types that can be iterated".to_string()),
    };

    // Register all built-in traits
    registry.register_trait(display_trait).ok();
    registry.register_trait(comparable_trait).ok();
    registry.register_trait(cloneable_trait).ok();
    registry.register_trait(iterable_trait).ok();

    registry
}

/// Trait constraint used in generic functions
#[derive(Clone, Debug)]
pub struct TraitBound {
    pub type_var: String,
    pub trait_name: String,
}

/// Trait-based generic function signature
#[derive(Clone, Debug)]
pub struct GenericFunction {
    pub name: String,
    pub type_params: Vec<String>,
    pub trait_bounds: Vec<TraitBound>,
    pub param_types: Vec<String>,
    pub return_type: Option<String>,
}

/// Trait object for dynamic dispatch (v3.0 simplified)
#[derive(Clone, Debug)]
pub struct TraitObject {
    pub trait_name: String,
    pub actual_type: String,
    pub value_id: u64, // Reference to actual value
}

impl TraitObject {
    /// Create a new trait object
    pub fn new(trait_name: String, actual_type: String, value_id: u64) -> Self {
        TraitObject {
            trait_name,
            actual_type,
            value_id,
        }
    }
}

/// Trait method resolution cache
pub struct MethodResolutionCache {
    cache: HashMap<String, (String, String)>, // "type:method" -> (trait, method)
}

impl MethodResolutionCache {
    pub fn new() -> Self {
        MethodResolutionCache {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, type_name: &str, method_name: &str) -> Option<(String, String)> {
        let key = format!("{}:{}", type_name, method_name);
        self.cache.get(&key).cloned()
    }

    pub fn insert(&mut self, type_name: &str, method_name: &str, trait_name: String) {
        let key = format!("{}:{}", type_name, method_name);
        self.cache.insert(key, (trait_name, method_name.to_string()));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

// Helper function to check trait compatibility
pub fn is_trait_compatible(
    registry: &TraitRegistry,
    actual_type: &str,
    expected_trait: &str,
) -> bool {
    registry.implements_trait(actual_type, expected_trait)
}

// Example trait implementations for built-in types
pub fn register_builtin_impls(registry: &mut TraitRegistry) -> Result<(), String> {
    // String implements Display
    let string_display = TraitImpl {
        trait_name: "Display".to_string(),
        for_type: "String".to_string(),
        methods: {
            let mut m = HashMap::new();
            m.insert(
                "to_string".to_string(),
                TraitImplMethod {
                    name: "to_string".to_string(),
                    params: vec![],
                    body_hash: 1001,
                },
            );
            m
        },
    };

    // Number implements Comparable
    let number_comparable = TraitImpl {
        trait_name: "Comparable".to_string(),
        for_type: "Number".to_string(),
        methods: {
            let mut m = HashMap::new();
            m.insert(
                "compare_to".to_string(),
                TraitImplMethod {
                    name: "compare_to".to_string(),
                    params: vec!["other".to_string()],
                    body_hash: 1002,
                },
            );
            m.insert(
                "equals".to_string(),
                TraitImplMethod {
                    name: "equals".to_string(),
                    params: vec!["other".to_string()],
                    body_hash: 1003,
                },
            );
            m
        },
    };

    // Array implements Iterable
    let array_iterable = TraitImpl {
        trait_name: "Iterable".to_string(),
        for_type: "Array".to_string(),
        methods: {
            let mut m = HashMap::new();
            m.insert(
                "iterator".to_string(),
                TraitImplMethod {
                    name: "iterator".to_string(),
                    params: vec![],
                    body_hash: 1004,
                },
            );
            m.insert(
                "has_next".to_string(),
                TraitImplMethod {
                    name: "has_next".to_string(),
                    params: vec![],
                    body_hash: 1005,
                },
            );
            m
        },
    };

    registry.register_impl(string_display)?;
    registry.register_impl(number_comparable)?;
    registry.register_impl(array_iterable)?;

    Ok(())
}
