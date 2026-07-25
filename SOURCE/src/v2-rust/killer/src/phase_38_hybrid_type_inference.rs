// KILLER PHASE 38: HYBRID TYPE INFERENCE (OPTION C)
// ============================================================================
// Implements simplified Killer syntax with automatic type inference
// GOAL: Support both implicit (inferred) and explicit types seamlessly
// 
// BEFORE (Verbose):
//   let name: String = "Alice"
//   let age: Int = 30
//   fn add(a: Int, b: Int) -> Int { return a + b }
//
// AFTER (Hybrid - Option C):
//   name = "Alice"              // Auto-infer String
//   age = 30                    // Auto-infer Int
//   fn add(a, b) { a + b }      // Auto-infer Int -> Int
//
// STILL SUPPORTED (Explicit when needed):
//   name: String = "Alice"      // Explicit type
//   fn calc(price: Float) -> Float { price * 1.1 }

use std::collections::HashMap;

// ============================================================================
// 1. TYPE INFERENCE CORE
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KillerType {
    String,
    Integer,
    Float,
    Boolean,
    List(Box<KillerType>),
    Map(Box<KillerType>, Box<KillerType>),
    Unknown,
}

impl std::fmt::Display for KillerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KillerType::String => write!(f, "String"),
            KillerType::Integer => write!(f, "Int"),
            KillerType::Float => write!(f, "Float"),
            KillerType::Boolean => write!(f, "Boolean"),
            KillerType::List(inner) => write!(f, "List<{}>", inner),
            KillerType::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            KillerType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeInferenceContext {
    pub variables: HashMap<String, KillerType>,
    pub functions: HashMap<String, (Vec<KillerType>, KillerType)>, // (param_types, return_type)
    pub scopes: Vec<HashMap<String, KillerType>>,
}

impl TypeInferenceContext {
    pub fn new() -> Self {
        TypeInferenceContext {
            variables: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn add_variable(&mut self, name: &str, typ: KillerType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), typ.clone());
        }
        self.variables.insert(name.to_string(), typ);
    }

    pub fn get_variable(&self, name: &str) -> Option<KillerType> {
        self.variables.get(name).cloned()
    }
}

// ============================================================================
// 2. VALUE INFERENCE - Deduce type from literal values
// ============================================================================

pub fn infer_from_literal(value: &str) -> KillerType {
    // String literals: "...", 'single quotes'
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return KillerType::String;
    }

    // Boolean literals
    if value == "true" || value == "false" {
        return KillerType::Boolean;
    }

    // Try parsing as integer
    if value.parse::<i64>().is_ok() {
        return KillerType::Integer;
    }

    // Try parsing as float
    if value.parse::<f64>().is_ok() {
        return KillerType::Float;
    }

    // List literals: [1, 2, 3]
    if value.starts_with('[') && value.ends_with(']') {
        return KillerType::List(Box::new(KillerType::Unknown));
    }

    // Map literals: {a: 1, b: 2}
    if value.starts_with('{') && value.ends_with('}') && value.contains(':') {
        return KillerType::Map(
            Box::new(KillerType::String),
            Box::new(KillerType::Unknown),
        );
    }

    KillerType::Unknown
}

// ============================================================================
// 3. EXPRESSION INFERENCE - Type from operations
// ============================================================================

pub fn infer_from_operation(left: &KillerType, op: &str, right: &KillerType) -> Result<KillerType, String> {
    match (left, op, right) {
        // Arithmetic: Int + Int = Int
        (KillerType::Integer, "+", KillerType::Integer) => Ok(KillerType::Integer),
        (KillerType::Integer, "-", KillerType::Integer) => Ok(KillerType::Integer),
        (KillerType::Integer, "*", KillerType::Integer) => Ok(KillerType::Integer),
        (KillerType::Integer, "/", KillerType::Integer) => Ok(KillerType::Float), // Division returns Float
        (KillerType::Integer, "%", KillerType::Integer) => Ok(KillerType::Integer),

        // Arithmetic: Float operations return Float
        (KillerType::Float, "+", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "-", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "*", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "/", KillerType::Float) => Ok(KillerType::Float),

        // Mixed Int/Float = Float
        (KillerType::Integer, "+", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "+", KillerType::Integer) => Ok(KillerType::Float),
        (KillerType::Integer, "-", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "-", KillerType::Integer) => Ok(KillerType::Float),
        (KillerType::Integer, "*", KillerType::Float) => Ok(KillerType::Float),
        (KillerType::Float, "*", KillerType::Integer) => Ok(KillerType::Float),

        // String concatenation
        (KillerType::String, "+", KillerType::String) => Ok(KillerType::String),

        // Comparison operators return Boolean
        (_, "<", _) | (_, ">", _) | (_, "<=", _) | (_, ">=", _) | (_, "==", _) | (_, "!=", _) => {
            Ok(KillerType::Boolean)
        }

        // Logical operators
        (KillerType::Boolean, "&&", KillerType::Boolean) => Ok(KillerType::Boolean),
        (KillerType::Boolean, "||", KillerType::Boolean) => Ok(KillerType::Boolean),

        _ => Err(format!(
            "Type mismatch: {} {:?} {} is invalid",
            left, op, right
        )),
    }
}

