//! `killer wasm script.killer [-o out.wasm]`: compile the numeric functions of a Killer script to WebAssembly.
//!
//! The script may contain only `fn` definitions (and `const NAME = number`). Every function takes numbers and
//! returns a number (WebAssembly `f64`), is exported under its own name, and may call the functions of the same
//! file (recursion too). Supported inside a function:
//!
//! * numbers, `true` / `false` (1 / 0), parameters and local variables, constants
//! * `+ - * / %`, unary `-`, comparisons `== != < <= > >=`, `&&` `||` `!` (short-circuit)
//! * `x = e`, `x += e` (also `-= *= /=`), `x++`, `x--`
//! * `if` / `else if` / `else`, `while`, `for i in range(n)` / `range(a, b)`, `break`, `continue`, `return`
//! * calls to your own functions and the math functions `sqrt abs floor ceil trunc min max`
//!
//! Anything else (strings, lists, objects, other builtins, `print`) is a compile error that names the line:
//! the output is a small module with no imports, so it runs in any WebAssembly host (a browser, `wasmtime`, Node).
//! A function that reaches its end without `return` gives 0. `%` is the floor modulo (the sign of the divisor), like
//! the Killer VM; dividing by zero gives NaN or infinity here instead of the VM's error.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Sym(&'static str),
    Nl,
}

const SYMS: &[&str] = &[
    "+=", "-=", "*=", "/=", "++", "--", "==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "(", ")", "{", "}", ",", "=", "<", ">", "!", ";",
];

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    while i < c.len() {
        let ch = c[i];
        if ch == '\n' {
            out.push((Tok::Nl, line));
            line += 1;
            i += 1;
        } else if ch.is_whitespace() {
            i += 1;
        } else if ch == '#' || (ch == '/' && c.get(i + 1) == Some(&'/')) {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.' || ((c[i] == 'e' || c[i] == 'E') && i > s)) {
                if (c[i] == 'e' || c[i] == 'E') && matches!(c.get(i + 1), Some('+') | Some('-')) {
                    i += 1;
                }
                i += 1;
            }
            let t: String = c[s..i].iter().collect();
            out.push((Tok::Num(t.parse().map_err(|_| format!("Line {}: bad number {}", line, t))?), line));
        } else if ch.is_alphabetic() || ch == '_' {
            let s = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push((Tok::Id(c[s..i].iter().collect()), line));
        } else if ch == '"' || ch == '\'' {
            return Err(format!("Line {}: strings cannot be compiled to WebAssembly (numbers only)", line));
        } else {
            let rest: String = c[i..(i + 2).min(c.len())].iter().collect();
            match SYMS.iter().find(|s| rest.starts_with(**s)) {
                Some(s) => {
                    out.push((Tok::Sym(s), line));
                    i += s.len();
                }
                None => return Err(format!("Line {}: unexpected character '{}'", line, ch)),
            }
        }
    }
    Ok(out)
}

#[derive(Debug)]
enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>, usize),
}

#[derive(Debug)]
enum Stmt {
    Set(String, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Expr, Vec<Stmt>),
    Break(usize),
    Continue(usize),
    Return(Option<Expr>),
    Eval(Expr),
}

struct Func {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
    line: usize,
}

struct P {
    t: Vec<(Tok, usize)>,
    i: usize,
    consts: HashMap<String, f64>,
}

