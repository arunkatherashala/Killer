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
//! Supported: CREATE TABLE [IF NOT EXISTS], DROP TABLE [IF EXISTS], INSERT INTO t [(cols)] VALUES (...) | SELECT ...,
//! UPDATE, DELETE, and SELECT [DISTINCT] items FROM source [INNER|LEFT|CROSS JOIN source ON cond | , source] ...
//! [WHERE] [GROUP BY] [HAVING] [ORDER BY expr [ASC|DESC], ...] [LIMIT n [OFFSET m]]. A source is a table with an
//! optional alias or a `(SELECT ...) alias`; columns may be qualified (`alias.col`). Expressions: numbers, 'strings',
//! NULL, columns, ? parameters, + - * / %, comparisons, AND OR NOT (three-valued), IS [NOT] NULL, [NOT] LIKE,
//! [NOT] IN (list | SELECT), [NOT] BETWEEN, EXISTS (SELECT), scalar subqueries (correlated ones included), CASE,
//! COUNT/SUM/AVG/MIN/MAX ([DISTINCT]) and a few scalar functions. Column types are accepted and not enforced.
//! No RIGHT/FULL joins, USING, UNION, indexes or transactions; one statement per call. Handles belong to the thread
//! that opened them.

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
    Col(String), // `name` or `qualifier.name`
    Param(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    IsNull(Box<Expr>, bool),
    Like(Box<Expr>, Box<Expr>, bool),
    In(Box<Expr>, Vec<Expr>, bool),
    InSub(Box<Expr>, Box<Select>, bool),
    Between(Box<Expr>, Box<Expr>, Box<Expr>, bool),
    Exists(Box<Select>),
    Sub(Box<Select>),
    Agg(String, Option<Box<Expr>>, bool), // function, argument (None = *), DISTINCT
    Func(String, Vec<Expr>),
    Case(Option<Box<Expr>>, Vec<(Expr, Expr)>, Option<Box<Expr>>),
}

#[derive(Clone, Debug)]
enum SelItem {
    Star(Option<String>),
    Item(Expr, String), // expression, output name
}

