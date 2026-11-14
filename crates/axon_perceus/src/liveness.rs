//! Liveness Analysis and Drop Insertion pass
//!
//! Determines when composite/heap objects (Structs, Tables) are last used
//! and inserts explicit `Drop` instructions at their exact end-of-life.

use crate::ir::*;
use crate::size::compute_type_size;
use axon_span::ByteSpan;
use axon_types::{GlobalContext, Type};
use std::collections::{HashMap, HashSet};

pub struct LivenessPass<'a> {
    ctx: &'a GlobalContext,
}

impl<'a> LivenessPass<'a> {
    pub fn new(ctx: &'a GlobalContext) -> Self {
        Self { ctx }
    }

    pub fn run(&self, func: &mut IrFunction) {
        for block in &mut func.blocks {
            self.process_block(block, func.var_types.as_slice());
        }
    }

    fn process_block(&self, block: &mut BasicBlock, var_types: &[Type]) {
        let mut last_use_map: HashMap<IrVar, usize> = HashMap::new();
        let mut returned_vars: HashSet<IrVar> = HashSet::new();

        if let Terminator::Return {
            value: Some(Operand::Var(ret_v)),
            ..
        } = &block.terminator
        {
            returned_vars.insert(*ret_v);
        }

        // 1. Scan instructions to find the latest index where each variable is read
        for (idx, inst) in block.instructions.iter().enumerate() {
            let used = Self::get_read_vars(inst);
            for v in used {
                last_use_map.insert(v, idx);
            }
        }

        // 2. Identify variables that allocate or hold drop-requiring types
        let mut drop_insertions: Vec<(usize, IrVar, usize, ByteSpan)> = Vec::new();

        for (idx, inst) in block.instructions.iter().enumerate() {
            if let Instruction::Assign { dest, span, .. } = inst {
                let v = *dest;
                let v_ty = var_types.get(v.0 as usize).unwrap_or(&Type::Unknown);
                let byte_size = compute_type_size(v_ty, self.ctx);

                // Only insert Drop for allocated types with non-zero size
                if byte_size > 0 && !returned_vars.contains(&v) {
                    if let Some(&last_idx) = last_use_map.get(&v) {
                        drop_insertions.push((last_idx + 1, v, byte_size, *span));
                    } else {
                        // Defined but never read: drop immediately after definition
                        drop_insertions.push((idx + 1, v, byte_size, *span));
                    }
                }
            }
        }

        // Sort insertions in descending order of index so inserting doesn't offset later indices
        drop_insertions.sort_by_key(|a| std::cmp::Reverse(a.0));

        // Deduplicate in case a variable was marked multiple times
        let mut seen_dropped = HashSet::new();
        for (insert_idx, var, byte_size, span) in drop_insertions {
            if seen_dropped.insert(var) {
                let safe_idx = insert_idx.min(block.instructions.len());
                block.instructions.insert(
                    safe_idx,
                    Instruction::Drop {
                        var,
                        byte_size,
                        span,
                    },
                );
            }
        }
    }

    fn get_read_vars(inst: &Instruction) -> Vec<IrVar> {
        let mut vars = Vec::new();
        match inst {
            Instruction::Assign { op, .. } => Self::collect_op_vars(op, &mut vars),
            Instruction::Drop { var, .. } => vars.push(*var),
            Instruction::ReuseToken { source, .. } => vars.push(*source),
            Instruction::SetField { target, value, .. } => {
                vars.push(*target);
                if let Operand::Var(v) = value {
                    vars.push(*v);
                }
            }
            Instruction::SetIndex {
                target,
                index,
                value,
                ..
            } => {
                vars.push(*target);
                if let Operand::Var(v) = index {
                    vars.push(*v);
                }
                if let Operand::Var(v) = value {
                    vars.push(*v);
                }
            }
            Instruction::Eval { op, .. } => Self::collect_op_vars(op, &mut vars),
        }
        vars
    }

    fn collect_op_vars(op: &IrOp, vars: &mut Vec<IrVar>) {
        match op {
            IrOp::Copy(Operand::Var(v)) => vars.push(*v),
            IrOp::Binary { left, right, .. } => {
                if let Operand::Var(v) = left {
                    vars.push(*v);
                }
                if let Operand::Var(v) = right {
                    vars.push(*v);
                }
            }
            IrOp::Alloc { fields, .. } | IrOp::AllocInPlace { fields, .. } => {
                for (_, f_op) in fields {
                    if let Operand::Var(v) = f_op {
                        vars.push(*v);
                    }
                }
            }
            IrOp::AllocTable {
                capacity: Operand::Var(v),
                ..
            } => {
                vars.push(*v);
            }
            IrOp::GetField { target, .. } => vars.push(*target),
            IrOp::GetIndex { target, index, .. } => {
                vars.push(*target);
                if let Operand::Var(v) = index {
                    vars.push(*v);
                }
            }
            IrOp::Call { args, .. }
            | IrOp::StaticMethodCall { args, .. }
            | IrOp::Spawn { args, .. } => {
                for a in args {
                    if let Operand::Var(v) = a {
                        vars.push(*v);
                    }
                }
            }
            IrOp::MethodCall { target, args, .. } => {
                vars.push(*target);
                for a in args {
                    if let Operand::Var(v) = a {
                        vars.push(*v);
                    }
                }
            }
            _ => {}
        }
    }
}
