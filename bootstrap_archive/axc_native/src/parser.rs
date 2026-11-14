//! Axon v5.0 Zero-Ceremony Recursive-Descent & Pratt Parser (`parser.rs`)

use crate::ast::*;
use crate::lexer::{tokenize, FStrSegment, Token, TokenKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
    pub hint: Option<String>,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek_n(&self, n: usize) -> &Token {
        self.tokens
            .get(self.pos + n)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eat_semi_opt(&mut self) {
        while self.eat(&TokenKind::Semi) {}
    }

    fn expect(&mut self, expected: &TokenKind, ctx: &str) -> Result<Token, ParseError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            let t = self.peek();
            Err(ParseError {
                msg: format!("Expected {:?} {}, found {:?}", expected, ctx, t.kind),
                line: t.line,
                col: t.col,
                hint: None,
            })
        }
    }

    fn expect_ident(&mut self, ctx: &str) -> Result<(String, usize), ParseError> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Ident(s) => {
                self.advance();
                Ok((s, t.line))
            }
            _ => Err(ParseError {
                msg: format!("Expected identifier {}, found {:?}", ctx, t.kind),
                line: t.line,
                col: t.col,
                hint: None,
            }),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut module_attrs = Vec::new();
        let mut uses = Vec::new();
        let mut structs = Vec::new();
        let mut interfaces = Vec::new();
        let mut funcs = Vec::new();

        while !self.check(&TokenKind::Eof) {
            self.eat_semi_opt();
            if self.check(&TokenKind::Eof) {
                break;
            }

            // Collect attributes (@shield, @route("/api"), etc.)
            let mut pending_attrs: Vec<(String, Option<String>)> = Vec::new();
            while let TokenKind::Attr(attr_name) = self.peek().kind.clone() {
                self.advance();
                let mut arg = None;
                if self.eat(&TokenKind::LParen) {
                    if let TokenKind::Str(s) = self.peek().kind.clone() {
                        self.advance();
                        arg = Some(s);
                    } else if let TokenKind::Ident(s) = self.peek().kind.clone() {
                        self.advance();
                        arg = Some(s);
                    }
                    let _ = self.eat(&TokenKind::RParen);
                }
                pending_attrs.push((attr_name, arg));
            }

            if !pending_attrs.is_empty() && !self.check(&TokenKind::Fn) {
                for (a, _) in pending_attrs.drain(..) {
                    if !module_attrs.contains(&a) {
                        module_attrs.push(a);
                    }
                }
                if self.check(&TokenKind::Eof) {
                    break;
                }
            }

            match &self.peek().kind {
                TokenKind::Use => {
                    let u = self.parse_use()?;
                    uses.push(u);
                }
                TokenKind::Struct => {
                    let s = self.parse_struct()?;
                    structs.push(s);
                }
                TokenKind::Enum => {
                    self.parse_enum_into_helpers(&mut funcs)?;
                }
                TokenKind::Interface => {
                    let iface = self.parse_interface()?;
                    interfaces.push(iface);
                }
                TokenKind::Impl => {
                    let mut impl_fns = self.parse_impl()?;
                    funcs.append(&mut impl_fns);
                }
                TokenKind::Fn => {
                    let mut f = self.parse_fn(pending_attrs)?;
                    if module_attrs.iter().any(|a| a == "shield")
                        && !f.attrs.iter().any(|(a, _)| a == "shield")
                    {
                        f.attrs.push(("shield".into(), None));
                    }
                    funcs.push(f);
                }
                _ => {
                    let t = self.peek();
                    return Err(ParseError {
                        msg: format!("Unexpected top-level token {:?}", t.kind),
                        line: t.line,
                        col: t.col,
                        hint: Some("Top-level items can be `fn`, `struct`, `interface`, `impl`, `enum`, or `use`.".into()),
                    });
                }
            }
            self.eat_semi_opt();
        }

        if module_attrs.iter().any(|a| a == "shield") {
            for f in &mut funcs {
                if !f.attrs.iter().any(|(a, _)| a == "shield") {
                    f.attrs.push(("shield".into(), None));
                }
            }
        }

        Ok(Program {
            module_attrs,
            uses,
            structs,
            interfaces,
            funcs,
        })
    }

    fn parse_use(&mut self) -> Result<UseDecl, ParseError> {
        let tok = self.advance(); // 'use'
        let mut path = Vec::new();
        let (first, _) = self.expect_ident("in `use` path")?;
        path.push(first);
        while self.eat(&TokenKind::ColonColon) || self.eat(&TokenKind::Dot) {
            if self.eat(&TokenKind::Star) {
                path.push("*".into());
                break;
            }
            let (seg, _) = self.expect_ident("after `::` in `use` path")?;
            path.push(seg);
        }
        self.eat_semi_opt();
        Ok(UseDecl {
            path,
            line: tok.line,
        })
    }

    fn parse_struct(&mut self) -> Result<StructDecl, ParseError> {
        let tok = self.advance(); // 'struct'
        let (name, _) = self.expect_ident("for struct name")?;
        // Optional generic params `[T]`
        if self.eat(&TokenKind::LBracket) {
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                self.advance();
            }
            self.expect(&TokenKind::RBracket, "closing struct generic parameters")?;
        }
        self.expect(&TokenKind::LBrace, "opening struct body")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let (fname, fline) = self.expect_ident("for struct field name")?;
            if !self.eat(&TokenKind::Colon) {
                let bad = self.peek();
                return Err(ParseError {
                    msg: format!("Expected `:` after struct field `{}`", fname),
                    line: bad.line,
                    col: bad.col,
                    hint: Some(format!("Write `{}: Type` inside `struct {}`.", fname, name)),
                });
            }
            let fty = self.parse_type()?;
            fields.push(StructField {
                name: fname,
                ty: fty,
                line: fline,
            });
            let _ = self.eat(&TokenKind::Comma);
            self.eat_semi_opt();
        }
        self.expect(&TokenKind::RBrace, "closing struct body")?;
        Ok(StructDecl {
            name,
            fields,
            line: tok.line,
        })
    }

    fn parse_enum_into_helpers(&mut self, funcs: &mut Vec<FnDecl>) -> Result<(), ParseError> {
        let tok = self.advance(); // 'enum'
        let (ename, _) = self.expect_ident("for enum name")?;
        if self.eat(&TokenKind::LBracket) {
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                self.advance();
            }
            let _ = self.eat(&TokenKind::RBracket);
        }
        self.expect(&TokenKind::LBrace, "opening enum body")?;
        let mut tag = 0i64;
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let (vname, vline) = self.expect_ident("for enum variant")?;
            let mut params = Vec::new();
            if self.eat(&TokenKind::LParen) {
                let mut idx = 0;
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    let pty = self.parse_type()?;
                    params.push(Param {
                        name: format!("p{}", idx),
                        mode: ParamMode::Immutable,
                        ty: pty,
                        default_val: None,
                        line: vline,
                    });
                    idx += 1;
                    let _ = self.eat(&TokenKind::Comma);
                }
                self.expect(&TokenKind::RParen, "closing enum variant")?;
            }
            funcs.push(FnDecl {
                name: format!("{}__{}", ename, vname),
                attrs: Vec::new(),
                params,
                ret_ty: TypeSpec::Named("i64".into()),
                body: vec![Stmt::Return {
                    val: Some(Expr::Int(tag, vline)),
                    line: tok.line,
                }],
                line: vline,
            });
            tag += 1;
            let _ = self.eat(&TokenKind::Comma);
        }
        self.expect(&TokenKind::RBrace, "closing enum body")?;
        Ok(())
    }

    fn parse_interface(&mut self) -> Result<InterfaceDecl, ParseError> {
        let tok = self.advance(); // 'interface'
        let (name, _) = self.expect_ident("for interface name")?;
        self.expect(&TokenKind::LBrace, "opening interface body")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.expect(&TokenKind::Fn, "for interface method")?;
            let (mname, _) = self.expect_ident("for interface method name")?;
            let params = self.parse_param_list(Some("Self"))?;
            let ret_ty = if self.eat(&TokenKind::Arrow) {
                self.parse_type()?
            } else {
                TypeSpec::Unit
            };
            self.eat_semi_opt();
            methods.push(InterfaceMethodSig {
                name: mname,
                params,
                ret_ty,
            });
        }
        self.expect(&TokenKind::RBrace, "closing interface body")?;
        Ok(InterfaceDecl {
            name,
            methods,
            line: tok.line,
        })
    }

    fn parse_impl(&mut self) -> Result<Vec<FnDecl>, ParseError> {
        self.advance(); // 'impl'
        let (target_ty, _) = self.expect_ident("for `impl` target type")?;
        if self.eat(&TokenKind::For) {
            // Optional `impl Interface for Struct`
            let (real_target, _) = self.expect_ident("for `impl ... for Target`")?;
            return self.parse_impl_body(&real_target);
        }
        self.parse_impl_body(&target_ty)
    }

    fn parse_impl_body(&mut self, target_ty: &str) -> Result<Vec<FnDecl>, ParseError> {
        self.expect(&TokenKind::LBrace, "opening `impl` block")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let mut attrs = Vec::new();
            while let TokenKind::Attr(a) = self.peek().kind.clone() {
                self.advance();
                attrs.push((a, None));
            }
            let ftok = self.expect(&TokenKind::Fn, "inside `impl` block")?;
            let (mname, _) = self.expect_ident("for method name")?;
            let params = self.parse_param_list(Some(target_ty))?;
            let ret_ty = if self.eat(&TokenKind::Arrow) {
                self.parse_type()?
            } else {
                TypeSpec::Unit
            };
            let body = self.parse_block()?;
            methods.push(FnDecl {
                name: format!("{}__{}", target_ty, mname),
                attrs,
                params,
                ret_ty,
                body,
                line: ftok.line,
            });
            self.eat_semi_opt();
        }
        self.expect(&TokenKind::RBrace, "closing `impl` block")?;
        Ok(methods)
    }

    fn parse_fn(&mut self, attrs: Vec<(String, Option<String>)>) -> Result<FnDecl, ParseError> {
        let tok = self.advance(); // 'fn'
        let (name, _) = self.expect_ident("for function name")?;
        // Optional generic type parameters `[T]`
        if self.eat(&TokenKind::LBracket) {
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                self.advance();
            }
            self.expect(&TokenKind::RBracket, "closing function generic parameters")?;
        }
        let params = self.parse_param_list(None)?;
        let ret_ty = if self.eat(&TokenKind::Arrow) {
            self.parse_type()?
        } else {
            TypeSpec::Unit
        };
        let body = self.parse_block()?;
        Ok(FnDecl {
            name,
            attrs,
            params,
            ret_ty,
            body,
            line: tok.line,
        })
    }

    fn parse_param_list(&mut self, self_ty: Option<&str>) -> Result<Vec<Param>, ParseError> {
        self.expect(&TokenKind::LParen, "opening parameter list")?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let mode = if self.eat(&TokenKind::Let) {
                ParamMode::LetExplicit
            } else if self.eat(&TokenKind::Mut) {
                ParamMode::Mut
            } else if self.eat(&TokenKind::Move) {
                ParamMode::Move
            } else if self.eat(&TokenKind::Isolated) {
                ParamMode::Isolated
            } else {
                ParamMode::Immutable
            };

            let (pname, pline) = self.expect_ident("for parameter name")?;
            if pname == "self" && !self.check(&TokenKind::Colon) {
                let sty = self_ty.unwrap_or("Self").to_string();
                params.push(Param {
                    name: pname,
                    mode,
                    ty: TypeSpec::Named(sty),
                    default_val: None,
                    line: pline,
                });
                let _ = self.eat(&TokenKind::Comma);
                continue;
            }

            self.expect(&TokenKind::Colon, "after parameter name")?;
            let pty = self.parse_type()?;
            let default_val = if self.eat(&TokenKind::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(Param {
                name: pname,
                mode,
                ty: pty,
                default_val,
                line: pline,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "closing parameter list")?;
        Ok(params)
    }

    pub fn parse_type(&mut self) -> Result<TypeSpec, ParseError> {
        let base = self.parse_base_type()?;
        // Error union `T ! E`
        if self.eat(&TokenKind::Bang) {
            let err_ty = self.parse_base_type()?;
            return Ok(TypeSpec::Result(Box::new(base), Box::new(err_ty)));
        }
        // Refinement clause `usize where idx < arr.len`
        if self.eat(&TokenKind::Where) {
            let pred = self.parse_expr()?;
            return Ok(TypeSpec::Refined(Box::new(base), Box::new(pred)));
        }
        Ok(base)
    }

    fn parse_base_type(&mut self) -> Result<TypeSpec, ParseError> {
        if self.eat(&TokenKind::Amp) {
            let _ = self.eat(&TokenKind::Mut);
            let inner = self.parse_base_type()?;
            return Ok(TypeSpec::Ref(Box::new(inner)));
        }
        if self.eat(&TokenKind::LParen) {
            if self.eat(&TokenKind::RParen) {
                return Ok(TypeSpec::Unit);
            }
            let mut elems = Vec::new();
            loop {
                elems.push(self.parse_type()?);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::RParen) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "closing tuple type")?;
            if elems.len() == 1 {
                return Ok(elems.pop().unwrap());
            }
            return Ok(TypeSpec::Tuple(elems));
        }
        if self.eat(&TokenKind::LBracket) {
            let inner = self.parse_type()?;
            if self.eat(&TokenKind::Semi) {
                let _ = self.parse_expr()?;
            }
            self.expect(&TokenKind::RBracket, "closing slice/array type")?;
            return Ok(TypeSpec::Generic("Slice".into(), vec![inner]));
        }

        let (mut name, _) = self.expect_ident("for type name")?;
        while self.eat(&TokenKind::ColonColon) {
            let (seg, _) = self.expect_ident("in qualified type name")?;
            name = seg;
        }
        // Friendly aliases for universal ease of learning
        let canonical = match name.as_str() {
            "int" => "i64".to_string(),
            "uint" => "u64".to_string(),
            "float" => "f64".to_string(),
            "decimal" | "money" => "dec".to_string(),
            "string" | "String" => "str".to_string(),
            _ => name,
        };

        if self.eat(&TokenKind::LBracket) {
            let mut args = Vec::new();
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                args.push(self.parse_type()?);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RBracket, "closing generic type `[...]`")?;
            return Ok(TypeSpec::Generic(canonical, args));
        }
        Ok(TypeSpec::Named(canonical))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LBrace, "opening block `{`")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.eat_semi_opt();
            if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.eat_semi_opt();
        }
        self.expect(&TokenKind::RBrace, "closing block `}`")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let line = self.peek().line;
        match &self.peek().kind {
            TokenKind::Let | TokenKind::Isolated => self.parse_let_or_isolated(),
            TokenKind::Mut => {
                // Axon v5.0 Zero-Ceremony: `mut x := 10` or `mut x: i64 = 10`
                self.advance();
                if self.eat(&TokenKind::LParen) {
                    let mut names = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        let (n, _) = self.expect_ident("in tuple destructuring")?;
                        names.push(n);
                        let _ = self.eat(&TokenKind::Comma);
                    }
                    self.expect(&TokenKind::RParen, "closing tuple destructuring")?;
                    if !self.eat(&TokenKind::ColonEq) {
                        self.expect(&TokenKind::Eq, "after tuple destructuring")?;
                    }
                    let init = self.parse_expr()?;
                    return Ok(Stmt::TupleDestruct {
                        names,
                        kind: VarKind::Mut,
                        init,
                        line,
                    });
                }
                let (name, _) = self.expect_ident("after `mut`")?;
                let ty = if self.eat(&TokenKind::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                if !self.eat(&TokenKind::ColonEq) {
                    self.expect(&TokenKind::Eq, "initializing mutable variable")?;
                }
                let init = self.parse_expr()?;
                Ok(Stmt::VarDecl {
                    name,
                    kind: VarKind::Mut,
                    ty,
                    init,
                    line,
                })
            }
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body, line })
            }
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::With => {
                self.advance();
                let (var, _) = self.expect_ident("in `with` statement")?;
                self.expect(&TokenKind::Eq, "in `with` binding")?;
                let init = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::With {
                    var,
                    init,
                    cleanup_fn: None,
                    body,
                    line,
                })
            }
            TokenKind::Return => {
                self.advance();
                if self.check(&TokenKind::Semi)
                    || self.check(&TokenKind::RBrace)
                    || self.check(&TokenKind::Eof)
                {
                    Ok(Stmt::Return { val: None, line })
                } else {
                    let val = self.parse_expr()?;
                    Ok(Stmt::Return {
                        val: Some(val),
                        line,
                    })
                }
            }
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break { line })
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue { line })
            }
            TokenKind::AsmBang => {
                self.advance();
                // Skip raw inline asm tokens cleanly
                self.expect(&TokenKind::LBrace, "opening `asm!` block")?;
                let mut depth = 1usize;
                while depth > 0 && !self.check(&TokenKind::Eof) {
                    if self.eat(&TokenKind::LBrace) {
                        depth += 1;
                    } else if self.eat(&TokenKind::RBrace) {
                        depth -= 1;
                    } else {
                        self.advance();
                    }
                }
                Ok(Stmt::ExprStmt {
                    expr: Expr::Int(0, line),
                    line,
                })
            }
            _ => {
                // Check for Axon v5.0 Zero-Ceremony `name := expr`!
                if let TokenKind::Ident(var_name) = self.peek().kind.clone() {
                    if self.peek_n(1).kind == TokenKind::ColonEq {
                        self.advance(); // ident
                        self.advance(); // :=
                        let init = self.parse_expr()?;
                        return Ok(Stmt::VarDecl {
                            name: var_name,
                            kind: VarKind::Immutable,
                            ty: None,
                            init,
                            line,
                        });
                    }
                }

                let expr = self.parse_expr()?;
                let assign_op = match self.peek().kind {
                    TokenKind::Eq => Some(AssignOp::Eq),
                    TokenKind::PlusEq => Some(AssignOp::AddEq),
                    TokenKind::MinusEq => Some(AssignOp::SubEq),
                    TokenKind::StarEq => Some(AssignOp::MulEq),
                    TokenKind::SlashEq => Some(AssignOp::DivEq),
                    _ => None,
                };
                if let Some(op) = assign_op {
                    self.advance();
                    let val = self.parse_expr()?;
                    Ok(Stmt::Assign {
                        target: expr,
                        op,
                        val,
                        line,
                    })
                } else if let Expr::Tuple(elems, _) = &expr {
                    if self.eat(&TokenKind::ColonEq) {
                        let mut names = Vec::new();
                        for e in elems {
                            if let Expr::Ident(n, _) = e {
                                names.push(n.clone());
                            }
                        }
                        let init = self.parse_expr()?;
                        return Ok(Stmt::TupleDestruct {
                            names,
                            kind: VarKind::Immutable,
                            init,
                            line,
                        });
                    }
                    Ok(Stmt::ExprStmt { expr, line })
                } else {
                    Ok(Stmt::ExprStmt { expr, line })
                }
            }
        }
    }

    fn parse_let_or_isolated(&mut self) -> Result<Stmt, ParseError> {
        let first = self.advance();
        let mut kind = if first.kind == TokenKind::Isolated {
            VarKind::Isolated
        } else {
            VarKind::Immutable
        };
        if self.eat(&TokenKind::Mut) {
            kind = VarKind::Mut;
        }

        if self.eat(&TokenKind::LParen) {
            let mut names = Vec::new();
            while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                let _ = self.eat(&TokenKind::Mut);
                let (n, _) = self.expect_ident("in tuple destructuring")?;
                names.push(n);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "closing tuple destructuring")?;
            if self.eat(&TokenKind::Colon) {
                let _ = self.parse_type()?;
            }
            self.expect(&TokenKind::Eq, "after tuple pattern")?;
            let init = self.parse_expr()?;
            return Ok(Stmt::TupleDestruct {
                names,
                kind,
                init,
                line: first.line,
            });
        }

        let (name, _) = self.expect_ident("for variable name")?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq, "initializing variable")?;
        let init = self.parse_expr()?;
        Ok(Stmt::VarDecl {
            name,
            kind,
            ty,
            init,
            line: first.line,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        let tok = self.advance(); // 'if'
        let cond = self.parse_expr()?;
        let then_body = self.parse_block()?;
        let else_body = if self.eat(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                vec![self.parse_if_stmt()?]
            } else {
                self.parse_block()?
            }
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
            line: tok.line,
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let tok = self.advance(); // 'for'
        let (var, _) = self.expect_ident("for loop variable")?;
        self.expect(&TokenKind::In, "after loop variable in `for`")?;
        let first_expr = self.parse_expr()?;
        if self.eat(&TokenKind::DotDot) {
            let end_expr = self.parse_expr()?;
            let body = self.parse_block()?;
            Ok(Stmt::ForRange {
                var,
                start: first_expr,
                end: end_expr,
                body,
                line: tok.line,
            })
        } else {
            let body = self.parse_block()?;
            Ok(Stmt::ForEach {
                var,
                collection: first_expr,
                body,
                line: tok.line,
            })
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_prec(1)
    }

    fn op_prec(kind: &TokenKind) -> Option<u8> {
        match kind {
            TokenKind::PipeGt => Some(1), // |>
            TokenKind::Or => Some(2),     // or (boolean OR & Result fallback / or return)
            TokenKind::And => Some(3),    // and
            TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::Is => Some(4),
            TokenKind::Pipe => Some(5),
            TokenKind::Caret => Some(6),
            TokenKind::Amp => Some(7),
            TokenKind::Shl | TokenKind::Shr => Some(8),
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::WrapAdd
            | TokenKind::WrapSub => Some(9),
            TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::WrapMul => Some(10),
            TokenKind::As => Some(11),
            _ => None,
        }
    }

    fn parse_prec(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;

        while let Some(prec) = Self::op_prec(&self.peek().kind) {
            if prec < min_prec {
                break;
            }
            let op_tok = self.advance();

            // Pipeline `lhs |> func` or `lhs |> func(extra_args...)`
            if op_tok.kind == TokenKind::PipeGt {
                let (callee, cline) = self.expect_ident("after pipeline `|>`")?;
                let mut args = vec![CallArg {
                    mode: ArgPassMode::ByVal,
                    name: None,
                    expr: lhs,
                }];
                if self.eat(&TokenKind::LParen) {
                    let mut extra = self.parse_call_args_inner()?;
                    args.append(&mut extra);
                    self.expect(&TokenKind::RParen, "closing pipeline call")?;
                }
                lhs = Expr::Call {
                    callee,
                    type_args: Vec::new(),
                    args,
                    line: cline,
                };
                continue;
            }

            // `lhs or return [expr]` (Zero-Ceremony early return on Result error!)
            if op_tok.kind == TokenKind::Or && self.eat(&TokenKind::Return) {
                let ret_val = if self.check(&TokenKind::Semi)
                    || self.check(&TokenKind::RBrace)
                    || self.check(&TokenKind::Eof)
                {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                lhs = Expr::OrReturn {
                    lhs: Box::new(lhs),
                    ret_val,
                    line: op_tok.line,
                };
                continue;
            }

            // `lhs as Type`
            if op_tok.kind == TokenKind::As {
                let ty = self.parse_base_type()?;
                lhs = Expr::Cast {
                    expr: Box::new(lhs),
                    ty,
                    line: op_tok.line,
                };
                continue;
            }

            let rhs = self.parse_prec(prec + 1)?;
            let bin_op = match op_tok.kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                TokenKind::WrapAdd => BinOp::WrapAdd,
                TokenKind::WrapSub => BinOp::WrapSub,
                TokenKind::WrapMul => BinOp::WrapMul,
                TokenKind::Amp => BinOp::BitAnd,
                TokenKind::Pipe => BinOp::BitOr,
                TokenKind::Caret => BinOp::BitXor,
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                TokenKind::EqEq | TokenKind::Is => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::GtEq,
                TokenKind::And => BinOp::And,
                TokenKind::Or => BinOp::Or,
                _ => unreachable!(),
            };
            lhs = Expr::Binary {
                op: bin_op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                line: op_tok.line,
            };
        }

        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Minus => {
                self.advance();
                let e = self.parse_unary()?;
                if let Expr::Int(v, l) = e {
                    return Ok(Expr::Int(v.wrapping_neg(), l));
                }
                if let Expr::Dec(v, l) = e {
                    return Ok(Expr::Dec(v.wrapping_neg(), l));
                }
                if let Expr::Float(v, l) = e {
                    return Ok(Expr::Float(-v, l));
                }
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(e),
                    line: tok.line,
                })
            }
            TokenKind::Not | TokenKind::Bang => {
                self.advance();
                let e = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(e),
                    line: tok.line,
                })
            }
            TokenKind::Move => {
                self.advance();
                let (id, l) = self.expect_ident("after `move`")?;
                Ok(Expr::MoveIdent(id, l))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::Dot) {
                let t = self.peek().clone();
                match t.kind {
                    TokenKind::Int(idx) => {
                        self.advance();
                        expr = Expr::TupleIdx {
                            base: Box::new(expr),
                            idx: idx as usize,
                            line: t.line,
                        };
                    }
                    TokenKind::Ident(field_or_method) => {
                        self.advance();
                        if self.eat(&TokenKind::LParen) {
                            let args = self.parse_call_args_inner()?;
                            self.expect(&TokenKind::RParen, "closing method call")?;
                            expr = Expr::MethodCall {
                                receiver: Box::new(expr),
                                method: field_or_method,
                                args,
                                line: t.line,
                            };
                        } else {
                            expr = Expr::Field {
                                base: Box::new(expr),
                                field: field_or_method,
                                line: t.line,
                            };
                        }
                    }
                    _ => {
                        return Err(ParseError {
                            msg: format!("Expected field/method name or tuple index after `.`, found {:?}", t.kind),
                            line: t.line,
                            col: t.col,
                            hint: None,
                        });
                    }
                }
            } else if self.eat(&TokenKind::LBracket) {
                let idx = self.parse_expr()?;
                self.expect(&TokenKind::RBracket, "closing index `[...]`")?;
                let l = expr.line();
                expr = Expr::Index {
                    base: Box::new(expr),
                    idx: Box::new(idx),
                    line: l,
                };
            } else if self.eat(&TokenKind::Question) {
                // Postfix `expr?` sugar -> `expr or return err`!
                let l = expr.line();
                expr = Expr::OrReturn {
                    lhs: Box::new(expr),
                    ret_val: None,
                    line: l,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int(v, tok.line))
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float(v, tok.line))
            }
            TokenKind::Dec(v) => {
                self.advance();
                Ok(Expr::Dec(v, tok.line))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true, tok.line))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false, tok.line))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s, tok.line))
            }
            TokenKind::FStr(segs) => {
                self.advance();
                let mut parts = Vec::new();
                for seg in segs {
                    match seg {
                        FStrSegment::Lit(s) => parts.push(FStrPart::Lit(s)),
                        FStrSegment::Expr(src_snippet) => {
                            let sub_toks = tokenize(&src_snippet).map_err(|le| ParseError {
                                msg: le.msg,
                                line: tok.line,
                                col: tok.col,
                                hint: None,
                            })?;
                            let mut sub_p = Parser::new(sub_toks);
                            let sub_e = sub_p.parse_expr()?;
                            parts.push(FStrPart::Expr(sub_e));
                        }
                    }
                }
                Ok(Expr::FStr(parts, tok.line))
            }
            TokenKind::Attr(attr_name) => {
                self.advance();
                if attr_name == "secret" {
                    self.expect(&TokenKind::LParen, "after `@secret`")?;
                    let secret_tok = self.peek().clone();
                    if let TokenKind::Str(s) = secret_tok.kind {
                        self.advance();
                        self.expect(&TokenKind::RParen, "closing `@secret(...)`")?;
                        Ok(Expr::SecretStr(s, tok.line))
                    } else {
                        Err(ParseError {
                            msg: "`@secret(...)` requires a compile-time string literal".into(),
                            line: secret_tok.line,
                            col: secret_tok.col,
                            hint: Some("Example: `key := @secret(\"MY-MASTER-KEY\")`".into()),
                        })
                    }
                } else {
                    Ok(Expr::Int(1, tok.line))
                }
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Spawn => {
                self.advance();
                let body = self.parse_block()?;
                Ok(Expr::Spawn {
                    body,
                    line: tok.line,
                })
            }
            TokenKind::LParen => {
                self.advance();
                if self.eat(&TokenKind::RParen) {
                    return Ok(Expr::Int(0, tok.line));
                }
                let first = self.parse_expr()?;
                if self.eat(&TokenKind::Comma) {
                    let mut elems = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        elems.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen, "closing tuple `(...)`")?;
                    Ok(Expr::Tuple(elems, tok.line))
                } else {
                    self.expect(&TokenKind::RParen, "closing parenthesized expression")?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                // Array / Comprehension literal `[a, b, c]` or `[x * 2 for x in arr]`
                self.advance();
                let mut count = 0i64;
                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    let _ = self.parse_expr()?;
                    count += 1;
                    if self.eat(&TokenKind::For) {
                        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                            self.advance();
                        }
                        break;
                    }
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBracket, "closing `[...]`")?;
                Ok(Expr::Int(count, tok.line))
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Check `Table[Particle]::with_capacity(n)` or generic call `func[T](...)`
                if name == "Table" && self.eat(&TokenKind::LBracket) {
                    let (elem_ty, _) = self.expect_ident("inside `Table[T]`")?;
                    self.expect(&TokenKind::RBracket, "closing `Table[T]`")?;
                    self.expect(&TokenKind::ColonColon, "after `Table[T]`")?;
                    let (_ctor, _) = self.expect_ident("for `Table[T]::with_capacity`")?;
                    self.expect(&TokenKind::LParen, "opening `Table` constructor")?;
                    let cap = self.parse_expr()?;
                    self.expect(&TokenKind::RParen, "closing `Table` constructor")?;
                    return Ok(Expr::TableAlloc {
                        elem_ty,
                        cap: Box::new(cap),
                        line: tok.line,
                    });
                }

                let mut callee = name;
                while self.eat(&TokenKind::ColonColon) {
                    let (seg, _) = self.expect_ident("after `::`")?;
                    callee = format!("{}__{}", callee, seg);
                }

                let mut type_args = Vec::new();
                if self.check(&TokenKind::LBracket) && self.looks_like_generic_call() {
                    self.advance(); // '['
                    while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                        type_args.push(self.parse_type()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RBracket, "closing generic call `[...]`")?;
                }

                if self.eat(&TokenKind::LParen) {
                    let args = self.parse_call_args_inner()?;
                    self.expect(&TokenKind::RParen, "closing function call `)`")?;
                    Ok(Expr::Call {
                        callee,
                        type_args,
                        args,
                        line: tok.line,
                    })
                } else {
                    Ok(Expr::Ident(callee, tok.line))
                }
            }
            _ => Err(ParseError {
                msg: format!("Expected expression, found {:?}", tok.kind),
                line: tok.line,
                col: tok.col,
                hint: None,
            }),
        }
    }

    fn looks_like_generic_call(&self) -> bool {
        let mut idx = 1usize;
        let mut depth = 1usize;
        while depth > 0 {
            match &self.peek_n(idx).kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        return self.peek_n(idx + 1).kind == TokenKind::LParen;
                    }
                }
                TokenKind::Eof | TokenKind::Semi | TokenKind::LBrace => return false,
                _ => {}
            }
            idx += 1;
        }
        false
    }

    fn parse_call_args_inner(&mut self) -> Result<Vec<CallArg>, ParseError> {
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let mode = if self.eat(&TokenKind::Mut) {
                ArgPassMode::Mut
            } else if self.eat(&TokenKind::Move) {
                ArgPassMode::Move
            } else {
                ArgPassMode::ByVal
            };

            // Named argument `port = 8080` or `port: 8080`
            let mut arg_name = None;
            if let TokenKind::Ident(n) = self.peek().kind.clone() {
                if self.peek_n(1).kind == TokenKind::Eq
                    || self.peek_n(1).kind == TokenKind::Colon
                {
                    arg_name = Some(n);
                    self.advance();
                    self.advance();
                }
            }

            let expr = self.parse_expr()?;
            args.push(CallArg {
                mode,
                name: arg_name,
                expr,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let tok = self.advance(); // 'if'
        let cond = self.parse_expr()?;
        let (then_body, then_tail) = self.parse_expr_block()?;
        self.expect(&TokenKind::Else, "for `if` expression (`if cond { a } else { b }`)")?;
        let (else_body, else_tail) = if self.check(&TokenKind::If) {
            let nested = self.parse_if_expr()?;
            (Vec::new(), nested)
        } else {
            self.parse_expr_block()?
        };
        Ok(Expr::IfExpr {
            cond: Box::new(cond),
            then_body,
            then_tail: Box::new(then_tail),
            else_body,
            else_tail: Box::new(else_tail),
            line: tok.line,
        })
    }

    fn parse_expr_block(&mut self) -> Result<(Vec<Stmt>, Expr), ParseError> {
        let l = self.peek().line;
        let mut stmts = self.parse_block()?;
        if let Some(Stmt::ExprStmt { expr, .. }) = stmts.last().cloned() {
            stmts.pop();
            Ok((stmts, expr))
        } else {
            Ok((stmts, Expr::Int(0, l)))
        }
    }
}