#[derive(Clone, Debug)]
enum Src {
    Table(String, Option<String>),
    Sub(Box<Select>, Option<String>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum JoinKind {
    Inner,
    Left,
    Cross,
}

#[derive(Clone, Debug)]
struct Join {
    kind: JoinKind,
    src: Src,
    on: Option<Expr>,
}

#[derive(Clone, Debug)]
struct FromClause {
    first: Src,
    joins: Vec<Join>,
}

#[derive(Clone, Debug)]
struct Select {
    distinct: bool,
    items: Vec<SelItem>,
    from: Option<FromClause>,
    filter: Option<Expr>,
    group: Vec<Expr>,
    having: Option<Expr>,
    order: Vec<(Expr, bool)>, // expression, descending
    limit: Option<usize>,
    offset: usize,
}

enum InsSrc {
    Values(Vec<Vec<Expr>>),
    Select(Select),
}

enum Stmt {
    Create { name: String, cols: Vec<String>, if_not_exists: bool },
    Drop { name: String, if_exists: bool },
    Insert { table: String, cols: Option<Vec<String>>, source: InsSrc },
    Select(Select),
    Update { table: String, sets: Vec<(String, Expr)>, filter: Option<Expr> },
    Delete { table: String, filter: Option<Expr> },
}

/// Deepest nesting of expressions the parser accepts (guards the stack).
const MAX_EXPR_DEPTH: usize = 200;

struct Parser {
    t: Vec<Tok>,
    p: usize,
    params: usize,
    depth: usize,
}

fn kw(t: &Tok, word: &str) -> bool {
    matches!(t, Tok::Word(w) if w.eq_ignore_ascii_case(word))
}

const FUNCS: &[&str] = &["coalesce", "ifnull", "nullif", "abs", "round", "upper", "lower", "length", "substr"];

fn reserved(w: &str) -> bool {
    matches!(
        w.to_lowercase().as_str(),
        "from" | "where" | "group" | "having" | "order" | "limit" | "offset" | "join" | "inner" | "left" | "right" | "full" | "cross" | "natural" | "outer" | "on" | "using" | "union" | "intersect" | "except" | "as" | "set" | "values" | "select" | "and" | "or" | "not" | "is" | "in" | "like" | "between" | "then" | "when" | "else" | "end" | "asc" | "desc"
    )
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.p)
    }
    fn peek_kw(&self, word: &str) -> bool {
        self.peek().map_or(false, |t| kw(t, word))
    }
    fn accept_kw(&mut self, word: &str) -> bool {
        if self.peek_kw(word) {
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

    fn enter(&mut self) -> Result<(), VmError> {
        self.depth += 1;
        if self.depth > MAX_EXPR_DEPTH {
            return err(format!("sql: expression nested deeper than {} levels", MAX_EXPR_DEPTH));
        }
        Ok(())
    }
    fn expr(&mut self) -> Result<Expr, VmError> {
        self.enter()?;
        let r = self.expr_inner();
        self.depth -= 1;
        r
    }
    fn expr_inner(&mut self) -> Result<Expr, VmError> {
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
            self.enter()?;
            let r = self.not();
            self.depth -= 1;
            return Ok(Expr::Not(Box::new(r?)));
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
        let neg = self.peek_kw("not") && self.t.get(self.p + 1).map_or(false, |t| kw(t, "like") || kw(t, "in") || kw(t, "between"));
        if neg {
            self.p += 1;
        }
        if self.accept_kw("like") {
            return Ok(Expr::Like(Box::new(l), Box::new(self.sum()?), neg));
        }
        if self.accept_kw("between") {
            let lo = self.sum()?;
            self.expect_kw("and")?;
            let hi = self.sum()?;
            return Ok(Expr::Between(Box::new(l), Box::new(lo), Box::new(hi), neg));
        }
        if self.accept_kw("in") {
            self.expect_sym("(")?;
            if self.accept_kw("select") {
                let s = parse_select(self)?;
                self.expect_sym(")")?;
                return Ok(Expr::InSub(Box::new(l), Box::new(s), neg));
            }
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
            self.enter()?;
            let r = self.unary();
            self.depth -= 1;
            return Ok(Expr::Neg(Box::new(r?)));
        }
        self.atom()
    }
    fn case_expr(&mut self) -> Result<Expr, VmError> {
        let operand = if self.peek_kw("when") { None } else { Some(Box::new(self.expr()?)) };
        let mut arms = Vec::new();
        while self.accept_kw("when") {
            let c = self.expr()?;
            self.expect_kw("then")?;
            arms.push((c, self.expr()?));
        }
        if arms.is_empty() {
            return err("sql: CASE needs at least one WHEN ... THEN");
        }
        let other = if self.accept_kw("else") { Some(Box::new(self.expr()?)) } else { None };
        self.expect_kw("end")?;
        Ok(Expr::Case(operand, arms, other))
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
                if self.accept_kw("select") {
                    let s = parse_select(self)?;
                    self.expect_sym(")")?;
                    return Ok(Expr::Sub(Box::new(s)));
                }
                let e = self.expr()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            Tok::Word(w) if w.eq_ignore_ascii_case("null") => Ok(Expr::Lit(Cell::Null)),
            Tok::Word(w) if w.eq_ignore_ascii_case("true") => Ok(Expr::Lit(Cell::Num(1.0))),
            Tok::Word(w) if w.eq_ignore_ascii_case("false") => Ok(Expr::Lit(Cell::Num(0.0))),
            Tok::Word(w) => {
                let lower = w.to_lowercase();
                let call = matches!(self.peek(), Some(Tok::Sym(s)) if s == "(");
                if lower == "case" {
                    return self.case_expr();
                }
                if lower == "exists" && call {
                    self.p += 1;
                    self.expect_kw("select")?;
                    let s = parse_select(self)?;
                    self.expect_sym(")")?;
                    return Ok(Expr::Exists(Box::new(s)));
                }
                if call && matches!(lower.as_str(), "count" | "sum" | "avg" | "min" | "max") {
                    self.p += 1;
                    let distinct = self.accept_kw("distinct");
                    let arg = if self.accept_sym("*") { None } else { Some(Box::new(self.expr()?)) };
                    if distinct && arg.is_none() {
                        return err("sql: COUNT(DISTINCT *) is not valid");
                    }
                    self.expect_sym(")")?;
                    return Ok(Expr::Agg(lower, arg, distinct));
                }
                if call {
                    if !FUNCS.contains(&lower.as_str()) {
                        return err(format!("sql: unknown function {}() (known: COUNT, SUM, AVG, MIN, MAX, {})", w, FUNCS.join(", ")));
                    }
                    self.p += 1;
                    let mut args = Vec::new();
                    if !self.accept_sym(")") {
                        args.push(self.expr()?);
                        while self.accept_sym(",") {
                            args.push(self.expr()?);
                        }
                        self.expect_sym(")")?;
                    }
                    return Ok(Expr::Func(lower, args));
                }
                if matches!(self.peek(), Some(Tok::Sym(s)) if s == ".") && matches!(self.t.get(self.p + 1), Some(Tok::Word(_))) {
                    self.p += 1;
                    let c = self.ident()?;
                    return Ok(Expr::Col(format!("{}.{}", w, c)));
                }
                Ok(Expr::Col(w))
            }
            other => err(format!("sql: unexpected {:?}", other)),
        }
    }
}

fn parse(sql: &str, given: usize) -> Result<Stmt, VmError> {
    let mut p = Parser { t: tokenize(sql)?, p: 0, params: 0, depth: 0 };
    let stmt = parse_stmt(&mut p)?;
    if p.p < p.t.len() {
        if p.peek_kw("union") || p.peek_kw("intersect") || p.peek_kw("except") {
            return err("sql: UNION, INTERSECT and EXCEPT are not supported");
        }
        return err(format!("sql: unexpected {} after the end of the statement (one statement per call)", p.near()));
    }
    if p.params != given {
        return err(format!("sql: the statement has {} '?' but {} parameter(s) were given", p.params, given));
    }
    Ok(stmt)
}

fn opt_alias(p: &mut Parser) -> Result<Option<String>, VmError> {
    if p.accept_kw("as") {
        return Ok(Some(p.ident()?));
    }
    if let Some(Tok::Word(w)) = p.peek() {
        if !reserved(w) {
            let w = w.clone();
            p.p += 1;
            return Ok(Some(w));
        }
    }
    Ok(None)
}

fn parse_src(p: &mut Parser) -> Result<Src, VmError> {
    if p.accept_sym("(") {
        p.expect_kw("select")?;
        let s = parse_select(p)?;
        p.expect_sym(")")?;
        let alias = opt_alias(p)?;
        return Ok(Src::Sub(Box::new(s), alias));
    }
    let name = p.ident()?;
    let alias = opt_alias(p)?;
    Ok(Src::Table(name, alias))
}

/// Parses a SELECT after the word SELECT has been consumed.
fn parse_select(p: &mut Parser) -> Result<Select, VmError> {
    let distinct = if p.accept_kw("distinct") {
        true
    } else {
        p.accept_kw("all");
        false
    };
    let mut items = Vec::new();
    loop {
        if p.accept_sym("*") {
            items.push(SelItem::Star(None));
        } else if matches!(p.peek(), Some(Tok::Word(_)))
            && matches!(p.t.get(p.p + 1), Some(Tok::Sym(s)) if s == ".")
            && matches!(p.t.get(p.p + 2), Some(Tok::Sym(s)) if s == "*")
        {
            let q = p.ident()?;
            p.p += 2;
            items.push(SelItem::Star(Some(q)));
        } else {
            let start = p.p;
            let e = p.expr()?;
            let name = if p.accept_kw("as") {
                p.ident()?
            } else if matches!(p.peek(), Some(Tok::Word(w)) if !reserved(w)) {
                p.ident()?
            } else {
                match &e {
                    Expr::Col(c) => c.rsplit('.').next().unwrap_or(c).to_string(),
                    _ => describe(&p.t[start..p.p]),
                }
            };
            items.push(SelItem::Item(e, name));
        }
        if !p.accept_sym(",") {
            break;
        }
    }
    let mut from = None;
    if p.accept_kw("from") {
        let first = parse_src(p)?;
        let mut joins = Vec::new();
        loop {
            let kind = if p.accept_sym(",") {
                JoinKind::Cross
            } else if p.accept_kw("cross") {
                p.expect_kw("join")?;
                JoinKind::Cross
            } else if p.accept_kw("inner") {
                p.expect_kw("join")?;
                JoinKind::Inner
            } else if p.accept_kw("left") {
                p.accept_kw("outer");
                p.expect_kw("join")?;
                JoinKind::Left
            } else if p.accept_kw("join") {
                JoinKind::Inner
            } else if p.peek_kw("right") || p.peek_kw("full") || p.peek_kw("natural") {
                return err(format!("sql: {} JOIN is not supported (use INNER, LEFT or CROSS; a RIGHT JOIN is a LEFT JOIN with the tables swapped)", p.near().trim_matches('\'').to_uppercase()));
            } else {
                break;
            };
            let src = parse_src(p)?;
            let on = if kind != JoinKind::Cross && p.accept_kw("on") {
                Some(p.expr()?)
            } else {
                None
            };
            if p.peek_kw("using") {
                return err("sql: JOIN ... USING is not supported: write ON a.x = b.x");
            }
            if kind == JoinKind::Left && on.is_none() {
                return err("sql: LEFT JOIN needs an ON condition");
            }
            joins.push(Join { kind, src, on });
        }
        from = Some(FromClause { first, joins });
    }
    let filter = if p.accept_kw("where") { Some(p.expr()?) } else { None };
    let mut group = Vec::new();
    if p.accept_kw("group") {
        p.expect_kw("by")?;
        group.push(p.expr()?);
        while p.accept_sym(",") {
            group.push(p.expr()?);
        }
    }
    let having = if p.accept_kw("having") { Some(p.expr()?) } else { None };
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
    Ok(Select { distinct, items, from, filter, group, having, order, limit, offset })
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
        if p.accept_kw("select") {
            let s = parse_select(p)?;
            return Ok(Stmt::Insert { table, cols, source: InsSrc::Select(s) });
        }
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
        return Ok(Stmt::Insert { table, cols, source: InsSrc::Values(rows) });
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
        return Ok(Stmt::Select(parse_select(p)?));
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

/// The row being evaluated. `quals[i]` is the table name or alias of column `i`; `outer` is the enclosing query's
/// row (for correlated subqueries).
struct Ctx<'a> {
    quals: &'a [String],
    cols: &'a [String],
    row: &'a [Cell],
    params: &'a [Cell],
    group: Option<&'a [&'a Vec<Cell>]>,
    db: &'a Db,
    outer: Option<&'a Ctx<'a>>,
}