impl P {
    fn line(&self) -> usize {
        self.t.get(self.i).or(self.t.last()).map_or(1, |x| x.1)
    }
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i).map(|x| &x.0)
    }
    fn skip_nl(&mut self) {
        while matches!(self.peek(), Some(Tok::Nl) | Some(Tok::Sym(";"))) {
            self.i += 1;
        }
    }
    fn sym(&mut self, s: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Sym(x)) if *x == s) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, s: &str) -> Result<(), String> {
        if self.sym(s) {
            Ok(())
        } else {
            Err(format!("Line {}: expected '{}'", self.line(), s))
        }
    }
    fn word(&mut self, w: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Id(x)) if x == w) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Id(x)) => {
                let x = x.clone();
                self.i += 1;
                Ok(x)
            }
            _ => Err(format!("Line {}: expected a name", self.line())),
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.skip_nl();
        self.expect("{")?;
        let mut out = Vec::new();
        loop {
            self.skip_nl();
            if self.sym("}") {
                return Ok(out);
            }
            if self.peek().is_none() {
                return Err(format!("Line {}: missing '}}'", self.line()));
            }
            out.push(self.stmt()?);
        }
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        let line = self.line();
        if self.word("if") {
            return self.if_rest();
        }
        if self.word("while") {
            let c = self.expr()?;
            return Ok(Stmt::While(c, self.block()?));
        }
        if self.word("for") {
            let v = self.ident()?;
            if !self.word("in") || !self.word("range") {
                return Err(format!("Line {}: only `for x in range(n)` or `range(a, b)` can be compiled", line));
            }
            self.expect("(")?;
            let first = self.expr()?;
            let (lo, hi) = if self.sym(",") { (first, self.expr()?) } else { (Expr::Num(0.0), first) };
            self.expect(")")?;
            return Ok(Stmt::For(v, lo, hi, self.block()?));
        }
        if self.word("break") {
            return Ok(Stmt::Break(line));
        }
        if self.word("continue") {
            return Ok(Stmt::Continue(line));
        }
        if self.word("return") {
            if matches!(self.peek(), Some(Tok::Nl) | Some(Tok::Sym("}")) | Some(Tok::Sym(";")) | None) {
                return Ok(Stmt::Return(None));
            }
            return Ok(Stmt::Return(Some(self.expr()?)));
        }
        // assignment forms: name = e, name op= e, name++, name--
        if let (Some(Tok::Id(name)), Some(Tok::Sym(op))) = (self.peek().cloned(), self.t.get(self.i + 1).map(|x| x.0.clone())) {
            if matches!(op, "=" | "+=" | "-=" | "*=" | "/=" | "++" | "--") {
                self.i += 2;
                let target = Expr::Var(name.clone());
                let value = match op {
                    "=" => self.expr()?,
                    "++" => Expr::Bin("+", Box::new(target), Box::new(Expr::Num(1.0))),
                    "--" => Expr::Bin("-", Box::new(target), Box::new(Expr::Num(1.0))),
                    _ => {
                        let r = self.expr()?;
                        Expr::Bin(&op[..1].to_string().leak()[..], Box::new(target), Box::new(r))
                    }
                };
                return Ok(Stmt::Set(name, value));
            }
        }
        Ok(Stmt::Eval(self.expr()?))
    }

    fn if_rest(&mut self) -> Result<Stmt, String> {
        let c = self.expr()?;
        let then = self.block()?;
        let save = self.i;
        self.skip_nl();
        let els = if self.word("else") {
            if self.word("if") {
                vec![self.if_rest()?]
            } else {
                self.block()?
            }
        } else {
            self.i = save;
            Vec::new()
        };
        Ok(Stmt::If(c, then, els))
    }

    fn expr(&mut self) -> Result<Expr, String> {
        self.level(0)
    }

    fn level(&mut self, lvl: usize) -> Result<Expr, String> {
        const LEVELS: &[&[&str]] = &[&["||"], &["&&"], &["==", "!="], &["<", "<=", ">", ">="], &["+", "-"], &["*", "/", "%"]];
        if lvl == LEVELS.len() {
            return self.unary();
        }
        let mut l = self.level(lvl + 1)?;
        loop {
            let op = LEVELS[lvl].iter().find(|o| matches!(self.peek(), Some(Tok::Sym(x)) if x == *o)).copied();
            match op {
                Some(o) => {
                    self.i += 1;
                    let r = self.level(lvl + 1)?;
                    l = Expr::Bin(o, Box::new(l), Box::new(r));
                }
                None => return Ok(l),
            }
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.sym("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.sym("!") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let line = self.line();
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Sym("(")) => {
                self.i += 1;
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                match name.as_str() {
                    "true" => return Ok(Expr::Num(1.0)),
                    "false" => return Ok(Expr::Num(0.0)),
                    _ => {}
                }
                if self.sym("(") {
                    let mut args = Vec::new();
                    if !self.sym(")") {
                        loop {
                            args.push(self.expr()?);
                            if self.sym(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    return Ok(Expr::Call(name, args, line));
                }
                if let Some(v) = self.consts.get(&name) {
                    return Ok(Expr::Num(*v));
                }
                Ok(Expr::Var(name))
            }
            _ => Err(format!("Line {}: this expression cannot be compiled to WebAssembly", line)),
        }
    }
}

fn parse(src: &str) -> Result<Vec<Func>, String> {
    let mut p = P { t: lex(src)?, i: 0, consts: HashMap::new() };
    let mut funcs = Vec::new();
    loop {
        p.skip_nl();
        if p.peek().is_none() {
            break;
        }
        let line = p.line();
        if p.word("const") {
            let name = p.ident()?;
            p.expect("=")?;
            let neg = p.sym("-");
            match p.peek().cloned() {
                Some(Tok::Num(n)) => {
                    p.i += 1;
                    p.consts.insert(name, if neg { -n } else { n });
                }
                _ => return Err(format!("Line {}: a const must be a number", line)),
            }
            continue;
        }
        if !p.word("fn") {
            return Err(format!("Line {}: only `fn` definitions and `const NAME = number` can be compiled to WebAssembly", line));
        }
        let name = p.ident()?;
        p.expect("(")?;
        let mut params = Vec::new();
        if !p.sym(")") {
            loop {
                params.push(p.ident()?);
                if p.sym(")") {
                    break;
                }
                p.expect(",")?;
            }
        }
        let body = p.block()?;
        funcs.push(Func { name, params, body, line });
    }
    if funcs.is_empty() {
        return Err("no functions to compile".to_string());
    }
    Ok(funcs)
}

// ---------------------------------------------------------------------------------------------
// code generation

fn uleb(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn sleb(mut v: i64, out: &mut Vec<u8>) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        let done = (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0);
        out.push(if done { b } else { b | 0x80 });
        if done {
            return;
        }
    }
}

struct Gen<'a> {
    code: Vec<u8>,
    locals: HashMap<String, u32>,
    nlocals: u32,
    funcs: &'a HashMap<String, (usize, usize)>, // name -> (index, arity)
    depth: u32,
    loops: Vec<(u32, u32)>, // (depth of the break block, depth of the continue block)
    temps: u32,
}

