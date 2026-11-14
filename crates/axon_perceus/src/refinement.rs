//! Refinement Types Bounds Check Elision Pass
//!
//! Uses static refinement constraints (`where idx < arr.len`) and loop bounds
//! to eliminate array/table bounds checks at compile time.

use crate::ir::*;
use lasso::Rodeo;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy)]
pub struct RefinementStats {
    pub elided_bounds_checks: usize,
    pub checked_bounds_checks: usize,
}

pub struct RefinementPass<'a> {
    interner: &'a Rodeo,
}

impl<'a> RefinementPass<'a> {
    pub fn new(interner: &'a Rodeo) -> Self {
        Self { interner }
    }

    pub fn run(&self, func: &mut IrFunction) -> RefinementStats {
        let mut stats = RefinementStats::default();

        // 1. Collect parameter-level refinement constraints
        let mut param_constraints: HashMap<IrVar, RefinementConstraint> = HashMap::new();
        for param in &func.params {
            if let Some(ref_c) = &param.refinement {
                param_constraints.insert(param.var, ref_c.clone());
            }
        }

        // 2. Scan instructions and check each index access
        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                match inst {
                    Instruction::Assign {
                        op:
                            IrOp::GetIndex {
                                target,
                                index,
                                bounds_check,
                            },
                        ..
                    } => {
                        if *bounds_check == BoundsCheckMode::Elided {
                            stats.elided_bounds_checks += 1;
                        } else if self.is_proven_in_bounds(
                            *target,
                            index,
                            &param_constraints,
                            &func.var_names,
                        ) {
                            *bounds_check = BoundsCheckMode::Elided;
                            stats.elided_bounds_checks += 1;
                        } else {
                            *bounds_check = BoundsCheckMode::Checked;
                            stats.checked_bounds_checks += 1;
                        }
                    }
                    Instruction::SetIndex {
                        target,
                        index,
                        bounds_check,
                        ..
                    } => {
                        if *bounds_check == BoundsCheckMode::Elided {
                            stats.elided_bounds_checks += 1;
                        } else if self.is_proven_in_bounds(
                            *target,
                            index,
                            &param_constraints,
                            &func.var_names,
                        ) {
                            *bounds_check = BoundsCheckMode::Elided;
                            stats.elided_bounds_checks += 1;
                        } else {
                            *bounds_check = BoundsCheckMode::Checked;
                            stats.checked_bounds_checks += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        stats
    }

    fn is_proven_in_bounds(
        &self,
        target_var: IrVar,
        index_op: &Operand,
        constraints: &HashMap<IrVar, RefinementConstraint>,
        var_names: &HashMap<IrVar, lasso::Spur>,
    ) -> bool {
        let idx_var = match index_op {
            Operand::Var(v) => *v,
            Operand::Const(_) => return false,
        };

        if let Some(constraint) = constraints.get(&idx_var) {
            match constraint {
                RefinementConstraint::LtField {
                    target: expected_target,
                    field,
                    ..
                } => {
                    let field_name = self.interner.resolve(field);
                    if field_name == "len" {
                        if let Some(target_name) = var_names.get(&target_var) {
                            return target_name == expected_target;
                        }
                    }
                }
                RefinementConstraint::LtConst { .. } => {
                    // Constant bounds can be verified if target capacity is known
                }
            }
        }

        false
    }
}