/// Column lookup by bare name, ignoring any qualifier (used for INSERT and UPDATE targets).
fn find_col(cols: &[String], name: &str) -> Result<usize, VmError> {
    let name = name.rsplit('.').next().unwrap_or(name);
    cols.iter().position(|c| c.eq_ignore_ascii_case(name)).ok_or_else(|| VmError::runtime_error(format!("sql: no column named {} (columns: {})", name, cols.join(", "))))
}

/// Finds `name` (`col` or `qualifier.col`) in this row or an enclosing one.
fn resolve<'x>(cx: &'x Ctx<'x>, name: &str) -> Result<(&'x Ctx<'x>, usize), VmError> {
    let (q, c) = match name.split_once('.') {
        Some((q, c)) => (Some(q), c),
        None => (None, name),
    };
    let mut cur = Some(cx);
    while let Some(x) = cur {
        let hits: Vec<usize> = (0..x.cols.len()).filter(|&i| x.cols[i].eq_ignore_ascii_case(c) && q.map_or(true, |q| x.quals[i].eq_ignore_ascii_case(q))).collect();
        match hits.len() {
            0 => cur = x.outer,
            1 => return Ok((x, hits[0])),
            _ => {
                let owners: Vec<String> = hits.iter().map(|&i| format!("{}.{}", x.quals[i], c)).collect();
                return err(format!("sql: ambiguous column name {}: it exists in several tables, write one of {}", name, owners.join(" or ")));
            }
        }
    }
    if let Some(q) = q {
        let mut cur = Some(cx);
        while let Some(x) = cur {
            if x.quals.iter().any(|a| a.eq_ignore_ascii_case(q)) {
                let cols: Vec<&str> = (0..x.cols.len()).filter(|&i| x.quals[i].eq_ignore_ascii_case(q)).map(|i| x.cols[i].as_str()).collect();
                return err(format!("sql: no column named {} in {} (columns: {})", c, q, cols.join(", ")));
            }
            cur = x.outer;
        }
        return err(format!("sql: no table or alias named {} (in {})", q, name));
    }
    err(format!("sql: no column named {} (columns: {})", c, cx.cols.join(", ")))
}

fn truthy(c: &Cell) -> bool {
    match c {
        Cell::Null => false,
        Cell::Num(n) => *n != 0.0,
        Cell::Text(s) => !s.is_empty(),
    }
}

fn tri(c: &Cell) -> Option<bool> {
    if *c == Cell::Null {
        None
    } else {
        Some(truthy(c))
    }
}

fn from_tri(b: Option<bool>) -> Cell {
    match b {
        None => Cell::Null,
        Some(v) => Cell::Num(v as u8 as f64),
    }
}

fn and3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

fn or3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
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

fn cmp_op(op: &str, a: &Cell, b: &Cell) -> Option<bool> {
    use std::cmp::Ordering::*;
    compare(a, b).map(|o| match op {
        "=" => o == Equal,
        "!=" | "<>" => o != Equal,
        "<" => o == Less,
        "<=" => o != Greater,
        ">" => o == Greater,
        _ => o != Less,
    })
}

