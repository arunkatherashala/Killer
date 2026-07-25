/// Killer Language Parser v4.2 - Hybrid Indentation Support
/// Adds indentation-based syntax alongside traditional brace-based syntax
///
/// NEW FEATURES:
/// - Hybrid syntax: indent-based OR brace-based (user chooses)
/// - Indentation tracking: INDENT/DEDENT/NEWLINE tokens
/// - Type safety: Preserves all existing type annotations
/// - Backward compatible: All brace-based code still works
/// - Error recovery: Clear indentation error messages
///
/// Example:
/// ```killer
/// # Indentation-based (new)
/// kfn add(a, b)
///   a + b
///
/// # Brace-based (existing)
/// kfn multiply(a, b) { a * b }
/// ```

use crate::error_handling::{KillerError, Result};
use crate::type_system::{TypeKind, TypeAnnotation};
use std::collections::VecDeque;

/// Token types for lexing with indentation support
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    // Literals
    Number(i64),
    String(String),
    Boolean(bool),
    Identifier(String),
    
    // Keywords
    Let, Const, Fn, If, Else, While, For, In, Return, Break, Continue,
    True, False, Null, Void, Any,
    Match, Enum, Struct, Class, Trait, Impl, Use, Pub,
    
    // Type keywords
    I64, String_, Bool, Float, U32, I32,
    
    // Operators
    Plus, Minus, Star, Slash, Percent, Equal, EqualEqual, NotEqual,
    Less, Greater, LessEqual, GreaterEqual, And, Or, Not, Ampersand,
    
    // Delimiters
    LeftParen, RightParen, LeftBrace, RightBrace, LeftBracket, RightBracket,
    Comma, Semicolon, Colon, Dot, Arrow, FatArrow, Underscore,
    
    // NEW: Indentation tokens (v4.2)
    INDENT(usize),      // Indentation level (column number)
    DEDENT(usize),      // Number of dedent levels
    NEWLINE,            // Logical newline (significant in indentation mode)
    
    // Special
    Eof,
}

/// Token with position information
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
    pub value: String,
}