// ============================================================================
// 4. FUNCTION SIGNATURE PARSING & INFERENCE
// ============================================================================

#[derive(Clone, Debug)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, Option<KillerType>)>, // (name, optional_type)
    pub return_type: Option<KillerType>,
    pub is_auto_return: bool, // implicit return from last expression
}

pub struct FunctionParser;

impl FunctionParser {
    // Parse function signatures supporting both:
    // - fn add(a, b) { ... }  (implicit types)
    // - kfn add(a, b) { ... }  (Killer branded)
    // - fn add(a: Int, b: Int) -> Int { ... }  (explicit types)
    pub fn parse_signature(signature: &str) -> Result<FunctionSignature, String> {
        let signature = signature.trim();

        // Extract function name (support both "fn " and "kfn ")
        let keyword_len = if signature.starts_with("kfn ") {
            4
        } else if signature.starts_with("fn ") {
            3
        } else {
            return Err("Function must start with 'fn' or 'kfn'".to_string());
        };

        let name_end = signature
            .find('(')
            .ok_or("Invalid function signature: missing (")?;
        let name = signature[keyword_len..name_end].trim().to_string();

        // Extract parameters section
        let params_start = name_end + 1;
        let params_end = signature
            .find(')')
            .ok_or("Invalid function signature: missing )")?;
        let params_str = &signature[params_start..params_end];

        // Parse parameters
        let mut params = Vec::new();
        for param in params_str.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }

            if param.contains(':') {
                // Explicit type: a: Int
                let parts: Vec<&str> = param.split(':').collect();
                let param_name = parts[0].trim().to_string();
                let type_str = parts[1].trim();
                let param_type = Self::parse_type(type_str)?;
                params.push((param_name, Some(param_type)));
            } else {
                // Implicit type (inferred later)
                params.push((param.to_string(), None));
            }
        }

        // Extract return type
        let rest = &signature[params_end + 1..];
        let return_type = if rest.contains("->") {
            let arrow_pos = rest.find("->").unwrap();
            let type_str = rest[arrow_pos + 2..].trim();
            let type_end = type_str.find('{').unwrap_or(type_str.len());
            Some(Self::parse_type(&type_str[..type_end].trim())?)
        } else {
            None
        };

        Ok(FunctionSignature {
            name,
            params,
            return_type,
            is_auto_return: true,
        })
    }

    pub fn parse_type(type_str: &str) -> Result<KillerType, String> {
        match type_str.trim() {
            "String" | "string" => Ok(KillerType::String),
            "Int" | "int" | "Integer" => Ok(KillerType::Integer),
            "Float" | "float" => Ok(KillerType::Float),
            "Boolean" | "bool" | "Bool" => Ok(KillerType::Boolean),
            _ if type_str.starts_with("List") => {
                Ok(KillerType::List(Box::new(KillerType::Unknown)))
            }
            _ if type_str.starts_with("Map") => {
                Ok(KillerType::Map(
                    Box::new(KillerType::Unknown),
                    Box::new(KillerType::Unknown),
                ))
            }
            _ => Err(format!("Unknown type: {}", type_str)),
        }
    }
}

// ============================================================================
// 5. VARIABLE DECLARATION PARSING
// ============================================================================

#[derive(Clone, Debug)]
pub struct VariableDeclaration {
    pub name: String,
    pub explicit_type: Option<KillerType>,
    pub value: String,
    pub inferred_type: Option<KillerType>,
}

pub struct VariableParser;

