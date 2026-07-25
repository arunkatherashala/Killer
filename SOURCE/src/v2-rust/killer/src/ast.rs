// killer_rcore/src/ast.rs
// Complete Abstract Syntax Tree for the Killer language.
// All types here exactly match what compiler.rs pattern-matches on.

// --------------------------- Legacy benchmark AST ----------------------------

/// Killer language AST node (used by JIT benchmark harness)
#[derive(Clone, Debug)]
pub enum ASTNode {
    Loop(LoopNode),
    Assign(String, Box<ASTNode>),
    Literal(i64),
    BinOp(Box<ASTNode>, String, Box<ASTNode>),
}

#[derive(Clone, Debug)]
pub struct LoopNode {
    pub var: String,
    pub start: Box<ASTNode>,
    pub end: Box<ASTNode>,
    pub step: Box<ASTNode>,
    pub body: Vec<ASTNode>,
}

#[derive(Clone, Debug)]
pub enum LoopExitType {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    NotEqual,
    Equal,
}

// ------------------------ Destructuring patterns -----------------------------

#[derive(Clone, Debug)]
pub enum Pattern {
    Identifier(String),
    Array(Vec<Pattern>),
    Object(Vec<(String, Pattern)>),
}

// ------------------------ Binary operator enum -------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, IntDiv, Mod, Pow,
    Eq, Ne, Gt, Ge, Lt, Le,
    And, Or,
}

// ---------------------------- Statement nodes --------------------------------

#[derive(Clone, Debug)]
pub enum Stmt {
    /// `let x = expr`
    Let     { pattern: Pattern, value: Box<Expr> },
    /// data-quality binding
    Quality { pattern: Pattern, value: Box<Expr> },
    /// Implicit assignment `x = expr`
    Assign  { pattern: Pattern, value: Box<Expr> },
    /// `obj[idx] = val`
    IndexAssign { object: String, index: Expr, value: Expr },
    /// `print(args…)`
    Print(Vec<Expr>),
    /// Expression as statement
    Expr(Expr),
    /// `if condition { then } else { else }`
    If {
        condition: Box<Expr>,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
    },
    /// `while condition { body }`
    While { condition: Box<Expr>, body: Vec<Stmt> },
    /// `do { body } while condition`
    DoWhile { condition: Box<Expr>, body: Vec<Stmt> },
    /// `for variable in iterable { body }`
    For {
        variable: String,
        iterable: Box<Expr>,
        is_for_of: bool,
        body: Vec<Stmt>,
    },
    /// C-style `for (init; condition; update) { body }`
    ForC {
        init: Option<Box<Stmt>>,
        condition: Option<Box<Expr>>,
        update: Option<Box<Expr>>,
        body: Vec<Stmt>,
    },
    /// Function declaration
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        ai_annotations: Vec<String>,
    },
    /// `return expr?`
    Return(Option<Box<Expr>>),
    Break,
    Continue,
    /// try / catch / finally
    Try {
        try_body: Vec<Stmt>,
        catch_var: String,
        catch_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
    },
    Throw(Box<Expr>),
    Yield(Box<Expr>),
    /// switch / case
    Switch {
        expression: Box<Expr>,
        cases: Vec<(Box<Expr>, Vec<Stmt>)>,
        default: Vec<Stmt>,
    },
    /// class definition
    Class {
        name: String,
        extends: Option<String>,
        methods: Vec<(String, Vec<String>, Vec<Stmt>)>,
    },
    // ── v2.2 additions ─────────────────────────────────────────────────────
    /// `async kfn name(params) { body }` — spawns on its own thread when called
    AsyncFunction {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// `spawn expr` — run expression in background thread, result discarded
    Spawn(Box<Expr>),
    /// `import "path/pkg"` or `import math` — load and execute a package
    /// When `symbols` is `Some`, only the listed names are imported (selective import).
    /// `None` means "import all" (backward compatible with bare `import "path"`).
    Import { path: String, symbols: Option<Vec<String>> },
    /// `export name1, name2` — mark names as publicly visible from this module
    Export(Vec<String>),
    /// `match expr: arm1 => body1, arm2 => body2, ...`
    Match {
        expression: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

/// A single pattern-match arm: `pattern [if guard] => body`
#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Box<Expr>>,
    pub body: Vec<Stmt>,
}

/// Patterns that can appear in match arms (richer than destructuring `Pattern`)
#[derive(Clone, Debug)]
pub enum MatchPattern {
    /// Literal value: `42`, `"hello"`, `true`, `null`
    Literal(Expr),
    /// Bind to a name (catch-all when used as `_` or any identifier)
    Identifier(String),
    /// Wildcard `_`
    Wildcard,
    /// Array destructure: `[a, b, c]`
    Array(Vec<MatchPattern>),
    /// Object destructure: `{x, y}` or `{x: pat, y: pat}`
    Object(Vec<(String, MatchPattern)>),
}

// ---------------------------- Expression nodes -------------------------------

#[derive(Clone, Debug)]
pub enum Expr {
    Number(f64),
    String(String),
    /// K-string interpolation (already resolved by line compiler)
    KString(String),
    Bool(bool),
    Null,
    This,
    Identifier(String),
    /// `left op right`
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    /// `condition ? then : else`
    Ternary { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> },
    /// `++x`
    PrefixInc(String),
    /// `--x`
    PrefixDec(String),
    /// `x++`
    PostfixInc(String),
    /// `x--`
    PostfixDec(String),
    /// Assignment expression: `name = value`
    Assign { name: String, value: Box<Expr> },
    /// Named function call: `name(args…)`
    Call { callee: String, args: Vec<Expr> },
    /// Built-in function call
    BuiltinCall { name: String, args: Vec<Expr> },
    /// Expression-level call: `expr(args…)`
    CallExpr { callee: Box<Expr>, args: Vec<Expr> },
    /// Method call: `object.method(args…)`
    MethodCall { object: Box<Expr>, method: String, args: Vec<Expr> },
    /// Array literal: `[a, b, c]`
    Array(Vec<Expr>),
    /// Index access: `object[index]`
    Index { object: Box<Expr>, index: Box<Expr> },
    /// Object / dict literal: `{key: val, …}`
    Dict(Vec<(Expr, Expr)>),
    /// Range expression: `start..end` or `start..end..step`
    Range { start: Box<Expr>, end: Box<Expr>, step: Option<Box<Expr>> },
    /// `new ClassName(args…)`
    New { class_name: String, args: Vec<Expr> },
    /// Function expression: `|params| { body }`
    FunctionExpr { params: Vec<String>, body: Vec<Stmt> },
    /// Spread: `...expr`
    Spread(Box<Expr>),
    /// `await expr` — block until a spawned Future resolves
    Await(Box<Expr>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_node_creation() {
        let node = ASTNode::Literal(42);
        match node {
            ASTNode::Literal(v) => assert_eq!(v, 42),
            _ => panic!("Wrong node type"),
        }
    }

    #[test]
    fn test_pattern_identifier() {
        let p = Pattern::Identifier("x".to_string());
        match p {
            Pattern::Identifier(name) => assert_eq!(name, "x"),
            _ => panic!("Wrong pattern"),
        }
    }

    #[test]
    fn test_binary_op_eq() {
        assert_eq!(BinaryOp::Add, BinaryOp::Add);
        assert_ne!(BinaryOp::Add, BinaryOp::Sub);
    }
}
