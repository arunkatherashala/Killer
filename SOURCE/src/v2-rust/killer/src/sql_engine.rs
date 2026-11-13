//! A small SQL database for Killer scripts, written in Rust with no dependencies: `sql_open`, `sql_exec`,
//! `sql_query`, `sql_close`.
//!
//! ```text
//! db = sql_open("app.db")                      # a file (saved after every change) or ":memory:"
//! sql_exec(db, "CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)")
//! sql_exec(db, "INSERT INTO users VALUES (?, ?, ?)", [1, "Ann", 30])       # -> rows changed
//! rows = sql_query(db, "SELECT name, age FROM users WHERE age > ? ORDER BY age DESC LIMIT 5", [18])
//! # rows is a list of dicts: [{"name": "Ann", "age": 30}]
//! ```
//!
//! Supported: CREATE TABLE [IF NOT EXISTS], DROP TABLE [IF EXISTS], INSERT INTO t [(cols)] VALUES (...), (...),
//! SELECT cols | * | COUNT/SUM/AVG/MIN/MAX FROM t [WHERE ...] [GROUP BY cols] [ORDER BY col [ASC|DESC], ...]
//! [LIMIT n [OFFSET m]], UPDATE t SET c = expr, ... [WHERE ...], DELETE FROM t [WHERE ...]. Expressions: numbers,
//! 'strings', NULL, column names, ? parameters, + - * / %, = != <> < <= > >=, AND OR NOT, IS [NOT] NULL, LIKE
//! (% and _), IN (list), parentheses. Column types are accepted and not enforced. There are no joins, subqueries,
//! indexes or transactions; one statement per call. Handles belong to the thread that opened them.

use crate::error::VmError;
use crate::value::{SharedArray, SharedDict, Value};
use std::cell::RefCell;
use std::collections::HashMap;

pub const NAMES: &[&str] = &["sql_open", "sql_exec", "sql_query", "sql_close"];

#[derive(Clone, Debug, PartialEq)]
enum Cell {
    Null,
    Num(f64),
    Text(String),
}

struct Table {
    cols: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

struct Db {
    path: Option<String>,
    tables: Vec<(String, Table)>,
}

thread_local! {
    static DBS: RefCell<HashMap<u64, Db>> = RefCell::new(HashMap::new());
    static NEXT: RefCell<u64> = const { RefCell::new(1) };
}

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    if !NAMES.contains(&name) {
        return None;
    }
    Some(dispatch(name, args))
}

fn handle(args: &[Value], who: &str) -> Result<u64, VmError> {
    match args.first() {
        Some(Value::Number(n)) if *n >= 1.0 && n.fract() == 0.0 => Ok(*n as u64),
        Some(Value::Integer(n)) if *n >= 1 => Ok(*n as u64),
        _ => err(format!("{}(): the first argument must be a database from sql_open()", who)),
    }
}

fn dispatch(name: &str, args: &[Value]) -> Result<Value, VmError> {
    match name {
        "sql_open" => {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return err("sql_open() expects a file path or \":memory:\""),
            };
            let db = if path == ":memory:" {
                Db { path: None, tables: Vec::new() }
            } else {
                crate::security::require_file_read()?;
                let tables = match std::fs::read_to_string(&path) {
                    Ok(text) => load(&text)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                    Err(e) => return err(format!("sql_open(): cannot read {}: {}", path, e)),
                };
                Db { path: Some(path), tables }
            };
            let id = NEXT.with(|n| {
                let mut n = n.borrow_mut();
                let id = *n;
                *n += 1;
                id
            });
            DBS.with(|d| d.borrow_mut().insert(id, db));
            Ok(Value::Number(id as f64))
        }
        "sql_close" => {
            let id = handle(args, name)?;
            DBS.with(|d| d.borrow_mut().remove(&id));
            Ok(Value::Null)
        }
        _ => {
            let id = handle(args, name)?;
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return err(format!("{}() expects the SQL text as its second argument", name)),
            };
            let params: Vec<Cell> = match args.get(2) {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(a)) => a.to_vec().iter().map(to_cell).collect::<Result<_, _>>()?,
                Some(_) => return err(format!("{}(): the parameters must be a list", name)),
            };
            DBS.with(|d| {
                let mut d = d.borrow_mut();
                let db = match d.get_mut(&id) {
                    Some(db) => db,
                    None => return err(format!("{}(): the database is closed", name)),
                };
                let stmt = parse(&sql, params.len())?;
                let is_query = matches!(stmt, Stmt::Select(_));
                if is_query != (name == "sql_query") {
                    return err(if is_query { "sql_exec() cannot run a SELECT: use sql_query()".to_string() } else { "sql_query() runs SELECT only: use sql_exec()".to_string() });
                }
                let (result, changed) = run(db, stmt, &params)?;
                if changed {
                    save(db)?;
                }
                Ok(result)
            })
        }
    }
}

fn to_cell(v: &Value) -> Result<Cell, VmError> {
    Ok(match v {
        Value::Null => Cell::Null,
        Value::Bool(b) => Cell::Num(if *b { 1.0 } else { 0.0 }),
        Value::Number(n) => Cell::Num(*n),
        Value::Integer(i) => Cell::Num(*i as f64),
        Value::Str(s) => Cell::Text(s.clone()),
        other => return err(format!("sql: a {} cannot be stored (use numbers, strings and null)", other.type_name())),
    })
}