const MATH1: &[(&str, u8)] = &[("sqrt", 0x9f), ("abs", 0x99), ("floor", 0x9c), ("ceil", 0x9b), ("trunc", 0x9d)];
const MATH2: &[(&str, u8)] = &[("min", 0xa4), ("max", 0xa5)];

fn is_cond(e: &Expr) -> bool {
    match e {
        Expr::Not(_) => true,
        Expr::Bin(op, _, _) => matches!(*op, "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||"),
        _ => false,
    }
}

impl<'a> Gen<'a> {
    fn local(&mut self, name: &str) -> u32 {
        if let Some(i) = self.locals.get(name) {
            return *i;
        }
        let i = self.locals.len() as u32;
        self.locals.insert(name.to_string(), i);
        self.nlocals += 1;
        i
    }
    fn temp(&mut self) -> u32 {
        self.temps += 1;
        let name = format!("%tmp{}", self.temps);
        self.local(&name)
    }
    fn f64c(&mut self, v: f64) {
        self.code.push(0x44);
        self.code.extend_from_slice(&v.to_le_bytes());
    }
    fn get(&mut self, i: u32) {
        self.code.push(0x20);
        uleb(i as u64, &mut self.code);
    }
    fn set(&mut self, i: u32) {
        self.code.push(0x21);
        uleb(i as u64, &mut self.code);
    }
    fn open(&mut self, op: u8, ty: u8) {
        self.code.push(op);
        self.code.push(ty);
        self.depth += 1;
    }
    fn close(&mut self) {
        self.code.push(0x0b);
        self.depth -= 1;
    }
    fn br(&mut self, target_depth: u32, conditional: bool) {
        self.code.push(if conditional { 0x0d } else { 0x0c });
        uleb((self.depth - target_depth) as u64, &mut self.code);
    }

