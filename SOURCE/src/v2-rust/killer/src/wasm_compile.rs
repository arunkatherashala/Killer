//! `killer wasm script.killer [-o out.wasm]`: compile the functions of a Killer script to WebAssembly.
//!
//! The script may contain only `fn` definitions and `const NAME = number`. Every function is exported under its own
//! name and may call the functions of the same file (recursion too). Four kinds of value exist:
//!
//! * numbers and booleans: WebAssembly `f64` (`true` / `false` are 1 / 0)
//! * strings: a pointer (`i32`) to `[len: u32][bytes]` in linear memory (ASCII text, one byte per character)
//! * lists of numbers: a pointer (`i32`) to a 12 byte header `[len][capacity][data pointer]`, elements are `f64`
//!
//! Types are inferred (parameters from how they are used and from the calls to the function; an annotation such as
//! `fn f(s: string, xs: array) -> string` always wins). A variable keeps one type. Supported inside a function:
//!
//! * numbers, `true` / `false`, string literals, `[a, b, c]` list literals, parameters, local variables, constants
//! * `+ - * / %`, unary `-`, comparisons, `&& || !`, string `+` (concatenation), string `==` / `!=`
//! * `s[i]` (one-character string), `xs[i]`, negative indexes count from the end, `xs[i] = e`, `xs[i] += e`
//! * `len(s)`, `len(xs)`, `push(xs, n)`, `sqrt abs floor ceil trunc min max`
//! * `x = e`, `x += e` (also `-= *= /=`), `x++`, `x--`
//! * `if` / `else if` / `else`, `while`, `for i in range(n)` / `range(a, b)`, `for x in list`, `for c in string`,
//!   `break`, `continue`, `return`
//!
//! Anything else (dicts, floats printed as text, other builtins, `print`) is a compile error that names the line. A
//! module that uses strings or lists has one memory (exported as `memory`), a bump allocator that never frees, and
//! exported helpers for the host (`killer_alloc`, `killer_str_new`, `killer_str_len`, `killer_list_new`,
//! `killer_list_len`, `killer_list_get`, `killer_list_set`, `killer_list_push`). A module that uses only numbers has
//! no memory and no imports. An index outside a string or list traps (the VM gives `null`). `%` is the floor modulo
//! like the Killer VM; dividing by zero gives NaN or infinity here instead of the VM's error.

use std::collections::HashMap;

const NL: char = 10u8 as char;
const BS: char = 92u8 as char;

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Id(String),
    Sym(&'static str),
    Nl,
}