fn from_cell(c: &Cell) -> Value {
    match c {
        Cell::Null => Value::Null,
        Cell::Num(n) => Value::Number(*n),
        Cell::Text(s) => Value::Str(s.clone()),
    }
}

// ---------------------------------------------------------------------------------------------
// tokens

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Num(f64),
    Str(String),
    Sym(String),
    Param,
}

fn tokenize(sql: &str) -> Result<Vec<Tok>, VmError> {
    let c: Vec<char> = sql.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() || ch == ';' {
            i += 1;
        } else if ch.is_alphabetic() || ch == '_' {
            let s = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Word(c[s..i].iter().collect()));
        } else if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            let t: String = c[s..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| VmError::runtime_error(format!("sql: bad number {}", t)))?));
        } else if ch == '\'' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= c.len() {
                    return err("sql: a string is missing its closing quote");
                }
                if c[i] == '\'' {
                    if i + 1 < c.len() && c[i + 1] == '\'' {
                        s.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                s.push(c[i]);
                i += 1;
            }
            out.push(Tok::Str(s));
        } else if ch == '?' {
            out.push(Tok::Param);
            i += 1;
        } else if matches!(ch, '<' | '>' | '!') && i + 1 < c.len() && matches!(c[i + 1], '=' | '>') {
            out.push(Tok::Sym(format!("{}{}", ch, c[i + 1])));
            i += 2;
        } else if "(),*=<>+-/%.".contains(ch) {
            out.push(Tok::Sym(ch.to_string()));
            i += 1;
        } else {
            return err(format!("sql: unexpected character '{}'", ch));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// syntax tree

#[derive(Clone, Debug)]
enum Expr {
    Lit(Cell),
    Col(String),
    Param(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    IsNull(Box<Expr>, bool),
    Like(Box<Expr>, Box<Expr>, bool),
    In(Box<Expr>, Vec<Expr>, bool),
    Agg(String, Option<Box<Expr>>),
}

struct Select {
    items: Vec<(Expr, String)>, // expression, output name
    star: bool,
    table: String,
    filter: Option<Expr>,
    group: Vec<String>,
    order: Vec<(Expr, bool)>, // expression, descending
    limit: Option<usize>,
    offset: usize,
}

enum Stmt {
    Create { name: String, cols: Vec<String>, if_not_exists: bool },
    Drop { name: String, if_exists: bool },
    Insert { table: String, cols: Option<Vec<String>>, rows: Vec<Vec<Expr>> },
    Select(Select),
    Update { table: String, sets: Vec<(String, Expr)>, filter: Option<Expr> },
    Delete { table: String, filter: Option<Expr> },
}

struct Parser {
    t: Vec<Tok>,
    p: usize,
    params: usize,
}

fn kw(t: &Tok, word: &str) -> bool {
    matches!(t, Tok::Word(w) if w.eq_ignore_ascii_case(word))
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.p)
    }
    fn accept_kw(&mut self, word: &str) -> bool {
        if self.peek().map_or(false, |t| kw(t, word)) {
            self.p += 1;
            true
        } else {
            false
        }
    }
    fn expect_kw(&mut self, word: &str) -> Result<(), VmError> {
        if self.accept_kw(word) {
            Ok(())
        } else {
            err(format!("sql: expected {} near {}", word.to_uppercase(), self.near()))
        }
    }
    fn accept_sym(&mut self, s: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Sym(x)) if x == s) {
            self.p += 1;
            true
        } else {
            false
        }
    }
    fn expect_sym(&mut self, s: &str) -> Result<(), VmError> {
        if self.accept_sym(s) {
            Ok(())
        } else {
            err(format!("sql: expected '{}' near {}", s, self.near()))
        }
    }
    fn near(&self) -> String {
        match self.peek() {
            Some(Tok::Word(w)) => format!("'{}'", w),
            Some(Tok::Sym(s)) => format!("'{}'", s),
            Some(Tok::Num(n)) => format!("{}", n),
            Some(Tok::Str(s)) => format!("'{}'", s),
            Some(Tok::Param) => "'?'".to_string(),
            None => "the end".to_string(),
        }
    }
    fn ident(&mut self) -> Result<String, VmError> {
        match self.peek() {
            Some(Tok::Word(w)) => {
                let w = w.clone();
                self.p += 1;
                Ok(w)
            }
            _ => err(format!("sql: expected a name near {}", self.near())),
        }
    }

    fn expr(&mut self) -> Result<Expr, VmError> {
        let mut l = self.and()?;
        while self.accept_kw("or") {
            l = Expr::Bin("or".into(), Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Expr, VmError> {
        let mut l = self.not()?;
        while self.accept_kw("and") {
            l = Expr::Bin("and".into(), Box::new(l), Box::new(self.not()?));
        }
        Ok(l)
    }
    fn not(&mut self) -> Result<Expr, VmError> {
        if self.accept_kw("not") {
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, VmError> {
        let l = self.sum()?;
        if self.accept_kw("is") {
            let neg = self.accept_kw("not");
            self.expect_kw("null")?;
            return Ok(Expr::IsNull(Box::new(l), neg));
        }
        let neg = self.peek().map_or(false, |t| kw(t, "not")) && self.t.get(self.p + 1).map_or(false, |t| kw(t, "like") || kw(t, "in"));
        if neg {
            self.p += 1;
        }
        if self.accept_kw("like") {
            return Ok(Expr::Like(Box::new(l), Box::new(self.sum()?), neg));
        }
        if self.accept_kw("in") {
            self.expect_sym("(")?;
            let mut items = vec![self.expr()?];
            while self.accept_sym(",") {
                items.push(self.expr()?);
            }
            self.expect_sym(")")?;
            return Ok(Expr::In(Box::new(l), items, neg));
        }
        for op in ["<=", ">=", "<>", "!=", "=", "<", ">"] {
            if self.accept_sym(op) {
                return Ok(Expr::Bin(op.into(), Box::new(l), Box::new(self.sum()?)));
            }
        }
        Ok(l)
    }
    fn sum(&mut self) -> Result<Expr, VmError> {
        let mut l = self.term()?;
        loop {
            if self.accept_sym("+") {
                l = Expr::Bin("+".into(), Box::new(l), Box::new(self.term()?));
            } else if self.accept_sym("-") {
                l = Expr::Bin("-".into(), Box::new(l), Box::new(self.term()?));
            } else {
                return Ok(l);
            }
        }
    }
    fn term(&mut self) -> Result<Expr, VmError> {
        let mut l = self.unary()?;
        loop {
            let op = ["*", "/", "%"].into_iter().find(|o| matches!(self.peek(), Some(Tok::Sym(x)) if x == o));
            match op {
                Some(o) => {
                    self.p += 1;
                    l = Expr::Bin(o.into(), Box::new(l), Box::new(self.unary()?));
                }
                None => return Ok(l),
            }
        }
    }
    fn unary(&mut self) -> Result<Expr, VmError> {
        if self.accept_sym("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }
    fn atom(&mut self) -> Result<Expr, VmError> {
        let t = match self.peek() {
            Some(t) => t.clone(),
            None => return err("sql: the statement ends too early"),
        };
        self.p += 1;
        match t {
            Tok::Num(n) => Ok(Expr::Lit(Cell::Num(n))),
            Tok::Str(s) => Ok(Expr::Lit(Cell::Text(s))),
            Tok::Param => {
                self.params += 1;
                Ok(Expr::Param(self.params - 1))
            }
            Tok::Sym(s) if s == "(" => {
                let e = self.expr()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            Tok::Word(w) if w.eq_ignore_ascii_case("null") => Ok(Expr::Lit(Cell::Null)),
            Tok::Word(w) if w.eq_ignore_ascii_case("true") => Ok(Expr::Lit(Cell::Num(1.0))),
            Tok::Word(w) if w.eq_ignore_ascii_case("false") => Ok(Expr::Lit(Cell::Num(0.0))),
            Tok::Word(w) => {
                let lower = w.to_lowercase();
                if matches!(lower.as_str(), "count" | "sum" | "avg" | "min" | "max") && matches!(self.peek(), Some(Tok::Sym(s)) if s == "(") {
                    self.p += 1;
                    let arg = if self.accept_sym("*") { None } else { Some(Box::new(self.expr()?)) };
                    self.expect_sym(")")?;
                    return Ok(Expr::Agg(lower, arg));
                }
                Ok(Expr::Col(w))
            }
            other => err(format!("sql: unexpected {:?}", other)),
        }
    }
}

fn parse(sql: &str, given: usize) -> Result<Stmt, VmError> {
    let mut p = Parser { t: tokenize(sql)?, p: 0, params: 0 };
    let stmt = parse_stmt(&mut p)?;
    if p.p < p.t.len() {
        return err(format!("sql: unexpected {} after the end of the statement (one statement per call)", p.near()));
    }
    if p.params != given {
        return err(format!("sql: the statement has {} '?' but {} parameter(s) were given", p.params, given));
    }
    Ok(stmt)
}

fn parse_stmt(p: &mut Parser) -> Result<Stmt, VmError> {
    if p.accept_kw("create") {
        p.expect_kw("table")?;
        let if_not_exists = if p.accept_kw("if") {
            p.expect_kw("not")?;
            p.expect_kw("exists")?;
            true
        } else {
            false
        };
        let name = p.ident()?;
        p.expect_sym("(")?;
        let mut cols = Vec::new();
        loop {
            cols.push(p.ident()?);
            // skip the type and constraints up to the next top-level comma or the closing parenthesis
            let mut depth = 0;
            while let Some(t) = p.peek() {
                match t {
                    Tok::Sym(s) if s == "(" => depth += 1,
                    Tok::Sym(s) if s == ")" => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    Tok::Sym(s) if s == "," && depth == 0 => break,
                    _ => {}
                }
                p.p += 1;
            }
            if !p.accept_sym(",") {
                break;
            }
        }
        p.expect_sym(")")?;
        return Ok(Stmt::Create { name, cols, if_not_exists });
    }
    if p.accept_kw("drop") {
        p.expect_kw("table")?;
        let if_exists = if p.accept_kw("if") {
            p.expect_kw("exists")?;
            true
        } else {
            false
        };
        return Ok(Stmt::Drop { name: p.ident()?, if_exists });
    }
    if p.accept_kw("insert") {
        p.expect_kw("into")?;
        let table = p.ident()?;
        let cols = if p.accept_sym("(") {
            let mut c = vec![p.ident()?];
            while p.accept_sym(",") {
                c.push(p.ident()?);
            }
            p.expect_sym(")")?;
            Some(c)
        } else {
            None
        };
        p.expect_kw("values")?;
        let mut rows = Vec::new();
        loop {
            p.expect_sym("(")?;
            let mut row = vec![p.expr()?];
            while p.accept_sym(",") {
                row.push(p.expr()?);
            }
            p.expect_sym(")")?;
            rows.push(row);
            if !p.accept_sym(",") {
                break;
            }
        }
        return Ok(Stmt::Insert { table, cols, rows });
    }
    if p.accept_kw("update") {
        let table = p.ident()?;
        p.expect_kw("set")?;
        let mut sets = Vec::new();
        loop {
            let c = p.ident()?;
            p.expect_sym("=")?;
            sets.push((c, p.expr()?));
            if !p.accept_sym(",") {
                break;
            }
        }
        let filter = if p.accept_kw("where") { Some(p.expr()?) } else { None };
        return Ok(Stmt::Update { table, sets, filter });
    }
    if p.accept_kw("delete") {
        p.expect_kw("from")?;
        let table = p.ident()?;
        let filter = if p.accept_kw("where") { Some(p.expr()?) } else { None };
        return Ok(Stmt::Delete { table, filter });
    }
    if p.accept_kw("select") {
        let mut items = Vec::new();
        let mut star = false;
        loop {
            if p.accept_sym("*") {
                star = true;
            } else {
                let start = p.p;
                let e = p.expr()?;
                let name = if p.accept_kw("as") {
                    p.ident()?
                } else {
                    match &e {
                        Expr::Col(c) => c.clone(),
                        _ => describe(&p.t[start..p.p]),
                    }
                };
                items.push((e, name));
            }
            if !p.accept_sym(",") {
                break;
            }
        }
        p.expect_kw("from")?;
        let table = p.ident()?;
        let filter = if p.accept_kw("where") { Some(p.expr()?) } else { None };
        let mut group = Vec::new();
        if p.accept_kw("group") {
            p.expect_kw("by")?;
            group.push(p.ident()?);
            while p.accept_sym(",") {
                group.push(p.ident()?);
            }
        }
        let mut order = Vec::new();
        if p.accept_kw("order") {
            p.expect_kw("by")?;
            loop {
                let e = p.expr()?;
                let desc = if p.accept_kw("desc") {
                    true
                } else {
                    p.accept_kw("asc");
                    false
                };
                order.push((e, desc));
                if !p.accept_sym(",") {
                    break;
                }
            }
        }
        let mut limit = None;
        let mut offset = 0;
        if p.accept_kw("limit") {
            limit = Some(count_arg(p)?);
            if p.accept_kw("offset") {
                offset = count_arg(p)?;
            }
        }
        return Ok(Stmt::Select(Select { items, star, table, filter, group, order, limit, offset }));
    }
    err(format!("sql: unknown statement near {} (supported: CREATE, DROP, INSERT, SELECT, UPDATE, DELETE)", p.near()))
}

fn count_arg(p: &mut Parser) -> Result<usize, VmError> {
    match p.peek() {
        Some(Tok::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => {
            let n = *n as usize;
            p.p += 1;
            Ok(n)
        }
        _ => err("sql: LIMIT and OFFSET need a whole number"),
    }
}

fn describe(toks: &[Tok]) -> String {
    toks.iter()
        .map(|t| match t {
            Tok::Word(w) => w.clone(),
            Tok::Num(n) => format!("{}", n),
            Tok::Str(s) => format!("'{}'", s),
            Tok::Sym(s) => s.clone(),
            Tok::Param => "?".to_string(),
        })
        .collect::<Vec<_>>()
        .join("")
}

// ---------------------------------------------------------------------------------------------
// evaluation

struct Ctx<'a> {
    cols: &'a [String],
    row: &'a [Cell],
    params: &'a [Cell],
    group: Option<&'a [&'a Vec<Cell>]>,
}

fn find_col(cols: &[String], name: &str) -> Result<usize, VmError> {
    let name = name.rsplit('.').next().unwrap_or(name);
    cols.iter().position(|c| c.eq_ignore_ascii_case(name)).ok_or_else(|| VmError::runtime_error(format!("sql: no column named {} (columns: {})", name, cols.join(", "))))
}

fn truthy(c: &Cell) -> bool {
    match c {
        Cell::Null => false,
        Cell::Num(n) => *n != 0.0,
        Cell::Text(s) => !s.is_empty(),
    }
}

fn compare(a: &Cell, b: &Cell) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Cell::Null, _) | (_, Cell::Null) => None,
        (Cell::Num(x), Cell::Num(y)) => x.partial_cmp(y),
        (Cell::Text(x), Cell::Text(y)) => Some(x.cmp(y)),
        (Cell::Num(_), Cell::Text(_)) => Some(std::cmp::Ordering::Less),
        (Cell::Text(_), Cell::Num(_)) => Some(std::cmp::Ordering::Greater),
    }
}

fn like(text: &str, pat: &str) -> bool {
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let p: Vec<char> = pat.to_lowercase().chars().collect();
    fn go(t: &[char], p: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('%') => (0..=t.len()).any(|i| go(&t[i..], &p[1..])),
            Some('_') => !t.is_empty() && go(&t[1..], &p[1..]),
            Some(c) => t.first() == Some(c) && go(&t[1..], &p[1..]),
        }
    }
    go(&t, &p)
}

fn eval(e: &Expr, cx: &Ctx) -> Result<Cell, VmError> {
    Ok(match e {
        Expr::Lit(c) => c.clone(),
        Expr::Param(i) => cx.params[*i].clone(),
        Expr::Col(n) => cx.row[find_col(cx.cols, n)?].clone(),
        Expr::Neg(x) => match eval(x, cx)? {
            Cell::Num(n) => Cell::Num(-n),
            Cell::Null => Cell::Null,
            Cell::Text(_) => return err("sql: cannot negate text"),
        },
        Expr::Not(x) => Cell::Num(if truthy(&eval(x, cx)?) { 0.0 } else { 1.0 }),
        Expr::IsNull(x, neg) => Cell::Num(((eval(x, cx)? == Cell::Null) != *neg) as u8 as f64),
        Expr::Like(x, pat, neg) => match (eval(x, cx)?, eval(pat, cx)?) {
            (Cell::Null, _) | (_, Cell::Null) => Cell::Null,
            (a, Cell::Text(p)) => {
                let s = match a {
                    Cell::Text(s) => s,
                    Cell::Num(n) => format!("{}", n),
                    Cell::Null => String::new(),
                };
                Cell::Num((like(&s, &p) != *neg) as u8 as f64)
            }
            _ => return err("sql: LIKE needs a text pattern"),
        },
        Expr::In(x, list, neg) => {
            let v = eval(x, cx)?;
            if v == Cell::Null {
                return Ok(Cell::Null);
            }
            let mut found = false;
            for item in list {
                if compare(&v, &eval(item, cx)?) == Some(std::cmp::Ordering::Equal) {
                    found = true;
                    break;
                }
            }
            Cell::Num((found != *neg) as u8 as f64)
        }
        Expr::Bin(op, l, r) => {
            if op == "and" || op == "or" {
                let a = truthy(&eval(l, cx)?);
                return Ok(Cell::Num(if op == "and" { a && truthy(&eval(r, cx)?) } else { a || truthy(&eval(r, cx)?) } as u8 as f64));
            }
            let (a, b) = (eval(l, cx)?, eval(r, cx)?);
            match op.as_str() {
                "=" | "!=" | "<>" | "<" | "<=" | ">" | ">=" => match compare(&a, &b) {
                    None => Cell::Null,
                    Some(o) => {
                        use std::cmp::Ordering::*;
                        let r = match op.as_str() {
                            "=" => o == Equal,
                            "!=" | "<>" => o != Equal,
                            "<" => o == Less,
                            "<=" => o != Greater,
                            ">" => o == Greater,
                            _ => o != Less,
                        };
                        Cell::Num(r as u8 as f64)
                    }
                },
                _ => match (a, b) {
                    (Cell::Null, _) | (_, Cell::Null) => Cell::Null,
                    (Cell::Num(x), Cell::Num(y)) => Cell::Num(match op.as_str() {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        "/" => {
                            if y == 0.0 {
                                return err("sql: division by zero");
                            }
                            x / y
                        }
                        _ => {
                            if y == 0.0 {
                                return err("sql: division by zero");
                            }
                            x % y
                        }
                    }),
                    (Cell::Text(x), Cell::Text(y)) if op == "+" => Cell::Text(x + &y),
                    _ => return err(format!("sql: '{}' needs two numbers", op)),
                },
            }
        }
        Expr::Agg(f, arg) => {
            let rows = match cx.group {
                Some(g) => g,
                None => return err(format!("sql: {}() can only be used in a SELECT list", f)),
            };
            let mut vals: Vec<Cell> = Vec::new();
            for r in rows {
                match arg {
                    None => vals.push(Cell::Num(1.0)),
                    Some(a) => {
                        let sub = Ctx { cols: cx.cols, row: r, params: cx.params, group: None };
                        let v = eval(a, &sub)?;
                        if v != Cell::Null {
                            vals.push(v);
                        }
                    }
                }
            }
            match f.as_str() {
                "count" => Cell::Num(vals.len() as f64),
                "sum" | "avg" => {
                    if vals.is_empty() {
                        return Ok(Cell::Null);
                    }
                    let mut s = 0.0;
                    for v in &vals {
                        match v {
                            Cell::Num(n) => s += n,
                            _ => return err(format!("sql: {}() needs numbers", f)),
                        }
                    }
                    Cell::Num(if f == "avg" { s / vals.len() as f64 } else { s })
                }
                _ => {
                    let mut best: Option<Cell> = None;
                    for v in vals {
                        let better = match &best {
                            None => true,
                            Some(b) => compare(&v, b) == Some(if f == "min" { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }),
                        };
                        if better {
                            best = Some(v);
                        }
                    }
                    best.unwrap_or(Cell::Null)
                }
            }
        }
    })
}

/// Every column named in `e` must exist, even when the table has no rows to evaluate it on.
fn check_cols(e: &Expr, cols: &[String]) -> Result<(), VmError> {
    match e {
        Expr::Col(n) => find_col(cols, n).map(|_| ()),
        Expr::Neg(x) | Expr::Not(x) | Expr::IsNull(x, _) => check_cols(x, cols),
        Expr::Bin(_, a, b) | Expr::Like(a, b, _) => {
            check_cols(a, cols)?;
            check_cols(b, cols)
        }
        Expr::In(a, l, _) => {
            check_cols(a, cols)?;
            l.iter().try_for_each(|x| check_cols(x, cols))
        }
        Expr::Agg(_, Some(a)) => check_cols(a, cols),
        _ => Ok(()),
    }
}

fn has_agg(e: &Expr) -> bool {
    match e {
        Expr::Agg(..) => true,
        Expr::Neg(x) | Expr::Not(x) | Expr::IsNull(x, _) => has_agg(x),
        Expr::Bin(_, a, b) | Expr::Like(a, b, _) => has_agg(a) || has_agg(b),
        Expr::In(a, l, _) => has_agg(a) || l.iter().any(has_agg),
        _ => false,
    }
}

fn table_mut<'a>(db: &'a mut Db, name: &str) -> Result<&'a mut Table, VmError> {
    db.tables
        .iter_mut()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, t)| t)
        .ok_or_else(|| VmError::runtime_error(format!("sql: no table named {}", name)))
}