impl VariableParser {
    // Parse variable declarations supporting both:
    // - name = "Alice"  (implicit type from value)
    // - name: String = "Alice"  (explicit type)
    pub fn parse_declaration(declaration: &str) -> Result<VariableDeclaration, String> {
        let declaration = declaration.trim().trim_end_matches(';');

        // Check if explicit type is provided
        if declaration.contains(':') && declaration.contains('=') {
            // name: String = "Alice"
            let colon_pos = declaration.find(':').unwrap();
            let equals_pos = declaration.find('=').unwrap();

            let name = declaration[..colon_pos].trim().to_string();
            let type_str = declaration[colon_pos + 1..equals_pos].trim();
            let value = declaration[equals_pos + 1..].trim().to_string();

            let explicit_type = FunctionParser::parse_type(type_str)?;
            let inferred_type = infer_from_literal(&value);

            Ok(VariableDeclaration {
                name,
                explicit_type: Some(explicit_type),
                value,
                inferred_type: if inferred_type == KillerType::Unknown {
                    None
                } else {
                    Some(inferred_type)
                },
            })
        } else if declaration.contains('=') {
            // name = "Alice" (implicit)
            let equals_pos = declaration.find('=').unwrap();
            let name = declaration[..equals_pos].trim().to_string();
            let value = declaration[equals_pos + 1..].trim().to_string();

            let inferred_type = infer_from_literal(&value);

            Ok(VariableDeclaration {
                name,
                explicit_type: None,
                value,
                inferred_type: if inferred_type == KillerType::Unknown {
                    None
                } else {
                    Some(inferred_type)
                },
            })
        } else {
            Err("Invalid variable declaration".to_string())
        }
    }
}

// ============================================================================
// 6. CONTROL FLOW SIMPLIFICATION (No parentheses)
// ============================================================================

#[derive(Clone, Debug)]
pub struct ControlFlowStatement {
    pub statement_type: String, // "if", "while", "for"
    pub condition: String,
    pub has_parens: bool,
}

pub struct ControlFlowParser;

impl ControlFlowParser {
    // Support both: if (x > 0) { ... } and if x > 0 { ... }
    pub fn parse_if_statement(statement: &str) -> Result<ControlFlowStatement, String> {
        let statement = statement.trim();

        if !statement.starts_with("if ") {
            return Err("Not an if statement".to_string());
        }

        let rest = &statement[3..];
        let (condition, has_parens) = if rest.starts_with('(') {
            // if (x > 0) { ... }
            let close_paren = rest
                .find(')')
                .ok_or("Missing closing parenthesis")?;
            (rest[1..close_paren].to_string(), true)
        } else {
            // if x > 0 { ... }
            let brace_pos = rest
                .find('{')
                .ok_or("Missing opening brace")?;
            (rest[..brace_pos].trim().to_string(), false)
        };

        Ok(ControlFlowStatement {
            statement_type: "if".to_string(),
            condition,
            has_parens,
        })
    }

    pub fn parse_while_statement(statement: &str) -> Result<ControlFlowStatement, String> {
        let statement = statement.trim();

        if !statement.starts_with("while ") {
            return Err("Not a while statement".to_string());
        }

        let rest = &statement[6..];
        let (condition, has_parens) = if rest.starts_with('(') {
            let close_paren = rest
                .find(')')
                .ok_or("Missing closing parenthesis")?;
            (rest[1..close_paren].to_string(), true)
        } else {
            let brace_pos = rest
                .find('{')
                .ok_or("Missing opening brace")?;
            (rest[..brace_pos].trim().to_string(), false)
        };

        Ok(ControlFlowStatement {
            statement_type: "while".to_string(),
            condition,
            has_parens,
        })
    }
}

// ============================================================================
// 7. UTILITY FUNCTIONS
// ============================================================================

