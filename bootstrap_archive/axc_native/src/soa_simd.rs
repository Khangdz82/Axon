//! Axon v5.0 Native `Table[T]` Columnar SoA Layout & Auto-SIMD Vectorizer (`soa_simd.rs`)
//! Performs Dead-Column Elimination and lowers `for row in table` data-parallel loops
//! into hardware SIMD kernels (x86_64 AVX2/FMA3 8-wide 256-bit & ARM64 NEON 4-wide 128-bit).

use crate::ast::*;
use crate::sema::StructLayout;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum SimdKernelOp {
    /// `dst_col[i] += src_col[i] * scale_expr` (Fused Multiply-Add `VFMADD231PS` / `FMLA.4S`)
    Fma {
        dst_col: usize,
        src_col: usize,
        scale_expr: Expr,
    },
    /// `dst_col[i] += src_col[i]` (`VADDPS` / `FADD.4S`)
    AddCol { dst_col: usize, src_col: usize },
    /// `dst_col[i] *= scale_expr` (`VMULPS` / `FMUL.4S`)
    ScaleCol { dst_col: usize, scale_expr: Expr },
}

#[derive(Debug, Clone)]
pub struct SoaSimdPlan {
    pub ops: Vec<SimdKernelOp>,
    pub eliminated_dead_columns: usize,
}

pub fn analyze_foreach_simd(
    iter_var: &str,
    struct_name: &str,
    body: &[Stmt],
    structs: &HashMap<String, StructLayout>,
) -> Option<SoaSimdPlan> {
    let layout = structs.get(struct_name)?;
    let total_cols = layout.fields.len();
    if total_cols == 0 {
        return None;
    }

    let mut ops = Vec::new();
    let mut touched: HashSet<usize> = HashSet::new();

    for stmt in body {
        match stmt {
            Stmt::Assign {
                target: Expr::Field { base, field: dst_f, .. },
                op: AssignOp::AddEq,
                val,
                ..
            } if matches!(base.as_ref(), Expr::Ident(v, _) if v == iter_var) => {
                let dst_col = layout.fields.iter().position(|(n, _)| n == dst_f)?;
                touched.insert(dst_col);

                match val {
                    Expr::Binary {
                        op: BinOp::Mul,
                        left,
                        right,
                        ..
                    } => {
                        if let Expr::Field {
                            base: b2,
                            field: src_f,
                            ..
                        } = left.as_ref()
                        {
                            if matches!(b2.as_ref(), Expr::Ident(v, _) if v == iter_var) {
                                let src_col =
                                    layout.fields.iter().position(|(n, _)| n == src_f)?;
                                touched.insert(src_col);
                                ops.push(SimdKernelOp::Fma {
                                    dst_col,
                                    src_col,
                                    scale_expr: (*right.clone()),
                                });
                                continue;
                            }
                        }
                        return None;
                    }
                    Expr::Field {
                        base: b2,
                        field: src_f,
                        ..
                    } if matches!(b2.as_ref(), Expr::Ident(v, _) if v == iter_var) => {
                        let src_col = layout.fields.iter().position(|(n, _)| n == src_f)?;
                        touched.insert(src_col);
                        ops.push(SimdKernelOp::AddCol { dst_col, src_col });
                    }
                    _ => return None,
                }
            }
            Stmt::Assign {
                target: Expr::Field { base, field: dst_f, .. },
                op: AssignOp::MulEq,
                val,
                ..
            } if matches!(base.as_ref(), Expr::Ident(v, _) if v == iter_var) => {
                let dst_col = layout.fields.iter().position(|(n, _)| n == dst_f)?;
                touched.insert(dst_col);
                ops.push(SimdKernelOp::ScaleCol {
                    dst_col,
                    scale_expr: val.clone(),
                });
            }
            _ => return None,
        }
    }

    if ops.is_empty() {
        return None;
    }

    let touched_columns = touched.len();
    let eliminated_dead_columns = total_cols.saturating_sub(touched_columns);
    Some(SoaSimdPlan {
        ops,
        eliminated_dead_columns,
    })
}
