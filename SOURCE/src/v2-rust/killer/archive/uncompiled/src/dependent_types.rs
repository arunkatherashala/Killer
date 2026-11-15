// Week 1: Dependent Types Parser
// Parses dependent type syntax: Vector[n], Matrix[m][n], fn foo[n: nat](v: Vector[n])

use crate::lexer::{Token, TokenKind};
use std::fmt;

/// Dependent type parameter (generic)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeParam {
    pub name: String,
    pub kind: TypeParamKind,
}

/// Kind of type parameter
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeParamKind {
    /// Natural numbers: 0, 1, 2, ...
    Nat,
    /// Integers: ..., -1, 0, 1, ...
    Int,
    /// Type-level types
    Type,
    /// Compile-time booleans
    Bool,
}

impl fmt::Display for TypeParamKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeParamKind::Nat => write!(f, "nat"),
            TypeParamKind::Int => write!(f, "int"),
            TypeParamKind::Type => write!(f, "type"),
            TypeParamKind::Bool => write!(f, "bool"),
        }
    }
}

/// Dependent type expression: Vector[n], Matrix[m][n], etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependentType {
    /// Base type with parameters: Vector[n]
    Named {
        name: String,
        params: Vec<DependentTypeArg>,
    },
    /// Function type: fn(Vector[n]) -> i32
    Function {
        params: Vec<(String, DependentType)>,
        return_type: Box<DependentType>,
    },
    /// Simple type without parameters
    Simple(String),
}

/// Argument to dependent type: n, m+n, 5, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependentTypeArg {
    /// Variable reference: n
    Var(String),
    /// Literal: 5, 10, etc.
    Literal(i64),
    /// Arithmetic: m + n, n - 1, etc.
    BinOp {
        left: Box<DependentTypeArg>,
        op: String,
        right: Box<DependentTypeArg>,
    },
}

impl fmt::Display for DependentTypeArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DependentTypeArg::Var(name) => write!(f, "{}", name),
            DependentTypeArg::Literal(n) => write!(f, "{}", n),
            DependentTypeArg::BinOp { left, op, right } => {
                write!(f, "{} {} {}", left, op, right)
            }
        }
    }
}

impl fmt::Display for DependentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DependentType::Named { name, params } => {
                write!(f, "{}", name)?;
                if !params.is_empty() {
                    write!(f, "[")?;
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", param)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            DependentType::Function {
                params,
                return_type,
            } => {
                write!(f, "fn(")?;
                for (i, (name, ty)) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, ") -> {}", return_type)
            }
            DependentType::Simple(name) => write!(f, "{}", name),
        }
    }
}

/// Function signature with dependent types
#[derive(Clone, Debug)]
pub struct FunctionSignature {
    pub name: String,
    /// Type parameters: [n: nat, m: nat]
    pub type_params: Vec<TypeParam>,
    /// Function parameters: (name, type)
    pub params: Vec<(String, DependentType)>,
    pub return_type: DependentType,
}

impl fmt::Display for FunctionSignature {
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
        for (i, (name, ty)) in self.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}: {}", name, ty)?;
        }
        write!(f, ")")?;
        
        // Return type
        write!(f, " -> {}", self.return_type)
    }
}

