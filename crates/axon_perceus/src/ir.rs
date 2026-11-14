//! Perceus Intermediate Representation (IR)
//!
//! Models memory management with explicit drop, allocation, reuse tokens,
//! in-place allocation (FBIP), checked/wrapping/saturating arithmetic,
//! and refinement-driven bounds check elision.

use axon_ast::{BinOp, ParamIntent};
use axon_span::ByteSpan;
use axon_types::{GlobalContext, Type};
use lasso::{Rodeo, Spur};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IrVar(pub u32);

impl fmt::Display for IrVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrConst {
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl fmt::Display for IrConst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrConst::Int(i) => write!(f, "{}", i),
            IrConst::Float(fl) => write!(f, "{}", fl),
            IrConst::Bool(b) => write!(f, "{}", b),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Var(IrVar),
    Const(IrConst),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Var(v) => write!(f, "{}", v),
            Operand::Const(c) => write!(f, "{}", c),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsCheckMode {
    Checked,
    Elided,
}

impl fmt::Display for BoundsCheckMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundsCheckMode::Checked => write!(f, "checked"),
            BoundsCheckMode::Elided => write!(f, "elided"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrOp {
    Copy(Operand),
    Binary {
        op: BinOp,
        left: Operand,
        right: Operand,
    },
    Alloc {
        type_name: Spur,
        byte_size: usize,
        fields: Vec<(Spur, Operand)>,
    },
    AllocInPlace {
        token: IrVar,
        type_name: Spur,
        byte_size: usize,
        fields: Vec<(Spur, Operand)>,
    },
    AllocTable {
        elem_type: Type,
        capacity: Operand,
    },
    GetField {
        target: IrVar,
        field: Spur,
    },
    GetIndex {
        target: IrVar,
        index: Operand,
        bounds_check: BoundsCheckMode,
    },
    Call {
        func: Spur,
        args: Vec<Operand>,
    },
    StaticMethodCall {
        target: Spur,
        method: Spur,
        args: Vec<Operand>,
    },
    MethodCall {
        target: IrVar,
        method: Spur,
        args: Vec<Operand>,
    },
    Spawn {
        func_name: Spur,
        args: Vec<Operand>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Assign {
        dest: IrVar,
        op: IrOp,
        span: ByteSpan,
    },
    Drop {
        var: IrVar,
        byte_size: usize,
        span: ByteSpan,
    },
    ReuseToken {
        dest: IrVar,
        source: IrVar,
        byte_size: usize,
        span: ByteSpan,
    },
    SetField {
        target: IrVar,
        field: Spur,
        value: Operand,
        span: ByteSpan,
    },
    SetIndex {
        target: IrVar,
        index: Operand,
        value: Operand,
        bounds_check: BoundsCheckMode,
        span: ByteSpan,
    },
    Eval {
        op: IrOp,
        span: ByteSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return {
        value: Option<Operand>,
        span: ByteSpan,
    },
    Branch {
        target: BlockId,
        span: ByteSpan,
    },
    CondBranch {
        cond: Operand,
        then_block: BlockId,
        else_block: BlockId,
        span: ByteSpan,
    },
}

impl Terminator {
    pub fn span(&self) -> ByteSpan {
        match self {
            Terminator::Return { span, .. } => *span,
            Terminator::Branch { span, .. } => *span,
            Terminator::CondBranch { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefinementConstraint {
    LtField {
        var: Spur,
        target: Spur,
        field: Spur,
    },
    LtConst {
        var: Spur,
        limit: i64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrParam {
    pub name: Spur,
    pub intent: ParamIntent,
    pub ty: Type,
    pub var: IrVar,
    pub refinement: Option<RefinementConstraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: Spur,
    pub params: Vec<IrParam>,
    pub ret_type: Type,
    pub blocks: Vec<BasicBlock>,
    pub var_types: Vec<Type>,
    pub var_names: HashMap<IrVar, Spur>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub ctx: GlobalContext,
}

impl IrFunction {
    pub fn display(&self, interner: &Rodeo) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let fn_name = interner.resolve(&self.name);
        let _ = write!(out, "fn {}(", fn_name);
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            let pname = interner.resolve(&p.name);
            let intent_str = match p.intent {
                ParamIntent::Let => "let ",
                ParamIntent::Mut => "mut ",
                ParamIntent::Move => "move ",
                ParamIntent::Isolated => "isolated ",
            };
            let ref_str = match &p.refinement {
                Some(RefinementConstraint::LtField { target, field, .. }) => {
                    format!(" where {} < {}.{}", pname, interner.resolve(target), interner.resolve(field))
                }
                Some(RefinementConstraint::LtConst { limit, .. }) => {
                    format!(" where {} < {}", pname, limit)
                }
                None => String::new(),
            };
            let _ = write!(out, "{}{}: {:?}{}", intent_str, pname, p.ty, ref_str);
        }
        let _ = writeln!(out, ") -> {:?} {{", self.ret_type);

        for block in &self.blocks {
            let _ = writeln!(out, "  {}:", block.id);
            for inst in &block.instructions {
                match inst {
                    Instruction::Assign { dest, op, .. } => {
                        let _ = write!(out, "    {} = ", dest);
                        Self::display_op(&mut out, op, interner);
                        out.push('\n');
                    }
                    Instruction::Drop { var, byte_size, .. } => {
                        let _ = writeln!(out, "    drop {} (size: {}B)", var, byte_size);
                    }
                    Instruction::ReuseToken { dest, source, byte_size, .. } => {
                        let _ = writeln!(out, "    reuse_token {} = {} (size: {}B)", dest, source, byte_size);
                    }
                    Instruction::SetField { target, field, value, .. } => {
                        let _ = writeln!(out, "    {}.{} = {}", target, interner.resolve(field), value);
                    }
                    Instruction::SetIndex { target, index, value, bounds_check, .. } => {
                        let _ = writeln!(out, "    {}[{}] = {} [bounds: {}]", target, index, value, bounds_check);
                    }
                    Instruction::Eval { op, .. } => {
                        out.push_str("    eval ");
                        Self::display_op(&mut out, op, interner);
                        out.push('\n');
                    }
                }
            }
            match &block.terminator {
                Terminator::Return { value: Some(val), .. } => {
                    let _ = writeln!(out, "    return {}", val);
                }
                Terminator::Return { value: None, .. } => {
                    let _ = writeln!(out, "    return");
                }
                Terminator::Branch { target, .. } => {
                    let _ = writeln!(out, "    goto {}", target);
                }
                Terminator::CondBranch { cond, then_block, else_block, .. } => {
                    let _ = writeln!(out, "    br_if {} then {} else {}", cond, then_block, else_block);
                }
            }
        }
        out.push_str("}\n");
        out
    }

    fn display_op(out: &mut String, op: &IrOp, interner: &Rodeo) {
        use std::fmt::Write;
        match op {
            IrOp::Copy(op) => {
                let _ = write!(out, "{}", op);
            }
            IrOp::Binary { op, left, right } => {
                let _ = write!(out, "{} {:?} {}", left, op, right);
            }
            IrOp::Alloc { type_name, byte_size, .. } => {
                let _ = write!(out, "alloc {} (size: {}B)", interner.resolve(type_name), byte_size);
            }
            IrOp::AllocInPlace { token, type_name, byte_size, .. } => {
                let _ = write!(
                    out,
                    "alloc_in_place(token: {}) {} (size: {}B)",
                    token,
                    interner.resolve(type_name),
                    byte_size
                );
            }
            IrOp::AllocTable { elem_type, capacity } => {
                let _ = write!(out, "alloc_table[{:?}] with capacity {}", elem_type, capacity);
            }
            IrOp::GetField { target, field } => {
                let _ = write!(out, "{}.{}", target, interner.resolve(field));
            }
            IrOp::GetIndex { target, index, bounds_check } => {
                let _ = write!(out, "{}[{}] [bounds: {}]", target, index, bounds_check);
            }
            IrOp::Call { func, args } => {
                let _ = write!(out, "call {}(", interner.resolve(func));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{}", a);
                }
                out.push(')');
            }
            IrOp::StaticMethodCall { target, method, args } => {
                let _ = write!(out, "call {}::{}(", interner.resolve(target), interner.resolve(method));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{}", a);
                }
                out.push(')');
            }
            IrOp::MethodCall { target, method, args } => {
                let _ = write!(out, "call {}.{}(", target, interner.resolve(method));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{}", a);
                }
                out.push(')');
            }
            IrOp::Spawn { func_name, args } => {
                let _ = write!(out, "spawn {}(", interner.resolve(func_name));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{}", a);
                }
                out.push(')');
            }
        }
    }
}