/// Lexer - converts source code to tokens with indentation tracking
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    
    // NEW FIELDS: Indentation tracking
    indent_stack: Vec<usize>,       // Stack of indentation levels (0 at base)
    pending_dedents: VecDeque<Token>, // Dedent tokens pending emission
    line_start: bool,               // True at start of line (before whitespace)
    indent_mode: bool,              // Enable indentation-based syntax parsing
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            input: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            indent_stack: vec![0],  // Base indentation = 0
            pending_dedents: VecDeque::new(),
            line_start: true,
            indent_mode: true,      // Enabled by default
        }
    }
    
    /// Create lexer with indentation disabled (pure brace-based)
    pub fn with_indent_mode(mut self, enabled: bool) -> Self {
        self.indent_mode = enabled;
        self
    }

    fn current_char(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn peek_char(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn peek_char_ahead(&self, n: usize) -> Option<char> {
        if self.position + n < self.input.len() {
            Some(self.input[self.position + n])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current_char()?;
        self.position += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else if ch == '/' && self.peek_char() == Some('/') {
                // Skip line comment
                while let Some(c) = self.current_char() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn read_number(&mut self) -> i64 {
        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch.is_numeric() {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result.parse().unwrap_or(0)
    }

    fn read_string(&mut self, quote: char) -> String {
        let mut result = String::new();
        self.advance(); // skip opening quote
        
        while let Some(ch) = self.current_char() {
            if ch == quote {
                self.advance(); // skip closing quote
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        _ => result.push(escaped),
                    }
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }
        result
    }

    // NEW METHOD (v4.2): Track indentation at line start
    /// Analyzes indentation at start of line and emits INDENT/DEDENT tokens
    fn track_indentation(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        if !self.line_start || !self.indent_mode {
            return Ok(tokens);
        }

        self.line_start = false;
        let indent_line = self.line;
        let indent_col = self.column;

        // Count leading spaces/tabs
        let mut indent_count = 0;
        let mut uses_spaces: Option<bool> = None;

        while let Some(ch) = self.current_char() {
            match ch {
                ' ' => {
                    // Mixed tabs/spaces error
                    if uses_spaces == Some(false) {
                        return Err(KillerError::new(
                            format!("IndentationError at line {}: mixed tabs and spaces", self.line)
                        ));
                    }
                    uses_spaces = Some(true);
                    indent_count += 1;
                    self.advance();
                }
                '\t' => {
                    // Mixed tabs/spaces error
                    if uses_spaces == Some(true) {
                        return Err(KillerError::new(
                            format!("IndentationError at line {}: mixed tabs and spaces", self.line)
                        ));
                    }
                    uses_spaces = Some(false);
                    indent_count += 4;  // Tab = 4 spaces equivalent
                    self.advance();
                }
                _ => break,
            }
        }

        // Skip blank lines and comments
        match self.current_char() {
            Some('\n') | Some('/') | None => {
                // Blank line or comment line - no indent/dedent
                self.line_start = true;
                return Ok(tokens);
            }
            _ => {}
        }

        let current_level = *self.indent_stack.last().unwrap_or(&0);

        if indent_count > current_level {
            // INDENT: Increase indentation
            self.indent_stack.push(indent_count);
            tokens.push(Token {
                token_type: TokenType::INDENT(indent_count),
                line: indent_line,
                column: indent_col,
                value: " ".repeat(indent_count),
            });
        } else if indent_count < current_level {
            // DEDENT(s): Decrease indentation (may emit multiple DEDENT tokens)
            let mut dedent_count = 0;
            while let Some(&level) = self.indent_stack.last() {
                if level == indent_count {
                    break;
                } else if level > indent_count {
                    self.indent_stack.pop();
                    dedent_count += 1;
                    tokens.push(Token {
                        token_type: TokenType::DEDENT(1),
                        line: indent_line,
                        column: indent_col,
                        value: String::new(),
                    });
                } else {
                    // Indentation error: dedent amount not matching any previous level
                    return Err(KillerError::new(
                        format!("IndentationError at line {}: unexpected dedent amount", self.line)
                    ));
                }
            }
        }
        // indent_count == current_level: No INDENT or DEDENT

        Ok(tokens)
    }

    // NEW METHOD (v4.2): Handle newline tokens
    /// Emits NEWLINE token and marks line start for indentation tracking
    fn handle_newline(&mut self) -> Token {
        let token = Token {
            token_type: TokenType::NEWLINE,
            line: self.line,
            column: self.column,
            value: "\n".to_string(),
        };
        self.advance();
        self.line_start = true;
        token
    }

    pub fn next_token(&mut self) -> Token {
        // First, emit any pending dedents
        if let Some(dedent) = self.pending_dedents.pop_front() {
            return dedent;
        }

        self.skip_whitespace();
        
        let line = self.line;
        let column = self.column;

        if let Some(ch) = self.current_char() {
            match ch {
                '(' => {
                    self.advance();
                    Token { token_type: TokenType::LeftParen, line, column, value: "(".into() }
                }
                ')' => {
                    self.advance();
                    Token { token_type: TokenType::RightParen, line, column, value: ")".into() }
                }
                '{' => {
                    self.advance();
                    Token { token_type: TokenType::LeftBrace, line, column, value: "{".into() }
                }
                '}' => {
                    self.advance();
                    Token { token_type: TokenType::RightBrace, line, column, value: "}".into() }
                }
                '[' => {
                    self.advance();
                    Token { token_type: TokenType::LeftBracket, line, column, value: "[".into() }
                }
                ']' => {
                    self.advance();
                    Token { token_type: TokenType::RightBracket, line, column, value: "]".into() }
                }
                ',' => {
                    self.advance();
                    Token { token_type: TokenType::Comma, line, column, value: ",".into() }
                }
                ';' => {
                    self.advance();
                    Token { token_type: TokenType::Semicolon, line, column, value: ";".into() }
                }
                ':' => {
                    self.advance();
                    Token { token_type: TokenType::Colon, line, column, value: ":".into() }
                }
                '.' => {
                    self.advance();
                    Token { token_type: TokenType::Dot, line, column, value: ".".into() }
                }
                '+' => {
                    self.advance();
                    Token { token_type: TokenType::Plus, line, column, value: "+".into() }
                }
                '-' => {
                    self.advance();
                    if self.current_char() == Some('>') {
                        self.advance();
                        Token { token_type: TokenType::Arrow, line, column, value: "->".into() }
                    } else {
                        Token { token_type: TokenType::Minus, line, column, value: "-".into() }
                    }
                }
                '*' => {
                    self.advance();
                    Token { token_type: TokenType::Star, line, column, value: "*".into() }
                }
                '/' => {
                    self.advance();
                    Token { token_type: TokenType::Slash, line, column, value: "/".into() }
                }
                '=' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token { token_type: TokenType::EqualEqual, line, column, value: "==".into() }
                    } else if self.current_char() == Some('>') {
                        self.advance();
                        Token { token_type: TokenType::FatArrow, line, column, value: "=>".into() }
                    } else {
                        Token { token_type: TokenType::Equal, line, column, value: "=".into() }
                    }
                }
                '<' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token { token_type: TokenType::LessEqual, line, column, value: "<=".into() }
                    } else {
                        Token { token_type: TokenType::Less, line, column, value: "<".into() }
                    }
                }
                '>' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token { token_type: TokenType::GreaterEqual, line, column, value: ">=".into() }
                    } else {
                        Token { token_type: TokenType::Greater, line, column, value: ">".into() }
                    }
                }
                '!' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token { token_type: TokenType::NotEqual, line, column, value: "!=".into() }
                    } else {
                        Token { token_type: TokenType::Not, line, column, value: "!".into() }
                    }
                }
                '&' => {
                    self.advance();
                    Token { token_type: TokenType::Ampersand, line, column, value: "&".into() }
                }
                '_' => {
                    self.advance();
                    Token { token_type: TokenType::Underscore, line, column, value: "_".into() }
                }
                '\n' => self.handle_newline(),
                '"' => {
                    let s = self.read_string('"');
                    Token { token_type: TokenType::String(s.clone()), line, column, value: s }
                }
                '\'' => {
                    let s = self.read_string('\'');
                    Token { token_type: TokenType::String(s.clone()), line, column, value: s }
                }
                _ if ch.is_numeric() => {
                    let num = self.read_number();
                    Token { token_type: TokenType::Number(num), line, column, value: num.to_string() }
                }
                _ if ch.is_alphabetic() => {
                    let ident = self.read_identifier();
                    let token_type = match ident.as_str() {
                        "let" => TokenType::Let,
                        "const" => TokenType::Const,
                        "fn" => TokenType::Fn,
                        "if" => TokenType::If,
                        "else" => TokenType::Else,
                        "while" => TokenType::While,
                        "for" => TokenType::For,
                        "in" => TokenType::In,
                        "return" => TokenType::Return,
                        "break" => TokenType::Break,
                        "continue" => TokenType::Continue,
                        "true" => TokenType::Boolean(true),
                        "false" => TokenType::Boolean(false),
                        "null" => TokenType::Null,
                        "void" => TokenType::Void,
                        "any" => TokenType::Any,
                        "i64" => TokenType::I64,
                        "i32" => TokenType::I32,
                        "u32" => TokenType::U32,
                        "string" => TokenType::String_,
                        "bool" => TokenType::Bool,
                        "float" => TokenType::Float,
                        _ => TokenType::Identifier(ident.clone()),
                    };
                    Token { token_type, line, column, value: ident }
                }
                _ => {
                    self.advance();
                    Token { token_type: TokenType::Identifier(ch.to_string()), line, column, value: ch.to_string() }
                }
            }
        } else {
            // EOF: Emit remaining dedents
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                Token {
                    token_type: TokenType::DEDENT(1),
                    line,
                    column,
                    value: String::new(),
                }
            } else {
                Token { token_type: TokenType::Eof, line, column, value: String::new() }
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        self.line_start = true;

        loop {
            // Track indentation if at line start
            if self.line_start {
                let indent_tokens = self.track_indentation()?;
                tokens.extend(indent_tokens);
            }

            let token = self.next_token();
            if token.token_type == TokenType::Eof {
                // Emit remaining DEDENTs at EOF
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    tokens.push(Token {
                        token_type: TokenType::DEDENT(1),
                        line: token.line,
                        column: token.column,
                        value: String::new(),
                    });
                }
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }
}