const SYMS: &[&str] = &[
    "+=", "-=", "*=", "/=", "++", "--", "==", "!=", "<=", ">=", "&&", "||", "->", "+", "-", "*", "/", "%", "(", ")", "{", "}", "[", "]", ",", ":", "=", "<", ">", "!",
    ";",
];

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    while i < c.len() {
        let ch = c[i];
        if ch == NL {
            out.push((Tok::Nl, line));
            line += 1;
            i += 1;
        } else if ch.is_whitespace() {
            i += 1;
        } else if ch == '#' || (ch == '/' && c.get(i + 1) == Some(&'/')) {
            while i < c.len() && c[i] != NL {
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
            let quote = ch;
            i += 1;
            let mut s = String::new();
            loop {
                match c.get(i) {
                    None => return Err(format!("Line {}: unterminated string", line)),
                    Some(&q) if q == quote => {
                        i += 1;
                        break;
                    }
                    Some(&b) if b == BS => {
                        let e = match c.get(i + 1) {
                            Some('n') => NL,
                            Some('t') => 9u8 as char,
                            Some('r') => 13u8 as char,
                            Some('0') => 0u8 as char,
                            Some(&o) if o == BS || o == '"' || o == '\'' => o,
                            _ => return Err(format!("Line {}: this string escape cannot be compiled", line)),
                        };
                        s.push(e);
                        i += 2;
                    }
                    Some(&NL) => return Err(format!("Line {}: unterminated string", line)),
                    Some(&x) if !x.is_ascii() => {
                        return Err(format!("Line {}: only ASCII text can be compiled to WebAssembly strings (one byte per character)", line))
                    }
                    Some(&x) => {
                        s.push(x);
                        i += 1;
                    }
                }
            }
            out.push((Tok::Str(s), line));
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum T {
    Num,
    Str,
    List,
}

impl T {
    fn vt(self) -> u8 {
        if self == T::Num {
            0x7c
        } else {
            0x7f
        }
    }
    fn name(self) -> &'static str {
        match self {
            T::Num => "number",
            T::Str => "string",
            T::List => "list",
        }
    }
}

fn type_name(n: &str) -> Option<T> {
    match n {
        "number" | "int" | "float" | "bool" | "boolean" => Some(T::Num),
        "string" | "str" => Some(T::Str),
        "array" | "list" => Some(T::List),
        _ => None,
    }
}

#[derive(Debug)]
enum Expr {
    Num(f64),
    Str(String),
    List(Vec<Expr>),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>, usize),
}

type Sn = (usize, Stmt);

#[derive(Debug)]
enum Stmt {
    Set(String, Expr),
    SetIndex(Expr, Expr, Expr),
    If(Expr, Vec<Sn>, Vec<Sn>),
    While(Expr, Vec<Sn>),
    For(String, Expr, Expr, Vec<Sn>),
    ForIn(String, Expr, Vec<Sn>),
    Break,
    Continue,
    Return(Option<Expr>),
    Eval(Expr),
}

struct Func {
    name: String,
    params: Vec<(String, Option<T>)>,
    ret: Option<T>,
    body: Vec<Sn>,
    line: usize,
}

struct P {
    t: Vec<(Tok, usize)>,
    i: usize,
    consts: HashMap<String, f64>,
}

fn pure(e: &Expr) -> bool {
    match e {
        Expr::Num(_) | Expr::Str(_) | Expr::Var(_) => true,
        Expr::List(v) => v.iter().all(pure),
        Expr::Neg(x) | Expr::Not(x) => pure(x),
        Expr::Bin(_, a, b) | Expr::Index(a, b) => pure(a) && pure(b),
        Expr::Call(..) => false,
    }
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
    fn type_ann(&mut self) -> Result<T, String> {
        let line = self.line();
        let n = self.ident()?;
        type_name(&n).ok_or_else(|| format!("Line {}: the type `{}` cannot be compiled (use number, string or array)", line, n))
    }

    fn block(&mut self) -> Result<Vec<Sn>, String> {
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
            let line = self.line();
            out.push((line, self.stmt()?));
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
            if !self.word("in") {
                return Err(format!("Line {}: expected `in`", line));
            }
            if matches!(self.peek(), Some(Tok::Id(x)) if x == "range") && matches!(self.t.get(self.i + 1), Some((Tok::Sym("("), _))) {
                self.i += 1;
                self.expect("(")?;
                let first = self.expr()?;
                let (lo, hi) = if self.sym(",") { (first, self.expr()?) } else { (Expr::Num(0.0), first) };
                self.expect(")")?;
                return Ok(Stmt::For(v, lo, hi, self.block()?));
            }
            let it = self.expr()?;
            return Ok(Stmt::ForIn(v, it, self.block()?));
        }
        if self.word("break") {
            return Ok(Stmt::Break);
        }
        if self.word("continue") {
            return Ok(Stmt::Continue);
        }
        if self.word("return") {
            if matches!(self.peek(), Some(Tok::Nl) | Some(Tok::Sym("}")) | Some(Tok::Sym(";")) | None) {
                return Ok(Stmt::Return(None));
            }
            return Ok(Stmt::Return(Some(self.expr()?)));
        }
        // assignment forms: target = e, target op= e, target++, target--   (target: a name or xs[i])
        let target = self.expr()?;
        let op = match self.peek() {
            Some(Tok::Sym(op)) if matches!(*op, "=" | "+=" | "-=" | "*=" | "/=" | "++" | "--") => Some(*op),
            _ => None,
        };
        let op = match op {
            Some(op) => op,
            None => return Ok(Stmt::Eval(target)),
        };
        self.i += 1;
        let (bop, rhs): (Option<&'static str>, Expr) = match op {
            "=" => (None, self.expr()?),
            "++" => (Some("+"), Expr::Num(1.0)),
            "--" => (Some("-"), Expr::Num(1.0)),
            "+=" => (Some("+"), self.expr()?),
            "-=" => (Some("-"), self.expr()?),
            "*=" => (Some("*"), self.expr()?),
            _ => (Some("/"), self.expr()?),
        };
        match target {
            Expr::Var(name) => {
                let v = match bop {
                    None => rhs,
                    Some(o) => Expr::Bin(o, Box::new(Expr::Var(name.clone())), Box::new(rhs)),
                };
                Ok(Stmt::Set(name, v))
            }
            Expr::Index(b, i) => {
                if bop.is_some() && !(pure(&b) && pure(&i)) {
                    return Err(format!("Line {}: put the list and index of a compound assignment in variables first (they are evaluated twice)", line));
                }
                let v = match bop {
                    None => rhs,
                    Some(o) => Expr::Bin(o, Box::new(Expr::Index(Box::new(clone_expr(&b)), Box::new(clone_expr(&i)))), Box::new(rhs)),
                };
                Ok(Stmt::SetIndex(*b, *i, v))
            }
            _ => Err(format!("Line {}: only a variable or an element xs[i] can be assigned", line)),
        }
    }

    fn if_rest(&mut self) -> Result<Stmt, String> {
        let c = self.expr()?;
        let then = self.block()?;
        let save = self.i;
        self.skip_nl();
        let els = if self.word("else") {
            if self.word("if") {
                let l = self.line();
                vec![(l, self.if_rest()?)]
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
        let mut e = self.primary()?;
        while self.sym("[") {
            let i = self.expr()?;
            self.expect("]")?;
            e = Expr::Index(Box::new(e), Box::new(i));
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let line = self.line();
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Str(s)) => {
                self.i += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::Sym("(")) => {
                self.i += 1;
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Sym("[")) => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip_nl();
                if !self.sym("]") {
                    loop {
                        self.skip_nl();
                        items.push(self.expr()?);
                        self.skip_nl();
                        if self.sym("]") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(Expr::List(items))
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
            _ => Err(format!("Line {}: this expression cannot be compiled to WebAssembly (no dicts, sets, lambdas or method calls)", line)),
        }
    }
}

fn clone_expr(e: &Expr) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(*n),
        Expr::Str(s) => Expr::Str(s.clone()),
        Expr::List(v) => Expr::List(v.iter().map(clone_expr).collect()),
        Expr::Var(v) => Expr::Var(v.clone()),
        Expr::Neg(x) => Expr::Neg(Box::new(clone_expr(x))),
        Expr::Not(x) => Expr::Not(Box::new(clone_expr(x))),
        Expr::Bin(o, a, b) => Expr::Bin(o, Box::new(clone_expr(a)), Box::new(clone_expr(b))),
        Expr::Index(a, b) => Expr::Index(Box::new(clone_expr(a)), Box::new(clone_expr(b))),
        Expr::Call(n, a, l) => Expr::Call(n.clone(), a.iter().map(clone_expr).collect(), *l),
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
                let n = p.ident()?;
                let t = if p.sym(":") { Some(p.type_ann()?) } else { None };
                params.push((n, t));
                if p.sym(")") {
                    break;
                }
                p.expect(",")?;
            }
        }
        let ret = if p.sym("->") { Some(p.type_ann()?) } else { None };
        let body = p.block()?;
        funcs.push(Func { name, params, ret, body, line });
    }
    if funcs.is_empty() {
        return Err("no functions to compile".to_string());
    }
    Ok(funcs)
}

// ---------------------------------------------------------------------------------------------
// type inference

struct Sig {
    params: Vec<Option<T>>,
    ret: Option<T>,
}

struct Typer {
    sigs: Vec<Sig>,
    names: Vec<Vec<String>>,
    idx: HashMap<String, usize>,
    vars: Vec<HashMap<String, Option<T>>>,
    cur: usize,
    line: usize,
    changed: bool,
}

type R<X> = Result<X, String>;

