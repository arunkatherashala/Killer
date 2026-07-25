#[cfg(test)]
use crate::ghost_vm;

// ── Tokens ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Let,
    If,
    Else,
    While,
    Fn,
    Return,
    Print,
    Ident(String),
    Number(i64),
    FloatLit(f64),
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Greater,
    Less,
    GreaterEq,
    LessEq,
    EqEq,
    NotEq,
    Semicolon,
    Comma,
    Eof,
}

// ── Lexer ───────────────────────────────────────────────────────────────────

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer { chars: source.chars().collect(), pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().map_or(false, |c| c.is_whitespace()) {
                self.advance();
            }
            if self.pos + 1 < self.chars.len()
                && self.chars[self.pos] == '/'
                && self.chars[self.pos + 1] == '/'
            {
                while self.peek().map_or(false, |c| c != '\n') {
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let c = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push(Token::Eof);
                    return Ok(tokens);
                }
            };
            match c {
                '+' => { self.advance(); tokens.push(Token::Plus); }
                '-' => { self.advance(); tokens.push(Token::Minus); }
                '*' => { self.advance(); tokens.push(Token::Star); }
                '/' => { self.advance(); tokens.push(Token::Slash); }
                '%' => { self.advance(); tokens.push(Token::Percent); }
                '(' => { self.advance(); tokens.push(Token::LParen); }
                ')' => { self.advance(); tokens.push(Token::RParen); }
                '{' => { self.advance(); tokens.push(Token::LBrace); }
                '}' => { self.advance(); tokens.push(Token::RBrace); }
                ';' => { self.advance(); tokens.push(Token::Semicolon); }
                ',' => { self.advance(); tokens.push(Token::Comma); }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::EqEq);
                    } else {
                        tokens.push(Token::Equals);
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::NotEq);
                    } else {
                        return Err("unexpected character '!'".to_string());
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::GreaterEq);
                    } else {
                        tokens.push(Token::Greater);
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::LessEq);
                    } else {
                        tokens.push(Token::Less);
                    }
                }
                c if c.is_ascii_digit() => {
                    let mut num_str = String::new();
                    let mut is_float = false;
                    while self.peek().map_or(false, |ch| ch.is_ascii_digit() || ch == '.') {
                        let ch = self.advance().unwrap();
                        if ch == '.' {
                            is_float = true;
                        }
                        num_str.push(ch);
                    }
                    if is_float {
                        let f: f64 = num_str.parse().map_err(|_| format!("invalid float: {num_str}"))?;
                        tokens.push(Token::FloatLit(f));
                    } else {
                        let n: i64 = num_str.parse().map_err(|_| format!("invalid number: {num_str}"))?;
                        tokens.push(Token::Number(n));
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while self.peek().map_or(false, |ch| ch.is_ascii_alphanumeric() || ch == '_') {
                        ident.push(self.advance().unwrap());
                    }
                    tokens.push(match ident.as_str() {
                        "let" => Token::Let,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "while" => Token::While,
                        "fn" => Token::Fn,
                        "return" => Token::Return,
                        "print" => Token::Print,
                        _ => Token::Ident(ident),
                    });
                }
                other => return Err(format!("unexpected character '{other}'")),
            }
        }
    }
}

// ── AST ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    BinOp { op: BinOpKind, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    Ident(String),
    NumberLit(i64),
    FloatLit(f64),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Mod, Eq, Lt, Gt, LtEq, GtEq, NotEq,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    LetStmt { name: String, expr: Expr },
    AssignStmt { name: String, expr: Expr },
    PrintStmt(Expr),
    IfStmt { cond: Expr, body: Vec<Stmt>, else_body: Option<Vec<Stmt>> },
    WhileStmt { cond: Expr, body: Vec<Stmt> },
    FnDecl { name: String, params: Vec<String>, body: Vec<Stmt> },
    ReturnStmt(Expr),
    ExprStmt(Expr),
}