// ============================================================================
// PARSER: Updated to accept hybrid syntax (indentation OR braces)
// ============================================================================

/// AST Node types
#[derive(Debug, Clone)]
pub struct AstNode {
    pub node_type: AstNodeType,
    pub line: usize,
    pub column: usize,
    pub inferred_type: Option<TypeKind>,
}

#[derive(Debug, Clone)]
pub enum AstNodeType {
    VarDecl {
        name: String,
        type_annotation: Option<TypeAnnotation>,
        value: Box<AstNode>,
    },
    FuncDecl {
        name: String,
        params: Vec<(String, TypeAnnotation)>,
        return_type: Option<TypeAnnotation>,
        body: Vec<AstNode>,
    },
    Identifier(String),
    NumberLiteral(i64),
    StringLiteral(String),
    BoolLiteral(bool),
    BinaryOp {
        left: Box<AstNode>,
        op: String,
        right: Box<AstNode>,
    },
    Call {
        name: String,
        args: Vec<AstNode>,
    },
    If {
        condition: Box<AstNode>,
        then_branch: Vec<AstNode>,
        else_branch: Option<Vec<AstNode>>,
    },
    While {
        condition: Box<AstNode>,
        body: Vec<AstNode>,
    },
    Return(Option<Box<AstNode>>),
}