pub fn format_function_signature(sig: &FunctionSignature) -> String {
    let params = sig
        .params
        .iter()
        .map(|(name, typ)| {
            if let Some(t) = typ {
                format!("{}: {}", name, t)
            } else {
                name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(ret) = &sig.return_type {
        format!("fn {}({}) -> {}", sig.name, params, ret)
    } else {
        format!("fn {}({})", sig.name, params)
    }
}

pub fn format_variable_declaration(decl: &VariableDeclaration) -> String {
    if let Some(typ) = &decl.explicit_type {
        format!("{}: {} = {}", decl.name, typ, decl.value)
    } else if let Some(typ) = &decl.inferred_type {
        format!("{} = {} // inferred: {}", decl.name, decl.value, typ)
    } else {
        format!("{} = {}", decl.name, decl.value)
    }
}

// ============================================================================
// 8. TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_inference_string() {
        assert_eq!(infer_from_literal("\"hello\""), KillerType::String);
        assert_eq!(infer_from_literal("'world'"), KillerType::String);
    }

    #[test]
    fn test_literal_inference_int() {
        assert_eq!(infer_from_literal("42"), KillerType::Integer);
        assert_eq!(infer_from_literal("-10"), KillerType::Integer);
    }

    #[test]
    fn test_literal_inference_float() {
        assert_eq!(infer_from_literal("3.14"), KillerType::Float);
        assert_eq!(infer_from_literal("-2.5"), KillerType::Float);
    }

    #[test]
    fn test_literal_inference_bool() {
        assert_eq!(infer_from_literal("true"), KillerType::Boolean);
        assert_eq!(infer_from_literal("false"), KillerType::Boolean);
    }

    #[test]
    fn test_operation_inference_int() {
        let result = infer_from_operation(&KillerType::Integer, "+", &KillerType::Integer);
        assert_eq!(result, Ok(KillerType::Integer));
    }

    #[test]
    fn test_operation_inference_mixed() {
        let result = infer_from_operation(&KillerType::Integer, "+", &KillerType::Float);
        assert_eq!(result, Ok(KillerType::Float));
    }

    #[test]
    fn test_operation_inference_comparison() {
        let result = infer_from_operation(&KillerType::Integer, "<", &KillerType::Integer);
        assert_eq!(result, Ok(KillerType::Boolean));
    }

    #[test]
    fn test_function_parser_implicit() {
        let sig = FunctionParser::parse_signature("fn add(a, b)").unwrap();
        assert_eq!(sig.name, "add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].0, "a");
        assert!(sig.params[0].1.is_none());
    }

    #[test]
    fn test_function_parser_explicit() {
        let sig = FunctionParser::parse_signature("fn add(a: Int, b: Int) -> Int").unwrap();
        assert_eq!(sig.name, "add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].1, Some(KillerType::Integer));
        assert_eq!(sig.return_type, Some(KillerType::Integer));
    }

    #[test]
    fn test_function_parser_kfn_implicit() {
        let sig = FunctionParser::parse_signature("kfn add(a, b)").unwrap();
        assert_eq!(sig.name, "add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].0, "a");
        assert!(sig.params[0].1.is_none());
    }

    #[test]
    fn test_function_parser_kfn_explicit() {
        let sig = FunctionParser::parse_signature("kfn add(a: Int, b: Int) -> Int").unwrap();
        assert_eq!(sig.name, "add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].1, Some(KillerType::Integer));
        assert_eq!(sig.return_type, Some(KillerType::Integer));
    }

    #[test]
    fn test_variable_parser_implicit() {
        let decl = VariableParser::parse_declaration("name = \"Alice\"").unwrap();
        assert_eq!(decl.name, "name");
        assert!(decl.explicit_type.is_none());
        assert_eq!(decl.inferred_type, Some(KillerType::String));
    }

    #[test]
    fn test_variable_parser_explicit() {
        let decl = VariableParser::parse_declaration("name: String = \"Alice\"").unwrap();
        assert_eq!(decl.name, "name");
        assert_eq!(decl.explicit_type, Some(KillerType::String));
        assert_eq!(decl.inferred_type, Some(KillerType::String));
    }

    #[test]
    fn test_control_flow_parser_if_with_parens() {
        let stmt = ControlFlowParser::parse_if_statement("if (x > 0) {").unwrap();
        assert_eq!(stmt.condition, "x > 0");
        assert!(stmt.has_parens);
    }

    #[test]
    fn test_control_flow_parser_if_without_parens() {
        let stmt = ControlFlowParser::parse_if_statement("if x > 0 {").unwrap();
        assert_eq!(stmt.condition, "x > 0");
        assert!(!stmt.has_parens);
    }

    #[test]
    fn test_control_flow_parser_while_without_parens() {
        let stmt = ControlFlowParser::parse_while_statement("while i < 10 {").unwrap();
        assert_eq!(stmt.condition, "i < 10");
        assert!(!stmt.has_parens);
    }
}