impl Typer {
    fn fail<X>(&self, m: String) -> R<X> {
        Err(format!("Line {}: {}", self.line, m))
    }
    fn var_ty(&self, name: &str) -> Option<Option<T>> {
        match self.names[self.cur].iter().position(|n| n == name) {
            Some(p) => Some(self.sigs[self.cur].params[p]),
            None => self.vars[self.cur].get(name).copied(),
        }
    }
    fn learn(&mut self, name: &str, t: T) -> R<()> {
        let cur = self.var_ty(name);
        match cur {
            Some(Some(c)) if c != t => self.fail(format!("`{}` is a {} elsewhere but a {} here (a compiled variable has one type)", name, c.name(), t.name())),
            Some(Some(_)) => Ok(()),
            _ => {
                match self.names[self.cur].iter().position(|n| n == name) {
                    Some(p) => self.sigs[self.cur].params[p] = Some(t),
                    None => {
                        self.vars[self.cur].insert(name.to_string(), Some(t));
                    }
                }
                self.changed = true;
                Ok(())
            }
        }
    }
    fn learn_ret(&mut self, fi: usize, t: T) -> R<()> {
        match self.sigs[fi].ret {
            Some(c) if c != t => self.fail(format!("the function returns a {} elsewhere but a {} here", c.name(), t.name())),
            Some(_) => Ok(()),
            None => {
                self.sigs[fi].ret = Some(t);
                self.changed = true;
                Ok(())
            }
        }
    }

    fn expr(&mut self, e: &Expr, want: Option<T>) -> R<Option<T>> {
        let r: Option<T> = match e {
            Expr::Num(_) => Some(T::Num),
            Expr::Str(_) => Some(T::Str),
            Expr::List(items) => {
                for it in items {
                    self.expr(it, Some(T::Num))?;
                }
                Some(T::List)
            }
            Expr::Var(v) => match self.var_ty(v) {
                None => None,
                Some(Some(t)) => Some(t),
                Some(None) => match want {
                    Some(w) => {
                        self.learn(v, w)?;
                        Some(w)
                    }
                    None => None,
                },
            },
            Expr::Neg(x) | Expr::Not(x) => {
                self.expr(x, Some(T::Num))?;
                Some(T::Num)
            }
            Expr::Bin(op, l, r) => match *op {
                "+" => {
                    let tl = self.expr(l, None)?;
                    let tr = self.expr(r, None)?;
                    let t = match (tl, tr) {
                        (Some(a), Some(b)) if a == b => Some(a),
                        (Some(a), Some(b)) => return self.fail(format!("cannot add a {} and a {} (convert first; str() is not available)", a.name(), b.name())),
                        (Some(a), None) => {
                            self.expr(r, Some(a))?;
                            Some(a)
                        }
                        (None, Some(b)) => {
                            self.expr(l, Some(b))?;
                            Some(b)
                        }
                        (None, None) => match want {
                            Some(w) => {
                                self.expr(l, Some(w))?;
                                self.expr(r, Some(w))?;
                                Some(w)
                            }
                            None => None,
                        },
                    };
                    if t == Some(T::List) {
                        return self.fail("lists cannot be added; use push(xs, n)".to_string());
                    }
                    t
                }
                "==" | "!=" => {
                    let tl = self.expr(l, None)?;
                    let tr = self.expr(r, None)?;
                    match (tl, tr) {
                        (Some(a), Some(b)) if a != b => return self.fail(format!("cannot compare a {} with a {}", a.name(), b.name())),
                        (Some(T::List), _) => return self.fail("lists cannot be compared in compiled code".to_string()),
                        (Some(a), None) => {
                            self.expr(r, Some(a))?;
                        }
                        (None, Some(b)) => {
                            self.expr(l, Some(b))?;
                        }
                        _ => {}
                    }
                    Some(T::Num)
                }
                _ => {
                    self.expr(l, Some(T::Num))?;
                    self.expr(r, Some(T::Num))?;
                    Some(T::Num)
                }
            },
            Expr::Index(b, i) => {
                let mut tb = self.expr(b, None)?;
                self.expr(i, Some(T::Num))?;
                if tb.is_none() {
                    tb = match want {
                        Some(T::Num) => {
                            self.expr(b, Some(T::List))?;
                            Some(T::List)
                        }
                        Some(T::Str) => {
                            self.expr(b, Some(T::Str))?;
                            Some(T::Str)
                        }
                        _ => None,
                    };
                }
                match tb {
                    Some(T::List) => Some(T::Num),
                    Some(T::Str) => Some(T::Str),
                    Some(T::Num) => return self.fail("a number cannot be indexed (if this is a parameter, annotate it: `xs: array` or `s: string`)".to_string()),
                    None => None,
                }
            }
            Expr::Call(name, args, line) => {
                let n = args.len();
                match name.as_str() {
                    "len" => {
                        if n != 1 {
                            return Err(format!("Line {}: len() takes one argument", line));
                        }
                        if self.expr(&args[0], None)? == Some(T::Num) {
                            return self.fail("len() needs a string or a list (annotate the parameter, for example `xs: array` or `s: string`)".to_string());
                        }
                        Some(T::Num)
                    }
                    "push" => {
                        if n != 2 {
                            return Err(format!("Line {}: push() takes a list and a number", line));
                        }
                        self.expr(&args[0], Some(T::List))?;
                        self.expr(&args[1], Some(T::Num))?;
                        Some(T::List)
                    }
                    "sqrt" | "abs" | "floor" | "ceil" | "trunc" | "min" | "max" => {
                        let want_n = if matches!(name.as_str(), "min" | "max") { 2 } else { 1 };
                        if n != want_n {
                            return Err(format!("Line {}: {}() takes {} argument{}", line, name, if want_n == 1 { "one" } else { "two" }, if want_n == 1 { "" } else { "s" }));
                        }
                        for a in args {
                            self.expr(a, Some(T::Num))?;
                        }
                        Some(T::Num)
                    }
                    _ => {
                        let fi = match self.idx.get(name) {
                            Some(&fi) => fi,
                            None => {
                                return Err(format!(
                                    "Line {}: `{}` is not a function of this file or a supported builtin (len push sqrt abs floor ceil trunc min max)",
                                    line, name
                                ))
                            }
                        };
                        if self.sigs[fi].params.len() != n {
                            return Err(format!("Line {}: {}() takes {} argument(s), got {}", line, name, self.sigs[fi].params.len(), n));
                        }
                        for (k, a) in args.iter().enumerate() {
                            match self.sigs[fi].params[k] {
                                Some(p) => {
                                    self.expr(a, Some(p))?;
                                }
                                None => {
                                    if let Some(t) = self.expr(a, None)? {
                                        self.sigs[fi].params[k] = Some(t);
                                        self.changed = true;
                                    }
                                }
                            }
                        }
                        if self.sigs[fi].ret.is_none() {
                            if let Some(w) = want {
                                self.sigs[fi].ret = Some(w);
                                self.changed = true;
                            }
                        }
                        self.sigs[fi].ret
                    }
                }
            }
        };
        if let (Some(r), Some(w)) = (r, want) {
            if r != w {
                return self.fail(format!("expected a {} here but this is a {}", w.name(), r.name()));
            }
        }
        Ok(r)
    }