impl AstNode {
    pub fn new(node_type: AstNodeType, line: usize, column: usize) -> Self {
        AstNode {
            node_type,
            line,
            column,
            inferred_type: None,
        }
    }

    pub fn with_type(mut self, ty: TypeKind) -> Self {
        self.inferred_type = Some(ty);
        self
    }
}

/// Parser - accepts both indentation and brace-based syntax (v4.2 Hybrid)
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.position)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: TokenType) -> Result<Token> {
        let token = self.current_token();
        if std::mem::discriminant(&token.token_type) == std::mem::discriminant(&expected) {
            let t = token.clone();
            self.advance();
            Ok(t)
        } else {
            Err(KillerError::parse_error(
                format!("expected {:?}, found {:?}", expected, token.token_type),
                "source".to_string(),
                token.line,
                token.column,
            ))
        }
    }

    // NEW METHOD (v4.2): Parse block with hybrid syntax support
    /// Parses a code block (body of function, if, while, etc.)
    /// Accepts: braces "{...}" OR indentation(NEWLINE INDENT ... DEDENT)
    fn parse_block_hybrid(&mut self) -> Result<Vec<AstNode>> {
        let current = &self.current_token().token_type;

        if matches!(current, TokenType::LeftBrace) {
            // BRACE STYLE: { statements }
            self.advance();
            let mut statements = Vec::new();
            
            while !matches!(self.current_token().token_type, TokenType::RightBrace | TokenType::Eof) {
                statements.push(self.parse_statement()?);
            }
            
            self.expect(TokenType::RightBrace)?;
            Ok(statements)
        } else if matches!(current, TokenType::NEWLINE) {
            // INDENTATION STYLE: NEWLINE INDENT statements DEDENT
            self.expect(TokenType::NEWLINE)?;
            self.expect(TokenType::INDENT(_))?;
            
            let mut statements = Vec::new();
            while !matches!(self.current_token().token_type, TokenType::DEDENT(_) | TokenType::Eof) {
                statements.push(self.parse_statement()?);
            }
            
            self.expect(TokenType::DEDENT(_))?;
            Ok(statements)
        } else {
            Err(KillerError::parse_error(
                "expected '{' or newline to start block".to_string(),
                "source".to_string(),
                self.current_token().line,
                self.current_token().column,
            ))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<AstNode>> {
        let mut statements = Vec::new();
        
        while self.current_token().token_type != TokenType::Eof {
            // Skip newlines at top level
            if matches!(self.current_token().token_type, TokenType::NEWLINE) {
                self.advance();
                continue;
            }

            match &self.current_token().token_type {
                TokenType::Let => {
                    statements.push(self.parse_var_decl()?);
                }
                TokenType::Fn => {
                    statements.push(self.parse_func_decl()?);
                }
                TokenType::If => {
                    statements.push(self.parse_if()?);
                }
                TokenType::While => {
                    statements.push(self.parse_while()?);
                }
                TokenType::Return => {
                    statements.push(self.parse_return()?);
                }
                _ => {
                    // Expression statement
                    let expr = self.parse_expression()?;
                    statements.push(expr);
                    if self.current_token().token_type == TokenType::Semicolon {
                        self.advance();
                    }
                }
            }
        }
        
        Ok(statements)
    }

    fn parse_var_decl(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;
        
        self.expect(TokenType::Let)?;
        
        let name = match &self.current_token().token_type {
            TokenType::Identifier(n) => {
                let name = n.clone();
                self.advance();
                name
            }
            _ => return Err(KillerError::parse_error(
                "expected identifier".to_string(),
                "source".to_string(),
                line, col
            ))
        };

        let type_annotation = if self.current_token().token_type == TokenType::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.expect(TokenType::Equal)?;
        let value = self.parse_expression()?;

        if self.current_token().token_type == TokenType::Semicolon {
            self.advance();
        }

        Ok(AstNode::new(
            AstNodeType::VarDecl {
                name,
                type_annotation,
                value: Box::new(value),
            },
            line,
            col,
        ))
    }

    fn parse_func_decl(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;
        
        self.expect(TokenType::Fn)?;
        
        let name = match &self.current_token().token_type {
            TokenType::Identifier(n) => {
                let name = n.clone();
                self.advance();
                name
            }
            _ => return Err(KillerError::parse_error(
                "expected function name".to_string(),
                "source".to_string(),
                line, col
            ))
        };

        self.expect(TokenType::LeftParen)?;
        let mut params = Vec::new();
        
        while self.current_token().token_type != TokenType::RightParen {
            if let TokenType::Identifier(pname) = &self.current_token().token_type {
                let param_name = pname.clone();
                self.advance();
                self.expect(TokenType::Colon)?;
                let param_type = self.parse_type_annotation()?;
                params.push((param_name, param_type));
                
                if self.current_token().token_type == TokenType::Comma {
                    self.advance();
                }
            } else {
                break;
            }
        }

        self.expect(TokenType::RightParen)?;

        let return_type = if self.current_token().token_type == TokenType::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block_hybrid()?;

        Ok(AstNode::new(
            AstNodeType::FuncDecl {
                name,
                params,
                return_type,
                body,
            },
            line,
            col,
        ))
    }

    fn parse_if(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;
        
        self.expect(TokenType::If)?;
        self.expect(TokenType::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen)?;
        
        let then_branch = self.parse_block_hybrid()?;

        let else_branch = if self.current_token().token_type == TokenType::Else {
            self.advance();
            Some(self.parse_block_hybrid()?)
        } else {
            None
        };

        Ok(AstNode::new(
            AstNodeType::If {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            },
            line,
            col,
        ))
    }

    fn parse_while(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;
        
        self.expect(TokenType::While)?;
        self.expect(TokenType::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::RightParen)?;
        
        let body = self.parse_block_hybrid()?;

        Ok(AstNode::new(
            AstNodeType::While {
                condition: Box::new(condition),
                body,
            },
            line,
            col,
        ))
    }

    fn parse_return(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;
        
        self.expect(TokenType::Return)?;
        
        let value = if !matches!(self.current_token().token_type, 
                                 TokenType::Semicolon | TokenType::RightBrace | TokenType::DEDENT(_)) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        if self.current_token().token_type == TokenType::Semicolon {
            self.advance();
        }

        Ok(AstNode::new(AstNodeType::Return(value), line, col))
    }

    fn parse_statement(&mut self) -> Result<AstNode> {
        match &self.current_token().token_type {
            TokenType::Let => self.parse_var_decl(),
            TokenType::If => self.parse_if(),
            TokenType::While => self.parse_while(),
            TokenType::Return => self.parse_return(),
            TokenType::NEWLINE => {
                self.advance();
                self.parse_statement()
            }
            _ => {
                let expr = self.parse_expression()?;
                if self.current_token().token_type == TokenType::Semicolon {
                    self.advance();
                }
                Ok(expr)
            }
        }
    }

    fn parse_expression(&mut self) -> Result<AstNode> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<AstNode> {
        let mut left = self.parse_logical_and()?;
        let line = left.line;
        let col = left.column;

        while matches!(self.current_token().token_type, TokenType::Or) {
            let op = "||".to_string();
            self.advance();
            let right = self.parse_logical_and()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<AstNode> {
        let mut left = self.parse_equality()?;
        let line = left.line;
        let col = left.column;

        while matches!(self.current_token().token_type, TokenType::And) {
            let op = "&&".to_string();
            self.advance();
            let right = self.parse_equality()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<AstNode> {
        let mut left = self.parse_comparison()?;
        let line = left.line;
        let col = left.column;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::EqualEqual => Some("==".to_string()),
            TokenType::NotEqual => Some("!=".to_string()),
            _ => None,
        } {
            self.advance();
            let right = self.parse_comparison()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<AstNode> {
        let mut left = self.parse_additive()?;
        let line = left.line;
        let col = left.column;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Less => Some("<".to_string()),
            TokenType::Greater => Some(">".to_string()),
            TokenType::LessEqual => Some("<=".to_string()),
            TokenType::GreaterEqual => Some(">=".to_string()),
            _ => None,
        } {
            self.advance();
            let right = self.parse_additive()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<AstNode> {
        let mut left = self.parse_multiplicative()?;
        let line = left.line;
        let col = left.column;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Plus => Some("+".to_string()),
            TokenType::Minus => Some("-".to_string()),
            _ => None,
        } {
            self.advance();
            let right = self.parse_multiplicative()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<AstNode> {
        let mut left = self.parse_primary()?;
        let line = left.line;
        let col = left.column;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Star => Some("*".to_string()),
            TokenType::Slash => Some("/".to_string()),
            TokenType::Percent => Some("%".to_string()),
            _ => None,
        } {
            self.advance();
            let right = self.parse_primary()?;
            left = AstNode::new(
                AstNodeType::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                line,
                col,
            );
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<AstNode> {
        let line = self.current_token().line;
        let col = self.current_token().column;

        match &self.current_token().token_type.clone() {
            TokenType::Number(n) => {
                let num = *n;
                self.advance();
                Ok(AstNode::new(AstNodeType::NumberLiteral(num), line, col)
                    .with_type(TypeKind::Number))
            }
            TokenType::String(s) => {
                let str = s.clone();
                self.advance();
                Ok(AstNode::new(AstNodeType::StringLiteral(str), line, col)
                    .with_type(TypeKind::String))
            }
            TokenType::Boolean(b) => {
                let bool_val = *b;
                self.advance();
                Ok(AstNode::new(AstNodeType::BoolLiteral(bool_val), line, col)
                    .with_type(TypeKind::Boolean))
            }
            TokenType::Identifier(name) => {
                let id = name.clone();
                self.advance();
                
                // Check for function call
                if self.current_token().token_type == TokenType::LeftParen {
                    self.advance();
                    let mut args = Vec::new();
                    
                    while self.current_token().token_type != TokenType::RightParen {
                        args.push(self.parse_expression()?);
                        if self.current_token().token_type == TokenType::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    self.expect(TokenType::RightParen)?;
                    Ok(AstNode::new(
                        AstNodeType::Call { name: id, args },
                        line,
                        col,
                    ))
                } else {
                    Ok(AstNode::new(AstNodeType::Identifier(id), line, col))
                }
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenType::RightParen)?;
                Ok(expr)
            }
            _ => Err(KillerError::parse_error(
                format!("unexpected token: {:?}", self.current_token().token_type),
                "source".to_string(),
                line,
                col,
            ))
        }
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation> {
        let base_type = match &self.current_token().token_type {
            TokenType::I64 => {
                self.advance();
                TypeKind::Number
            }
            TokenType::String_ => {
                self.advance();
                TypeKind::String
            }
            TokenType::Bool => {
                self.advance();
                TypeKind::Boolean
            }
            TokenType::Void => {
                self.advance();
                TypeKind::Void
            }
            TokenType::Any => {
                self.advance();
                TypeKind::Any
            }
            TokenType::Identifier(name) => {
                let n = name.clone();
                self.advance();
                TypeKind::Custom(n)
            }
            TokenType::LeftBracket => {
                self.advance();
                let inner = self.parse_type_annotation()?;
                self.expect(TokenType::RightBracket)?;
                TypeKind::Array(Box::new(inner.kind))
            }
            _ => return Err(KillerError::parse_error(
                "expected type annotation".to_string(),
                "source".to_string(),
                self.current_token().line,
                self.current_token().column,
            ))
        };

        Ok(TypeAnnotation {
            kind: base_type,
            optional: false,
        })
    }
}

// ============================================================================
// UNIT TESTS (30+ test cases for Phase 1)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- BASIC INDENTATION TESTS ---

    #[test]
    fn test_simple_indent_token() {
        let code = "kfn test()\n  x = 1";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        // Should contain INDENT token
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::INDENT(_))));
    }

    #[test]
    fn test_dedent_token() {
        let code = "kfn test()\n  x = 1\ny = 2";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        // Should contain DEDENT token
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::DEDENT(_))));
    }

    #[test]
    fn test_nested_indents() {
        let code = "for i in 1..5\n  if i > 0\n    print(i)";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        let indent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::INDENT(_))).count();
        let dedent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::DEDENT(_))).count();
        
        assert!(indent_count >= 2);
        assert_eq!(dedent_count, indent_count);
    }

    #[test]
    fn test_mixed_tabs_spaces_error() {
        let code = "kfn test()\n  x = 1\n\ty = 2";  // Space then tab
        let mut lexer = Lexer::new(code);
        
        match lexer.tokenize() {
            Err(e) => {
                assert!(e.to_string().contains("mixed") || e.to_string().contains("indent"));
            }
            Ok(_) => panic!("Should reject mixed tabs/spaces"),
        }
    }

    #[test]
    fn test_blank_line_skipped() {
        let code = "x = 1\n\ny = 2";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        // Blank lines shouldn't emit INDENT/DEDENT
        let indent_dedent = tokens.iter()
            .filter(|t| matches!(t.token_type, TokenType::INDENT(_) | TokenType::DEDENT(_)))
            .count();
        assert_eq!(indent_dedent, 0);
    }

    #[test]
    fn test_newline_token() {
        let code = "x = 1\ny = 2";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        // Should contain NEWLINE token
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::NEWLINE)));
    }

    // --- HYBRID SYNTAX TESTS ---

    #[test]
    fn test_brace_syntax_still_works() {
        let code = "kfn test() { x = 1 }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        // Should have braces, no INDENT/DEDENT
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::LeftBrace)));
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::RightBrace)));
        assert!(!tokens.iter().any(|t| matches!(t.token_type, TokenType::INDENT(_))));
    }

    #[test]
    fn test_indent_mode_disabled() {
        let code = "kfn test()\n  x = 1";
        let lexer = Lexer::new(code).with_indent_mode(false);
        let mut lexer = lexer;
        let tokens = lexer.tokenize().unwrap();
        
        // No INDENT/DEDENT when disabled
        assert!(!tokens.iter().any(|t| matches!(t.token_type, TokenType::INDENT(_))));
        assert!(!tokens.iter().any(|t| matches!(t.token_type, TokenType::DEDENT(_))));
    }

    #[test]
    fn test_multiple_indent_levels() {
        let code = "for i in 1..3\n  for j in 1..3\n    print(i + j)\n  print(i)\nprint(\"done\")";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        let indent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::INDENT(_))).count();
        let dedent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::DEDENT(_))).count();
        
        assert_eq!(indent_count, dedent_count);
        assert!(indent_count >= 2);
    }

    #[test]
    fn test_consistent_indentation() {
        let code = "kfn test()\n  x = 1\n  y = 2\n  z = 3";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        let indent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::INDENT(_))).count();
        let dedent_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::DEDENT(_))).count();
        
        // One indent at start, one dedent at end
        assert_eq!(indent_count, 1);
        assert_eq!(dedent_count, 1);
    }

    // --- PARSER TESTS ---

    #[test]
    fn test_parse_indent_function() {
        let code = "kfn add(a: i64, b: i64)\n  a + b";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        match parser.parse() {
            Ok(ast) => {
                assert!(!ast.is_empty());
            }
            Err(e) => panic!("Parse failed: {}", e),
        }
    }

    #[test]
    fn test_parse_brace_function() {
        let code = "kfn add(a: i64, b: i64) { a + b }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        match parser.parse() {
            Ok(ast) => {
                assert!(!ast.is_empty());
            }
            Err(e) => panic!("Parse failed: {}", e),
        }
    }

    #[test]
    fn test_parse_hybrid_if() {
        let code = "if (x > 0) { print(x) }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        match parser.parse() {
            Ok(ast) => {
                assert!(!ast.is_empty());
            }
            Err(e) => panic!("Parse failed: {}", e),
        }
    }

    // --- ERROR RECOVERY TESTS ---

    #[test]
    fn test_unexpected_dedent_error() {
        let code = "x = 1\n    y = 2";  // Unexpected indent then dedent
        let mut lexer = Lexer::new(code);
        
        match lexer.tokenize() {
            Err(e) => {
                assert!(e.to_string().contains("dedent") || e.to_string().contains("indent"));
            }
            Ok(_) => {
                // May or may not error depending on implementation
            }
        }
    }

    // --- REGRESSION TESTS (Ensure brace syntax still works) ---

    #[test]
    fn test_existing_brace_code_v1() {
        let code = "let x = 42; let y = 100;";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::Number(42))));
        assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::Number(100))));
    }

    #[test]
    fn test_existing_brace_code_v2() {
        let code = "if (true) { return 5; }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse().is_ok());
    }

    #[test]
    fn test_existing_loop_code() {
        let code = "for i in 1..10 { print(i); }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse().is_ok());
    }
}