/// Runs one statement; returns (result, whether the data changed).
fn run(db: &mut Db, stmt: Stmt, params: &[Cell]) -> Result<(Value, bool), VmError> {
    match stmt {
        Stmt::Create { name, cols, if_not_exists } => {
            if db.tables.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                if if_not_exists {
                    return Ok((Value::Number(0.0), false));
                }
                return err(format!("sql: table {} already exists", name));
            }
            for (i, c) in cols.iter().enumerate() {
                if cols[..i].iter().any(|d| d.eq_ignore_ascii_case(c)) {
                    return err(format!("sql: column {} appears twice", c));
                }
            }
            db.tables.push((name, Table { cols, rows: Vec::new() }));
            Ok((Value::Number(0.0), true))
        }
        Stmt::Drop { name, if_exists } => {
            let before = db.tables.len();
            db.tables.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
            if db.tables.len() == before && !if_exists {
                return err(format!("sql: no table named {}", name));
            }
            Ok((Value::Number(0.0), db.tables.len() != before))
        }
        Stmt::Insert { table, cols, rows } => {
            let t = table_mut(db, &table)?;
            let idx: Vec<usize> = match &cols {
                Some(c) => c.iter().map(|n| find_col(&t.cols, n)).collect::<Result<_, _>>()?,
                None => (0..t.cols.len()).collect(),
            };
            let mut new_rows = Vec::new();
            for r in &rows {
                if r.len() != idx.len() {
                    return err(format!("sql: {} value(s) for {} column(s)", r.len(), idx.len()));
                }
                let mut row = vec![Cell::Null; t.cols.len()];
                let empty: Vec<String> = Vec::new();
                for (k, e) in r.iter().enumerate() {
                    row[idx[k]] = eval(e, &Ctx { cols: &empty, row: &[], params, group: None })?;
                }
                new_rows.push(row);
            }
            let n = new_rows.len();
            t.rows.extend(new_rows);
            Ok((Value::Number(n as f64), true))
        }
        Stmt::Update { table, sets, filter } => {
            let t = table_mut(db, &table)?;
            let cols = t.cols.clone();
            let targets: Vec<usize> = sets.iter().map(|(c, _)| find_col(&cols, c)).collect::<Result<_, _>>()?;
            for (_, e) in &sets {
                check_cols(e, &cols)?;
            }
            if let Some(f) = &filter {
                check_cols(f, &cols)?;
            }
            let mut changed = 0;
            for row in t.rows.iter_mut() {
                let cx = Ctx { cols: &cols, row, params, group: None };
                if let Some(f) = &filter {
                    if !truthy(&eval(f, &cx)?) {
                        continue;
                    }
                }
                let mut vals = Vec::new();
                for (_, e) in &sets {
                    vals.push(eval(e, &cx)?);
                }
                for (k, v) in targets.iter().zip(vals) {
                    row[*k] = v;
                }
                changed += 1;
            }
            Ok((Value::Number(changed as f64), changed > 0))
        }
        Stmt::Delete { table, filter } => {
            let t = table_mut(db, &table)?;
            let cols = t.cols.clone();
            if let Some(f) = &filter {
                check_cols(f, &cols)?;
            }
            let before = t.rows.len();
            let mut keep = Vec::new();
            for row in std::mem::take(&mut t.rows) {
                let remove = match &filter {
                    None => true,
                    Some(f) => truthy(&eval(f, &Ctx { cols: &cols, row: &row, params, group: None })?),
                };
                if !remove {
                    keep.push(row);
                }
            }
            t.rows = keep;
            let n = before - t.rows.len();
            Ok((Value::Number(n as f64), n > 0))
        }
        Stmt::Select(s) => Ok((select(db, s, params)?, false)),
    }
}