/// Parser for dependent types
pub struct DependentTypeParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl DependentTypeParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        DependentTypeParser { tokens, pos: 0 }
    }
    
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }
    
    fn advance(&mut self) {
        self.pos += 1;
    }
    
    fn current_kind(&self) -> Option<&TokenKind> {
        self.current().map(|t| &t.kind)
    }
    
    fn expect(&mut self, expected: &str) -> Result<(), String> {
        match (expected, self.current_kind()) {
            // Identifier tokens
            ("fn", Some(TokenKind::Identifier(s))) if s == "fn" => {
                self.advance();
                Ok(())
            }
            // Punctuation tokens
            ("(", Some(TokenKind::LParen)) => {
                self.advance();
                Ok(())
            }
            (")", Some(TokenKind::RParen)) => {
                self.advance();
                Ok(())
            }
            ("->", Some(TokenKind::Arrow)) => {
                self.advance();
                Ok(())
            }
            _ => {
                let found = format!("{:?}", self.current_kind());
                Err(format!("Expected '{}', found {}", expected, found))
            }
        }
    }
    
    /// Parse function signature with dependent types
    /// fn foo[n: nat](v: Vector[n]) -> i32
    pub fn parse_function_signature(&mut self) -> Result<FunctionSignature, String> {
        self.expect("fn")?;
        
        // Function name
        let name = match self.current_kind() {
            Some(TokenKind::Identifier(s)) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => return Err("Expected function name".to_string()),
        };
        
        // Type parameters [n: nat, m: nat]
        let type_params = if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
            self.advance(); // consume [
            self.parse_type_params()?
        } else {
            Vec::new()
        };
        
        // Function parameters
        self.expect("(")?;
        let params = self.parse_function_params()?;
        self.expect(")")?;
        
        // Return type
        self.expect("->")?;
        let return_type = self.parse_dependent_type()?;
        
        Ok(FunctionSignature {
            name,
            type_params,
            params,
            return_type,
        })
    }
    
    /// Parse type parameters: [n: nat, m: nat]
    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        let mut params = Vec::new();
        
        loop {
            // Parse param name
            let name = match self.current_kind() {
                Some(TokenKind::Identifier(s)) => {
                    let n = s.clone();
                    self.advance();
                    n
                }
                _ => return Err("Expected type parameter name".to_string()),
            };
            
            // Parse colon
            match self.current_kind() {
                Some(TokenKind::Colon) => self.advance(),
                _ => return Err("Expected ':' after type parameter".to_string()),
            }
            
            // Parse kind (nat, int, type, bool)
            let kind = match self.current_kind() {
                Some(TokenKind::Identifier(s)) => {
                    let k = match s.as_str() {
                        "nat" => TypeParamKind::Nat,
                        "int" => TypeParamKind::Int,
                        "type" => TypeParamKind::Type,
                        "bool" => TypeParamKind::Bool,
                        _ => return Err(format!("Unknown type kind: {}", s)),
                    };
                    self.advance();
                    k
                }
                _ => return Err("Expected type kind (nat, int, type, bool)".to_string()),
            };
            
            params.push(TypeParam { name, kind });
            
            // Check for comma (more params) or ] (end)
            match self.current_kind() {
                Some(TokenKind::Comma) => {
                    self.advance();
                    continue;
                }
                Some(TokenKind::RBracket) => {
                    self.advance();
                    break;
                }
                _ => return Err("Expected ',' or ']' in type parameters".to_string()),
            }
        }
        
        Ok(params)
    }
    
    /// Parse function parameters: (v: Vector[n], idx: Idx[n])
    fn parse_function_params(&mut self) -> Result<Vec<(String, DependentType)>, String> {
        let mut params = Vec::new();
        
        loop {
            match self.current_kind() {
                Some(TokenKind::RParen) => break,
                Some(TokenKind::Identifier(s)) => {
                    let name = s.clone();
                    self.advance();
                    
                    // Expect :
                    match self.current_kind() {
                        Some(TokenKind::Colon) => self.advance(),
                        _ => return Err("Expected ':' after parameter name".to_string()),
                    }
                    
                    // Parse type
                    let ty = self.parse_dependent_type()?;
                    params.push((name, ty));
                    
                    // Check for comma or end
                    match self.current_kind() {
                        Some(TokenKind::Comma) => {
                            self.advance();
                        }
                        Some(TokenKind::RParen) => break,
                        _ => return Err("Expected ',' or ')' in parameters".to_string()),
                    }
                }
                _ => return Err("Expected parameter name".to_string()),
            }
        }
        
        Ok(params)
    }
    
    /// Parse dependent type: Vector[n], Matrix[m][n], i32, etc.
    pub fn parse_dependent_type(&mut self) -> Result<DependentType, String> {
        match self.current_kind() {
            Some(TokenKind::Identifier(name)) => {
                let base_name = name.clone();
                self.advance();
                
                // Check for type arguments [...]
                let params = if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.parse_dependent_type_args()?
                } else {
                    Vec::new()
                };
                
                if params.is_empty() {
                    Ok(DependentType::Simple(base_name))
                } else {
                    Ok(DependentType::Named {
                        name: base_name,
                        params,
                    })
                }
            }
            _ => Err(format!("Expected type name, found {:?}", self.current_kind())),
        }
    }
    
    /// Parse type arguments: [n], [m][n], etc.
    fn parse_dependent_type_args(&mut self) -> Result<Vec<DependentTypeArg>, String> {
        let mut all_args = Vec::new();
        
        loop {
            match self.current_kind() {
                Some(TokenKind::LBracket) => {
                    self.advance();
                    
                    // Parse comma-separated arguments
                    let mut bracket_args = Vec::new();
                    loop {
                        let arg = self.parse_dependent_type_arg()?;
                        bracket_args.push(arg);
                        
                        match self.current_kind() {
                            Some(TokenKind::Comma) => {
                                self.advance();
                            }
                            Some(TokenKind::RBracket) => {
                                self.advance();
                                break;
                            }
                            _ => return Err("Expected ',' or ']' in type arguments".to_string()),
                        }
                    }
                    
                    all_args.extend(bracket_args);
                }
                _ => break,
            }
        }
        
        Ok(all_args)
    }
    
    /// Parse single type argument: n, m+n, 5, etc.
    fn parse_dependent_type_arg(&mut self) -> Result<DependentTypeArg, String> {
        // For now, simple implementation: variable or number
        match self.current_kind() {
            Some(TokenKind::Identifier(s)) => {
                let name = s.clone();
                self.advance();
                Ok(DependentTypeArg::Var(name))
            }
            Some(TokenKind::Number(n)) => {
                let num = *n as i64;
                self.advance();
                Ok(DependentTypeArg::Literal(num))
            }
            _ => Err(format!("Expected variable or number in type argument, found {:?}", self.current_kind())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_param_display() {
        let param = TypeParam {
            name: "n".to_string(),
            kind: TypeParamKind::Nat,
        };
        assert_eq!(format!("{}: {}", param.name, param.kind), "n: nat");
    }
    
    #[test]
    fn test_dependent_type_display_simple() {
        let ty = DependentType::Simple("i32".to_string());
        assert_eq!(ty.to_string(), "i32");
    }
    
    #[test]
    fn test_dependent_type_display_named() {
        let ty = DependentType::Named {
            name: "Vector".to_string(),
            params: vec![DependentTypeArg::Var("n".to_string())],
        };
        assert_eq!(ty.to_string(), "Vector[n]");
    }
    
    #[test]
    fn test_function_signature_display() {
        let sig = FunctionSignature {
            name: "process".to_string(),
            type_params: vec![TypeParam {
                name: "n".to_string(),
                kind: TypeParamKind::Nat,
            }],
            params: vec![(
                "v".to_string(),
                DependentType::Named {
                    name: "Vector".to_string(),
                    params: vec![DependentTypeArg::Var("n".to_string())],
                },
            )],
            return_type: DependentType::Simple("i32".to_string()),
        };
        
        let s = sig.to_string();
        assert!(s.contains("fn process[n: nat]"));
        assert!(s.contains("v: Vector[n]"));
        assert!(s.contains("-> i32"));
    }
}