    /// Leaves an f64 on the stack.
    fn value(&mut self, e: &Expr) -> Result<(), String> {
        if is_cond(e) {
            self.cond(e)?;
            self.code.push(0xb8); // f64.convert_i32_u
            return Ok(());
        }
        match e {
            Expr::Num(n) => self.f64c(*n),
            Expr::Var(v) => match self.locals.get(v) {
                Some(i) => {
                    let i = *i;
                    self.get(i)
                }
                None => return Err(format!("the variable `{}` is used before it is assigned", v)),
            },
            Expr::Neg(x) => {
                self.value(x)?;
                self.code.push(0x9a);
            }
            Expr::Bin(op, l, r) => {
                if *op == "%" {
                    // a % b = a - floor(a / b) * b (floor modulo, the sign of the divisor, like the Killer VM)
                    let (ta, tb) = (self.temp(), self.temp());
                    self.value(l)?;
                    self.set(ta);
                    self.value(r)?;
                    self.set(tb);
                    self.get(ta);
                    self.get(ta);
                    self.get(tb);
                    self.code.push(0xa3);
                    self.code.push(0x9c);
                    self.get(tb);
                    self.code.push(0xa2);
                    self.code.push(0xa1);
                } else {
                    self.value(l)?;
                    self.value(r)?;
                    self.code.push(match *op {
                        "+" => 0xa0,
                        "-" => 0xa1,
                        "*" => 0xa2,
                        "/" => 0xa3,
                        _ => return Err(format!("operator {} cannot be compiled", op)),
                    });
                }
            }
            Expr::Call(name, args, line) => {
                if let Some((_, op)) = MATH1.iter().find(|(n, _)| n == name) {
                    if args.len() != 1 {
                        return Err(format!("Line {}: {}() takes one argument", line, name));
                    }
                    self.value(&args[0])?;
                    self.code.push(*op);
                } else if let Some((_, op)) = MATH2.iter().find(|(n, _)| n == name) {
                    if args.len() != 2 {
                        return Err(format!("Line {}: {}() takes two arguments", line, name));
                    }
                    self.value(&args[0])?;
                    self.value(&args[1])?;
                    self.code.push(*op);
                } else if let Some(&(idx, arity)) = self.funcs.get(name) {
                    if args.len() != arity {
                        return Err(format!("Line {}: {}() takes {} argument(s), got {}", line, name, arity, args.len()));
                    }
                    for a in args {
                        self.value(a)?;
                    }
                    self.code.push(0x10);
                    uleb(idx as u64, &mut self.code);
                } else {
                    return Err(format!("Line {}: `{}` is not a function of this file or a supported math function (sqrt abs floor ceil trunc min max)", line, name));
                }
            }
            Expr::Not(_) => unreachable!(),
        }
        Ok(())
    }

    /// Leaves an i32 (0 or 1) on the stack.
    fn cond(&mut self, e: &Expr) -> Result<(), String> {
        match e {
            Expr::Not(x) => {
                self.cond(x)?;
                self.code.push(0x45); // i32.eqz
            }
            Expr::Bin(op, l, r) if matches!(*op, "&&" | "||") => {
                self.cond(l)?;
                self.open(0x04, 0x7f); // if (result i32)
                if *op == "&&" {
                    self.cond(r)?;
                    self.code.push(0x05);
                    self.code.push(0x41);
                    sleb(0, &mut self.code);
                } else {
                    self.code.push(0x41);
                    sleb(1, &mut self.code);
                    self.code.push(0x05);
                    self.cond(r)?;
                }
                self.close();
            }
            Expr::Bin(op, l, r) if matches!(*op, "==" | "!=" | "<" | "<=" | ">" | ">=") => {
                self.value(l)?;
                self.value(r)?;
                self.code.push(match *op {
                    "==" => 0x61,
                    "!=" => 0x62,
                    "<" => 0x63,
                    ">" => 0x64,
                    "<=" => 0x65,
                    _ => 0x66,
                });
            }
            other => {
                self.value(other)?;
                self.f64c(0.0);
                self.code.push(0x62); // f64.ne
            }
        }
        Ok(())
    }