fn select(db: &mut Db, s: Select, params: &[Cell]) -> Result<Value, VmError> {
    let t = table_mut(db, &s.table)?;
    let cols = t.cols.clone();
    for (e, _) in &s.items {
        check_cols(e, &cols)?;
    }
    if let Some(f) = &s.filter {
        check_cols(f, &cols)?;
    }
    for g in &s.group {
        find_col(&cols, g)?;
    }
    let mut rows: Vec<&Vec<Cell>> = Vec::new();
    for row in &t.rows {
        let keep = match &s.filter {
            None => true,
            Some(f) => truthy(&eval(f, &Ctx { cols: &cols, row, params, group: None })?),
        };
        if keep {
            rows.push(row);
        }
    }
    let aggregated = s.items.iter().any(|(e, _)| has_agg(e)) || !s.group.is_empty();
    // each output row: (cells, sort keys)
    let mut out: Vec<(Vec<Cell>, Vec<Cell>)> = Vec::new();
    let names: Vec<String> = if s.star { cols.clone() } else { Vec::new() }.into_iter().chain(s.items.iter().map(|(_, n)| n.clone())).collect();
    if aggregated {
        if s.star {
            return err("sql: * cannot be combined with GROUP BY or aggregate functions");
        }
        let gidx: Vec<usize> = s.group.iter().map(|g| find_col(&cols, g)).collect::<Result<_, _>>()?;
        let mut groups: Vec<(Vec<Cell>, Vec<&Vec<Cell>>)> = Vec::new();
        for r in &rows {
            let key: Vec<Cell> = gidx.iter().map(|i| r[*i].clone()).collect();
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, g)) => g.push(r),
                None => groups.push((key, vec![r])),
            }
        }
        if groups.is_empty() && s.group.is_empty() {
            groups.push((Vec::new(), Vec::new())); // an aggregate over no rows still gives one row
        }
        for (_, g) in &groups {
            let first: Vec<Cell> = g.first().map(|r| (*r).clone()).unwrap_or_else(|| vec![Cell::Null; cols.len()]);
            let cx = Ctx { cols: &cols, row: &first, params, group: Some(g) };
            let mut cells = Vec::new();
            for (e, _) in &s.items {
                cells.push(eval(e, &cx)?);
            }
            let mut keys = Vec::new();
            for (e, _) in &s.order {
                keys.push(eval(e, &cx).or_else(|_| match e {
                    Expr::Col(n) => s.items.iter().position(|(_, nm)| nm.eq_ignore_ascii_case(n)).map(|i| cells[i].clone()).ok_or_else(|| VmError::runtime_error(format!("sql: cannot order by {}", n))),
                    _ => err("sql: cannot order by this expression"),
                })?);
            }
            out.push((cells, keys));
        }
    } else {
        for r in &rows {
            let cx = Ctx { cols: &cols, row: r, params, group: None };
            let mut cells: Vec<Cell> = if s.star { (*r).clone() } else { Vec::new() };
            for (e, _) in &s.items {
                cells.push(eval(e, &cx)?);
            }
            let mut keys = Vec::new();
            for (e, _) in &s.order {
                keys.push(eval(e, &cx).or_else(|err0| match e {
                    Expr::Col(n) => names.iter().position(|nm| nm.eq_ignore_ascii_case(n)).map(|i| cells[i].clone()).ok_or(err0),
                    _ => Err(err0),
                })?);
            }
            out.push((cells, keys));
        }
    }
    if !s.order.is_empty() {
        out.sort_by(|a, b| {
            for (i, (_, desc)) in s.order.iter().enumerate() {
                let o = match (&a.1[i], &b.1[i]) {
                    (Cell::Null, Cell::Null) => std::cmp::Ordering::Equal,
                    (Cell::Null, _) => std::cmp::Ordering::Less,
                    (_, Cell::Null) => std::cmp::Ordering::Greater,
                    (x, y) => compare(x, y).unwrap_or(std::cmp::Ordering::Equal),
                };
                let o = if *desc { o.reverse() } else { o };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        });
    }
    let end = s.limit.map_or(out.len(), |l| (s.offset + l).min(out.len()));
    let slice = if s.offset >= out.len() { &out[0..0] } else { &out[s.offset..end] };
    let result: Vec<Value> = slice
        .iter()
        .map(|(cells, _)| {
            let mut d: HashMap<String, Value> = HashMap::new();
            for (n, c) in names.iter().zip(cells) {
                d.insert(n.clone(), from_cell(c));
            }
            Value::Dict(SharedDict::new(d))
        })
        .collect();
    Ok(Value::Array(SharedArray::new(result)))
}