fn like(text: &str, pat: &str) -> bool {
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let p: Vec<char> = pat.to_lowercase().chars().collect();
    // iterative two-pointer match with backtracking to the last `%` (O(len(t) * len(p)) worst case)
    let (mut i, mut j) = (0usize, 0usize);
    let (mut star, mut mark): (Option<usize>, usize) = (None, 0);
    while i < t.len() {
        if j < p.len() && p[j] == '%' {
            star = Some(j);
            mark = i;
            j += 1;
        } else if j < p.len() && (p[j] == '_' || p[j] == t[i]) {
            i += 1;
            j += 1;
        } else if let Some(sj) = star {
            mark += 1;
            i = mark;
            j = sj + 1;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '%' {
        j += 1;
    }
    j == p.len()
}

fn cell_text(c: &Cell) -> String {
    match c {
        Cell::Text(s) => s.clone(),
        Cell::Num(n) => format!("{}", n),
        Cell::Null => String::new(),
    }
}

fn call_func(name: &str, a: &[Cell]) -> Result<Cell, VmError> {
    let want = |n: usize| -> Result<(), VmError> {
        if a.len() == n {
            Ok(())
        } else {
            err(format!("sql: {}() takes {} argument(s), got {}", name, n, a.len()))
        }
    };
    Ok(match name {
        "coalesce" => {
            if a.is_empty() {
                return err("sql: coalesce() needs at least one argument");
            }
            a.iter().find(|c| **c != Cell::Null).cloned().unwrap_or(Cell::Null)
        }
        "ifnull" => {
            want(2)?;
            if a[0] != Cell::Null { a[0].clone() } else { a[1].clone() }
        }
        "nullif" => {
            want(2)?;
            if compare(&a[0], &a[1]) == Some(std::cmp::Ordering::Equal) { Cell::Null } else { a[0].clone() }
        }
        "abs" => {
            want(1)?;
            match &a[0] {
                Cell::Null => Cell::Null,
                Cell::Num(n) => Cell::Num(n.abs()),
                Cell::Text(_) => return err("sql: abs() needs a number"),
            }
        }
        "round" => {
            if a.is_empty() || a.len() > 2 {
                return err("sql: round() takes 1 or 2 arguments");
            }
            let digits = match a.get(1) {
                None => 0.0,
                Some(Cell::Num(d)) => *d,
                Some(Cell::Null) => return Ok(Cell::Null),
                Some(_) => return err("sql: round() digits must be a number"),
            };
            match &a[0] {
                Cell::Null => Cell::Null,
                Cell::Num(n) => {
                    let m = 10f64.powi(digits as i32);
                    Cell::Num((n * m).round() / m)
                }
                Cell::Text(_) => return err("sql: round() needs a number"),
            }
        }
        "upper" | "lower" | "length" => {
            want(1)?;
            if a[0] == Cell::Null {
                return Ok(Cell::Null);
            }
            let s = cell_text(&a[0]);
            match name {
                "upper" => Cell::Text(s.to_uppercase()),
                "lower" => Cell::Text(s.to_lowercase()),
                _ => Cell::Num(s.chars().count() as f64),
            }
        }
        _ => {
            // substr(text, start [, length]), 1-based, a negative start counts from the end
            if a.len() < 2 || a.len() > 3 {
                return err("sql: substr() takes 2 or 3 arguments");
            }
            if a.iter().any(|c| *c == Cell::Null) {
                return Ok(Cell::Null);
            }
            let s: Vec<char> = cell_text(&a[0]).chars().collect();
            let num = |c: &Cell| match c {
                Cell::Num(n) => Ok(n.trunc() as i64),
                _ => err("sql: substr() start and length must be numbers"),
            };
            let n = s.len() as i64;
            let mut start = num(&a[1])?;
            let mut len = if a.len() == 3 { num(&a[2])? } else { n + 1 };
            if start < 0 {
                start = n + start + 1;
            }
            if start < 1 {
                len += start - 1;
                start = 1;
            }
            if len <= 0 || start > n {
                Cell::Text(String::new())
            } else {
                let end = (start - 1 + len).min(n);
                Cell::Text(s[(start - 1) as usize..end as usize].iter().collect())
            }
        }
    })
}

fn scalar_sub(s: &Select, cx: &Ctx) -> Result<Cell, VmError> {
    let (names, rows) = run_select(cx.db, s, cx.params, Some(cx))?;
    if names.len() != 1 {
        return err(format!("sql: a subquery used as a value must return one column, this one returns {}", names.len()));
    }
    Ok(rows.into_iter().next().map(|mut r| r.remove(0)).unwrap_or(Cell::Null))
}

fn in_list(v: &Cell, items: &[Cell], neg: bool) -> Cell {
    if *v == Cell::Null {
        return Cell::Null;
    }
    let mut saw_null = false;
    for it in items {
        match compare(v, it) {
            Some(std::cmp::Ordering::Equal) => return from_tri(Some(!neg)),
            None => saw_null = true,
            _ => {}
        }
    }
    if saw_null {
        Cell::Null
    } else {
        from_tri(Some(neg))
    }
}

fn eval(e: &Expr, cx: &Ctx) -> Result<Cell, VmError> {
    Ok(match e {
        Expr::Lit(c) => c.clone(),
        Expr::Param(i) => cx.params[*i].clone(),
        Expr::Col(n) => {
            let (owner, i) = resolve(cx, n)?;
            owner.row.get(i).cloned().unwrap_or(Cell::Null)
        }
        Expr::Neg(x) => match eval(x, cx)? {
            Cell::Num(n) => Cell::Num(-n),
            Cell::Null => Cell::Null,
            Cell::Text(_) => return err("sql: cannot negate text"),
        },
        Expr::Not(x) => from_tri(tri(&eval(x, cx)?).map(|b| !b)),
        Expr::IsNull(x, neg) => Cell::Num(((eval(x, cx)? == Cell::Null) != *neg) as u8 as f64),
        Expr::Like(x, pat, neg) => match (eval(x, cx)?, eval(pat, cx)?) {
            (Cell::Null, _) | (_, Cell::Null) => Cell::Null,
            (a, Cell::Text(p)) => Cell::Num((like(&cell_text(&a), &p) != *neg) as u8 as f64),
            _ => return err("sql: LIKE needs a text pattern"),
        },
        Expr::In(x, list, neg) => {
            let v = eval(x, cx)?;
            let items = list.iter().map(|i| eval(i, cx)).collect::<Result<Vec<_>, _>>()?;
            in_list(&v, &items, *neg)
        }
        Expr::InSub(x, s, neg) => {
            let v = eval(x, cx)?;
            let (names, rows) = run_select(cx.db, s, cx.params, Some(cx))?;
            if names.len() != 1 {
                return err(format!("sql: the subquery after IN must return one column, this one returns {}", names.len()));
            }
            let items: Vec<Cell> = rows.into_iter().map(|mut r| r.remove(0)).collect();
            in_list(&v, &items, *neg)
        }
        Expr::Between(x, lo, hi, neg) => {
            let v = eval(x, cx)?;
            let ge = cmp_op(">=", &v, &eval(lo, cx)?);
            let le = cmp_op("<=", &v, &eval(hi, cx)?);
            from_tri(and3(ge, le).map(|b| b != *neg))
        }
        Expr::Exists(s) => {
            let (_, rows) = run_select(cx.db, s, cx.params, Some(cx))?;
            Cell::Num((!rows.is_empty()) as u8 as f64)
        }
        Expr::Sub(s) => scalar_sub(s, cx)?,
        Expr::Func(name, args) => {
            let vals = args.iter().map(|a| eval(a, cx)).collect::<Result<Vec<_>, _>>()?;
            call_func(name, &vals)?
        }
        Expr::Case(operand, arms, other) => {
            let base = match operand {
                Some(o) => Some(eval(o, cx)?),
                None => None,
            };
            for (cond, result) in arms {
                let hit = match &base {
                    Some(b) => compare(b, &eval(cond, cx)?) == Some(std::cmp::Ordering::Equal),
                    None => truthy(&eval(cond, cx)?),
                };
                if hit {
                    return eval(result, cx);
                }
            }
            match other {
                Some(o) => eval(o, cx)?,
                None => Cell::Null,
            }
        }
        Expr::Bin(op, l, r) => {
            if op == "and" || op == "or" {
                let a = tri(&eval(l, cx)?);
                // short-circuit when the left side already decides the result
                if (op == "and" && a == Some(false)) || (op == "or" && a == Some(true)) {
                    return Ok(from_tri(a));
                }
                let b = tri(&eval(r, cx)?);
                return Ok(from_tri(if op == "and" { and3(a, b) } else { or3(a, b) }));
            }
            let (a, b) = (eval(l, cx)?, eval(r, cx)?);
            match op.as_str() {
                "=" | "!=" | "<>" | "<" | "<=" | ">" | ">=" => from_tri(cmp_op(op, &a, &b)),
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
        Expr::Agg(f, arg, distinct) => {
            let rows = match cx.group {
                Some(g) => g,
                None => return err(format!("sql: {}() is only allowed in the SELECT list, HAVING and ORDER BY (not in WHERE or ON)", f.to_uppercase())),
            };
            let mut vals: Vec<Cell> = Vec::new();
            for r in rows {
                match arg {
                    None => vals.push(Cell::Num(1.0)),
                    Some(a) => {
                        let sub = Ctx { quals: cx.quals, cols: cx.cols, row: r, params: cx.params, group: None, db: cx.db, outer: cx.outer };
                        let v = eval(a, &sub)?;
                        if v != Cell::Null {
                            vals.push(v);
                        }
                    }
                }
            }
            if *distinct {
                let mut seen: Vec<Cell> = Vec::new();
                for v in vals {
                    if !seen.contains(&v) {
                        seen.push(v);
                    }
                }
                vals = seen;
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

/// Every column named in `e` must exist, even when there are no rows to evaluate it on. Subqueries are checked when
/// they run.
fn check_cols(e: &Expr, cx: &Ctx) -> Result<(), VmError> {
    match e {
        Expr::Col(n) => resolve(cx, n).map(|_| ()),
        Expr::Neg(x) | Expr::Not(x) | Expr::IsNull(x, _) | Expr::InSub(x, _, _) => check_cols(x, cx),
        Expr::Bin(_, a, b) | Expr::Like(a, b, _) => {
            check_cols(a, cx)?;
            check_cols(b, cx)
        }
        Expr::Between(a, b, c, _) => {
            check_cols(a, cx)?;
            check_cols(b, cx)?;
            check_cols(c, cx)
        }
        Expr::In(a, l, _) => {
            check_cols(a, cx)?;
            l.iter().try_for_each(|x| check_cols(x, cx))
        }
        Expr::Func(_, l) => l.iter().try_for_each(|x| check_cols(x, cx)),
        Expr::Case(o, arms, other) => {
            if let Some(o) = o {
                check_cols(o, cx)?;
            }
            for (c, r) in arms {
                check_cols(c, cx)?;
                check_cols(r, cx)?;
            }
            match other {
                Some(o) => check_cols(o, cx),
                None => Ok(()),
            }
        }
        Expr::Agg(_, Some(a), _) => check_cols(a, cx),
        _ => Ok(()),
    }
}

fn has_agg(e: &Expr) -> bool {
    match e {
        Expr::Agg(..) => true,
        Expr::Neg(x) | Expr::Not(x) | Expr::IsNull(x, _) | Expr::InSub(x, _, _) => has_agg(x),
        Expr::Bin(_, a, b) | Expr::Like(a, b, _) => has_agg(a) || has_agg(b),
        Expr::Between(a, b, c, _) => has_agg(a) || has_agg(b) || has_agg(c),
        Expr::In(a, l, _) => has_agg(a) || l.iter().any(has_agg),
        Expr::Func(_, l) => l.iter().any(has_agg),
        Expr::Case(o, arms, other) => o.as_deref().map_or(false, has_agg) || arms.iter().any(|(c, r)| has_agg(c) || has_agg(r)) || other.as_deref().map_or(false, has_agg),
        _ => false,
    }
}

fn find_table<'a>(db: &'a Db, name: &str) -> Result<&'a Table, VmError> {
    db.tables
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, t)| t)
        .ok_or_else(|| VmError::runtime_error(format!("sql: no table named {} (tables: {})", name, db.tables.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", "))))
}

fn table_mut<'a>(db: &'a mut Db, name: &str) -> Result<&'a mut Table, VmError> {
    if !db.tables.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
        find_table(db, name)?;
    }
    Ok(db.tables.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, t)| t).unwrap())
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
        Stmt::Insert { table, cols, source } => {
            // build the new rows with the database only read, then append them
            let new_rows = {
                let t = find_table(db, &table)?;
                let idx: Vec<usize> = match &cols {
                    Some(c) => c.iter().map(|n| find_col(&t.cols, n)).collect::<Result<_, _>>()?,
                    None => (0..t.cols.len()).collect(),
                };
                let values: Vec<Vec<Cell>> = match &source {
                    InsSrc::Values(rows) => {
                        let mut out = Vec::new();
                        for r in rows {
                            let cx = Ctx { quals: &[], cols: &[], row: &[], params, group: None, db: &*db, outer: None };
                            out.push(r.iter().map(|e| eval(e, &cx)).collect::<Result<Vec<_>, _>>()?);
                        }
                        out
                    }
                    InsSrc::Select(s) => run_select(db, s, params, None)?.1,
                };
                let mut new_rows = Vec::new();
                for r in values {
                    if r.len() != idx.len() {
                        return err(format!("sql: {} value(s) for {} column(s)", r.len(), idx.len()));
                    }
                    let mut row = vec![Cell::Null; t.cols.len()];
                    for (k, v) in r.into_iter().enumerate() {
                        row[idx[k]] = v;
                    }
                    new_rows.push(row);
                }
                new_rows
            };
            let n = new_rows.len();
            table_mut(db, &table)?.rows.extend(new_rows);
            Ok((Value::Number(n as f64), true))
        }
        Stmt::Update { table, sets, filter } => {
            let updates: Vec<(usize, Vec<(usize, Cell)>)> = {
                let t = find_table(db, &table)?;
                let quals = vec![table.clone(); t.cols.len()];
                let targets: Vec<usize> = sets.iter().map(|(c, _)| find_col(&t.cols, c)).collect::<Result<_, _>>()?;
                let empty = Ctx { quals: &quals, cols: &t.cols, row: &[], params, group: None, db: &*db, outer: None };
                for (_, e) in &sets {
                    check_cols(e, &empty)?;
                }
                if let Some(f) = &filter {
                    check_cols(f, &empty)?;
                }
                let mut ups = Vec::new();
                for (ri, row) in t.rows.iter().enumerate() {
                    let cx = Ctx { quals: &quals, cols: &t.cols, row, params, group: None, db: &*db, outer: None };
                    if let Some(f) = &filter {
                        if !truthy(&eval(f, &cx)?) {
                            continue;
                        }
                    }
                    let mut vals = Vec::new();
                    for (k, (_, e)) in sets.iter().enumerate() {
                        vals.push((targets[k], eval(e, &cx)?));
                    }
                    ups.push((ri, vals));
                }
                ups
            };
            let changed = updates.len();
            let t = table_mut(db, &table)?;
            for (ri, vals) in updates {
                for (k, v) in vals {
                    t.rows[ri][k] = v;
                }
            }
            Ok((Value::Number(changed as f64), changed > 0))
        }
        Stmt::Delete { table, filter } => {
            let remove: Vec<bool> = {
                let t = find_table(db, &table)?;
                let quals = vec![table.clone(); t.cols.len()];
                if let Some(f) = &filter {
                    check_cols(f, &Ctx { quals: &quals, cols: &t.cols, row: &[], params, group: None, db: &*db, outer: None })?;
                }
                let mut mask = Vec::new();
                for row in &t.rows {
                    mask.push(match &filter {
                        None => true,
                        Some(f) => truthy(&eval(f, &Ctx { quals: &quals, cols: &t.cols, row, params, group: None, db: &*db, outer: None })?),
                    });
                }
                mask
            };
            let t = table_mut(db, &table)?;
            let mut i = 0;
            t.rows.retain(|_| {
                i += 1;
                !remove[i - 1]
            });
            let n = remove.iter().filter(|r| **r).count();
            Ok((Value::Number(n as f64), n > 0))
        }
        Stmt::Select(s) => {
            let (names, rows) = run_select(db, &s, params, None)?;
            let result: Vec<Value> = rows
                .iter()
                .map(|cells| {
                    let mut d: HashMap<String, Value> = HashMap::new();
                    for (n, c) in names.iter().zip(cells) {
                        d.insert(n.clone(), from_cell(c));
                    }
                    Value::Dict(SharedDict::new(d))
                })
                .collect();
            Ok((Value::Array(SharedArray::new(result)), false))
        }
    }
}