    fn block(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for s in stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Set(name, e) => {
                // a name is a local the first time it is assigned (the right side may not use it before then)
                self.value(e)?;
                let i = self.local(name);
                self.set(i);
            }
            Stmt::Eval(e) => {
                self.value(e)?;
                self.code.push(0x1a); // drop
            }
            Stmt::Return(e) => {
                match e {
                    Some(e) => self.value(e)?,
                    None => self.f64c(0.0),
                }
                self.code.push(0x0f);
            }
            Stmt::If(c, t, f) => {
                self.cond(c)?;
                self.open(0x04, 0x40);
                self.block(t)?;
                if !f.is_empty() {
                    self.code.push(0x05);
                    self.block(f)?;
                }
                self.close();
            }
            Stmt::While(c, body) => {
                self.open(0x02, 0x40); // block (break target)
                let brk = self.depth;
                self.open(0x03, 0x40); // loop
                let top = self.depth;
                self.cond(c)?;
                self.code.push(0x45);
                self.br(brk, true);
                self.open(0x02, 0x40); // block (continue target)
                let cont = self.depth;
                self.loops.push((brk, cont));
                self.block(body)?;
                self.loops.pop();
                self.close();
                self.br(top, false);
                self.close();
                self.close();
            }
            Stmt::For(v, lo, hi, body) => {
                // the bound is evaluated once, before the loop
                let end = self.temp();
                self.value(hi)?;
                self.set(end);
                self.value(lo)?;
                let i = self.local(v);
                self.set(i);
                self.open(0x02, 0x40);
                let brk = self.depth;
                self.open(0x03, 0x40);
                let top = self.depth;
                self.get(i);
                self.get(end);
                self.code.push(0x63); // f64.lt
                self.code.push(0x45);
                self.br(brk, true);
                self.open(0x02, 0x40);
                let cont = self.depth;
                self.loops.push((brk, cont));
                self.block(body)?;
                self.loops.pop();
                self.close();
                self.get(i);
                self.f64c(1.0);
                self.code.push(0xa0);
                self.set(i);
                self.br(top, false);
                self.close();
                self.close();
            }
            Stmt::Break(line) => match self.loops.last() {
                Some(&(b, _)) => self.br(b, false),
                None => return Err(format!("Line {}: `break` outside a loop", line)),
            },
            Stmt::Continue(line) => match self.loops.last() {
                Some(&(_, c)) => self.br(c, false),
                None => return Err(format!("Line {}: `continue` outside a loop", line)),
            },
        }
        Ok(())
    }
}

/// Compile `source` (functions of numbers) to a WebAssembly module; returns the bytes and the exported names.
pub fn compile(source: &str) -> Result<(Vec<u8>, Vec<(String, usize)>), String> {
    let funcs = parse(source)?;
    let mut index: HashMap<String, (usize, usize)> = HashMap::new();
    for (i, f) in funcs.iter().enumerate() {
        if index.insert(f.name.clone(), (i, f.params.len())).is_some() {
            return Err(format!("Line {}: the function {} is defined twice", f.line, f.name));
        }
    }
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    for f in &funcs {
        let mut g = Gen { code: Vec::new(), locals: HashMap::new(), nlocals: 0, funcs: &index, depth: 0, loops: Vec::new(), temps: 0 };
        for p in &f.params {
            if g.locals.insert(p.clone(), g.locals.len() as u32).is_some() {
                return Err(format!("Line {}: parameter {} appears twice", f.line, p));
            }
        }
        g.block(&f.body).map_err(|e| if e.starts_with("Line ") { e } else { format!("Line {}: in function {}: {}", f.line, f.name, e) })?;
        g.f64c(0.0); // falling off the end gives 0
        g.code.push(0x0b);
        let extra = g.locals.len() - f.params.len();
        let mut body = Vec::new();
        if extra > 0 {
            uleb(1, &mut body);
            uleb(extra as u64, &mut body);
            body.push(0x7c);
        } else {
            uleb(0, &mut body);
        }
        body.extend_from_slice(&g.code);
        let mut sized = Vec::new();
        uleb(body.len() as u64, &mut sized);
        sized.extend_from_slice(&body);
        bodies.push(sized);
    }
    let mut m: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut section = |id: u8, content: Vec<u8>, m: &mut Vec<u8>| {
        m.push(id);
        uleb(content.len() as u64, m);
        m.extend_from_slice(&content);
    };
    // types: one per distinct parameter count
    let mut arities: Vec<usize> = funcs.iter().map(|f| f.params.len()).collect();
    arities.sort();
    arities.dedup();
    let mut ty = Vec::new();
    uleb(arities.len() as u64, &mut ty);
    for a in &arities {
        ty.push(0x60);
        uleb(*a as u64, &mut ty);
        ty.extend(std::iter::repeat(0x7c).take(*a));
        ty.push(0x01);
        ty.push(0x7c);
    }
    section(1, ty, &mut m);
    let mut fs = Vec::new();
    uleb(funcs.len() as u64, &mut fs);
    for f in &funcs {
        uleb(arities.iter().position(|a| *a == f.params.len()).unwrap() as u64, &mut fs);
    }
    section(3, fs, &mut m);
    let mut ex = Vec::new();
    uleb(funcs.len() as u64, &mut ex);
    for (i, f) in funcs.iter().enumerate() {
        uleb(f.name.len() as u64, &mut ex);
        ex.extend_from_slice(f.name.as_bytes());
        ex.push(0x00);
        uleb(i as u64, &mut ex);
    }
    section(7, ex, &mut m);
    let mut code = Vec::new();
    uleb(bodies.len() as u64, &mut code);
    for b in bodies {
        code.extend_from_slice(&b);
    }
    section(10, code, &mut m);
    Ok((m, funcs.iter().map(|f| (f.name.clone(), f.params.len())).collect()))
}