// ---------------------------------------------------------------------------------------------
// saving and loading (a plain text file: `T name TAB col ...` then `R TAB cell ...`; a cell is `N`, `n<number>`
// or `s<text>` with TAB, newline and backslash written as `\t`, `\n` and a doubled backslash)

fn esc(s: &str) -> String {
    let bs = char::from(92u8);
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\t' => {
                o.push(bs);
                o.push('t');
            }
            '\n' => {
                o.push(bs);
                o.push('n');
            }
            '\r' => {
                o.push(bs);
                o.push('r');
            }
            c if c == bs => {
                o.push(bs);
                o.push(bs);
            }
            c => o.push(c),
        }
    }
    o
}

fn unesc(s: &str) -> String {
    let bs = char::from(92u8);
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == bs {
            match it.next() {
                Some('t') => o.push('\t'),
                Some('n') => o.push('\n'),
                Some('r') => o.push('\r'),
                Some(x) => o.push(x),
                None => o.push(bs),
            }
        } else {
            o.push(c);
        }
    }
    o
}

fn save(db: &Db) -> Result<(), VmError> {
    let path = match &db.path {
        Some(p) => p,
        None => return Ok(()),
    };
    crate::security::require_file_write()?;
    let mut text = String::from("KILLERSQL 1\n");
    for (name, t) in &db.tables {
        text.push_str(&format!("T\t{}\t{}\n", esc(name), t.cols.iter().map(|c| esc(c)).collect::<Vec<_>>().join("\t")));
        for r in &t.rows {
            let cells: Vec<String> = r
                .iter()
                .map(|c| match c {
                    Cell::Null => "N".to_string(),
                    Cell::Num(n) => format!("n{:?}", n),
                    Cell::Text(s) => format!("s{}", esc(s)),
                })
                .collect();
            text.push_str(&format!("R\t{}\n", cells.join("\t")));
        }
    }
    // write to a sibling file and rename, so a crash never leaves a half-written database
    let tmp = format!("{}.tmp", path);
    std::fs::write(&tmp, text).map_err(|e| VmError::runtime_error(format!("sql: cannot write {}: {}", path, e)))?;
    std::fs::rename(&tmp, path).map_err(|e| VmError::runtime_error(format!("sql: cannot write {}: {}", path, e)))
}