// ── Parser ──────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {tok:?}"))
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Fn => self.parse_fn_decl(),
            Token::Return => self.parse_return(),
            Token::Print => self.parse_print(),
            Token::Ident(_) => {
                if self.pos + 1 < self.tokens.len() && self.tokens[self.pos + 1] == Token::Equals {
                    self.parse_assign()
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    Ok(Stmt::ExprStmt(expr))
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name = match self.advance() {
            Token::Ident(n) => n,
            t => return Err(format!("expected identifier after 'let', got {t:?}")),
        };
        self.expect(&Token::Equals)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::LetStmt { name, expr })
    }

    fn parse_assign(&mut self) -> Result<Stmt, String> {
        let name = match self.advance() {
            Token::Ident(n) => n,
            t => return Err(format!("expected identifier, got {t:?}")),
        };
        self.expect(&Token::Equals)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::AssignStmt { name, expr })
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        self.advance();
        self.expect(&Token::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::PrintStmt(expr))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance();
        let cond = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        let else_body = if *self.peek() == Token::Else {
            self.advance();
            self.expect(&Token::LBrace)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::IfStmt { cond, body, else_body })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance();
        let cond = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Stmt::WhileStmt { cond, body })
    }

    fn parse_fn_decl(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name = match self.advance() {
            Token::Ident(n) => n,
            t => return Err(format!("expected function name, got {t:?}")),
        };
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        if *self.peek() != Token::RParen {
            loop {
                match self.advance() {
                    Token::Ident(p) => params.push(p),
                    t => return Err(format!("expected parameter name, got {t:?}")),
                }
                if *self.peek() == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Stmt::FnDecl { name, params, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.advance();
        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::ReturnStmt(expr))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace {
            if *self.peek() == Token::Eof {
                return Err("unexpected end of input, expected '}'".to_string());
            }
            stmts.push(self.parse_stmt()?);
        }
        self.advance();
        Ok(stmts)
    }

    // Precedence: comparison < addition < multiplication < unary < primary
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match self.peek() {
                Token::Greater => BinOpKind::Gt,
                Token::Less => BinOpKind::Lt,
                Token::GreaterEq => BinOpKind::GtEq,
                Token::LessEq => BinOpKind::LtEq,
                Token::EqEq => BinOpKind::Eq,
                Token::NotEq => BinOpKind::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOpKind::Add,
                Token::Minus => BinOpKind::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOpKind::Mul,
                Token::Slash => BinOpKind::Div,
                Token::Percent => BinOpKind::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if *self.peek() == Token::Minus {
            self.advance();
            let expr = self.parse_primary()?;
            return Ok(Expr::BinOp {
                op: BinOpKind::Sub,
                left: Box::new(Expr::NumberLit(0)),
                right: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Number(n) => { self.advance(); Ok(Expr::NumberLit(n)) }
            Token::FloatLit(f) => { self.advance(); Ok(Expr::FloatLit(f)) }
            Token::Ident(name) => {
                self.advance();
                if *self.peek() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.peek() != Token::RParen {
                        loop {
                            args.push(self.parse_expr()?);
                            if *self.peek() == Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            t => Err(format!("unexpected token in expression: {t:?}")),
        }
    }
}

// ── Codegen ─────────────────────────────────────────────────────────────────

struct FnInfo {
    params: Vec<String>,
    body: Vec<Stmt>,
    param_addrs: Vec<u16>,
}

struct Codegen {
    output: Vec<String>,
    vars: std::collections::HashMap<String, u16>,
    next_addr: u16,
    label_counter: usize,
    functions: std::collections::HashMap<String, FnInfo>,
    return_label: Option<String>,
}

impl Codegen {
    fn new() -> Self {
        Codegen {
            output: Vec::new(),
            vars: std::collections::HashMap::new(),
            next_addr: 0,
            label_counter: 0,
            functions: std::collections::HashMap::new(),
            return_label: None,
        }
    }

    fn alloc_var(&mut self, name: &str) -> u16 {
        let addr = self.next_addr;
        self.vars.insert(name.to_string(), addr);
        self.next_addr += 8;
        addr
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("__{prefix}_{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    fn emit(&mut self, line: &str) {
        self.output.push(line.to_string());
    }

    fn compile_program(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
        self.emit("halt");
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::LetStmt { name, expr } => {
                self.compile_expr(expr)?;
                let addr = self.alloc_var(name);
                self.emit(&format!("store {addr}"));
            }
            Stmt::AssignStmt { name, expr } => {
                self.compile_expr(expr)?;
                let addr = *self.vars.get(name)
                    .ok_or_else(|| format!("undefined variable: {name}"))?;
                self.emit(&format!("store {addr}"));
            }
            Stmt::PrintStmt(expr) => {
                self.compile_expr(expr)?;
                self.emit("syscall 3");
            }
            Stmt::IfStmt { cond, body, else_body } => {
                let then_label = self.fresh_label("then");
                let end_label = self.fresh_label("endif");

                self.compile_expr(cond)?;
                self.emit(&format!("jmpif {then_label}"));

                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.compile_stmt(s)?;
                    }
                }
                self.emit(&format!("jmp {end_label}"));
                self.emit(&format!("{then_label}:"));
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.emit(&format!("{end_label}:"));
            }
            Stmt::WhileStmt { cond, body } => {
                let start_label = self.fresh_label("wloop");
                let body_label = self.fresh_label("wbody");
                let end_label = self.fresh_label("wend");

                self.emit(&format!("{start_label}:"));
                self.compile_expr(cond)?;
                self.emit(&format!("jmpif {body_label}"));
                self.emit(&format!("jmp {end_label}"));
                self.emit(&format!("{body_label}:"));
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.emit(&format!("jmp {start_label}"));
                self.emit(&format!("{end_label}:"));
            }
            Stmt::FnDecl { name, params, body } => {
                let mut param_addrs = Vec::new();
                for _p in params {
                    let addr = self.next_addr;
                    self.next_addr += 8;
                    param_addrs.push(addr);
                }
                self.functions.insert(name.clone(), FnInfo {
                    params: params.clone(),
                    body: body.clone(),
                    param_addrs,
                });
            }
            Stmt::ReturnStmt(expr) => {
                self.compile_expr(expr)?;
                if let Some(label) = &self.return_label {
                    let label = label.clone();
                    self.emit(&format!("jmp {label}"));
                }
            }
            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
                self.emit("pop");
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::NumberLit(n) => {
                self.emit(&format!("push {n}"));
            }
            Expr::FloatLit(f) => {
                self.emit(&format!("fconst {f}"));
            }
            Expr::Ident(name) => {
                let addr = *self.vars.get(name)
                    .ok_or_else(|| format!("undefined variable: {name}"))?;
                self.emit(&format!("load {addr}"));
            }
            Expr::BinOp { op, left, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinOpKind::Add => self.emit("add"),
                    BinOpKind::Sub => self.emit("sub"),
                    BinOpKind::Mul => self.emit("mul"),
                    BinOpKind::Div => self.emit("div"),
                    BinOpKind::Mod => self.emit("mod"),
                    BinOpKind::Eq  => self.emit("eq"),
                    BinOpKind::Lt  => self.emit("lt"),
                    BinOpKind::Gt  => self.emit("gt"),
                    BinOpKind::GtEq => {
                        // a >= b  ↔  NOT (a < b)
                        self.emit("lt");
                        self.emit("push 0");
                        self.emit("eq");
                    }
                    BinOpKind::LtEq => {
                        // a <= b  ↔  NOT (a > b)
                        self.emit("gt");
                        self.emit("push 0");
                        self.emit("eq");
                    }
                    BinOpKind::NotEq => {
                        // a != b  ↔  NOT (a == b)
                        self.emit("eq");
                        self.emit("push 0");
                        self.emit("eq");
                    }
                }
            }
            Expr::Call { name, args } => {
                let fn_params;
                let fn_body;
                let fn_param_addrs;
                {
                    let info = self.functions.get(name)
                        .ok_or_else(|| format!("undefined function: {name}"))?;
                    if args.len() != info.params.len() {
                        return Err(format!(
                            "function '{}' expects {} args, got {}",
                            name, info.params.len(), args.len()
                        ));
                    }
                    fn_params = info.params.clone();
                    fn_body = info.body.clone();
                    fn_param_addrs = info.param_addrs.clone();
                }

                for (i, arg) in args.iter().enumerate() {
                    self.compile_expr(arg)?;
                    self.emit(&format!("store {}", fn_param_addrs[i]));
                }

                let saved_vars = self.vars.clone();
                for (i, param_name) in fn_params.iter().enumerate() {
                    self.vars.insert(param_name.clone(), fn_param_addrs[i]);
                }

                let ret_label = self.fresh_label("ret");
                let saved_return = self.return_label.take();
                self.return_label = Some(ret_label.clone());

                for s in &fn_body {
                    self.compile_stmt(s)?;
                }

                self.emit(&format!("{ret_label}:"));

                self.return_label = saved_return;
                self.vars = saved_vars;
            }
        }
        Ok(())
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

pub fn compile_ghost_lang(source: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    let mut codegen = Codegen::new();
    codegen.compile_program(&program)?;
    Ok(codegen.output.join("\n"))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gl_let_print() {
        let asm = compile_ghost_lang("let x = 42; print(x);").unwrap();
        assert!(asm.contains("push 42"), "missing push 42:\n{asm}");
        assert!(asm.contains("store"), "missing store:\n{asm}");
        assert!(asm.contains("load"), "missing load:\n{asm}");
        assert!(asm.contains("syscall 3"), "missing syscall 3:\n{asm}");
    }

    #[test]
    fn test_gl_arithmetic() {
        let asm = compile_ghost_lang("let a = 10; let b = 20; let c = a + b; print(c);").unwrap();
        assert!(asm.contains("push 10"), "missing push 10:\n{asm}");
        assert!(asm.contains("push 20"), "missing push 20:\n{asm}");
        assert!(asm.contains("add"), "missing add:\n{asm}");
        assert!(asm.contains("syscall 3"), "missing syscall 3:\n{asm}");
    }

    #[test]
    fn test_gl_while() {
        let asm = compile_ghost_lang("let i = 0; while i < 5 { i = i + 1; } print(i);").unwrap();
        assert!(asm.contains("jmp"), "missing jmp:\n{asm}");
        assert!(asm.contains("jmpif"), "missing jmpif:\n{asm}");
        assert!(asm.contains("lt"), "missing lt:\n{asm}");
    }

    #[test]
    fn test_gl_if() {
        let asm = compile_ghost_lang("let x = 10; if x > 5 { print(1); }").unwrap();
        assert!(asm.contains("gt"), "missing gt:\n{asm}");
        assert!(asm.contains("jmpif"), "missing jmpif:\n{asm}");
    }

    #[test]
    fn test_gl_function() {
        let asm = compile_ghost_lang(
            "fn double(n) { return n + n; } let r = double(21); print(r);"
        ).unwrap();
        assert!(asm.contains("push 21"), "missing push 21:\n{asm}");
        assert!(asm.contains("add"), "missing add:\n{asm}");
        assert!(asm.contains("syscall 3"), "missing syscall 3:\n{asm}");
    }

    #[test]
    fn test_gl_end_to_end() {
        let asm = compile_ghost_lang("let x = 42; print(x);").unwrap();
        let mut capsule = ghost_vm::assemble_capsule(&asm).expect("assemble failed");
        let mut host = ghost_vm::InteractiveHost::new();
        let status = ghost_vm::run(&mut capsule, &mut host, Some(1000)).expect("run failed");
        assert_eq!(status, ghost_vm::RunStatus::Halted);
        let output = String::from_utf8_lossy(&host.output_buffer);
        assert_eq!(output, "42\n", "expected '42\\n', got '{output}'");
    }

    #[test]
    fn test_gl_while_end_to_end() {
        let asm = compile_ghost_lang(
            "let i = 0; while i < 5 { i = i + 1; } print(i);"
        ).unwrap();
        let mut capsule = ghost_vm::assemble_capsule(&asm).expect("assemble failed");
        let mut host = ghost_vm::InteractiveHost::new();
        let status = ghost_vm::run(&mut capsule, &mut host, Some(10_000)).expect("run failed");
        assert_eq!(status, ghost_vm::RunStatus::Halted);
        let output = String::from_utf8_lossy(&host.output_buffer);
        assert_eq!(output, "5\n", "expected '5\\n', got '{output}'");
    }
}
