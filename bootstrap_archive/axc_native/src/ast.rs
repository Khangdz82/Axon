//! Axon v5.0 Abstract Syntax Tree (`ast.rs`)
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub enum TypeSpec {
    Named(String),
    Generic(String, Vec<TypeSpec>),
    Ref(Box<TypeSpec>),
    Tuple(Vec<TypeSpec>),
    Result(Box<TypeSpec>, Box<TypeSpec>), // T ! E
    Refined(Box<TypeSpec>, Box<Expr>),    // usize where idx < arr.len
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    Immutable,
    LetExplicit,
    Mut,
    Move,
    Isolated,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub mode: ParamMode,
    pub ty: TypeSpec,
    pub default_val: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: TypeSpec,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct InterfaceMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: TypeSpec,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<InterfaceMethodSig>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub attrs: Vec<(String, Option<String>)>,
    pub params: Vec<Param>,
    pub ret_ty: TypeSpec,
    pub body: Vec<Stmt>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub module_attrs: Vec<String>,
    pub uses: Vec<UseDecl>,
    pub structs: Vec<StructDecl>,
    pub interfaces: Vec<InterfaceDecl>,
    pub funcs: Vec<FnDecl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Immutable,
    Mut,
    Isolated,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        kind: VarKind,
        ty: Option<TypeSpec>,
        init: Expr,
        line: usize,
    },
    TupleDestruct {
        names: Vec<String>,
        kind: VarKind,
        init: Expr,
        line: usize,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        val: Expr,
        line: usize,
    },
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        line: usize,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    ForRange {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    ForEach {
        var: String,
        collection: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    With {
        var: String,
        init: Expr,
        cleanup_fn: Option<String>,
        body: Vec<Stmt>,
        line: usize,
    },
    Return {
        val: Option<Expr>,
        line: usize,
    },
    Break {
        line: usize,
    },
    Continue {
        line: usize,
    },
    ExprStmt {
        expr: Expr,
        line: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Eq,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    WrapAdd,
    WrapSub,
    WrapMul,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or, // Logical OR on bool/int, OR fallback value on Result `T!E`!
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgPassMode {
    ByVal,
    Mut,
    Move,
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub mode: ArgPassMode,
    pub name: Option<String>,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub enum FStrPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, usize),
    Float(f64, usize),
    Dec(i64, usize),
    Bool(bool, usize),
    Str(String, usize),
    SecretStr(String, usize), // @secret("...")
    FStr(Vec<FStrPart>, usize),
    Ident(String, usize),
    MoveIdent(String, usize),
    Tuple(Vec<Expr>, usize),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        line: usize,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        line: usize,
    },
    OrReturn {
        lhs: Box<Expr>,
        ret_val: Option<Box<Expr>>,
        line: usize,
    },
    Cast {
        expr: Box<Expr>,
        ty: TypeSpec,
        line: usize,
    },
    Field {
        base: Box<Expr>,
        field: String,
        line: usize,
    },
    TupleIdx {
        base: Box<Expr>,
        idx: usize,
        line: usize,
    },
    Index {
        base: Box<Expr>,
        idx: Box<Expr>,
        line: usize,
    },
    Call {
        callee: String,
        type_args: Vec<TypeSpec>,
        args: Vec<CallArg>,
        line: usize,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<CallArg>,
        line: usize,
    },
    TableAlloc {
        elem_ty: String,
        cap: Box<Expr>,
        line: usize,
    },
    IfExpr {
        cond: Box<Expr>,
        then_body: Vec<Stmt>,
        then_tail: Box<Expr>,
        else_body: Vec<Stmt>,
        else_tail: Box<Expr>,
        line: usize,
    },
    Spawn {
        body: Vec<Stmt>,
        line: usize,
    },
}

impl Expr {
    pub fn line(&self) -> usize {
        match self {
            Expr::Int(_, l)
            | Expr::Float(_, l)
            | Expr::Dec(_, l)
            | Expr::Bool(_, l)
            | Expr::Str(_, l)
            | Expr::SecretStr(_, l)
            | Expr::FStr(_, l)
            | Expr::Ident(_, l)
            | Expr::MoveIdent(_, l)
            | Expr::Tuple(_, l) => *l,
            Expr::Unary { line, .. }
            | Expr::Binary { line, .. }
            | Expr::OrReturn { line, .. }
            | Expr::Cast { line, .. }
            | Expr::Field { line, .. }
            | Expr::TupleIdx { line, .. }
            | Expr::Index { line, .. }
            | Expr::Call { line, .. }
            | Expr::MethodCall { line, .. }
            | Expr::TableAlloc { line, .. }
            | Expr::IfExpr { line, .. }
            | Expr::Spawn { line, .. } => *line,
        }
    }
}
