//! axon_ast: Flat Index-Based AST allocated on bumpalo arena

use axon_span::ByteSpan;
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use lasso::Spur;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StmtId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    I8, I16, I32, I64,
    U8, U16, U32, U64, Usize,
    F32, F64,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeData {
    Primitive(PrimitiveType, ByteSpan),
    Named(Spur, ByteSpan),
    Table(Spur, ByteSpan),
    Reference {
        inner: TypeId,
        is_mut: bool,
        span: ByteSpan,
    },
    Refined {
        base: TypeId,
        predicate: ExprId,
        span: ByteSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    AddWrap, SubWrap, MulWrap,
    AddSat, SubSat, MulSat,
    Eq, NotEq, Lt, LtEq, Gt, GtEq,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamIntent {
    Let,
    Mut,
    Move,
    Isolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallIntent {
    Let,
    Mut,
    Move,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprData<'a> {
    Int(i64, ByteSpan),
    Float(f64, ByteSpan),
    Bool(bool, ByteSpan),
    Ident(Spur, ByteSpan),
    Binary {
        op: BinOp,
        left: ExprId,
        right: ExprId,
        span: ByteSpan,
    },
    FieldAccess {
        target: ExprId,
        field: Spur,
        span: ByteSpan,
    },
    IndexAccess {
        target: ExprId,
        index: ExprId,
        span: ByteSpan,
    },
    Call {
        callee: Spur,
        args: &'a [(CallIntent, ExprId)],
        span: ByteSpan,
    },
    StaticMethodCall {
        target: Spur,
        method: Spur,
        args: &'a [ExprId],
        span: ByteSpan,
    },
    MethodCall {
        target: ExprId,
        method: Spur,
        args: &'a [ExprId],
        span: ByteSpan,
    },
    Spawn {
        body: &'a [StmtId],
        span: ByteSpan,
    },
}

impl<'a> ExprData<'a> {
    pub fn span(&self) -> ByteSpan {
        match self {
            ExprData::Int(_, span)
            | ExprData::Float(_, span)
            | ExprData::Bool(_, span)
            | ExprData::Ident(_, span)
            | ExprData::Binary { span, .. }
            | ExprData::FieldAccess { span, .. }
            | ExprData::IndexAccess { span, .. }
            | ExprData::Call { span, .. }
            | ExprData::StaticMethodCall { span, .. }
            | ExprData::MethodCall { span, .. }
            | ExprData::Spawn { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtData<'a> {
    Let {
        name: Spur,
        ty: Option<TypeId>,
        init: ExprId,
        span: ByteSpan,
    },
    IsolatedLet {
        name: Spur,
        ty: Option<TypeId>,
        init: ExprId,
        span: ByteSpan,
    },
    Assign {
        target: ExprId,
        op: AssignOp,
        value: ExprId,
        span: ByteSpan,
    },
    For {
        loop_var: Spur,
        iter_target: ExprId,
        body: &'a [StmtId],
        span: ByteSpan,
    },
    While {
        cond: ExprId,
        body: &'a [StmtId],
        span: ByteSpan,
    },
    If {
        cond: ExprId,
        then_branch: &'a [StmtId],
        else_branch: Option<&'a [StmtId]>,
        span: ByteSpan,
    },
    Return {
        is_move: bool,
        value: Option<ExprId>,
        span: ByteSpan,
    },
    Expr(ExprId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldDef {
    pub name: Spur,
    pub ty: TypeId,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamDef {
    pub intent: ParamIntent,
    pub name: Spur,
    pub ty: TypeId,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef<'a> {
    pub name: Spur,
    pub fields: &'a [FieldDef],
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDef<'a> {
    pub name: Spur,
    pub params: &'a [ParamDef],
    pub ret_type: Option<TypeId>,
    pub body: &'a [StmtId],
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program<'a> {
    pub structs: &'a [StructDef<'a>],
    pub functions: &'a [FuncDef<'a>],
}

#[derive(Clone, Copy)]
pub struct FrozenAst<'b, 'a> {
    pub exprs: &'b [ExprData<'a>],
    pub stmts: &'b [StmtData<'a>],
    pub types: &'b [TypeData],
}

impl<'b, 'a> FrozenAst<'b, 'a> {
    pub fn get_expr(&self, id: ExprId) -> &ExprData<'a> {
        &self.exprs[id.0 as usize]
    }

    pub fn get_stmt(&self, id: StmtId) -> &StmtData<'a> {
        &self.stmts[id.0 as usize]
    }

    pub fn get_type(&self, id: TypeId) -> &TypeData {
        &self.types[id.0 as usize]
    }
}

pub struct AstArena<'a> {
    pub bump: &'a Bump,
    pub exprs: BumpVec<'a, ExprData<'a>>,
    pub stmts: BumpVec<'a, StmtData<'a>>,
    pub types: BumpVec<'a, TypeData>,
}

impl<'a> AstArena<'a> {
    pub fn new(bump: &'a Bump) -> Self {
        Self {
            bump,
            exprs: BumpVec::new_in(bump),
            stmts: BumpVec::new_in(bump),
            types: BumpVec::new_in(bump),
        }
    }

    pub fn freeze<'b>(&'b self) -> FrozenAst<'b, 'a> {
        FrozenAst {
            exprs: self.exprs.as_slice(),
            stmts: self.stmts.as_slice(),
            types: self.types.as_slice(),
        }
    }

    pub fn alloc_expr(&mut self, expr: ExprData<'a>) -> ExprId {
        let id = ExprId(self.exprs.len() as u32);
        self.exprs.push(expr);
        id
    }

    pub fn get_expr(&self, id: ExprId) -> &ExprData<'a> {
        &self.exprs[id.0 as usize]
    }

    pub fn alloc_stmt(&mut self, stmt: StmtData<'a>) -> StmtId {
        let id = StmtId(self.stmts.len() as u32);
        self.stmts.push(stmt);
        id
    }

    pub fn get_stmt(&self, id: StmtId) -> &StmtData<'a> {
        &self.stmts[id.0 as usize]
    }

    pub fn alloc_type(&mut self, ty: TypeData) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    pub fn get_type(&self, id: TypeId) -> &TypeData {
        &self.types[id.0 as usize]
    }
}
