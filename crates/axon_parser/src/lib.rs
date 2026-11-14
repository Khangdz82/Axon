//! axon_parser: Hand-written Recursive Descent + Pratt Expression Parser

use axon_ast::*;
use axon_lexer::{Token, TokenKind};
use axon_span::ByteSpan;
use bumpalo::collections::Vec as BumpVec;
use lasso::{Rodeo, Spur};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
#[error("Parser error at {span:?}: {msg}")]
pub struct ParseError {
    pub span: ByteSpan,
    pub msg: String,
}

pub struct Parser<'a, 'b> {
    tokens: &'b [Token],
    pos: usize,
    arena: &'b mut AstArena<'a>,
    interner: &'b mut Rodeo,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(
        tokens: &'b [Token],
        arena: &'b mut AstArena<'a>,
        interner: &'b mut Rodeo,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            arena,
            interner,
        }
    }

    fn current(&self) -> Token {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos]
        } else {
            *self.tokens.last().unwrap()
        }
    }

    #[allow(dead_code)]
    fn peek(&self, offset: usize) -> Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        self.tokens[idx]
    }

    fn advance(&mut self) -> Token {
        let tok = self.current();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn error<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        let tok = self.current();
        Err(ParseError {
            span: tok.span,
            msg: msg.into(),
        })
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        let tok = self.current();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            self.error(format!("Expected {:?}, found {:?}", expected, tok.kind))
        }
    }

    fn expect_ident(&mut self) -> Result<(Spur, ByteSpan), ParseError> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Ident(spur) => {
                self.advance();
                Ok((spur, tok.span))
            }
            _ => self.error(format!("Expected identifier, found {:?}", tok.kind)),
        }
    }

    fn expect_member_ident(&mut self) -> Result<(Spur, ByteSpan), ParseError> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Ident(spur) => {
                self.advance();
                Ok((spur, tok.span))
            }
            TokenKind::Join => {
                self.advance();
                let spur = self.interner.get_or_intern("join");
                Ok((spur, tok.span))
            }
            TokenKind::Spawn => {
                self.advance();
                let spur = self.interner.get_or_intern("spawn");
                Ok((spur, tok.span))
            }
            TokenKind::Where => {
                self.advance();
                let spur = self.interner.get_or_intern("where");
                Ok((spur, tok.span))
            }
            _ => self.error(format!("Expected member identifier, found {:?}", tok.kind)),
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        let tok = self.current();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_program(&mut self) -> Result<Program<'a>, ParseError> {
        let mut structs = BumpVec::new_in(self.arena.bump);
        let mut functions = BumpVec::new_in(self.arena.bump);

        while self.current().kind != TokenKind::Eof {
            match self.current().kind {
                TokenKind::Struct => {
                    structs.push(self.parse_struct()?);
                }
                TokenKind::Fn => {
                    functions.push(self.parse_function()?);
                }
                _ => {
                    return self.error(format!(
                        "Expected top-level declaration (struct or fn), found {:?}",
                        self.current().kind
                    ));
                }
            }
        }

        Ok(Program {
            structs: structs.into_bump_slice(),
            functions: functions.into_bump_slice(),
        })
    }

    pub fn parse_struct(&mut self) -> Result<StructDef<'a>, ParseError> {
        let start = self.expect(TokenKind::Struct)?.span;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut fields = BumpVec::new_in(self.arena.bump);
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            let (fname, f_span) = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let fty = self.parse_type()?;
            fields.push(FieldDef {
                name: fname,
                ty: fty,
                span: f_span,
            });
            self.match_token(TokenKind::Comma);
        }

        let end = self.expect(TokenKind::RBrace)?.span;
        Ok(StructDef {
            name,
            fields: fields.into_bump_slice(),
            span: start.merge(end),
        })
    }

    pub fn parse_type(&mut self) -> Result<TypeId, ParseError> {
        let tok = self.current();
        let ty_id = match tok.kind {
            TokenKind::Amp => {
                let start = self.advance().span;
                let is_mut = self.match_token(TokenKind::Mut);
                let inner = self.parse_type()?;
                let end = match self.arena.get_type(inner) {
                    TypeData::Primitive(_, s)
                    | TypeData::Named(_, s)
                    | TypeData::Table(_, s)
                    | TypeData::Reference { span: s, .. }
                    | TypeData::Refined { span: s, .. } => *s,
                };
                self.arena.alloc_type(TypeData::Reference {
                    inner,
                    is_mut,
                    span: start.merge(end),
                })
            }
            TokenKind::Table => {
                let start = self.advance().span;
                self.expect(TokenKind::LBracket)?;
                let (elem, _) = self.expect_ident()?;
                let end = self.expect(TokenKind::RBracket)?.span;
                self.arena.alloc_type(TypeData::Table(elem, start.merge(end)))
            }
            TokenKind::Ident(spur) => {
                let span = tok.span;
                self.advance();
                let name = self.interner.resolve(&spur);
                let prim = match name {
                    "i8" => Some(PrimitiveType::I8),
                    "i16" => Some(PrimitiveType::I16),
                    "i32" => Some(PrimitiveType::I32),
                    "i64" => Some(PrimitiveType::I64),
                    "u8" => Some(PrimitiveType::U8),
                    "u16" => Some(PrimitiveType::U16),
                    "u32" => Some(PrimitiveType::U32),
                    "u64" => Some(PrimitiveType::U64),
                    "usize" => Some(PrimitiveType::Usize),
                    "f32" => Some(PrimitiveType::F32),
                    "f64" => Some(PrimitiveType::F64),
                    "bool" => Some(PrimitiveType::Bool),
                    "void" => Some(PrimitiveType::Void),
                    _ => None,
                };
                if let Some(p) = prim {
                    self.arena.alloc_type(TypeData::Primitive(p, span))
                } else {
                    self.arena.alloc_type(TypeData::Named(spur, span))
                }
            }
            _ => return self.error(format!("Expected type name, found {:?}", tok.kind)),
        };

        // Support Refinement Types: `T where <predicate>`
        if self.match_token(TokenKind::Where) {
            let pred = self.parse_expr(0)?;
            let pred_span = self.arena.get_expr(pred).span();
            let base_span = match self.arena.get_type(ty_id) {
                TypeData::Primitive(_, s)
                | TypeData::Named(_, s)
                | TypeData::Table(_, s)
                | TypeData::Reference { span: s, .. } => *s,
                TypeData::Refined { span, .. } => *span,
            };
            return Ok(self.arena.alloc_type(TypeData::Refined {
                base: ty_id,
                predicate: pred,
                span: base_span.merge(pred_span),
            }));
        }

        Ok(ty_id)
    }

    pub fn parse_function(&mut self) -> Result<FuncDef<'a>, ParseError> {
        let start = self.expect(TokenKind::Fn)?.span;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;

        let mut params = BumpVec::new_in(self.arena.bump);
        if self.current().kind != TokenKind::RParen {
            params.push(self.parse_param()?);
            while self.match_token(TokenKind::Comma) {
                if self.current().kind == TokenKind::RParen {
                    break;
                }
                params.push(self.parse_param()?);
            }
        }
        self.expect(TokenKind::RParen)?;

        let ret_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        let end = self.expect(TokenKind::RBrace)?.span;

        Ok(FuncDef {
            name,
            params: params.into_bump_slice(),
            ret_type,
            body,
            span: start.merge(end),
        })
    }

    fn parse_param(&mut self) -> Result<ParamDef, ParseError> {
        let mut intent = ParamIntent::Let;
        let start = self.current().span;
        match self.current().kind {
            TokenKind::Mut => {
                self.advance();
                intent = ParamIntent::Mut;
            }
            TokenKind::Move => {
                self.advance();
                intent = ParamIntent::Move;
            }
            TokenKind::Isolated => {
                self.advance();
                intent = ParamIntent::Isolated;
            }
            TokenKind::Let => {
                self.advance();
                intent = ParamIntent::Let;
            }
            _ => {}
        }

        let (pname, pspan) = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;

        Ok(ParamDef {
            intent,
            name: pname,
            ty,
            span: start.merge(pspan),
        })
    }

    fn parse_block(&mut self) -> Result<&'a [StmtId], ParseError> {
        let mut stmts = BumpVec::new_in(self.arena.bump);
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts.into_bump_slice())
    }

    pub fn parse_stmt(&mut self) -> Result<StmtId, ParseError> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Let => {
                let start = self.advance().span;
                let (name, _) = self.expect_ident()?;
                let ty = if self.match_token(TokenKind::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect(TokenKind::Eq)?;
                let init = self.parse_expr(0)?;
                self.match_token(TokenKind::Semi);
                let end = self.arena.get_expr(init).span();
                Ok(self.arena.alloc_stmt(StmtData::Let {
                    name,
                    ty,
                    init,
                    span: start.merge(end),
                }))
            }

            TokenKind::Isolated => {
                let start = self.advance().span;
                let (name, _) = self.expect_ident()?;
                let ty = if self.match_token(TokenKind::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect(TokenKind::Eq)?;
                let init = self.parse_expr(0)?;
                self.match_token(TokenKind::Semi);
                let end = self.arena.get_expr(init).span();
                Ok(self.arena.alloc_stmt(StmtData::IsolatedLet {
                    name,
                    ty,
                    init,
                    span: start.merge(end),
                }))
            }

            TokenKind::For => {
                let start = self.advance().span;
                let (loop_var, _) = self.expect_ident()?;
                self.expect(TokenKind::In)?;
                let (iter_name, ispan) = self.expect_ident()?;
                let iter_expr = self.arena.alloc_expr(ExprData::Ident(iter_name, ispan));

                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block()?;
                let end = self.expect(TokenKind::RBrace)?.span;

                Ok(self.arena.alloc_stmt(StmtData::For {
                    loop_var,
                    iter_target: iter_expr,
                    body,
                    span: start.merge(end),
                }))
            }

            TokenKind::While => {
                let start = self.advance().span;
                let cond = self.parse_expr(0)?;
                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block()?;
                let end = self.expect(TokenKind::RBrace)?.span;
                Ok(self.arena.alloc_stmt(StmtData::While {
                    cond,
                    body,
                    span: start.merge(end),
                }))
            }

            TokenKind::If => {
                let start = self.advance().span;
                let cond = self.parse_expr(0)?;
                self.expect(TokenKind::LBrace)?;
                let then_branch = self.parse_block()?;
                let mut end = self.expect(TokenKind::RBrace)?.span;

                let else_branch = if self.match_token(TokenKind::Else) {
                    if self.current().kind == TokenKind::If {
                        let stmt = self.parse_stmt()?;
                        let mut v = BumpVec::new_in(self.arena.bump);
                        v.push(stmt);
                        Some(v.into_bump_slice())
                    } else {
                        self.expect(TokenKind::LBrace)?;
                        let block = self.parse_block()?;
                        end = self.expect(TokenKind::RBrace)?.span;
                        Some(block)
                    }
                } else {
                    None
                };

                Ok(self.arena.alloc_stmt(StmtData::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: start.merge(end),
                }))
            }

            TokenKind::Return => {
                let start = self.advance().span;
                let is_move = self.match_token(TokenKind::Move);
                let (val, end) = if self.current().kind != TokenKind::Semi
                    && self.current().kind != TokenKind::RBrace
                {
                    let e = self.parse_expr(0)?;
                    let span = self.arena.get_expr(e).span();
                    (Some(e), span)
                } else {
                    (None, start)
                };
                self.match_token(TokenKind::Semi);
                Ok(self.arena.alloc_stmt(StmtData::Return {
                    is_move,
                    value: val,
                    span: start.merge(end),
                }))
            }

            _ => {
                let expr = self.parse_expr(0)?;
                // Check if this is an assignment: expr op expr
                let assign_op = match self.current().kind {
                    TokenKind::Eq => Some(AssignOp::Assign),
                    TokenKind::PlusEq => Some(AssignOp::PlusAssign),
                    TokenKind::MinusEq => Some(AssignOp::MinusAssign),
                    TokenKind::StarEq => Some(AssignOp::StarAssign),
                    TokenKind::SlashEq => Some(AssignOp::SlashAssign),
                    _ => None,
                };

                if let Some(op) = assign_op {
                    self.advance();
                    let val = self.parse_expr(0)?;
                    self.match_token(TokenKind::Semi);
                    let start = self.arena.get_expr(expr).span();
                    let end = self.arena.get_expr(val).span();
                    Ok(self.arena.alloc_stmt(StmtData::Assign {
                        target: expr,
                        op,
                        value: val,
                        span: start.merge(end),
                    }))
                } else {
                    self.match_token(TokenKind::Semi);
                    Ok(self.arena.alloc_stmt(StmtData::Expr(expr)))
                }
            }
        }
    }

    // Pratt Expression Parser
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<ExprId, ParseError> {
        let mut lhs = self.parse_primary()?;

        loop {
            // Postfix operators
            match self.current().kind {
                TokenKind::Dot => {
                    self.advance();
                    let (member, m_span) = self.expect_member_ident()?;
                    if self.match_token(TokenKind::LParen) {
                        let mut args = BumpVec::new_in(self.arena.bump);
                        if self.current().kind != TokenKind::RParen {
                            args.push(self.parse_expr(0)?);
                            while self.match_token(TokenKind::Comma) {
                                if self.current().kind == TokenKind::RParen {
                                    break;
                                }
                                args.push(self.parse_expr(0)?);
                            }
                        }
                        let end = self.expect(TokenKind::RParen)?.span;
                        let start = self.arena.get_expr(lhs).span();
                        lhs = self.arena.alloc_expr(ExprData::MethodCall {
                            target: lhs,
                            method: member,
                            args: args.into_bump_slice(),
                            span: start.merge(end),
                        });
                        continue;
                    } else {
                        let start = self.arena.get_expr(lhs).span();
                        lhs = self.arena.alloc_expr(ExprData::FieldAccess {
                            target: lhs,
                            field: member,
                            span: start.merge(m_span),
                        });
                        continue;
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr(0)?;
                    let end = self.expect(TokenKind::RBracket)?.span;
                    let start = self.arena.get_expr(lhs).span();
                    lhs = self.arena.alloc_expr(ExprData::IndexAccess {
                        target: lhs,
                        index: idx,
                        span: start.merge(end),
                    });
                    continue;
                }
                _ => {}
            }

            // Infix Binary Operators
            let (op, (l_bp, r_bp)) = match self.current().kind {
                TokenKind::OrOr => (BinOp::Or, (1, 2)),
                TokenKind::AndAnd => (BinOp::And, (3, 4)),
                TokenKind::EqEq => (BinOp::Eq, (5, 6)),
                TokenKind::NotEq => (BinOp::NotEq, (5, 6)),
                TokenKind::Lt => (BinOp::Lt, (5, 6)),
                TokenKind::LtEq => (BinOp::LtEq, (5, 6)),
                TokenKind::Gt => (BinOp::Gt, (5, 6)),
                TokenKind::GtEq => (BinOp::GtEq, (5, 6)),

                TokenKind::Plus => (BinOp::Add, (7, 8)),
                TokenKind::Minus => (BinOp::Sub, (7, 8)),
                TokenKind::PlusWrap => (BinOp::AddWrap, (7, 8)),
                TokenKind::MinusWrap => (BinOp::SubWrap, (7, 8)),
                TokenKind::PlusSat => (BinOp::AddSat, (7, 8)),
                TokenKind::MinusSat => (BinOp::SubSat, (7, 8)),

                TokenKind::Star => (BinOp::Mul, (9, 10)),
                TokenKind::Slash => (BinOp::Div, (9, 10)),
                TokenKind::Percent => (BinOp::Mod, (9, 10)),
                TokenKind::StarWrap => (BinOp::MulWrap, (9, 10)),
                TokenKind::StarSat => (BinOp::MulSat, (9, 10)),

                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.advance();
            let rhs = self.parse_expr(r_bp)?;
            let start = self.arena.get_expr(lhs).span();
            let end = self.arena.get_expr(rhs).span();

            lhs = self.arena.alloc_expr(ExprData::Binary {
                op,
                left: lhs,
                right: rhs,
                span: start.merge(end),
            });
        }

        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<ExprId, ParseError> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Minus => {
                let start = self.advance().span;
                let next_tok = self.current();
                match next_tok.kind {
                    TokenKind::Int(v) => {
                        self.advance();
                        Ok(self.arena.alloc_expr(ExprData::Int(-v, start.merge(next_tok.span))))
                    }
                    TokenKind::Float(v) => {
                        self.advance();
                        Ok(self.arena.alloc_expr(ExprData::Float(-v, start.merge(next_tok.span))))
                    }
                    _ => {
                        let inner = self.parse_expr(11)?;
                        let end = self.arena.get_expr(inner).span();
                        let zero = self.arena.alloc_expr(ExprData::Int(0, start));
                        Ok(self.arena.alloc_expr(ExprData::Binary {
                            op: BinOp::Sub,
                            left: zero,
                            right: inner,
                            span: start.merge(end),
                        }))
                    }
                }
            }
            TokenKind::Int(v) => {
                self.advance();
                Ok(self.arena.alloc_expr(ExprData::Int(v, tok.span)))
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(self.arena.alloc_expr(ExprData::Float(v, tok.span)))
            }
            TokenKind::True => {
                self.advance();
                Ok(self.arena.alloc_expr(ExprData::Bool(true, tok.span)))
            }
            TokenKind::False => {
                self.advance();
                Ok(self.arena.alloc_expr(ExprData::Bool(false, tok.span)))
            }
            TokenKind::Spawn => {
                let start = self.advance().span;
                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block()?;
                let end = self.expect(TokenKind::RBrace)?.span;
                Ok(self.arena.alloc_expr(ExprData::Spawn {
                    body,
                    span: start.merge(end),
                }))
            }
            TokenKind::Table => {
                // Table[Type]::with_capacity(...)
                let start = self.advance().span;
                self.expect(TokenKind::LBracket)?;
                let (elem, _) = self.expect_ident()?;
                self.expect(TokenKind::RBracket)?;
                self.expect(TokenKind::ColonColon)?;
                let (method, _) = self.expect_ident()?;
                self.expect(TokenKind::LParen)?;
                let mut args = BumpVec::new_in(self.arena.bump);
                if self.current().kind != TokenKind::RParen {
                    args.push(self.parse_expr(0)?);
                    while self.match_token(TokenKind::Comma) {
                        if self.current().kind == TokenKind::RParen {
                            break;
                        }
                        args.push(self.parse_expr(0)?);
                    }
                }
                let end = self.expect(TokenKind::RParen)?.span;
                Ok(self.arena.alloc_expr(ExprData::StaticMethodCall {
                    target: elem,
                    method,
                    args: args.into_bump_slice(),
                    span: start.merge(end),
                }))
            }
            TokenKind::Ident(spur) => {
                let span = tok.span;
                self.advance();

                // Check for function call
                if self.match_token(TokenKind::LParen) {
                    let mut args = BumpVec::new_in(self.arena.bump);
                    if self.current().kind != TokenKind::RParen {
                        args.push(self.parse_call_arg()?);
                        while self.match_token(TokenKind::Comma) {
                            if self.current().kind == TokenKind::RParen {
                                break;
                            }
                            args.push(self.parse_call_arg()?);
                        }
                    }
                    let end = self.expect(TokenKind::RParen)?.span;
                    Ok(self.arena.alloc_expr(ExprData::Call {
                        callee: spur,
                        args: args.into_bump_slice(),
                        span: span.merge(end),
                    }))
                } else if self.match_token(TokenKind::ColonColon) {
                    // Static call like Type::method()
                    let (method, _) = self.expect_ident()?;
                    self.expect(TokenKind::LParen)?;
                    let mut args = BumpVec::new_in(self.arena.bump);
                    if self.current().kind != TokenKind::RParen {
                        args.push(self.parse_expr(0)?);
                        while self.match_token(TokenKind::Comma) {
                            if self.current().kind == TokenKind::RParen {
                                break;
                            }
                            args.push(self.parse_expr(0)?);
                        }
                    }
                    let end = self.expect(TokenKind::RParen)?.span;
                    Ok(self.arena.alloc_expr(ExprData::StaticMethodCall {
                        target: spur,
                        method,
                        args: args.into_bump_slice(),
                        span: span.merge(end),
                    }))
                } else {
                    Ok(self.arena.alloc_expr(ExprData::Ident(spur, span)))
                }
            }
            TokenKind::LParen => {
                let _start = self.advance().span;
                let inner = self.parse_expr(0)?;
                let _end = self.expect(TokenKind::RParen)?.span;
                Ok(inner)
            }
            _ => self.error(format!("Unexpected token in expression: {:?}", tok.kind)),
        }
    }

    fn parse_call_arg(&mut self) -> Result<(CallIntent, ExprId), ParseError> {
        let intent = match self.current().kind {
            TokenKind::Mut => {
                self.advance();
                CallIntent::Mut
            }
            TokenKind::Move => {
                self.advance();
                CallIntent::Move
            }
            TokenKind::Let => {
                self.advance();
                CallIntent::Let
            }
            _ => CallIntent::Let,
        };
        let expr = self.parse_expr(0)?;
        Ok((intent, expr))
    }
}
