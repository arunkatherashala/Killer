//! Text → [`crate::ast::Stmt`] parser (token-based).
//!
//! Uses [`crate::lexer::lex_with_newlines`] after [`crate::compiler::preprocess_killer_source`].
//! Covers common Killer syntax; extend here as the language grows.

use crate::ast::{BinaryOp, Expr, Pattern, Stmt};
use crate::error::VmError;
use crate::lexer::{lex_with_newlines, Token, TokenKind};

/// Preprocess, lex (with newlines), and parse a Killer program into AST statements.
pub fn parse_killer_program(source: &str) -> Result<Vec<Stmt>, VmError> {
    let pre = crate::compiler::preprocess_killer_source(source);
    let tokens = lex_with_newlines(&pre).map_err(|m| VmError::parse_error_simple(m))?;
    Parser::new(&tokens).parse_program()
}

/// Parse + [`crate::compiler::compile_killer_ast`] + VM run.
pub fn run_killer_parsed(source: &str) -> Result<(), VmError> {
    let stmts = parse_killer_program(source)?;
    let program = crate::compiler::compile_killer_ast(&stmts)?;
    let mut machine = crate::vm::VirtualMachine::new();
    machine.run(&program)
}

struct Parser<'a> {
    t: &'a [Token],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Parser { t: tokens, i: 0 }
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.t.get(self.i).map(|x| &x.kind)
    }

    fn advance(&mut self) {
        if self.i < self.t.len() {
            self.i += 1;
        }
    }

    fn eof(&self) -> bool {
        matches!(self.peek(), None | Some(TokenKind::Eof))
    }

    fn skip_noise(&mut self) {
        while matches!(
            self.peek(),
            Some(TokenKind::Newline) | Some(TokenKind::Semicolon)
        ) {
            self.advance();
        }
    }

    fn expect(&mut self, expected: &str, ok: bool) -> Result<(), VmError> {
        if ok {
            Ok(())
        } else {
            Err(VmError::parse_error_simple(format!(
                "stmt parser: expected {expected} at token {:?}",
                self.peek()
            )))
        }
    }

    fn parse_program(mut self) -> Result<Vec<Stmt>, VmError> {
        let mut out = Vec::new();
        self.skip_noise();
        while !self.eof() {
            self.parse_into(&mut out)?;
            self.skip_noise();
        }
        Ok(out)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, VmError> {
        let mut body = Vec::new();
        self.skip_noise();
        while !matches!(self.peek(), Some(TokenKind::RBrace) | None | Some(TokenKind::Eof)) {
            self.parse_into(&mut body)?;
            self.skip_noise();
        }
        Ok(body)
    }

    /// Dispatches a single "statement slot" which may emit 1+ stmts
    /// (`export fn …` emits both a Function and an Export).
    fn parse_into(&mut self, out: &mut Vec<Stmt>) -> Result<(), VmError> {
        self.skip_noise();
        if let Some(TokenKind::Identifier(name)) = self.peek() {
            if name == "export" {
                self.advance();
                return self.parse_export(out);
            }
        }
        out.push(self.parse_stmt()?);
        Ok(())
    }

    fn parse_stmt(&mut self) -> Result<Stmt, VmError> {
        self.skip_noise();
        match self.peek().cloned() {
            Some(TokenKind::Let) => self.parse_let(),
            Some(TokenKind::Print) => self.parse_print(),
            Some(TokenKind::If) => self.parse_if(),
            Some(TokenKind::While) => self.parse_while(),
            Some(TokenKind::Match) => self.parse_match(),
            Some(TokenKind::For) => self.parse_for(),
            Some(TokenKind::Return) => self.parse_return(),
            Some(TokenKind::Break) => {
                self.advance();
                Ok(Stmt::Break)
            }
            Some(TokenKind::Continue) => {
                self.advance();
                Ok(Stmt::Continue)
            }
            Some(TokenKind::Fn) => self.parse_function(false),
            Some(TokenKind::Async) => {
                self.advance();
                self.expect("fn/kfn after async", matches!(self.peek(), Some(TokenKind::Fn)))?;
                self.parse_function(true)
            }
            Some(TokenKind::Import) => self.parse_import(),
            Some(TokenKind::Spawn) => {
                self.advance();
                let e = self.parse_expr(0)?;
                Ok(Stmt::Spawn(Box::new(e)))
            }
            Some(TokenKind::Await) => {
                self.advance();
                let e = self.parse_expr(0)?;
                Ok(Stmt::Expr(Expr::Await(Box::new(e))))
            }
            Some(TokenKind::Try) => self.parse_try(),
            Some(TokenKind::Class) => self.parse_class(),
            Some(TokenKind::Throw) => {
                self.advance();
                let e = self.parse_expr(0)?;
                Ok(Stmt::Throw(Box::new(e)))
            }
            Some(TokenKind::Identifier(name)) => {
                let n = name.clone();
                if n == "from" {
                    self.advance();
                    return self.parse_from_import();
                }
                self.advance();
                self.parse_after_ident(n)
            }
            Some(_) => {
                let e = self.parse_expr(0)?;
                Ok(Stmt::Expr(e))
            }
            None => Err(VmError::parse_error_simple("stmt parser: unexpected end of input")),
        }
    }

    fn parse_after_ident(&mut self, name: String) -> Result<Stmt, VmError> {
        match self.peek() {
            Some(TokenKind::Equal) => {
                self.advance();
                let value = self.parse_expr(0)?;
                Ok(Stmt::Assign {
                    pattern: Pattern::Identifier(name),
                    value: Box::new(value),
                })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let args = self.parse_expr_list_until_rparen()?;
                let expr = Expr::Call { callee: name, args };
                let expr = self.parse_postfix(expr)?;
                Ok(Stmt::Expr(expr))
            }
            Some(TokenKind::Dot) | Some(TokenKind::LBracket) => {
                let lhs = self.parse_postfix(Expr::Identifier(name.clone()))?;
                if matches!(self.peek(), Some(TokenKind::Equal)) {
                    self.advance();
                    let value = self.parse_expr(0)?;
                    match lhs {
                        Expr::Index { object: _, index } => Ok(Stmt::IndexAssign {
                            object: name,
                            index: *index,
                            value,
                        }),
                        _ => Err(VmError::parse_error_simple("invalid assignment target")),
                    }
                } else {
                    Ok(Stmt::Expr(lhs))
                }
            }
            _ => {
                let expr = Expr::Identifier(name);
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, VmError> {
        self.advance(); // let
        let pat = match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                Pattern::Identifier(n)
            }
            _ => {
                return Err(VmError::parse_error_simple(
                    "stmt parser: let expects identifier",
                ))
            }
        };
        self.expect("=", matches!(self.peek(), Some(TokenKind::Equal)))?;
        self.advance();
        let value = self.parse_expr(0)?;
        Ok(Stmt::Let {
            pattern: pat,
            value: Box::new(value),
        })
    }

    fn parse_print(&mut self) -> Result<Stmt, VmError> {
        self.advance();
        self.expect("(", matches!(self.peek(), Some(TokenKind::LParen)))?;
        self.advance();
        let args = self.parse_expr_list_until_rparen()?;
        Ok(Stmt::Print(args))
    }

    fn parse_if(&mut self) -> Result<Stmt, VmError> {
        self.advance();
        let condition = Box::new(self.parse_expr(0)?);
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let then_branch = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        let else_branch = if matches!(self.peek(), Some(TokenKind::Else)) {
            self.advance();
            if matches!(self.peek(), Some(TokenKind::If)) {
                vec![self.parse_if()?]
            } else {
                self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
                self.advance();
                let b = self.parse_block()?;
                self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
                self.advance();
                b
            }
        } else {
            vec![]
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, VmError> {
        self.advance();
        let condition = Box::new(self.parse_expr(0)?);
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let body = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, VmError> {
        self.advance();
        let variable = match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return Err(VmError::parse_error_simple("for: expected variable")),
        };
        let is_for_of = if matches!(self.peek(), Some(TokenKind::Of)) {
            self.advance();
            true
        } else {
            self.expect("in", matches!(self.peek(), Some(TokenKind::In)))?;
            self.advance();
            false
        };
        let iterable = Box::new(self.parse_expr(0)?);
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let body = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        Ok(Stmt::For {
            variable,
            iterable,
            is_for_of,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, VmError> {
        self.advance();
        if matches!(
            self.peek(),
            Some(TokenKind::Newline) | Some(TokenKind::Semicolon) | Some(TokenKind::RBrace) | None | Some(TokenKind::Eof)
        ) {
            return Ok(Stmt::Return(None));
        }
        let e = self.parse_expr(0)?;
        Ok(Stmt::Return(Some(Box::new(e))))
    }

    fn parse_function(&mut self, is_async: bool) -> Result<Stmt, VmError> {
        self.advance(); // fn / kfn
        let name = match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return Err(VmError::parse_error_simple("function: expected name")),
        };
        self.expect("(", matches!(self.peek(), Some(TokenKind::LParen)))?;
        self.advance();
        let mut params = Vec::new();
        if !matches!(self.peek(), Some(TokenKind::RParen)) {
            loop {
                match self.peek() {
                    Some(TokenKind::Identifier(n)) => {
                        params.push(n.clone());
                        self.advance();
                    }
                    _ => {
                        return Err(VmError::parse_error_simple(
                            "function: expected parameter name",
                        ))
                    }
                }
                match self.peek() {
                    Some(TokenKind::Comma) => self.advance(),
                    Some(TokenKind::RParen) => break,
                    _ => {
                        return Err(VmError::parse_error_simple(
                            "function: expected , or ) in parameter list",
                        ))
                    }
                }
            }
        }
        self.expect(")", matches!(self.peek(), Some(TokenKind::RParen)))?;
        self.advance();
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let body = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        if is_async {
            Ok(Stmt::AsyncFunction {
                name,
                params,
                body,
            })
        } else {
            Ok(Stmt::Function {
                name,
                params,
                body,
                ai_annotations: vec![],
            })
        }
    }

    /// `from "path" import name1, name2` — `from` already consumed.
    fn parse_from_import(&mut self) -> Result<Stmt, VmError> {
        let path = match self.peek() {
            Some(TokenKind::String(s)) => {
                let s = s.clone();
                self.advance();
                s
            }
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => {
                return Err(VmError::parse_error_simple(
                    "from import: expected path string or module name after `from`",
                ))
            }
        };
        self.expect(
            "`import` after path in from-import",
            matches!(self.peek(), Some(TokenKind::Import)),
        )?;
        self.advance();
        let mut symbols = Vec::new();
        match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                symbols.push(n.clone());
                self.advance();
            }
            _ => {
                return Err(VmError::parse_error_simple(
                    "from import: expected at least one identifier after `import`",
                ))
            }
        }
        while matches!(self.peek(), Some(TokenKind::Comma)) {
            self.advance();
            match self.peek() {
                Some(TokenKind::Identifier(n)) => {
                    symbols.push(n.clone());
                    self.advance();
                }
                _ => {
                    return Err(VmError::parse_error_simple(
                        "from import: expected identifier after `,`",
                    ))
                }
            }
        }
        Ok(Stmt::Import {
            path,
            symbols: Some(symbols),
        })
    }

    fn parse_import(&mut self) -> Result<Stmt, VmError> {
        self.advance(); // consume `import`
        if matches!(self.peek(), Some(TokenKind::LBrace)) {
            // import { x, y } from "module"
            self.advance();
            let mut symbols = Vec::new();
            if !matches!(self.peek(), Some(TokenKind::RBrace)) {
                loop {
                    match self.peek() {
                        Some(TokenKind::Identifier(n)) => {
                            symbols.push(n.clone());
                            self.advance();
                        }
                        _ => {
                            return Err(VmError::parse_error_simple(
                                "import: expected identifier in symbol list",
                            ))
                        }
                    }
                    match self.peek() {
                        Some(TokenKind::Comma) => self.advance(),
                        Some(TokenKind::RBrace) => break,
                        _ => {
                            return Err(VmError::parse_error_simple(
                                "import: expected , or } in symbol list",
                            ))
                        }
                    }
                }
            }
            self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
            self.advance();
            match self.peek() {
                Some(TokenKind::Identifier(n)) if n == "from" => self.advance(),
                _ => {
                    return Err(VmError::parse_error_simple(
                        "import: expected `from` after { ... }",
                    ))
                }
            }
            let path = match self.peek() {
                Some(TokenKind::String(s)) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                Some(TokenKind::Identifier(n)) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => {
                    return Err(VmError::parse_error_simple(
                        "import: expected path string or name after `from`",
                    ))
                }
            };
            Ok(Stmt::Import { path, symbols: Some(symbols) })
        } else {
            // import "path" or import name (backward compatible, imports all)
            let path = match self.peek() {
                Some(TokenKind::String(s)) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                Some(TokenKind::Identifier(n)) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => {
                    return Err(VmError::parse_error_simple(
                        "import: expected path string or name",
                    ))
                }
            };
            Ok(Stmt::Import { path, symbols: None })
        }
    }

    /// Parse after the `export` keyword has been consumed.
    /// `export name1, name2` → Export
    /// `export { name1, name2 }` → Export
    /// `export fn name(…) { … }` → Function node + Export node
    fn parse_export(&mut self, out: &mut Vec<Stmt>) -> Result<(), VmError> {
        match self.peek() {
            Some(TokenKind::Identifier(n)) if n != "fn" => {
                let mut names = vec![n.clone()];
                self.advance();
                while matches!(self.peek(), Some(TokenKind::Comma)) {
                    self.advance();
                    match self.peek() {
                        Some(TokenKind::Identifier(name)) => {
                            names.push(name.clone());
                            self.advance();
                        }
                        _ => {
                            return Err(VmError::parse_error_simple(
                                "export: expected identifier after `,`",
                            ))
                        }
                    }
                }
                out.push(Stmt::Export(names));
                Ok(())
            }
            Some(TokenKind::LBrace) => {
                self.advance();
                let mut names = Vec::new();
                if !matches!(self.peek(), Some(TokenKind::RBrace)) {
                    loop {
                        match self.peek() {
                            Some(TokenKind::Identifier(n)) => {
                                names.push(n.clone());
                                self.advance();
                            }
                            _ => {
                                return Err(VmError::parse_error_simple(
                                    "export: expected identifier in export list",
                                ))
                            }
                        }
                        match self.peek() {
                            Some(TokenKind::Comma) => self.advance(),
                            Some(TokenKind::RBrace) => break,
                            _ => {
                                return Err(VmError::parse_error_simple(
                                    "export: expected , or } in export list",
                                ))
                            }
                        }
                    }
                }
                self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
                self.advance();
                out.push(Stmt::Export(names));
                Ok(())
            }
            Some(TokenKind::Fn) => {
                let func_stmt = self.parse_function(false)?;
                let name = match &func_stmt {
                    Stmt::Function { name, .. } => name.clone(),
                    _ => unreachable!(),
                };
                out.push(func_stmt);
                out.push(Stmt::Export(vec![name]));
                Ok(())
            }
            _ => Err(VmError::parse_error_simple(
                "export: expected identifier, {, or fn after export",
            )),
        }
    }

    fn parse_expr_list_until_rparen(&mut self) -> Result<Vec<Expr>, VmError> {
        let mut args = Vec::new();
        if matches!(self.peek(), Some(TokenKind::RParen)) {
            self.advance();
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr(0)?);
            match self.peek() {
                Some(TokenKind::Comma) => {
                    self.advance();
                    continue;
                }
                Some(TokenKind::RParen) => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(VmError::parse_error_simple(
                        "expected `,` or `)` in argument list",
                    ))
                }
            }
        }
        Ok(args)
    }

    /// Pratt parser: `min_bp` = minimum binding power to accept for infix.
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, VmError> {
        self.skip_noise();
        let mut lhs = self.parse_prefix()?;
        self.skip_noise();
        loop {
            if matches!(self.peek(), Some(TokenKind::Question)) {
                if 3 < min_bp {
                    break;
                }
                self.advance();
                let then_e = self.parse_expr(0)?;
                self.expect(":", matches!(self.peek(), Some(TokenKind::Colon)))?;
                self.advance();
                let else_e = self.parse_expr(2)?;
                lhs = Expr::Ternary {
                    condition: Box::new(lhs),
                    then_expr: Box::new(then_e),
                    else_expr: Box::new(else_e),
                };
                self.skip_noise();
                continue;
            }
            let kind = match self.peek() {
                Some(k) => k,
                None => break,
            };
            let (l_bp, r_bp, bop) = match infix_binding(kind) {
                Some(x) => x,
                None => break,
            };
            if l_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(r_bp)?;
            lhs = Expr::Binary {
                left: Box::new(lhs),
                op: bop,
                right: Box::new(rhs),
            };
            self.skip_noise();
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, VmError> {
        match self.peek().cloned() {
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(TokenKind::String(s)) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Some(TokenKind::KString(s)) => {
                self.advance();
                Ok(Expr::KString(s))
            }
            Some(TokenKind::Template(s)) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Some(TokenKind::True) => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Some(TokenKind::False) => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(Expr::Null)
            }
            Some(TokenKind::This) => {
                self.advance();
                Ok(Expr::This)
            }
            Some(TokenKind::Minus) => {
                self.advance();
                let inner = self.parse_prefix()?;
                Ok(Expr::Binary {
                    left: Box::new(Expr::Number(0.0)),
                    op: BinaryOp::Sub,
                    right: Box::new(inner),
                })
            }
            Some(TokenKind::Identifier(name)) => {
                let n = name.clone();
                self.advance();
                self.parse_postfix(Expr::Identifier(n))
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let e = self.parse_expr(0)?;
                self.expect(")", matches!(self.peek(), Some(TokenKind::RParen)))?;
                self.advance();
                self.parse_postfix(e)
            }
            Some(TokenKind::LBracket) => {
                self.advance();
                let mut elts = Vec::new();
                if !matches!(self.peek(), Some(TokenKind::RBracket)) {
                    loop {
                        elts.push(self.parse_expr(0)?);
                        match self.peek() {
                            Some(TokenKind::Comma) => self.advance(),
                            Some(TokenKind::RBracket) => break,
                            _ => {
                                return Err(VmError::parse_error_simple(
                                    "array literal: expected , or ]",
                                ))
                            }
                        }
                    }
                }
                self.expect("]", matches!(self.peek(), Some(TokenKind::RBracket)))?;
                self.advance();
                self.parse_postfix(Expr::Array(elts))
            }
            Some(TokenKind::LBrace) => self.parse_dict_literal(),
            Some(t) => Err(VmError::parse_error_simple(format!(
                "expr: unexpected start token {:?}",
                t
            ))),
            None => Err(VmError::parse_error_simple("expr: unexpected EOF")),
        }
    }

    fn parse_try(&mut self) -> Result<Stmt, VmError> {
        self.advance(); // try
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let try_body = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        self.skip_noise();

        self.expect("catch", matches!(self.peek(), Some(TokenKind::Catch)))?;
        self.advance();

        let catch_var = match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                n
            }
            Some(TokenKind::LBrace) => "error".to_string(),
            _ => return Err(VmError::parse_error_simple("try: expected variable name or { after catch")),
        };

        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        let catch_body = self.parse_block()?;
        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();
        self.skip_noise();

        let finally_body = if matches!(self.peek(), Some(TokenKind::Finally)) {
            self.advance();
            self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
            self.advance();
            let fb = self.parse_block()?;
            self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
            self.advance();
            fb
        } else {
            vec![]
        };

        Ok(Stmt::Try {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        })
    }

    fn parse_class(&mut self) -> Result<Stmt, VmError> {
        self.advance(); // class
        let name = match self.peek() {
            Some(TokenKind::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                n
            }
            _ => return Err(VmError::parse_error_simple("class: expected class name")),
        };

        let extends = if matches!(self.peek(), Some(TokenKind::Extends)) {
            self.advance();
            match self.peek() {
                Some(TokenKind::Identifier(n)) => {
                    let n = n.clone();
                    self.advance();
                    Some(n)
                }
                _ => return Err(VmError::parse_error_simple("class: expected parent class name after extends")),
            }
        } else {
            None
        };

        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        self.skip_noise();

        let mut methods: Vec<(String, Vec<String>, Vec<Stmt>)> = Vec::new();
        while !matches!(self.peek(), Some(TokenKind::RBrace) | None | Some(TokenKind::Eof)) {
            self.skip_noise();
            if matches!(self.peek(), Some(TokenKind::RBrace)) {
                break;
            }

            let method_name = match self.peek() {
                Some(TokenKind::Fn) => {
                    self.advance();
                    match self.peek() {
                        Some(TokenKind::Identifier(n)) => {
                            let n = n.clone();
                            self.advance();
                            n
                        }
                        _ => return Err(VmError::parse_error_simple("class: expected method name")),
                    }
                }
                Some(TokenKind::Identifier(n)) => {
                    let n = n.clone();
                    self.advance();
                    n
                }
                _ => return Err(VmError::parse_error_simple(
                    "class: expected method definition (fn name(...) { ... })"
                )),
            };

            self.expect("(", matches!(self.peek(), Some(TokenKind::LParen)))?;
            self.advance();
            let mut params = Vec::new();
            if !matches!(self.peek(), Some(TokenKind::RParen)) {
                loop {
                    match self.peek() {
                        Some(TokenKind::Identifier(p)) => {
                            params.push(p.clone());
                            self.advance();
                        }
                        _ => return Err(VmError::parse_error_simple("class method: expected parameter name")),
                    }
                    match self.peek() {
                        Some(TokenKind::Comma) => self.advance(),
                        Some(TokenKind::RParen) => break,
                        _ => return Err(VmError::parse_error_simple("class method: expected , or )")),
                    }
                }
            }
            self.expect(")", matches!(self.peek(), Some(TokenKind::RParen)))?;
            self.advance();

            self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
            self.advance();
            let body = self.parse_block()?;
            self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
            self.advance();
            self.skip_noise();

            methods.push((method_name, params, body));
        }

        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();

        Ok(Stmt::Class {
            name,
            extends,
            methods,
        })
    }

    fn parse_match(&mut self) -> Result<Stmt, VmError> {
        self.advance(); // consume 'match'
        let expression = Box::new(self.parse_expr(0)?);
        
        self.expect("{", matches!(self.peek(), Some(TokenKind::LBrace)))?;
        self.advance();
        self.skip_noise();

        let mut arms = Vec::new();
        while !matches!(self.peek(), Some(TokenKind::RBrace) | None | Some(TokenKind::Eof)) {
            self.skip_noise();
            if matches!(self.peek(), Some(TokenKind::RBrace)) {
                break;
            }

            // Parse pattern
            let pattern = self.parse_match_pattern()?;

            // Optional guard: `if condition`
            let guard = if matches!(self.peek(), Some(TokenKind::If)) {
                self.advance();
                Some(Box::new(self.parse_expr(0)?))
            } else {
                None
            };

            // Expect =>
            self.expect("=>", matches!(self.peek(), Some(TokenKind::Arrow)))?;
            self.advance();

            // Parse body (can be a block or single expression)
            let body = if matches!(self.peek(), Some(TokenKind::LBrace)) {
                self.advance();
                let b = self.parse_block()?;
                self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
                self.advance();
                b
            } else {
                // Single expression treated as statement
                let expr = self.parse_expr(0)?;
                vec![Stmt::Expr(expr)]
            };

            arms.push(crate::ast::MatchArm {
                pattern,
                guard,
                body,
            });

            // Optional comma
            if matches!(self.peek(), Some(TokenKind::Comma)) {
                self.advance();
            }

            self.skip_noise();
        }

        self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
        self.advance();

        Ok(Stmt::Match { expression, arms })
    }

    fn parse_match_pattern(&mut self) -> Result<crate::ast::MatchPattern, VmError> {
        use crate::ast::MatchPattern;

        match self.peek().cloned() {
            Some(TokenKind::Identifier(name)) if name == "_" => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            Some(TokenKind::Identifier(name)) => {
                let n = name.clone();
                self.advance();
                Ok(MatchPattern::Identifier(n))
            }
            Some(TokenKind::LBracket) => {
                // Array pattern: [a, b, c]
                self.advance();
                let mut patterns = Vec::new();
                
                if !matches!(self.peek(), Some(TokenKind::RBracket)) {
                    loop {
                        patterns.push(self.parse_match_pattern()?);
                        if matches!(self.peek(), Some(TokenKind::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                self.expect("]", matches!(self.peek(), Some(TokenKind::RBracket)))?;
                self.advance();
                Ok(MatchPattern::Array(patterns))
            }
            Some(TokenKind::LBrace) => {
                // Object pattern: {x, y} or {x: pat, y: pat}
                self.advance();
                let mut fields = Vec::new();

                if !matches!(self.peek(), Some(TokenKind::RBrace)) {
                    loop {
                        let field_name = match self.peek() {
                            Some(TokenKind::Identifier(n)) => {
                                let name = n.clone();
                                self.advance();
                                name
                            }
                            _ => return Err(VmError::parse_error_simple("match pattern: expected field name")),
                        };

                        let pattern = if matches!(self.peek(), Some(TokenKind::Colon)) {
                            self.advance();
                            self.parse_match_pattern()?
                        } else {
                            MatchPattern::Identifier(field_name.clone())
                        };

                        fields.push((field_name, pattern));

                        if matches!(self.peek(), Some(TokenKind::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                self.expect("}", matches!(self.peek(), Some(TokenKind::RBrace)))?;
                self.advance();
                Ok(MatchPattern::Object(fields))
            }
            // Literal patterns: numbers, strings, booleans, null
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Number(n)))
            }
            Some(TokenKind::String(s)) => {
                let s_clone = s.clone();
                self.advance();
                Ok(MatchPattern::Literal(Expr::String(s_clone)))
            }
            Some(TokenKind::True) => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(true)))
            }
            Some(TokenKind::False) => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Bool(false)))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(MatchPattern::Literal(Expr::Null))
            }
            _ => Err(VmError::parse_error_simple("match pattern: expected pattern")),
        }
    }

    fn parse_dict_literal(&mut self) -> Result<Expr, VmError> {
        self.advance(); // {
        let mut pairs = Vec::new();
        if matches!(self.peek(), Some(TokenKind::RBrace)) {
            self.advance();
            return Ok(Expr::Dict(pairs));
        }
        loop {
            let key = self.parse_expr(0)?;
            self.expect(":", matches!(self.peek(), Some(TokenKind::Colon)))?;
            self.advance();
            let val = self.parse_expr(0)?;
            pairs.push((key, val));
            match self.peek() {
                Some(TokenKind::Comma) => self.advance(),
                Some(TokenKind::RBrace) => break,
                _ => {
                    return Err(VmError::parse_error_simple(
                        "dict literal: expected , or }",
                    ))
                }
            }
        }
        self.advance(); // }
        self.parse_postfix(Expr::Dict(pairs))
    }

    fn parse_postfix(&mut self, mut lhs: Expr) -> Result<Expr, VmError> {
        loop {
            match self.peek().cloned() {
                Some(TokenKind::LParen) => {
                    let callee = match lhs {
                        Expr::Identifier(name) => {
                            self.advance();
                            let args = self.parse_expr_list_until_rparen()?;
                            Expr::Call { callee: name, args }
                        }
                        other => {
                            self.advance();
                            let args = self.parse_expr_list_until_rparen()?;
                            Expr::CallExpr {
                                callee: Box::new(other),
                                args,
                            }
                        }
                    };
                    lhs = callee;
                }
                Some(TokenKind::LBracket) => {
                    self.advance();
                    let idx = self.parse_expr(0)?;
                    self.expect("]", matches!(self.peek(), Some(TokenKind::RBracket)))?;
                    self.advance();
                    lhs = Expr::Index {
                        object: Box::new(lhs),
                        index: Box::new(idx),
                    };
                }
                Some(TokenKind::Dot) => {
                    self.advance();
                    let member = match self.peek() {
                        Some(TokenKind::Identifier(n)) => {
                            let n = n.clone();
                            self.advance();
                            n
                        }
                        _ => {
                            return Err(VmError::parse_error_simple(
                                "expected member name after `.`",
                            ))
                        }
                    };
                    if matches!(self.peek(), Some(TokenKind::LParen)) {
                        self.advance();
                        let args = self.parse_expr_list_until_rparen()?;
                        lhs = Expr::MethodCall {
                            object: Box::new(lhs),
                            method: member,
                            args,
                        };
                    } else {
                        lhs = Expr::Index {
                            object: Box::new(lhs),
                            index: Box::new(Expr::String(member)),
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(lhs)
    }
}

fn infix_binding(kind: &TokenKind) -> Option<(u8, u8, BinaryOp)> {
    use BinaryOp::*;
    Some(match kind {
        TokenKind::OrOr => (1, 2, Or),
        TokenKind::AndAnd => (3, 4, And),
        TokenKind::EqualEqual => (5, 6, Eq),
        TokenKind::BangEqual => (5, 6, Ne),
        TokenKind::Less => (5, 6, Lt),
        TokenKind::LessEqual => (5, 6, Le),
        TokenKind::Greater => (5, 6, Gt),
        TokenKind::GreaterEqual => (5, 6, Ge),
        TokenKind::Plus => (7, 8, Add),
        TokenKind::Minus => (7, 8, Sub),
        TokenKind::Star => (9, 10, Mul),
        TokenKind::Slash => (9, 10, Div),
        TokenKind::DoubleSlash => (9, 10, IntDiv),
        TokenKind::Percent => (9, 10, Mod),
        TokenKind::DoubleStar => (11, 12, Pow),
        _ => return None,
    })
}
