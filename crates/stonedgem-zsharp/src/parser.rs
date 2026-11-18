//! Parser for all three file kinds. Z# compatibility: `func`, typed
//! declarations (`int x = 1`), `global`, conditions without parentheses and
//! statements ended by line breaks. Z#++ adds the rest (see docs/ZSHARP.md).

use std::rc::Rc;

use crate::ast::*;
use crate::lexer::{lex_at, StrPart, Tok, Token};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    kind: Kind,
}

type R<T> = Result<T, String>;

const KEYWORDS: &[&str] = &[
    "func", "let", "const", "var", "global", "if", "else", "while", "for", "in", "return", "break", "continue", "struct", "impl", "on", "import",
    "match", "try", "catch", "throw", "true", "false", "null", "and", "or", "not", "extern", "kernel", "fragment",
];

pub fn parse(src: &str, kind: Kind) -> R<Vec<Item>> {
    let toks = crate::lexer::lex(src)?;
    let mut p = Parser { toks, pos: 0, kind };
    p.file()
}

/// Parse one expression (used for `${…}` in strings).
pub fn parse_expr_at(src: &str, line: usize) -> R<Expr> {
    let toks = lex_at(src, line)?;
    let mut p = Parser { toks, pos: 0, kind: Kind::Program };
    p.skip_nl();
    let e = p.expr(false)?;
    p.skip_nl();
    if p.peek() != &Tok::Eof {
        return Err(format!("line {line}: unexpected {} in `${{…}}`", p.peek()));
    }
    Ok(e)
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }
    fn line(&self) -> usize {
        self.toks[self.pos].line
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn at_op(&self, o: &str) -> bool {
        matches!(self.peek(), Tok::Op(x) if *x == o)
    }
    fn at_kw(&self, k: &str) -> bool {
        matches!(self.peek(), Tok::Ident(x) if x == k)
    }
    fn eat_op(&mut self, o: &str) -> bool {
        if self.at_op(o) {
            self.next();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, k: &str) -> bool {
        if self.at_kw(k) {
            self.next();
            true
        } else {
            false
        }
    }
    fn err<T>(&self, what: &str) -> R<T> {
        Err(format!("line {}: expected {what}, found {}", self.line(), self.peek()))
    }
    fn expect_op(&mut self, o: &str) -> R<()> {
        if self.eat_op(o) {
            Ok(())
        } else {
            self.err(&format!("`{o}`"))
        }
    }
    fn ident(&mut self) -> R<String> {
        match self.peek().clone() {
            Tok::Ident(s) if !KEYWORDS.contains(&s.as_str()) || s == "global" => {
                self.next();
                Ok(s)
            }
            _ => self.err("a name"),
        }
    }
    fn skip_nl(&mut self) {
        while self.peek() == &Tok::Newline {
            self.next();
        }
    }
    /// After a line break, does the next line continue this expression
    /// (`.method()`, `?.x`, `|> f`)?
    fn continues_on_next_line(&self) -> bool {
        let mut n = 0;
        while self.peek_at(n) == &Tok::Newline {
            n += 1;
        }
        n > 0 && matches!(self.peek_at(n), Tok::Op(".") | Tok::Op("?.") | Tok::Op("|>"))
    }
    fn end_stmt(&mut self) -> R<()> {
        match self.peek() {
            Tok::Newline => {
                self.next();
                Ok(())
            }
            Tok::Op("}") | Tok::Eof => Ok(()),
            _ => self.err("the end of the line"),
        }
    }

    // ── Items ──────────────────────────────────────────────────────────

    fn file(&mut self) -> R<Vec<Item>> {
        let mut items = vec![];
        loop {
            self.skip_nl();
            if self.peek() == &Tok::Eof {
                return Ok(items);
            }
            items.push(self.item()?);
        }
    }

    fn item(&mut self) -> R<Item> {
        let line = self.line();
        let gpu_or = |k: Kind, what: &str| -> R<()> {
            if k == Kind::Gpu {
                Err(format!("line {line}: {what} isn't allowed in .zsg (GPU) files"))
            } else {
                Ok(())
            }
        };
        if self.eat_kw("extern") {
            if !self.eat_kw("func") {
                return self.err("`func` after `extern`");
            }
            let f = self.func(true)?;
            return Ok(Item::Func(Rc::new(f)));
        }
        if self.eat_kw("func") {
            let f = self.func(self.kind == Kind::Header)?;
            return Ok(Item::Func(Rc::new(f)));
        }
        if self.eat_kw("struct") {
            return self.struct_item(line);
        }
        if self.at_kw("kernel") || self.at_kw("fragment") {
            if self.kind != Kind::Gpu {
                return Err(format!("line {line}: `{}` belongs in a .zsg file", if self.at_kw("kernel") { "kernel" } else { "fragment" }));
            }
            let is_kernel = self.at_kw("kernel");
            self.next();
            let f = Rc::new(self.func(false)?);
            return Ok(if is_kernel { Item::Kernel(f) } else { Item::Fragment(f) });
        }
        if self.eat_kw("import") {
            let path = match self.next() {
                Tok::Str(parts) => match parts.as_slice() {
                    [StrPart::Lit(s)] => s.clone(),
                    _ => return Err(format!("line {line}: an import path can't use `${{…}}`")),
                },
                _ => return Err(format!("line {line}: expected a file name after `import`")),
            };
            self.end_stmt()?;
            return Ok(Item::Import { path, line });
        }
        match self.kind {
            Kind::Header => {
                if self.at_kw("const") {
                    let s = self.stmt()?;
                    return Ok(Item::Stmt(s));
                }
                Err(format!("line {line}: a .zsx header holds declarations only (func signatures, struct, const, import), not {}", self.peek()))
            }
            Kind::Gpu => {
                if self.at_kw("const") {
                    let s = self.stmt()?;
                    return Ok(Item::Stmt(s));
                }
                Err(format!("line {line}: a .zsg file holds kernels, fragments, helper funcs, structs and consts, not {}", self.peek()))
            }
            Kind::Program => {
                if self.eat_kw("impl") {
                    gpu_or(self.kind, "impl")?;
                    let name = self.ident()?;
                    self.skip_nl();
                    self.expect_op("{")?;
                    let mut methods = vec![];
                    loop {
                        self.skip_nl();
                        if self.eat_op("}") {
                            break;
                        }
                        if !self.eat_kw("func") {
                            return self.err("`func` inside `impl`");
                        }
                        methods.push(Rc::new(self.func(false)?));
                    }
                    return Ok(Item::Impl { name, methods, line });
                }
                if self.eat_kw("on") {
                    let event = self.ident()?;
                    let filter = match self.peek() {
                        Tok::Str(_) => Some(self.primary(true)?),
                        _ => None,
                    };
                    let mut params = vec![];
                    if self.eat_op("(") {
                        while !self.eat_op(")") {
                            params.push(self.ident()?);
                            self.eat_op(",");
                        }
                    }
                    let body = self.block()?;
                    return Ok(Item::On { event, filter, params, body, line });
                }
                Ok(Item::Stmt(self.stmt()?))
            }
        }
    }

    fn struct_item(&mut self, line: usize) -> R<Item> {
        let name = self.ident()?;
        self.skip_nl();
        self.expect_op("{")?;
        let mut fields = vec![];
        loop {
            self.skip_nl();
            if self.eat_op("}") {
                break;
            }
            let f = self.ident()?;
            let ty = if self.eat_op(":") { Some(self.ty()?) } else { None };
            fields.push((f, ty));
            self.eat_op(",");
        }
        Ok(Item::Struct { name, fields, line })
    }

    fn ty(&mut self) -> R<Type> {
        if self.eat_op("[") {
            let name = self.ident()?;
            self.expect_op("]")?;
            return Ok(Type { name, array: true });
        }
        let name = self.ident()?;
        if self.eat_op("<") {
            let inner = self.ident()?;
            self.expect_op(">")?;
            return Ok(Type { name: format!("{name}<{inner}>"), array: name == "list" });
        }
        Ok(Type { name, array: false })
    }

    fn params(&mut self) -> R<Vec<Param>> {
        self.expect_op("(")?;
        let mut ps = vec![];
        loop {
            self.skip_nl();
            if self.eat_op(")") {
                return Ok(ps);
            }
            let mut kind = String::new();
            if (self.at_kw("buf") || self.at_kw("uniform")) && matches!(self.peek_at(1), Tok::Ident(_)) {
                if let Tok::Ident(k) = self.next() {
                    kind = k;
                }
            }
            let name = if self.at_kw("self") {
                self.next();
                "self".to_string()
            } else {
                self.ident()?
            };
            let ty = if self.eat_op(":") { Some(self.ty()?) } else { None };
            let default = if self.eat_op("=") { Some(self.expr(false)?) } else { None };
            ps.push(Param { name, ty, default, kind });
            self.skip_nl();
            if !self.eat_op(",") {
                self.skip_nl();
                self.expect_op(")")?;
                return Ok(ps);
            }
        }
    }

    fn func(&mut self, decl_only: bool) -> R<FuncDef> {
        let line = self.line();
        let name = self.ident()?;
        let params = self.params()?;
        let ret = if self.eat_op("->") { Some(self.ty()?) } else { None };
        // Z# puts the brace on the next line.
        let mut n = 0;
        while self.peek_at(n) == &Tok::Newline {
            n += 1;
        }
        let has_body = matches!(self.peek_at(n), Tok::Op("{"));
        if decl_only {
            if has_body && self.kind == Kind::Header {
                return Err(format!("line {line}: `{name}` has a body; in a .zsx header write only its signature"));
            }
            if has_body {
                return Err(format!("line {line}: an `extern func` has no body"));
            }
            self.end_stmt()?;
            return Ok(FuncDef { name, params, ret, body: None, line });
        }
        if !has_body {
            return Err(format!("line {line}: `{name}` needs a body `{{ … }}`"));
        }
        let body = self.block()?;
        Ok(FuncDef { name, params, ret, body: Some(body), line })
    }

    // ── Statements ─────────────────────────────────────────────────────

    fn block(&mut self) -> R<Vec<Stmt>> {
        self.skip_nl();
        self.expect_op("{")?;
        let mut out = vec![];
        loop {
            self.skip_nl();
            if self.eat_op("}") {
                return Ok(out);
            }
            if self.peek() == &Tok::Eof {
                return self.err("`}`");
            }
            out.push(self.stmt()?);
        }
    }

    fn stmt(&mut self) -> R<Stmt> {
        let line = self.line();
        let global = self.eat_kw("global");
        if self.at_kw("let") || self.at_kw("const") || self.at_kw("var") {
            let constant = self.at_kw("const");
            self.next();
            let name = self.ident()?;
            let ty = if self.eat_op(":") { Some(self.ty()?) } else { None };
            let value = if self.eat_op("=") {
                self.skip_nl();
                Some(self.expr(false)?)
            } else {
                None
            };
            if constant && value.is_none() {
                return Err(format!("line {line}: `const {name}` needs a value"));
            }
            self.end_stmt()?;
            return Ok(Stmt::Let { name, ty, value, global, constant, line });
        }
        // Z#: `int x = 1`, `Vec2 p = NVec2(1, 2)`, `string s`
        if let (Tok::Ident(t), Tok::Ident(n)) = (self.peek().clone(), self.peek_at(1).clone()) {
            if !KEYWORDS.contains(&t.as_str()) && !KEYWORDS.contains(&n.as_str()) && matches!(self.peek_at(2), Tok::Op("=") | Tok::Newline | Tok::Eof | Tok::Op("}")) {
                self.next();
                self.next();
                let value = if self.eat_op("=") {
                    self.skip_nl();
                    Some(self.expr(false)?)
                } else {
                    None
                };
                self.end_stmt()?;
                return Ok(Stmt::Let { name: n, ty: Some(Type { name: t, array: false }), value, global, constant: false, line });
            }
        }
        if global {
            return Err(format!("line {line}: `global` goes before a declaration (`global int x = 1`)"));
        }
        if self.eat_kw("if") {
            return self.if_rest(line);
        }
        if self.eat_kw("while") {
            let cond = self.expr(true)?;
            let body = self.block()?;
            self.end_stmt()?;
            return Ok(Stmt::While { cond, body, line });
        }
        if self.eat_kw("for") {
            let var = self.ident()?;
            let index = if self.eat_op(",") { Some(self.ident()?) } else { None };
            if !self.eat_kw("in") {
                return self.err("`in`");
            }
            let iter = self.expr(true)?;
            let body = self.block()?;
            self.end_stmt()?;
            return Ok(Stmt::For { var, index, iter, body, line });
        }
        if self.eat_kw("return") {
            let v = if matches!(self.peek(), Tok::Newline | Tok::Op("}") | Tok::Eof) { None } else { Some(self.expr(false)?) };
            self.end_stmt()?;
            return Ok(Stmt::Return(v, line));
        }
        if self.eat_kw("break") {
            self.end_stmt()?;
            return Ok(Stmt::Break(line));
        }
        if self.eat_kw("continue") {
            self.end_stmt()?;
            return Ok(Stmt::Continue(line));
        }
        if self.eat_kw("throw") {
            let v = self.expr(false)?;
            self.end_stmt()?;
            return Ok(Stmt::Throw(v, line));
        }
        if self.eat_kw("try") {
            let body = self.block()?;
            let save = self.pos;
            self.skip_nl();
            if !self.eat_kw("catch") {
                self.pos = save;
                return self.err("`catch` after `try { … }`");
            }
            let var = if matches!(self.peek(), Tok::Ident(_)) { Some(self.ident()?) } else { None };
            let catch = self.block()?;
            self.end_stmt()?;
            return Ok(Stmt::Try { body, var, catch, line });
        }
        if self.at_op("{") {
            let b = self.block()?;
            self.end_stmt()?;
            return Ok(Stmt::Block(b));
        }
        let e = self.expr(false)?;
        for op in ["=", "+=", "-=", "*=", "/=", "%="] {
            if self.at_op(op) {
                self.next();
                if !matches!(e, Expr::Var(..) | Expr::Index(..) | Expr::Field(..)) {
                    return Err(format!("line {line}: can't assign to that"));
                }
                self.skip_nl();
                let value = self.expr(false)?;
                self.end_stmt()?;
                let op: &'static str = match op {
                    "=" => "=",
                    "+=" => "+=",
                    "-=" => "-=",
                    "*=" => "*=",
                    "/=" => "/=",
                    _ => "%=",
                };
                return Ok(Stmt::Assign { target: e, op, value, line });
            }
        }
        self.end_stmt()?;
        Ok(Stmt::Expr(e, line))
    }

    fn if_rest(&mut self, line: usize) -> R<Stmt> {
        let cond = self.expr(true)?;
        let then = self.block()?;
        let save = self.pos;
        self.skip_nl();
        let els = if self.eat_kw("else") {
            if self.eat_kw("if") {
                let l = self.line();
                Some(vec![self.if_rest(l)?])
            } else {
                let b = self.block()?;
                self.end_stmt()?;
                Some(b)
            }
        } else {
            self.pos = save;
            self.end_stmt()?;
            None
        };
        Ok(Stmt::If { cond, then, els, line })
    }

    // ── Expressions ────────────────────────────────────────────────────

    /// `no_struct`: in `if`/`while`/`for` heads, `Name {` starts the block.
    pub fn expr(&mut self, no_struct: bool) -> R<Expr> {
        let mut e = self.ternary(no_struct)?;
        loop {
            if self.continues_on_next_line() && {
                let mut n = 0;
                while self.peek_at(n) == &Tok::Newline {
                    n += 1;
                }
                matches!(self.peek_at(n), Tok::Op("|>"))
            } {
                self.skip_nl();
            }
            if !self.eat_op("|>") {
                return Ok(e);
            }
            self.skip_nl();
            let line = self.line();
            let rhs = self.ternary(no_struct)?;
            e = match rhs {
                Expr::Call(f, mut args, l) => {
                    args.insert(0, e);
                    Expr::Call(f, args, l)
                }
                f => Expr::Call(Box::new(f), vec![e], line),
            };
        }
    }

    fn ternary(&mut self, ns: bool) -> R<Expr> {
        let c = self.coalesce(ns)?;
        if self.eat_op("?") {
            self.skip_nl();
            let a = self.expr(ns)?;
            self.skip_nl();
            self.expect_op(":")?;
            self.skip_nl();
            let b = self.expr(ns)?;
            return Ok(Expr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn binary_level(&mut self, ns: bool, ops: &[&'static str], kw: &[(&str, &'static str)], next: fn(&mut Self, bool) -> R<Expr>) -> R<Expr> {
        let mut l = next(self, ns)?;
        loop {
            let op = if let Some(o) = ops.iter().find(|o| self.at_op(o)) {
                self.next();
                *o
            } else if let Some((_, o)) = kw.iter().find(|(k, _)| self.at_kw(k)) {
                self.next();
                *o
            } else {
                return Ok(l);
            };
            self.skip_nl();
            let r = next(self, ns)?;
            l = Expr::Binary(op, Box::new(l), Box::new(r));
        }
    }

    fn coalesce(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["??"], &[], Self::or)
    }
    fn or(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["||"], &[("or", "||")], Self::and)
    }
    fn and(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["&&"], &[("and", "&&")], Self::equality)
    }
    fn equality(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["==", "!="], &[], Self::compare)
    }
    fn compare(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["<=", ">=", "<", ">"], &[("in", "in")], Self::range)
    }
    fn range(&mut self, ns: bool) -> R<Expr> {
        let l = self.additive(ns)?;
        if self.at_op("..") || self.at_op("..=") {
            let incl = self.at_op("..=");
            self.next();
            let r = self.additive(ns)?;
            return Ok(Expr::Range(Box::new(l), Box::new(r), incl));
        }
        Ok(l)
    }
    fn additive(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["+", "-"], &[], Self::multiplicative)
    }
    fn multiplicative(&mut self, ns: bool) -> R<Expr> {
        self.binary_level(ns, &["*", "/", "%"], &[], Self::power)
    }
    fn power(&mut self, ns: bool) -> R<Expr> {
        let base = self.unary(ns)?;
        if self.eat_op("**") {
            self.skip_nl();
            let exp = self.power(ns)?;
            return Ok(Expr::Binary("**", Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }
    fn unary(&mut self, ns: bool) -> R<Expr> {
        if self.eat_op("-") {
            return Ok(Expr::Unary("-", Box::new(self.unary(ns)?)));
        }
        if self.eat_op("!") || self.eat_kw("not") {
            return Ok(Expr::Unary("!", Box::new(self.unary(ns)?)));
        }
        self.postfix(ns)
    }

    fn args(&mut self, close: &str) -> R<Vec<Expr>> {
        let mut out = vec![];
        loop {
            self.skip_nl();
            if self.eat_op(close) {
                return Ok(out);
            }
            out.push(self.expr(false)?);
            self.skip_nl();
            if !self.eat_op(",") {
                self.skip_nl();
                self.expect_op(close)?;
                return Ok(out);
            }
        }
    }

    fn postfix(&mut self, ns: bool) -> R<Expr> {
        let mut e = self.primary(ns)?;
        loop {
            if self.continues_on_next_line() {
                let mut n = 0;
                while self.peek_at(n) == &Tok::Newline {
                    n += 1;
                }
                if matches!(self.peek_at(n), Tok::Op(".") | Tok::Op("?.")) {
                    self.skip_nl();
                }
            }
            let line = self.line();
            if self.eat_op("(") {
                let a = self.args(")")?;
                e = Expr::Call(Box::new(e), a, line);
            } else if self.eat_op("[") {
                self.skip_nl();
                let i = self.expr(false)?;
                self.skip_nl();
                self.expect_op("]")?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.at_op(".") || self.at_op("?.") {
                let opt = self.at_op("?.");
                self.next();
                let name = match self.next() {
                    Tok::Ident(s) => s,
                    Tok::Int(i) => i.to_string(),
                    t => return Err(format!("line {line}: expected a field name after `.`, found {t}")),
                };
                e = Expr::Field(Box::new(e), name, opt);
            } else {
                return Ok(e);
            }
        }
    }

    fn struct_lit_ahead(&self) -> bool {
        if !matches!(self.peek_at(1), Tok::Op("{")) {
            return false;
        }
        let mut n = 2;
        while self.peek_at(n) == &Tok::Newline {
            n += 1;
        }
        matches!(self.peek_at(n), Tok::Op("}")) || (matches!(self.peek_at(n), Tok::Ident(_)) && matches!(self.peek_at(n + 1), Tok::Op(":")))
    }

    fn primary(&mut self, ns: bool) -> R<Expr> {
        let line = self.line();
        match self.peek().clone() {
            Tok::Int(i) => {
                self.next();
                Ok(Expr::Int(i))
            }
            Tok::Float(f) => {
                self.next();
                Ok(Expr::Float(f))
            }
            Tok::Color(c) => {
                self.next();
                Ok(Expr::Color(c))
            }
            Tok::Str(parts) => {
                self.next();
                let mut segs = vec![];
                for p in parts {
                    match p {
                        StrPart::Lit(s) => {
                            if !s.is_empty() {
                                segs.push(StrSeg::Lit(s))
                            }
                        }
                        StrPart::Expr(src, l) => segs.push(StrSeg::Expr(parse_expr_at(&src, l)?)),
                    }
                }
                Ok(Expr::Str(segs))
            }
            Tok::Op("(") => {
                self.next();
                self.skip_nl();
                let e = self.expr(false)?;
                self.skip_nl();
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => {
                self.next();
                Ok(Expr::List(self.args("]")?))
            }
            Tok::Op("{") => {
                self.next();
                let mut kv = vec![];
                loop {
                    self.skip_nl();
                    if self.eat_op("}") {
                        return Ok(Expr::Map(kv));
                    }
                    let k = match self.peek().clone() {
                        Tok::Ident(s) => {
                            self.next();
                            Expr::Str(vec![StrSeg::Lit(s)])
                        }
                        _ => self.expr(false)?,
                    };
                    self.expect_op(":")?;
                    self.skip_nl();
                    let v = self.expr(false)?;
                    kv.push((k, v));
                    self.skip_nl();
                    self.eat_op(",");
                }
            }
            Tok::Op("|") | Tok::Op("||") => {
                // |a, b| expr   or   |a| { block }   or   || expr
                let empty = self.at_op("||");
                self.next();
                let mut params = vec![];
                if !empty {
                    while !self.eat_op("|") {
                        let name = self.ident()?;
                        let ty = if self.eat_op(":") { Some(self.ty()?) } else { None };
                        params.push(Param { name, ty, default: None, kind: String::new() });
                        self.eat_op(",");
                    }
                }
                let body = if self.at_op("{") { self.block()? } else { vec![Stmt::Return(Some(self.expr(ns)?), line)] };
                Ok(Expr::Lambda(Rc::new(FuncDef { name: "lambda".into(), params, ret: None, body: Some(body), line })))
            }
            Tok::Ident(s) => match s.as_str() {
                "true" => {
                    self.next();
                    Ok(Expr::Bool(true))
                }
                "false" => {
                    self.next();
                    Ok(Expr::Bool(false))
                }
                "null" => {
                    self.next();
                    Ok(Expr::Null)
                }
                "func" => {
                    self.next();
                    let params = self.params()?;
                    let ret = if self.eat_op("->") { Some(self.ty()?) } else { None };
                    let body = self.block()?;
                    Ok(Expr::Lambda(Rc::new(FuncDef { name: "lambda".into(), params, ret, body: Some(body), line })))
                }
                "match" => {
                    self.next();
                    let v = self.expr(true)?;
                    self.skip_nl();
                    self.expect_op("{")?;
                    let mut arms = vec![];
                    loop {
                        self.skip_nl();
                        if self.eat_op("}") {
                            break;
                        }
                        let mut pats = vec![];
                        if self.at_kw("_") {
                            self.next();
                        } else {
                            loop {
                                pats.push(self.range(false)?);
                                if !self.eat_op("|") {
                                    break;
                                }
                            }
                        }
                        let guard = if self.eat_kw("if") { Some(self.expr(false)?) } else { None };
                        self.expect_op("=>")?;
                        self.skip_nl();
                        let l = self.line();
                        let body = if self.at_op("{") { self.block()? } else { vec![Stmt::Expr(self.expr(false)?, l)] };
                        self.eat_op(",");
                        arms.push(Arm { pats, guard, body });
                    }
                    Ok(Expr::Match(Box::new(v), arms))
                }
                _ if KEYWORDS.contains(&s.as_str()) => self.err("an expression"),
                _ => {
                    if !ns && s.starts_with(|c: char| c.is_uppercase()) && self.struct_lit_ahead() {
                        self.next();
                        self.next();
                        let mut fields = vec![];
                        loop {
                            self.skip_nl();
                            if self.eat_op("}") {
                                return Ok(Expr::StructLit(s, fields));
                            }
                            let f = self.ident()?;
                            self.expect_op(":")?;
                            self.skip_nl();
                            fields.push((f, self.expr(false)?));
                            self.skip_nl();
                            self.eat_op(",");
                        }
                    }
                    self.next();
                    Ok(Expr::Var(s, line))
                }
            },
            _ => self.err("an expression"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsharp_compatible() {
        let src = "// Z#\nfunc Main()\n{\n    int i = 0\n    string s = \"r\"\n    i += 2\n    while i < 10\n    {\n        i += 1\n    }\n    if s == \"r\"\n    {\n        Printl(s + \" is r\")\n    }\n}\nfunc GlobalFunction()\n{\n    global int x = 12\n}\n";
        let items = parse(src, Kind::Program).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn zsharp_plus_plus() {
        let src = r#"
struct Point { x: float, y: float }
impl Point { func len(self) -> float { return sqrt(self.x * self.x + self.y * self.y) } }
on load { Print("hi") }
let p = Point { x: 3, y: 4 }
let xs = [1, 2, 3] |> map(|v| v * 2)
const name = "Z#++"
let s = "len ${p.len()} of ${name}"
let kind = match p.x { 0 => "zero", 1 | 2 | 3 => "small", _ => "big" }
for i, v in xs { if v > 2 and not false { continue } }
try { throw "x" } catch e { Print(e) }
let r = p?.x ?? 0
let t = r > 1 ? "big" : "small"
"#;
        let items = parse(src, Kind::Program).unwrap();
        assert!(items.len() >= 12, "{}", items.len());
    }

    #[test]
    fn kinds_are_enforced() {
        assert!(parse("func f(a: int) -> int\nstruct S { a: int }\nconst K = 1\n", Kind::Header).is_ok());
        assert!(parse("func f(a: int) -> int { return a }\n", Kind::Header).unwrap_err().contains("only its signature"));
        assert!(parse("Print(1)\n", Kind::Header).unwrap_err().contains("declarations only"));
        assert!(parse("kernel k(buf a: [float]) { a[gid] = 1.0 }\n", Kind::Program).unwrap_err().contains(".zsg"));
        assert!(parse("kernel k(buf a: [float]) { a[gid] = 1.0 }\nfragment f(uv: vec2) -> color { return vec4(uv.x, uv.y, 0.0, 1.0) }\n", Kind::Gpu).is_ok());
    }
}