    fn stmts(&mut self, b: &[Sn]) -> R<()> {
        for (line, s) in b {
            self.line = *line;
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> R<()> {
        match s {
            Stmt::Set(name, e) => {
                let want = self.var_ty(name).flatten();
                match self.expr(e, want)? {
                    Some(t) => self.learn(name, t)?,
                    None => {
                        if self.var_ty(name).is_none() {
                            let cur = self.cur;
                            self.vars[cur].insert(name.clone(), None);
                        }
                    }
                }
            }
            Stmt::SetIndex(b, i, v) => {
                self.expr(b, Some(T::List))?;
                self.expr(i, Some(T::Num))?;
                self.expr(v, Some(T::Num))?;
            }
            Stmt::If(c, t, f) => {
                self.expr(c, Some(T::Num))?;
                self.stmts(t)?;
                self.stmts(f)?;
            }
            Stmt::While(c, b) => {
                self.expr(c, Some(T::Num))?;
                self.stmts(b)?;
            }
            Stmt::For(v, lo, hi, b) => {
                self.expr(lo, Some(T::Num))?;
                self.expr(hi, Some(T::Num))?;
                self.learn(v, T::Num)?;
                self.stmts(b)?;
            }
            Stmt::ForIn(v, e, b) => {
                match self.expr(e, None)? {
                    Some(T::List) => self.learn(v, T::Num)?,
                    Some(T::Str) => self.learn(v, T::Str)?,
                    Some(T::Num) => return self.fail("`for x in ...` needs a list or a string here (if it is a parameter, annotate it: `xs: array`)".to_string()),
                    None => {}
                }
                self.stmts(b)?;
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    let want = self.sigs[self.cur].ret;
                    if let Some(t) = self.expr(e, want)? {
                        let cur = self.cur;
                        self.learn_ret(cur, t)?;
                    }
                }
            }
            Stmt::Eval(e) => {
                self.expr(e, None)?;
            }
            Stmt::Break | Stmt::Continue => {}
        }
        Ok(())
    }

    fn infer(&mut self, funcs: &[Func]) -> R<()> {
        loop {
            for _ in 0..100 {
                self.changed = false;
                for (fi, f) in funcs.iter().enumerate() {
                    self.cur = fi;
                    self.line = f.line;
                    self.stmts(&f.body)?;
                }
                if !self.changed {
                    break;
                }
            }
            // whatever is still unknown has no constraint: it is a number
            let mut any = false;
            for fi in 0..self.sigs.len() {
                for p in 0..self.sigs[fi].params.len() {
                    if self.sigs[fi].params[p].is_none() {
                        self.sigs[fi].params[p] = Some(T::Num);
                        any = true;
                    }
                }
                if self.sigs[fi].ret.is_none() {
                    self.sigs[fi].ret = Some(T::Num);
                    any = true;
                }
                for v in self.vars[fi].values_mut() {
                    if v.is_none() {
                        *v = Some(T::Num);
                        any = true;
                    }
                }
            }
            if !any {
                return Ok(());
            }
        }
    }
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

/// String literals, placed in linear memory from address 8 (address 0 is the empty string / null).
struct Data {
    bytes: Vec<u8>,
    map: HashMap<String, u32>,
}

const DATA_BASE: u32 = 8;

impl Data {
    fn add(&mut self, s: &str) -> u32 {
        if let Some(a) = self.map.get(s) {
            return *a;
        }
        let at = DATA_BASE + self.bytes.len() as u32;
        self.bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(s.as_bytes());
        while self.bytes.len() % 4 != 0 {
            self.bytes.push(0);
        }
        self.map.insert(s.to_string(), at);
        at
    }
}

/// The runtime helpers appended after the user's functions (their function index is `nuser + H as u32`).
#[derive(Clone, Copy)]
enum H {
    Alloc,
    StrNew,
    Copy,
    Concat,
    StrEq,
    Idx,
    StrIndex,
    ListNew,
    ListPush,
    ListGet,
    ListSet,
    Len,
}
const NHELPERS: usize = 12;

struct Gen<'a> {
    code: Vec<u8>,
    locals: HashMap<String, u32>,
    ltypes: Vec<T>,
    ty: &'a mut Typer,
    data: &'a mut Data,
    nuser: u32,
    fi: usize,
    ret: T,
    depth: u32,
    loops: Vec<(u32, u32)>, // (depth of the break block, depth of the continue block)
    temps: u32,
    line: usize,
    heap: bool,
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
    fn local(&mut self, name: &str, t: T) -> u32 {
        if let Some(i) = self.locals.get(name) {
            return *i;
        }
        let i = self.ltypes.len() as u32;
        self.locals.insert(name.to_string(), i);
        self.ltypes.push(t);
        i
    }
    fn temp(&mut self, t: T) -> u32 {
        self.temps += 1;
        let name = format!("%tmp{}", self.temps);
        self.local(&name, t)
    }
    /// The type of an expression (the inference pass has already settled every type).
    fn ty(&mut self, e: &Expr) -> Result<T, String> {
        self.ty.cur = self.fi;
        self.ty.line = self.line;
        match self.ty.expr(e, None)? {
            Some(t) => Ok(t),
            None => Err("a value is used before it is assigned (or its type cannot be worked out)".to_string()),
        }
    }
    fn f64c(&mut self, v: f64) {
        self.code.push(0x44);
        self.code.extend_from_slice(&v.to_le_bytes());
    }
    fn i32c(&mut self, v: i32) {
        self.code.push(0x41);
        sleb(v as i64, &mut self.code);
    }
    fn get(&mut self, i: u32) {
        self.code.push(0x20);
        uleb(i as u64, &mut self.code);
    }
    fn set(&mut self, i: u32) {
        self.code.push(0x21);
        uleb(i as u64, &mut self.code);
    }
    fn call_h(&mut self, h: H) {
        self.heap = true;
        self.code.push(0x10);
        uleb((self.nuser + h as u32) as u64, &mut self.code);
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
    /// The default value of a type (a function that falls off its end, a bare `return`).
    fn default_value(&mut self, t: T) {
        match t {
            T::Num => self.f64c(0.0),
            T::Str => self.i32c(0),
            T::List => self.call_h(H::ListNew),
        }
    }

    /// Leaves the value of `e` (an f64 for a number, an i32 pointer for a string or list) on the stack.
    fn value(&mut self, e: &Expr) -> Result<(), String> {
        if is_cond(e) {
            self.cond(e)?;
            self.code.push(0xb8); // f64.convert_i32_u
            return Ok(());
        }
        match e {
            Expr::Num(n) => self.f64c(*n),
            Expr::Str(s) => {
                self.heap = true;
                let a = self.data.add(s);
                self.i32c(a as i32);
            }
            Expr::List(items) => {
                self.call_h(H::ListNew);
                for it in items {
                    self.value(it)?;
                    self.call_h(H::ListPush);
                }
            }
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
            Expr::Index(b, i) => {
                let tb = self.ty(b)?;
                self.value(b)?;
                self.value(i)?;
                self.call_h(if tb == T::Str { H::StrIndex } else { H::ListGet });
            }
            Expr::Bin(op, l, r) => {
                if *op == "+" && self.ty(e)? == T::Str {
                    self.value(l)?;
                    self.value(r)?;
                    self.call_h(H::Concat);
                } else if *op == "%" {
                    // a % b = a - floor(a / b) * b (floor modulo, the sign of the divisor, like the Killer VM)
                    let (ta, tb) = (self.temp(T::Num), self.temp(T::Num));
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
            Expr::Call(name, args, _) => {
                if name == "len" {
                    self.value(&args[0])?;
                    self.call_h(H::Len);
                    self.code.push(0xb8); // f64.convert_i32_u
                } else if name == "push" {
                    self.value(&args[0])?;
                    self.value(&args[1])?;
                    self.call_h(H::ListPush);
                } else if let Some((_, op)) = MATH1.iter().find(|(n, _)| n == name) {
                    self.value(&args[0])?;
                    self.code.push(*op);
                } else if let Some((_, op)) = MATH2.iter().find(|(n, _)| n == name) {
                    self.value(&args[0])?;
                    self.value(&args[1])?;
                    self.code.push(*op);
                } else {
                    let idx = self.ty.idx[name];
                    for a in args {
                        self.value(a)?;
                    }
                    self.code.push(0x10);
                    uleb(idx as u64, &mut self.code);
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
                    self.i32c(0);
                } else {
                    self.i32c(1);
                    self.code.push(0x05);
                    self.cond(r)?;
                }
                self.close();
            }
            Expr::Bin(op, l, r) if matches!(*op, "==" | "!=") && self.ty(l)? == T::Str => {
                self.value(l)?;
                self.value(r)?;
                self.call_h(H::StrEq);
                if *op == "!=" {
                    self.code.push(0x45);
                }
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

    fn block(&mut self, stmts: &[Sn]) -> Result<(), String> {
        for s in stmts {
            self.line = s.0;
            self.stmt(&s.1).map_err(|e| if e.starts_with("Line ") { e } else { format!("Line {}: {}", s.0, e) })?;
        }
        Ok(())
    }

    /// `for v in <list or string>`: the length is read once, before the loop.
    fn for_in(&mut self, v: &str, e: &Expr, body: &[Sn]) -> Result<(), String> {
        let te = self.ty(e)?;
        let holder = self.temp(te);
        let (k, n) = (self.temp(T::Str), self.temp(T::Str)); // i32 counters
        self.value(e)?;
        self.set(holder);
        self.get(holder);
        self.call_h(H::Len);
        self.set(n);
        self.i32c(0);
        self.set(k);
        let var = self.local(v, if te == T::Str { T::Str } else { T::Num });
        self.open(0x02, 0x40);
        let brk = self.depth;
        self.open(0x03, 0x40);
        let top = self.depth;
        self.get(k);
        self.get(n);
        self.code.push(0x4f); // i32.ge_u
        self.br(brk, true);
        self.get(holder);
        self.get(k);
        self.code.push(0xb8);
        self.call_h(if te == T::Str { H::StrIndex } else { H::ListGet });
        self.set(var);
        self.open(0x02, 0x40);
        let cont = self.depth;
        self.loops.push((brk, cont));
        self.block(body)?;
        self.loops.pop();
        self.close();
        self.get(k);
        self.i32c(1);
        self.code.push(0x6a);
        self.set(k);
        self.br(top, false);
        self.close();
        self.close();
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Set(name, e) => {
                // a name is a local the first time it is assigned (the right side may not use it before then)
                let t = self.ty(e)?;
                self.value(e)?;
                let i = self.local(name, t);
                self.set(i);
            }
            Stmt::SetIndex(b, i, v) => {
                self.value(b)?;
                self.value(i)?;
                self.value(v)?;
                self.call_h(H::ListSet);
            }
            Stmt::Eval(e) => {
                self.value(e)?;
                self.code.push(0x1a); // drop
            }
            Stmt::Return(e) => {
                match e {
                    Some(e) => self.value(e)?,
                    None => self.default_value(self.ret),
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
                let end = self.temp(T::Num);
                self.value(hi)?;
                self.set(end);
                self.value(lo)?;
                let i = self.local(v, T::Num);
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
            Stmt::ForIn(v, e, body) => self.for_in(v, e, body)?,
            Stmt::Break => match self.loops.last() {
                Some(&(b, _)) => self.br(b, false),
                None => return Err("`break` outside a loop".to_string()),
            },
            Stmt::Continue => match self.loops.last() {
                Some(&(_, c)) => self.br(c, false),
                None => return Err("`continue` outside a loop".to_string()),
            },
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// runtime helpers, written as raw WebAssembly

struct Asm {
    c: Vec<u8>,
}

impl Asm {
    fn op(&mut self, b: u8) -> &mut Self {
        self.c.push(b);
        self
    }
    fn lg(&mut self, i: u32) -> &mut Self {
        self.c.push(0x20);
        uleb(i as u64, &mut self.c);
        self
    }
    fn ls(&mut self, i: u32) -> &mut Self {
        self.c.push(0x21);
        uleb(i as u64, &mut self.c);
        self
    }
    fn ic(&mut self, v: i32) -> &mut Self {
        self.c.push(0x41);
        sleb(v as i64, &mut self.c);
        self
    }
    fn call(&mut self, f: u32) -> &mut Self {
        self.c.push(0x10);
        uleb(f as u64, &mut self.c);
        self
    }
    fn mem(&mut self, op: u8, align: u8, off: u32) -> &mut Self {
        self.c.push(op);
        self.c.push(align);
        uleb(off as u64, &mut self.c);
        self
    }
    fn ld32(&mut self, off: u32) -> &mut Self {
        self.mem(0x28, 2, off)
    }
    fn st32(&mut self, off: u32) -> &mut Self {
        self.mem(0x36, 2, off)
    }
    fn ld8(&mut self, off: u32) -> &mut Self {
        self.mem(0x2d, 0, off)
    }
    fn st8(&mut self, off: u32) -> &mut Self {
        self.mem(0x3a, 0, off)
    }
    fn ldf(&mut self) -> &mut Self {
        self.mem(0x2b, 3, 0)
    }
    fn stf(&mut self) -> &mut Self {
        self.mem(0x39, 3, 0)
    }
    fn begin(&mut self, op: u8) -> &mut Self {
        self.c.push(op);
        self.c.push(0x40);
        self
    }
    fn end(&mut self) -> &mut Self {
        self.c.push(0x0b);
        self
    }
    fn br(&mut self, n: u32) -> &mut Self {
        self.c.push(0x0c);
        uleb(n as u64, &mut self.c);
        self
    }
    fn br_if(&mut self, n: u32) -> &mut Self {
        self.c.push(0x0d);
        uleb(n as u64, &mut self.c);
        self
    }
}

struct Helper {
    params: Vec<u8>,
    result: Option<u8>,
    locals: Vec<u8>,
    code: Vec<u8>,
}

const I: u8 = 0x7f;
const F: u8 = 0x7c;

fn helpers(nuser: u32) -> Vec<Helper> {
    let h = |x: H| nuser + x as u32;
    let mut v = Vec::new();
    // Alloc(n) -> ptr: bump allocator, 8-byte aligned, grows the memory when needed (traps if it cannot)
    let mut a = Asm { c: Vec::new() };
    a.op(0x23).c.push(0); // global.get 0 (the heap pointer)
    a.ls(1);
    a.lg(0).ic(7).op(0x6a).ic(-8).op(0x71).ls(0);
    a.lg(1).lg(0).op(0x6a).op(0x24).c.push(0); // global.set 0
    a.op(0x23).c.push(0);
    a.op(0x3f).c.push(0); // memory.size
    a.ic(16).op(0x74).op(0x4b); // heap > size * 65536 (unsigned)
    a.begin(0x04);
    a.op(0x23).c.push(0);
    a.op(0x3f).c.push(0);
    a.ic(16).op(0x74).op(0x6b);
    a.ic(65535).op(0x6a).ic(16).op(0x76);
    a.op(0x40).c.push(0); // memory.grow
    a.ic(-1).op(0x46);
    a.begin(0x04).op(0x00).end(); // grow failed: trap
    a.end();
    a.lg(1).end();
    v.push(Helper { params: vec![I], result: Some(I), locals: vec![I], code: a.c });
    // StrNew(len) -> ptr (the length is stored, the bytes are the caller's)
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ic(4).op(0x6a).call(h(H::Alloc)).ls(1);
    a.lg(1).lg(0).st32(0);
    a.lg(1).end();
    v.push(Helper { params: vec![I], result: Some(I), locals: vec![I], code: a.c });
    // Copy(dst, src, n)
    let mut a = Asm { c: Vec::new() };
    a.begin(0x02).begin(0x03);
    a.lg(3).lg(2).op(0x4f).br_if(1);
    a.lg(0).lg(3).op(0x6a);
    a.lg(1).lg(3).op(0x6a).ld8(0);
    a.st8(0);
    a.lg(3).ic(1).op(0x6a).ls(3);
    a.br(0);
    a.end().end().end();
    v.push(Helper { params: vec![I, I, I], result: None, locals: vec![I], code: a.c });
    // Concat(a, b) -> ptr
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).ls(2);
    a.lg(1).ld32(0).ls(3);
    a.lg(2).lg(3).op(0x6a).call(h(H::StrNew)).ls(4);
    a.lg(4).ic(4).op(0x6a).lg(0).ic(4).op(0x6a).lg(2).call(h(H::Copy));
    a.lg(4).ic(4).op(0x6a).lg(2).op(0x6a).lg(1).ic(4).op(0x6a).lg(3).call(h(H::Copy));
    a.lg(4).end();
    v.push(Helper { params: vec![I, I], result: Some(I), locals: vec![I, I, I], code: a.c });
    // StrEq(a, b) -> 0/1
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).ls(2);
    a.lg(2).lg(1).ld32(0).op(0x47).begin(0x04).ic(0).op(0x0f).end();
    a.begin(0x02).begin(0x03);
    a.lg(3).lg(2).op(0x4f).br_if(1);
    a.lg(0).lg(3).op(0x6a).ld8(4);
    a.lg(1).lg(3).op(0x6a).ld8(4);
    a.op(0x47).begin(0x04).ic(0).op(0x0f).end();
    a.lg(3).ic(1).op(0x6a).ls(3);
    a.br(0);
    a.end().end();
    a.ic(1).end();
    v.push(Helper { params: vec![I, I], result: Some(I), locals: vec![I, I], code: a.c });
    // Idx(len, k: f64) -> i32 index (negative counts from the end; out of range traps)
    let mut a = Asm { c: Vec::new() };
    a.lg(1).op(0xaa).ls(2);
    a.lg(2).ic(0).op(0x48).begin(0x04).lg(2).lg(0).op(0x6a).ls(2).end();
    a.lg(2).lg(0).op(0x4f).begin(0x04).op(0x00).end();
    a.lg(2).end();
    v.push(Helper { params: vec![I, F], result: Some(I), locals: vec![I], code: a.c });
    // StrIndex(s, k: f64) -> one-character string
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).lg(1).call(h(H::Idx)).ls(2);
    a.ic(1).call(h(H::StrNew)).ls(3);
    a.lg(3).lg(0).lg(2).op(0x6a).ld8(4).st8(4);
    a.lg(3).end();
    v.push(Helper { params: vec![I, F], result: Some(I), locals: vec![I, I], code: a.c });
    // ListNew() -> ptr (16 bytes: len, capacity, data pointer; fresh memory is zero)
    let mut a = Asm { c: Vec::new() };
    a.ic(16).call(h(H::Alloc)).end();
    v.push(Helper { params: vec![], result: Some(I), locals: vec![], code: a.c });
    // ListPush(l, x: f64) -> l
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).ls(2);
    a.lg(0).ld32(4).ls(3);
    a.lg(2).lg(3).op(0x46).begin(0x04);
    a.ic(4).lg(3).ic(1).op(0x74).lg(3).op(0x45).op(0x1b).ls(3); // new capacity: 4, or double
    a.lg(3).ic(3).op(0x74).call(h(H::Alloc)).ls(4);
    a.lg(4).lg(0).ld32(8).lg(2).ic(3).op(0x74).call(h(H::Copy));
    a.lg(0).lg(4).st32(8);
    a.lg(0).lg(3).st32(4);
    a.end();
    a.lg(0).ld32(8).lg(2).ic(3).op(0x74).op(0x6a).lg(1).stf();
    a.lg(0).lg(2).ic(1).op(0x6a).st32(0);
    a.lg(0).end();
    v.push(Helper { params: vec![I, F], result: Some(I), locals: vec![I, I, I], code: a.c });
    // ListGet(l, k: f64) -> f64
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).lg(1).call(h(H::Idx)).ls(2);
    a.lg(0).ld32(8).lg(2).ic(3).op(0x74).op(0x6a).ldf().end();
    v.push(Helper { params: vec![I, F], result: Some(F), locals: vec![I], code: a.c });
    // ListSet(l, k: f64, x: f64)
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).lg(1).call(h(H::Idx)).ls(3);
    a.lg(0).ld32(8).lg(3).ic(3).op(0x74).op(0x6a).lg(2).stf().end();
    v.push(Helper { params: vec![I, F, F], result: None, locals: vec![I], code: a.c });
    // Len(p) -> the length stored at the start of a string or a list
    let mut a = Asm { c: Vec::new() };
    a.lg(0).ld32(0).end();
    v.push(Helper { params: vec![I], result: Some(I), locals: vec![], code: a.c });
    assert_eq!(v.len(), NHELPERS);
    v
}

// ---------------------------------------------------------------------------------------------

/// One exported function: its name, parameter names with types and the result type.
pub struct Export {
    pub name: String,
    pub params: Vec<(String, &'static str)>,
    pub ret: &'static str,
}

fn type_key(params: &[u8], result: Option<u8>) -> Vec<u8> {
    let mut k = params.to_vec();
    k.push(0xff);
    if let Some(r) = result {
        k.push(r);
    }
    k
}

/// Compile `source` to a WebAssembly module; returns the bytes and the exported functions with their types.
pub fn compile_full(source: &str) -> Result<(Vec<u8>, Vec<Export>), String> {
    let funcs = parse(source)?;
    let mut idx: HashMap<String, usize> = HashMap::new();
    for (i, f) in funcs.iter().enumerate() {
        if idx.insert(f.name.clone(), i).is_some() {
            return Err(format!("Line {}: the function {} is defined twice", f.line, f.name));
        }
        for (k, (p, _)) in f.params.iter().enumerate() {
            if f.params[..k].iter().any(|(q, _)| q == p) {
                return Err(format!("Line {}: parameter {} appears twice", f.line, p));
            }
        }
    }
    let mut typer = Typer {
        sigs: funcs.iter().map(|f| Sig { params: f.params.iter().map(|p| p.1).collect(), ret: f.ret }).collect(),
        names: funcs.iter().map(|f| f.params.iter().map(|p| p.0.clone()).collect()).collect(),
        idx: idx.clone(),
        vars: funcs.iter().map(|_| HashMap::new()).collect(),
        cur: 0,
        line: 1,
        changed: false,
    };
    typer.infer(&funcs)?;
    let nuser = funcs.len() as u32;
    let mut data = Data { bytes: Vec::new(), map: HashMap::new() };
    let mut bodies: Vec<(Vec<u8>, Vec<u8>, Option<u8>)> = Vec::new(); // (code with locals, param types, result type)
    let mut heap = false;
    let mut exports: Vec<Export> = Vec::new();
    for (fi, f) in funcs.iter().enumerate() {
        let sig_params: Vec<T> = typer.sigs[fi].params.iter().map(|p| p.unwrap()).collect();
        let ret = typer.sigs[fi].ret.unwrap();
        let mut g = Gen {
            code: Vec::new(),
            locals: HashMap::new(),
            ltypes: Vec::new(),
            ty: &mut typer,
            data: &mut data,
            nuser,
            fi,
            ret,
            depth: 0,
            loops: Vec::new(),
            temps: 0,
            line: f.line,
            heap: false,
        };
        for (k, (p, _)) in f.params.iter().enumerate() {
            g.locals.insert(p.clone(), k as u32);
            g.ltypes.push(sig_params[k]);
        }
        g.block(&f.body).map_err(|e| if e.starts_with("Line ") { e } else { format!("Line {}: in function {}: {}", f.line, f.name, e) })?;
        g.default_value(ret); // falling off the end gives 0 / the empty string / an empty list
        g.code.push(0x0b);
        let mut body = Vec::new();
        let extra = g.ltypes.len() - f.params.len();
        uleb(extra as u64, &mut body);
        for t in &g.ltypes[f.params.len()..] {
            uleb(1, &mut body);
            body.push(t.vt());
        }
        body.extend_from_slice(&g.code);
        let mut sized = Vec::new();
        uleb(body.len() as u64, &mut sized);
        sized.extend_from_slice(&body);
        heap |= g.heap || sig_params.iter().any(|t| *t != T::Num) || ret != T::Num || g.ltypes.iter().any(|t| *t != T::Num);
        bodies.push((sized, sig_params.iter().map(|t| t.vt()).collect(), Some(ret.vt())));
        exports.push(Export { name: f.name.clone(), params: f.params.iter().zip(&sig_params).map(|(p, t)| (p.0.clone(), t.name())).collect(), ret: ret.name() });
    }
    let hs = helpers(nuser);
    let mut host_exports: Vec<(&str, u32)> = Vec::new();
    if heap {
        for f in &funcs {
            if f.name == "memory" || f.name.starts_with("killer_") {
                return Err(format!("Line {}: the function name {} is reserved for the memory and host helpers of a module that uses strings or lists", f.line, f.name));
            }
        }
        host_exports = vec![
            ("killer_alloc", nuser + H::Alloc as u32),
            ("killer_str_new", nuser + H::StrNew as u32),
            ("killer_str_len", nuser + H::Len as u32),
            ("killer_list_new", nuser + H::ListNew as u32),
            ("killer_list_len", nuser + H::Len as u32),
            ("killer_list_get", nuser + H::ListGet as u32),
            ("killer_list_set", nuser + H::ListSet as u32),
            ("killer_list_push", nuser + H::ListPush as u32),
        ];
        for h in &hs {
            let mut sized = Vec::new();
            let mut body = Vec::new();
            uleb(h.locals.len() as u64, &mut body);
            for t in &h.locals {
                uleb(1, &mut body);
                body.push(*t);
            }
            body.extend_from_slice(&h.code);
            uleb(body.len() as u64, &mut sized);
            sized.extend_from_slice(&body);
            bodies.push((sized, h.params.clone(), h.result));
        }
    }
    // types: one per distinct signature
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut tyidx: Vec<usize> = Vec::new();
    for (_, params, result) in &bodies {
        let k = type_key(params, *result);
        let at = match keys.iter().position(|x| *x == k) {
            Some(p) => p,
            None => {
                keys.push(k);
                keys.len() - 1
            }
        };
        tyidx.push(at);
    }
    let mut m: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let section = |id: u8, content: Vec<u8>, m: &mut Vec<u8>| {
        m.push(id);
        uleb(content.len() as u64, m);
        m.extend_from_slice(&content);
    };
    let mut ty = Vec::new();
    uleb(keys.len() as u64, &mut ty);
    for k in &keys {
        let split = k.iter().position(|b| *b == 0xff).unwrap();
        ty.push(0x60);
        uleb(split as u64, &mut ty);
        ty.extend_from_slice(&k[..split]);
        let res = &k[split + 1..];
        uleb(res.len() as u64, &mut ty);
        ty.extend_from_slice(res);
    }
    section(1, ty, &mut m);
    let mut fs = Vec::new();
    uleb(bodies.len() as u64, &mut fs);
    for t in &tyidx {
        uleb(*t as u64, &mut fs);
    }
    section(3, fs, &mut m);
    let heap_start = (DATA_BASE + data.bytes.len() as u32 + 7) & !7;
    if heap {
        let pages = (heap_start as u64 + 65535) / 65536;
        let mut mem = Vec::new();
        uleb(1, &mut mem);
        mem.push(0x00);
        uleb(pages.max(1), &mut mem);
        section(5, mem, &mut m);
        let mut gl = Vec::new();
        uleb(1, &mut gl);
        gl.push(0x7f);
        gl.push(0x01);
        gl.push(0x41);
        sleb(heap_start as i64, &mut gl);
        gl.push(0x0b);
        section(6, gl, &mut m);
    }
    let mut ex = Vec::new();
    uleb((funcs.len() + host_exports.len() + heap as usize) as u64, &mut ex);
    for (i, f) in funcs.iter().enumerate() {
        uleb(f.name.len() as u64, &mut ex);
        ex.extend_from_slice(f.name.as_bytes());
        ex.push(0x00);
        uleb(i as u64, &mut ex);
    }
    if heap {
        uleb(6, &mut ex);
        ex.extend_from_slice(b"memory");
        ex.push(0x02);
        uleb(0, &mut ex);
        for (n, i) in &host_exports {
            uleb(n.len() as u64, &mut ex);
            ex.extend_from_slice(n.as_bytes());
            ex.push(0x00);
            uleb(*i as u64, &mut ex);
        }
    }
    section(7, ex, &mut m);
    let mut code = Vec::new();
    uleb(bodies.len() as u64, &mut code);
    for (b, _, _) in &bodies {
        code.extend_from_slice(b);
    }
    section(10, code, &mut m);
    if heap && !data.bytes.is_empty() {
        let mut d = Vec::new();
        uleb(1, &mut d);
        d.push(0x00);
        d.push(0x41);
        sleb(DATA_BASE as i64, &mut d);
        d.push(0x0b);
        uleb(data.bytes.len() as u64, &mut d);
        d.extend_from_slice(&data.bytes);
        section(11, d, &mut m);
    }
    Ok((m, exports))
}

/// Compile `source` to a WebAssembly module; returns the bytes and the exported names with their parameter counts.
pub fn compile(source: &str) -> Result<(Vec<u8>, Vec<(String, usize)>), String> {
    let (m, ex) = compile_full(source)?;
    Ok((m, ex.into_iter().map(|e| (e.name, e.params.len())).collect()))
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
    match compile_full(&src) {
        Ok((bytes, exports)) => {
            let target = out.unwrap_or_else(|| format!("{}.wasm", file.trim_end_matches(".killer")));
            if let Err(e) = std::fs::write(&target, &bytes) {
                eprintln!("killer wasm: cannot write {}: {}", target, e);
                return 1;
            }
            println!("wrote {} ({} bytes)", target, bytes.len());
            let typed = exports.iter().any(|e| e.ret != "number" || e.params.iter().any(|p| p.1 != "number"));
            let uses_memory = typed || bytes.windows(6).any(|w| w == b"memory");
            for e in exports {
                if e.ret == "number" && e.params.iter().all(|p| p.1 == "number") {
                    let a = e.params.len();
                    println!("  export {}({} number{})", e.name, a, if a == 1 { "" } else { "s" });
                } else {
                    let ps: Vec<String> = e.params.iter().map(|p| format!("{}: {}", p.0, p.1)).collect();
                    println!("  export {}({}) -> {}", e.name, ps.join(", "), e.ret);
                }
            }
            if uses_memory {
                println!("  memory + host helpers: killer_alloc killer_str_new killer_str_len killer_list_new killer_list_len killer_list_get killer_list_set killer_list_push");
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
        // a numbers-only module has no memory
        assert!(!m.windows(6).any(|w| w == b"memory"));
    }

    #[test]
    fn unsupported_things_are_clear_errors() {
        assert!(compile("fn f() { return {\"a\": 1} }").unwrap_err().contains("cannot be compiled"));
        assert!(compile("fn f() {\n  return \"x\" * 2\n}").unwrap_err().starts_with("Line 2"));
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