/// `killer wasm script.killer [-o out.wasm]`
pub fn cmd_wasm(args: &[String]) -> i32 {
    let mut file = None;
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                i += 1;
                out = args.get(i).cloned();
                if out.is_none() {
                    eprintln!("killer wasm: -o needs a file name");
                    return 2;
                }
            }
            x if x.starts_with('-') => {
                eprintln!("killer wasm: unknown option {}", x);
                return 2;
            }
            x => file = Some(x.to_string()),
        }
        i += 1;
    }
    let file = match file {
        Some(f) => f,
        None => {
            eprintln!("usage: killer wasm <file.killer> [-o out.wasm]");
            return 2;
        }
    };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("killer: cannot read {}: {}", file, e);
            return 1;
        }
    };
    match compile(&src) {
        Ok((bytes, exports)) => {
            let target = out.unwrap_or_else(|| format!("{}.wasm", file.trim_end_matches(".killer")));
            if let Err(e) = std::fs::write(&target, &bytes) {
                eprintln!("killer wasm: cannot write {}: {}", target, e);
                return 1;
            }
            println!("wrote {} ({} bytes)", target, bytes.len());
            for (n, a) in exports {
                println!("  export {}({} number{})", n, a, if a == 1 { "" } else { "s" });
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_header_and_exports() {
        let (m, ex) = compile("fn add(a, b) { return a + b }\nfn one() { return 1 }").unwrap();
        assert_eq!(&m[..8], &[0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]);
        assert_eq!(ex, vec![("add".to_string(), 2), ("one".to_string(), 0)]);
    }

    #[test]
    fn unsupported_things_are_clear_errors() {
        assert!(compile("fn f() { return \"x\" }").unwrap_err().contains("strings"));
        assert!(compile("println(1)").unwrap_err().contains("only `fn`"));
        assert!(compile("fn f(x) { return g(x) }").unwrap_err().contains("not a function"));
        assert!(compile("fn f(x) { return y }").unwrap_err().contains("before it is assigned"));
        assert!(compile("fn f(x) { break }").unwrap_err().contains("outside a loop"));
        assert!(compile("fn f(x) { return x }\nfn f(y) { return y }").unwrap_err().contains("twice"));
    }

    #[test]
    fn leb128() {
        let mut v = Vec::new();
        uleb(624485, &mut v);
        assert_eq!(v, vec![0xe5, 0x8e, 0x26]);
        let mut s = Vec::new();
        sleb(-123456, &mut s);
        assert_eq!(s, vec![0xc0, 0xbb, 0x78]);
    }
}