/// Replaces a bare name that is not a column but is the alias of a select-list expression by that expression
/// (so `HAVING n >= 2` works when the list says `COUNT(*) AS n`).
fn use_aliases(e: &Expr, items: &[SelItem], base: &Ctx) -> Expr {
    let f = |x: &Expr| Box::new(use_aliases(x, items, base));
    match e {
        Expr::Col(n) if !n.contains('.') && resolve(base, n).is_err() => items
            .iter()
            .find_map(|it| match it {
                SelItem::Item(x, nm) if nm.eq_ignore_ascii_case(n) => Some(x.clone()),
                _ => None,
            })
            .unwrap_or_else(|| e.clone()),
        Expr::Neg(x) => Expr::Neg(f(x)),
        Expr::Not(x) => Expr::Not(f(x)),
        Expr::IsNull(x, n) => Expr::IsNull(f(x), *n),
        Expr::Bin(o, a, b) => Expr::Bin(o.clone(), f(a), f(b)),
        Expr::Like(a, b, n) => Expr::Like(f(a), f(b), *n),
        Expr::Between(a, b, c, n) => Expr::Between(f(a), f(b), f(c), *n),
        Expr::In(a, l, n) => Expr::In(f(a), l.iter().map(|x| use_aliases(x, items, base)).collect(), *n),
        Expr::InSub(a, s, n) => Expr::InSub(f(a), s.clone(), *n),
        Expr::Func(n, l) => Expr::Func(n.clone(), l.iter().map(|x| use_aliases(x, items, base)).collect()),
        Expr::Agg(n, a, d) => Expr::Agg(n.clone(), a.as_ref().map(|x| f(x)), *d),
        Expr::Case(o, arms, other) => Expr::Case(o.as_ref().map(|x| f(x)), arms.iter().map(|(c, r)| (use_aliases(c, items, base), use_aliases(r, items, base))).collect(), other.as_ref().map(|x| f(x))),
        _ => e.clone(),
    }
}