fn load(text: &str) -> Result<Vec<(String, Table)>, VmError> {
    let mut lines = text.lines();
    if lines.next().map(|l| l.trim()) != Some("KILLERSQL 1") {
        return err("sql_open(): this file is not a Killer SQL database");
    }
    let mut tables: Vec<(String, Table)> = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        match parts.first() {
            Some(&"T") if parts.len() >= 3 => tables.push((unesc(parts[1]), Table { cols: parts[2..].iter().map(|c| unesc(c)).collect(), rows: Vec::new() })),
            Some(&"R") => {
                let t = match tables.last_mut() {
                    Some((_, t)) => t,
                    None => return err("sql_open(): the database file is damaged (a row before any table)"),
                };
                let mut row = Vec::new();
                for c in &parts[1..] {
                    row.push(match c.chars().next() {
                        Some('N') => Cell::Null,
                        Some('n') => Cell::Num(c[1..].parse().map_err(|_| VmError::runtime_error("sql_open(): the database file is damaged (bad number)".to_string()))?),
                        Some('s') => Cell::Text(unesc(&c[1..])),
                        _ => return err("sql_open(): the database file is damaged (bad cell)"),
                    });
                }
                if row.len() != t.cols.len() {
                    return err("sql_open(): the database file is damaged (row length)");
                }
                t.rows.push(row);
            }
            _ if line.trim().is_empty() => {}
            _ => return err("sql_open(): the database file is damaged"),
        }
    }
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(db: &mut Db, sql: &str, p: &[Cell]) -> Value {
        let stmt = parse(sql, p.len()).unwrap();
        run(db, stmt, p).unwrap().0
    }

    #[test]
    fn insert_select_update_delete() {
        let mut db = Db { path: None, tables: Vec::new() };
        exec(&mut db, "CREATE TABLE u (id INTEGER, name TEXT, age INTEGER)", &[]);
        assert!(matches!(exec(&mut db, "INSERT INTO u VALUES (1, 'Ann', 30), (2, 'Bo', 20), (3, 'Cy', NULL)", &[]), Value::Number(n) if n == 3.0));
        let r = exec(&mut db, "SELECT name FROM u WHERE age > ? ORDER BY name DESC", &[Cell::Num(10.0)]);
        match r {
            Value::Array(a) => assert_eq!(a.to_vec().len(), 2),
            _ => panic!(),
        }
        assert!(matches!(exec(&mut db, "UPDATE u SET age = age + 1 WHERE name = 'Bo'", &[]), Value::Number(n) if n == 1.0));
        assert!(matches!(exec(&mut db, "DELETE FROM u WHERE age IS NULL", &[]), Value::Number(n) if n == 1.0));
    }

    #[test]
    fn like_and_escaping_round_trip() {
        assert!(like("Hello", "h%o"));
        assert!(like("abc", "a_c"));
        assert!(!like("abc", "a_d"));
        let s = format!("a{}b\tc\nd", char::from(92u8));
        assert_eq!(unesc(&esc(&s)), s);
    }
}
