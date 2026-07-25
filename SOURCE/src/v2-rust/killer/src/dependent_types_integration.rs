// Week 2: Dependent Types Integration
// Bridges dependent_types parser with main compiler pipeline

use crate::dependent_types::{DependentTypeParser, FunctionSignature, TypeParam};
use crate::lexer::{Token, TokenKind};
use std::collections::HashMap;

/// Metadata about a function with dependent types
#[derive(Clone, Debug)]
pub struct DependentTypeFunctionMeta {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub param_names: Vec<String>,
    pub param_types: Vec<String>, // Stored as strings for now
    pub return_type: String,
}

/// Registry of functions with dependent types
#[derive(Default, Clone)]
pub struct DependentTypeRegistry {
    functions: HashMap<String, DependentTypeFunctionMeta>,
}

impl DependentTypeRegistry {
    pub fn new() -> Self {
        DependentTypeRegistry::default()
    }

    /// Register a function with dependent types
    pub fn register_function(&mut self, func: DependentTypeFunctionMeta) {
        self.functions.insert(func.name.clone(), func);
    }

    /// Look up a function's dependent type metadata
    pub fn lookup_function(&self, name: &str) -> Option<&DependentTypeFunctionMeta> {
        self.functions.get(name)
    }

    /// Check if a function has dependent types
    pub fn has_dependent_types(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get all registered functions with dependent types
    pub fn all_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}

/// Extract dependent type information from function tokens
/// 
/// Format: fn foo[n: nat](v: Vector[n], idx: Idx[n]) -> i32
/// 
/// This function handles the special case where the parser encounters
/// type parameters before function parameters.
pub fn extract_dependent_types_from_tokens(
    tokens: &[Token],
    start_pos: usize,
) -> Option<(DependentTypeFunctionMeta, usize)> {
    // Check if we have enough tokens and if next token after function name is [
    if start_pos >= tokens.len() {
        return None;
    }
    
    // Look for pattern: Identifier (function name) followed by LBracket (type params)
    let mut pos = start_pos;
    
    // Get function name (should be current)
    let func_name = match &tokens[pos].kind {
        TokenKind::Identifier(name) => {
            let n = name.clone();
            pos += 1;
            n
        }
        _ => return None,
    };
    
    // Check if next is [  
    if pos >= tokens.len() {
        return None;
    }
    
    if !matches!(&tokens[pos].kind, TokenKind::LBracket) {
        // No type parameters - not a dependent types function
        return None;
    }
    
    // Try to parse as dependent type function
    let mut parser = DependentTypeParser::new(tokens[pos..].to_vec());
    
    // We need to construct fake tokens for the parser
    // The parser expects: [n: nat](...)->, but we only have from [
    // Skip the actual parsing for now - just recognize the pattern
    
    // For MVP: only recognize if there's a [ followed by identifiers and :
    pos += 1; // consume [
    
    let mut type_params = Vec::new();
    let mut bracket_depth = 1;
    let mut in_type_params = true;
    let mut _found_identifiers: Vec<String> = Vec::new();
    
    while pos < tokens.len() && bracket_depth > 0 {
        match &tokens[pos].kind {
            TokenKind::LBracket => {
                bracket_depth += 1;
                pos += 1;
            }
            TokenKind::RBracket => {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    in_type_params = false;
                }
                pos += 1;
            }
            TokenKind::Identifier(name) if in_type_params => {
                // This might be a type parameter
                pos += 1;
                // For MVP, just track that we found identifiers
            }
            _ => pos += 1,
        }
    }
    
    // Simple implementation: return a basic metadata struct
    // Full type information extraction is deferred to later phases
    Some((
        DependentTypeFunctionMeta {
            name: func_name,
            type_params,
            param_names: Vec::new(),
            param_types: Vec::new(),
            return_type: String::from("unknown"),
        },
        pos,
    ))
}

/// Check if a function signature contains dependent types
/// (Simple check: looks for [ after function name)
pub fn has_dependent_type_syntax(tokens: &[Token], fn_start: usize) -> bool {
    // Skip 'fn' keyword
    let mut pos = fn_start + 1;
    
    // Skip function name
    if pos < tokens.len() && matches!(&tokens[pos].kind, TokenKind::Identifier(_)) {
        pos += 1;
    }
    
    // Check for [ (indicates type parameters)
    pos < tokens.len() && matches!(&tokens[pos].kind, TokenKind::LBracket)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependent_type_registry_new() {
        let registry = DependentTypeRegistry::new();
        assert_eq!(registry.all_functions().len(), 0);
    }

    #[test]
    fn test_register_and_lookup() {
        let mut registry = DependentTypeRegistry::new();
        
        let func = DependentTypeFunctionMeta {
            name: "process".to_string(),
            type_params: vec![],
            param_names: vec!["v".to_string()],
            param_types: vec!["Vector[n]".to_string()],
            return_type: "i32".to_string(),
        };
        
        registry.register_function(func);
        assert!(registry.has_dependent_types("process"));
        
        let looked_up = registry.lookup_function("process");
        assert!(looked_up.is_some());
        assert_eq!(looked_up.unwrap().name, "process");
    }

    #[test]
    fn test_lookup_nonexistent() {
        let registry = DependentTypeRegistry::new();
        assert!(!registry.has_dependent_types("unknown"));
        assert!(registry.lookup_function("unknown").is_none());
    }
}