/// A set of rows with the table (or alias) and column name of each column.
struct Rel {
    quals: Vec<String>,
    cols: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

fn src_qual(src: &Src) -> String {
    match src {
        Src::Table(n, a) => a.clone().unwrap_or_else(|| n.clone()),
        Src::Sub(_, a) => a.clone().unwrap_or_default(),
    }
}

fn load_source(db: &Db, src: &Src, params: &[Cell], outer: Option<&Ctx>) -> Result<Rel, VmError> {
    let q = src_qual(src);
    match src {
        Src::Table(name, _) => {
            let t = find_table(db, name)?;
            Ok(Rel { quals: vec![q; t.cols.len()], cols: t.cols.clone(), rows: t.rows.clone() })
        }
        Src::Sub(s, _) => {
            let (names, rows) = run_select(db, s, params, outer)?;
            Ok(Rel { quals: vec![q; names.len()], cols: names, rows })
        }
    }
}

/// Runs a SELECT; returns the output column names and the rows. `outer` is the enclosing row for a correlated
/// subquery.
fn run_select(db: &Db, s: &Select, params: &[Cell], outer: Option<&Ctx>) -> Result<(Vec<String>, Vec<Vec<Cell>>), VmError> {
    // FROM and JOINs
    let mut rel = match &s.from {
        None => Rel { quals: Vec::new(), cols: Vec::new(), rows: vec![Vec::new()] },
        Some(f) => load_source(db, &f.first, params, outer)?,
    };
    if let Some(f) = &s.from {
        let mut seen: Vec<String> = vec![src_qual(&f.first)];
        for j in &f.joins {
            let right = load_source(db, &j.src, params, outer)?;
            let q = src_qual(&j.src);
            if !q.is_empty() && seen.iter().any(|x| x.eq_ignore_ascii_case(&q)) {
                return err(format!("sql: the name {} is used twice in FROM; give one of them an alias (FROM {} AS other)", q, q));
            }
            seen.push(q);
            let mut quals = rel.quals.clone();
            quals.extend(right.quals.iter().cloned());
            let mut cols = rel.cols.clone();
            cols.extend(right.cols.iter().cloned());
            if let Some(on) = &j.on {
                if has_agg(on) {
                    return err("sql: aggregate functions are not allowed in ON");
                }
                check_cols(on, &Ctx { quals: &quals, cols: &cols, row: &[], params, group: None, db, outer })?;
            }
            let mut rows = Vec::new();
            for l in &rel.rows {
                let mut matched = false;
                for r in &right.rows {
                    let mut row = l.clone();
                    row.extend(r.iter().cloned());
                    let ok = match &j.on {
                        None => true,
                        Some(on) => truthy(&eval(on, &Ctx { quals: &quals, cols: &cols, row: &row, params, group: None, db, outer })?),
                    };
                    if ok {
                        matched = true;
                        rows.push(row);
                    }
                }
                if !matched && j.kind == JoinKind::Left {
                    let mut row = l.clone();
                    row.extend(std::iter::repeat(Cell::Null).take(right.cols.len()));
                    rows.push(row);
                }
            }
            rel = Rel { quals, cols, rows };
        }
    }
    let base = Ctx { quals: &rel.quals, cols: &rel.cols, row: &[], params, group: None, db, outer };

    // the output columns: either a column of the FROM rows (star) or an expression
    enum Proj<'e> {
        Col(usize),
        Expr(&'e Expr),
    }
    let mut proj: Vec<Proj> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut star_pos: Vec<(usize, usize)> = Vec::new(); // (position in names, column index) for star columns
    for it in &s.items {
        match it {
            SelItem::Star(q) => {
                let mut any = false;
                for i in 0..rel.cols.len() {
                    if q.as_ref().map_or(true, |q| rel.quals[i].eq_ignore_ascii_case(q)) {
                        any = true;
                        star_pos.push((names.len(), i));
                        names.push(rel.cols[i].clone());
                        proj.push(Proj::Col(i));
                    }
                }
                if !any {
                    return match q {
                        Some(q) => err(format!("sql: no table or alias named {} for {}.*", q, q)),
                        None => err("sql: SELECT * needs a FROM clause"),
                    };
                }
            }
            SelItem::Item(e, n) => {
                check_cols(e, &base)?;
                names.push(n.clone());
                proj.push(Proj::Expr(e));
            }
        }
    }
    // two star columns with the same name (a join of tables that share a column) are named table.column
    for &(pos, ci) in &star_pos {
        let dup = star_pos.iter().filter(|(_, cj)| rel.cols[*cj].eq_ignore_ascii_case(&rel.cols[ci])).count() > 1;
        if dup && !rel.quals[ci].is_empty() {
            names[pos] = format!("{}.{}", rel.quals[ci], rel.cols[ci]);
        }
    }

    let filter = s.filter.as_ref().map(|f| use_aliases(f, &s.items, &base));
    if let Some(f) = &filter {
        if has_agg(f) {
            return err("sql: aggregate functions are not allowed in WHERE (use HAVING)");
        }
        check_cols(f, &base)?;
    }
    // GROUP BY may name an output alias or position
    let mut group: Vec<Expr> = Vec::new();
    for g in &s.group {
        let item_expr = |i: usize| -> Option<Expr> {
            let mut k = 0;
            for it in &s.items {
                match it {
                    SelItem::Item(e, _) => {
                        if k == i {
                            return Some(e.clone());
                        }
                        k += 1;
                    }
                    SelItem::Star(_) => return None,
                }
            }
            None
        };
        let resolved = match g {
            Expr::Lit(Cell::Num(n)) if n.fract() == 0.0 && *n >= 1.0 => match item_expr(*n as usize - 1) {
                Some(e) => e,
                None => return err(format!("sql: GROUP BY position {} does not name a select-list expression", n)),
            },
            Expr::Col(n) if !n.contains('.') && resolve(&base, n).is_err() => match s.items.iter().find_map(|it| match it {
                SelItem::Item(e, nm) if nm.eq_ignore_ascii_case(n) => Some(e.clone()),
                _ => None,
            }) {
                Some(e) => e,
                None => g.clone(),
            },
            _ => g.clone(),
        };
        if has_agg(&resolved) {
            return err("sql: aggregate functions are not allowed in GROUP BY");
        }
        check_cols(&resolved, &base)?;
        group.push(resolved);
    }
    let having = s.having.as_ref().map(|h| use_aliases(h, &s.items, &base));
    if let Some(h) = &having {
        check_cols(h, &base)?;
    }
    let is_output = |e: &Expr| match e {
        Expr::Lit(Cell::Num(n)) => n.fract() == 0.0,
        Expr::Col(n) => !n.contains('.') && names.iter().any(|nm| nm.eq_ignore_ascii_case(n)),
        _ => false,
    };
    for (e, _) in &s.order {
        if !is_output(e) {
            check_cols(e, &base)?;
        }
    }

    let mut rows: Vec<&Vec<Cell>> = Vec::new();
    for row in &rel.rows {
        let keep = match &filter {
            None => true,
            Some(f) => truthy(&eval(f, &Ctx { quals: &rel.quals, cols: &rel.cols, row, params, group: None, db, outer })?),
        };
        if keep {
            rows.push(row);
        }
    }

    let order_key = |e: &Expr, cells: &[Cell], cx: &Ctx| -> Result<Cell, VmError> {
        match e {
            Expr::Lit(Cell::Num(n)) if n.fract() == 0.0 => {
                let i = *n as i64;
                if i >= 1 && (i as usize) <= cells.len() {
                    return Ok(cells[i as usize - 1].clone());
                }
                return err(format!("sql: ORDER BY position {} is out of range (the query returns {} column(s))", n, cells.len()));
            }
            Expr::Col(n) if !n.contains('.') => {
                if let Some(i) = names.iter().position(|nm| nm.eq_ignore_ascii_case(n)) {
                    return Ok(cells[i].clone());
                }
            }
            _ => {}
        }
        eval(e, cx)
    };
    let project = |cx: &Ctx| -> Result<Vec<Cell>, VmError> {
        let mut cells = Vec::new();
        for p in &proj {
            cells.push(match p {
                Proj::Col(i) => cx.row[*i].clone(),
                Proj::Expr(e) => eval(e, cx)?,
            });
        }
        Ok(cells)
    };

    let aggregated = s.items.iter().any(|it| matches!(it, SelItem::Item(e, _) if has_agg(e))) || !group.is_empty() || s.having.is_some() || s.order.iter().any(|(e, _)| has_agg(e));
    let mut out: Vec<(Vec<Cell>, Vec<Cell>)> = Vec::new(); // (cells, sort keys)
    if aggregated {
        if s.items.iter().any(|it| matches!(it, SelItem::Star(_))) {
            return err("sql: * cannot be combined with GROUP BY or aggregate functions");
        }
        let mut groups: Vec<(Vec<Cell>, Vec<&Vec<Cell>>)> = Vec::new();
        for r in &rows {
            let cx = Ctx { quals: &rel.quals, cols: &rel.cols, row: r, params, group: None, db, outer };
            let key: Vec<Cell> = group.iter().map(|g| eval(g, &cx)).collect::<Result<_, _>>()?;
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, g)) => g.push(r),
                None => groups.push((key, vec![r])),
            }
        }
        if groups.is_empty() && group.is_empty() {
            groups.push((Vec::new(), Vec::new())); // an aggregate over no rows still gives one row
        }
        for (_, g) in &groups {
            let first: Vec<Cell> = g.first().map(|r| (*r).clone()).unwrap_or_else(|| vec![Cell::Null; rel.cols.len()]);
            let cx = Ctx { quals: &rel.quals, cols: &rel.cols, row: &first, params, group: Some(g), db, outer };
            if let Some(h) = &having {
                if !truthy(&eval(h, &cx)?) {
                    continue;
                }
            }
            let cells = project(&cx)?;
            let mut keys = Vec::new();
            for (e, _) in &s.order {
                keys.push(order_key(e, &cells, &cx)?);
            }
            out.push((cells, keys));
        }
    } else {
        for r in &rows {
            let cx = Ctx { quals: &rel.quals, cols: &rel.cols, row: r, params, group: None, db, outer };
            let cells = project(&cx)?;
            let mut keys = Vec::new();
            for (e, _) in &s.order {
                keys.push(order_key(e, &cells, &cx)?);
            }
            out.push((cells, keys));
        }
    }
    if s.distinct {
        let mut seen: Vec<Vec<Cell>> = Vec::new();
        out.retain(|(cells, _)| {
            if seen.contains(cells) {
                false
            } else {
                seen.push(cells.clone());
                true
            }
        });
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
    let total = out.len();
    let end = s.limit.map_or(total, |l| s.offset.saturating_add(l).min(total));
    let result: Vec<Vec<Cell>> = if s.offset >= total { Vec::new() } else { out.drain(s.offset..end).map(|(c, _)| c).collect() };
    Ok((names, result))
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
    fn like_is_not_exponential() {
        let text = "a".repeat(5000);
        let pat = format!("{}b", "%a".repeat(40));
        let t0 = std::time::Instant::now();
        assert!(!like(&text, &pat));
        assert!(like(&text, "%a%a%a%"));
        assert!(like("", "%"));
        assert!(!like("", "_"));
        assert!(like("abcabc", "%c"));
        assert!(t0.elapsed().as_secs() < 5);
    }

    #[test]
    fn deeply_nested_expressions_are_rejected() {
        // debug-build frames are large; the real interpreter thread has a 512 MB stack
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let deep = format!("SELECT {}1{}", "(".repeat(5000), ")".repeat(5000));
                assert!(parse(&deep, 0).is_err());
                assert!(parse("SELECT ((1 + 2) * 3)", 0).is_ok());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn deep_not_rejected() {
        let nots = format!("SELECT * FROM t WHERE {}1", "NOT ".repeat(5000));
        assert!(parse(&nots, 0).is_err());
    }

    #[test]
    fn deep_neg_rejected() {
        let negs = format!("SELECT {}1", "-".repeat(5000));
        assert!(parse(&negs, 0).is_err());
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
