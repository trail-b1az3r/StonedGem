//! The syntax tree shared by .zsp programs, .zsx headers and .zsg kernels.

use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Vec<StrSeg>),
    Color(u32),
    Var(String, usize),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    StructLit(String, Vec<(String, Expr)>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    /// `cond ? a : b`
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>, usize),
    Index(Box<Expr>, Box<Expr>),
    /// `a.b` (or `a?.b` when the flag is set)
    Field(Box<Expr>, String, bool),
    Lambda(Rc<FuncDef>),
    /// `a..b` / `a..=b`
    Range(Box<Expr>, Box<Expr>, bool),
    Match(Box<Expr>, Vec<Arm>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrSeg {
    Lit(String),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    /// Alternatives (`1 | 2`); empty = `_`.
    pub pats: Vec<Expr>,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    /// `let`/`const`/typed declaration (`int x = 1`), maybe `global`.
    Let { name: String, ty: Option<Type>, value: Option<Expr>, global: bool, constant: bool, line: usize },
    /// `target op= value` (op "=" for plain assignment).
    Assign { target: Expr, op: &'static str, value: Expr, line: usize },
    Expr(Expr, usize),
    If { cond: Expr, then: Vec<Stmt>, els: Option<Vec<Stmt>>, line: usize },
    While { cond: Expr, body: Vec<Stmt>, line: usize },
    For { var: String, index: Option<String>, iter: Expr, body: Vec<Stmt>, line: usize },
    Return(Option<Expr>, usize),
    Break(usize),
    Continue(usize),
    Throw(Expr, usize),
    Try { body: Vec<Stmt>, var: Option<String>, catch: Vec<Stmt>, line: usize },
    Block(Vec<Stmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub name: String,
    /// `[float]` (a .zsg buffer) / `list<int>`.
    pub array: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub default: Option<Expr>,
    /// .zsg: `buf` / `uniform` / "" (plain).
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    /// None: a declaration only (.zsx header / `extern func`).
    pub body: Option<Vec<Stmt>>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Func(Rc<FuncDef>),
    Struct { name: String, fields: Vec<(String, Option<Type>)>, line: usize },
    Impl { name: String, methods: Vec<Rc<FuncDef>>, line: usize },
    /// `on event(params) { … }` or `on event "filter" (params) { … }`
    On { event: String, filter: Option<Expr>, params: Vec<String>, body: Vec<Stmt>, line: usize },
    Import { path: String, line: usize },
    /// .zsg compute kernel.
    Kernel(Rc<FuncDef>),
    /// .zsg fragment shader (per pixel, returns a colour).
    Fragment(Rc<FuncDef>),
    /// A top-level statement (run in order before `Main`).
    Stmt(Stmt),
}

/// Which kind of source a file is; decides what it may contain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `.zsp` (or Z#'s `.zs`): a program.
    Program,
    /// `.zsx`: declarations only — function signatures, structs, constants.
    Header,
    /// `.zsg`: GPU code — kernels, fragment shaders and typed helpers.
    Gpu,
}

impl Kind {
    pub fn from_path(path: &str) -> Option<Kind> {
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        match ext.as_str() {
            "zsp" | "zs" => Some(Kind::Program),
            "zsx" => Some(Kind::Header),
            "zsg" => Some(Kind::Gpu),
            _ => None,
        }
    }
}
