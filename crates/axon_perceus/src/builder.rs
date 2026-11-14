//! Perceus IR Builder
//!
//! Lowers typed AST into Perceus IR with basic blocks, explicit variables,
//! operands, and memory operations.

use crate::ir::*;
use crate::size::compute_type_size;
use axon_ast::*;
use axon_span::ByteSpan;
use axon_types::{GlobalContext, Type};
use lasso::{Rodeo, Spur};
use std::collections::{HashMap, HashSet};

pub struct IrBuilder<'a> {
    ctx: &'a GlobalContext,
    interner: &'a Rodeo,
    current_block: BlockId,
    blocks: Vec<BasicBlock>,
    var_types: Vec<Type>,
    var_names: HashMap<IrVar, Spur>,
    env: HashMap<Spur, IrVar>,
    next_var_id: u32,
    next_block_id: u32,
    pub outlined_functions: Vec<IrFunction>,
    fiber_count: u32,
}

impl<'a> IrBuilder<'a> {
    pub fn new(ctx: &'a GlobalContext, interner: &'a Rodeo) -> Self {
        let entry_block = BasicBlock {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Return {
                value: None,
                span: ByteSpan::DUMMY,
            },
        };

        Self {
            ctx,
            interner,
            current_block: BlockId(0),
            blocks: vec![entry_block],
            var_types: Vec::new(),
            var_names: HashMap::new(),
            env: HashMap::new(),
            next_var_id: 0,
            next_block_id: 1,
            outlined_functions: Vec::new(),
            fiber_count: 0,
        }
    }

    fn new_var(&mut self, ty: Type, name: Option<Spur>) -> IrVar {
        let id = self.next_var_id;
        self.next_var_id += 1;
        let var = IrVar(id);
        self.var_types.push(ty);
        if let Some(n) = name {
            self.var_names.insert(var, n);
            self.env.insert(n, var);
        }
        var
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.next_block_id;
        self.next_block_id += 1;
        let block_id = BlockId(id);
        self.blocks.push(BasicBlock {
            id: block_id,
            instructions: Vec::new(),
            terminator: Terminator::Return {
                value: None,
                span: ByteSpan::DUMMY,
            },
        });
        block_id
    }

    fn emit(&mut self, inst: Instruction) {
        let b = self
            .blocks
            .iter_mut()
            .find(|b| b.id == self.current_block)
            .expect("Current block not found");
        b.instructions.push(inst);
    }

    fn set_terminator(&mut self, term: Terminator) {
        let b = self
            .blocks
            .iter_mut()
            .find(|b| b.id == self.current_block)
            .expect("Current block not found");
        b.terminator = term;
    }

    pub fn build_function(&mut self, func: &FuncDef<'_>, ast: FrozenAst<'_, '_>) -> IrFunction {
        let mut params = Vec::new();

        for p in func.params {
            let p_ty = self
                .ctx
                .resolve_ast_type(p.ty, ast, self.interner)
                .unwrap_or(Type::Unknown);
            let var = self.new_var(p_ty.clone(), Some(p.name));

            // Extract refinement constraints if present:
            let refinement = self.extract_refinement(p.ty, p.name, ast);

            params.push(IrParam {
                name: p.name,
                intent: p.intent,
                ty: p_ty,
                var,
                refinement,
            });
        }

        let ret_type = match func.ret_type {
            Some(t_id) => self
                .ctx
                .resolve_ast_type(t_id, ast, self.interner)
                .unwrap_or(Type::VOID),
            None => Type::VOID,
        };

        for &stmt_id in func.body {
            self.lower_stmt(stmt_id, ast);
        }

        IrFunction {
            name: func.name,
            params,
            ret_type,
            blocks: std::mem::take(&mut self.blocks),
            var_types: std::mem::take(&mut self.var_types),
            var_names: std::mem::take(&mut self.var_names),
        }
    }

    fn extract_refinement(
        &self,
        ty_id: TypeId,
        param_name: Spur,
        ast: FrozenAst<'_, '_>,
    ) -> Option<RefinementConstraint> {
        let ty_data = ast.get_type(ty_id);
        if let TypeData::Refined { predicate, .. } = ty_data {
            let pred_expr = ast.get_expr(*predicate);
            if let ExprData::Binary {
                op: BinOp::Lt,
                left,
                right,
                ..
            } = pred_expr
            {
                let left_expr = ast.get_expr(*left);
                let right_expr = ast.get_expr(*right);

                if let ExprData::Ident(left_spur, _) = left_expr {
                    if *left_spur == param_name {
                        match right_expr {
                            ExprData::FieldAccess { target, field, .. } => {
                                if let ExprData::Ident(target_spur, _) = ast.get_expr(*target) {
                                    return Some(RefinementConstraint::LtField {
                                        var: param_name,
                                        target: *target_spur,
                                        field: *field,
                                    });
                                }
                            }
                            ExprData::Int(limit, _) => {
                                return Some(RefinementConstraint::LtConst {
                                    var: param_name,
                                    limit: *limit,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    fn lower_stmt(&mut self, stmt_id: StmtId, ast: FrozenAst<'_, '_>) {
        let stmt = ast.get_stmt(stmt_id).clone();
        match stmt {
            StmtData::Let {
                name,
                ty,
                init,
                span,
            }
            | StmtData::IsolatedLet {
                name,
                ty,
                init,
                span,
            } => {
                let operand = self.lower_expr(init, ast);
                let var_ty = match ty {
                    Some(t_id) => self
                        .ctx
                        .resolve_ast_type(t_id, ast, self.interner)
                        .unwrap_or(Type::Unknown),
                    None => {
                        if let Operand::Var(init_var) = operand {
                            self.var_types
                                .get(init_var.0 as usize)
                                .cloned()
                                .unwrap_or(Type::Unknown)
                        } else {
                            Type::Unknown
                        }
                    }
                };
                let var = self.new_var(var_ty, Some(name));
                self.emit(Instruction::Assign {
                    dest: var,
                    op: IrOp::Copy(operand),
                    span,
                });
            }

            StmtData::Assign {
                target,
                op,
                value,
                span,
            } => {
                let target_expr = ast.get_expr(target).clone();
                let val_operand = self.lower_expr(value, ast);

                match target_expr {
                    ExprData::Ident(var_spur, _) => {
                        if let Some(&var) = self.env.get(&var_spur) {
                            let final_operand = match op {
                                AssignOp::Assign => val_operand,
                                AssignOp::PlusAssign => {
                                    let tmp = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: tmp,
                                        op: IrOp::Binary {
                                            op: BinOp::Add,
                                            left: Operand::Var(var),
                                            right: val_operand,
                                        },
                                        span,
                                    });
                                    Operand::Var(tmp)
                                }
                                AssignOp::MinusAssign => {
                                    let tmp = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: tmp,
                                        op: IrOp::Binary {
                                            op: BinOp::Sub,
                                            left: Operand::Var(var),
                                            right: val_operand,
                                        },
                                        span,
                                    });
                                    Operand::Var(tmp)
                                }
                                AssignOp::StarAssign => {
                                    let tmp = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: tmp,
                                        op: IrOp::Binary {
                                            op: BinOp::Mul,
                                            left: Operand::Var(var),
                                            right: val_operand,
                                        },
                                        span,
                                    });
                                    Operand::Var(tmp)
                                }
                                AssignOp::SlashAssign => {
                                    let tmp = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: tmp,
                                        op: IrOp::Binary {
                                            op: BinOp::Div,
                                            left: Operand::Var(var),
                                            right: val_operand,
                                        },
                                        span,
                                    });
                                    Operand::Var(tmp)
                                }
                            };
                            self.emit(Instruction::Assign {
                                dest: var,
                                op: IrOp::Copy(final_operand),
                                span,
                            });
                        }
                    }
                    ExprData::FieldAccess {
                        target: obj_expr,
                        field,
                        span: f_span,
                    } => {
                        let obj_op = self.lower_expr(obj_expr, ast);
                        if let Operand::Var(target_var) = obj_op {
                            let final_val = match op {
                                AssignOp::Assign => val_operand,
                                _ => {
                                    let bin_op = match op {
                                        AssignOp::PlusAssign => BinOp::Add,
                                        AssignOp::MinusAssign => BinOp::Sub,
                                        AssignOp::StarAssign => BinOp::Mul,
                                        AssignOp::SlashAssign => BinOp::Div,
                                        _ => unreachable!(),
                                    };
                                    let cur_val_var = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: cur_val_var,
                                        op: IrOp::GetField {
                                            target: target_var,
                                            field,
                                        },
                                        span: f_span,
                                    });
                                    let tmp = self.new_var(Type::Unknown, None);
                                    self.emit(Instruction::Assign {
                                        dest: tmp,
                                        op: IrOp::Binary {
                                            op: bin_op,
                                            left: Operand::Var(cur_val_var),
                                            right: val_operand,
                                        },
                                        span,
                                    });
                                    Operand::Var(tmp)
                                }
                            };
                            self.emit(Instruction::SetField {
                                target: target_var,
                                field,
                                value: final_val,
                                span,
                            });
                        }
                    }
                    ExprData::IndexAccess {
                        target: arr_expr,
                        index: idx_expr,
                        ..
                    } => {
                        let arr_op = self.lower_expr(arr_expr, ast);
                        let idx_op = self.lower_expr(idx_expr, ast);
                        if let Operand::Var(arr_var) = arr_op {
                            self.emit(Instruction::SetIndex {
                                target: arr_var,
                                index: idx_op,
                                value: val_operand,
                                bounds_check: BoundsCheckMode::Checked,
                                span,
                            });
                        }
                    }
                    _ => {}
                }
            }

            StmtData::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_op = self.lower_expr(cond, ast);
                let then_bb = self.new_block();
                let else_bb = self.new_block();
                let merge_bb = self.new_block();

                self.set_terminator(Terminator::CondBranch {
                    cond: cond_op,
                    then_block: then_bb,
                    else_block: else_bb,
                    span,
                });

                // Then block
                self.current_block = then_bb;
                let then_stmts: Vec<StmtId> = then_branch.to_vec();
                for s in then_stmts {
                    self.lower_stmt(s, ast);
                }
                self.set_terminator(Terminator::Branch {
                    target: merge_bb,
                    span,
                });

                // Else block
                self.current_block = else_bb;
                if let Some(else_branch_slice) = else_branch {
                    let else_stmts: Vec<StmtId> = else_branch_slice.to_vec();
                    for s in else_stmts {
                        self.lower_stmt(s, ast);
                    }
                }
                self.set_terminator(Terminator::Branch {
                    target: merge_bb,
                    span,
                });

                self.current_block = merge_bb;
            }

            StmtData::While { cond, body, span } => {
                let header_bb = self.new_block();
                let body_bb = self.new_block();
                let exit_bb = self.new_block();

                self.set_terminator(Terminator::Branch {
                    target: header_bb,
                    span,
                });

                self.current_block = header_bb;
                let cond_op = self.lower_expr(cond, ast);
                self.set_terminator(Terminator::CondBranch {
                    cond: cond_op,
                    then_block: body_bb,
                    else_block: exit_bb,
                    span,
                });

                self.current_block = body_bb;
                let body_stmts: Vec<StmtId> = body.to_vec();
                for s in body_stmts {
                    self.lower_stmt(s, ast);
                }
                self.set_terminator(Terminator::Branch {
                    target: header_bb,
                    span,
                });

                self.current_block = exit_bb;
            }

            StmtData::For {
                loop_var,
                iter_target,
                body,
                span,
            } => {
                let target_op = self.lower_expr(iter_target, ast);
                let target_var = match target_op {
                    Operand::Var(v) => v,
                    Operand::Const(_) => return,
                };

                let header_bb = self.new_block();
                let body_bb = self.new_block();
                let exit_bb = self.new_block();

                let idx_var = self.new_var(Type::USIZE, None);
                self.emit(Instruction::Assign {
                    dest: idx_var,
                    op: IrOp::Copy(Operand::Const(IrConst::Int(0))),
                    span,
                });

                self.set_terminator(Terminator::Branch {
                    target: header_bb,
                    span,
                });

                self.current_block = header_bb;
                let len_var = self.new_var(Type::USIZE, None);
                let len_spur = self.interner.get("len").unwrap_or(loop_var);
                self.emit(Instruction::Assign {
                    dest: len_var,
                    op: IrOp::GetField {
                        target: target_var,
                        field: len_spur,
                    },
                    span,
                });
                let cond_var = self.new_var(Type::BOOL, None);
                self.emit(Instruction::Assign {
                    dest: cond_var,
                    op: IrOp::Binary {
                        op: BinOp::Lt,
                        left: Operand::Var(idx_var),
                        right: Operand::Var(len_var),
                    },
                    span,
                });
                self.set_terminator(Terminator::CondBranch {
                    cond: Operand::Var(cond_var),
                    then_block: body_bb,
                    else_block: exit_bb,
                    span,
                });

                self.current_block = body_bb;
                let target_ty = self
                    .var_types
                    .get(target_var.0 as usize)
                    .cloned()
                    .unwrap_or(Type::Unknown);
                let elem_ty = match target_ty {
                    Type::Table(s_name) => {
                        if self.ctx.structs.contains_key(&s_name) {
                            Type::Struct(s_name)
                        } else {
                            Type::Unknown
                        }
                    }
                    _ => Type::Unknown,
                };
                let item_var = self.new_var(elem_ty, Some(loop_var));
                // In for-in loops, bounds check is elided because 0 <= idx < len
                self.emit(Instruction::Assign {
                    dest: item_var,
                    op: IrOp::GetIndex {
                        target: target_var,
                        index: Operand::Var(idx_var),
                        bounds_check: BoundsCheckMode::Elided,
                    },
                    span,
                });

                let body_stmts: Vec<StmtId> = body.to_vec();
                for s in body_stmts {
                    self.lower_stmt(s, ast);
                }

                // idx += 1
                let next_idx = self.new_var(Type::USIZE, None);
                self.emit(Instruction::Assign {
                    dest: next_idx,
                    op: IrOp::Binary {
                        op: BinOp::Add,
                        left: Operand::Var(idx_var),
                        right: Operand::Const(IrConst::Int(1)),
                    },
                    span,
                });
                self.emit(Instruction::Assign {
                    dest: idx_var,
                    op: IrOp::Copy(Operand::Var(next_idx)),
                    span,
                });

                self.set_terminator(Terminator::Branch {
                    target: header_bb,
                    span,
                });

                self.current_block = exit_bb;
            }

            StmtData::Return { value, span, .. } => {
                let ret_val = value.map(|e| self.lower_expr(e, ast));
                self.set_terminator(Terminator::Return {
                    value: ret_val,
                    span,
                });
            }

            StmtData::Expr(expr_id) => {
                let _ = self.lower_expr(expr_id, ast);
            }
        }
    }

    pub fn lower_expr(&mut self, expr_id: ExprId, ast: FrozenAst<'_, '_>) -> Operand {
        let expr = ast.get_expr(expr_id).clone();

        match expr {
            ExprData::Int(val, _) => Operand::Const(IrConst::Int(val)),
            ExprData::Float(val, _) => Operand::Const(IrConst::Float(val)),
            ExprData::Bool(val, _) => Operand::Const(IrConst::Bool(val)),
            ExprData::Ident(spur, _) => {
                if let Some(&var) = self.env.get(&spur) {
                    Operand::Var(var)
                } else {
                    let var = self.new_var(Type::Unknown, Some(spur));
                    Operand::Var(var)
                }
            }
            ExprData::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left_op = self.lower_expr(left, ast);
                let right_op = self.lower_expr(right, ast);

                let left_ty = if let Operand::Var(v) = &left_op {
                    self.var_types
                        .get(v.0 as usize)
                        .cloned()
                        .unwrap_or(Type::Unknown)
                } else if let Operand::Const(IrConst::Float(_)) = &left_op {
                    Type::F32
                } else {
                    Type::Unknown
                };

                let right_ty = if let Operand::Var(v) = &right_op {
                    self.var_types
                        .get(v.0 as usize)
                        .cloned()
                        .unwrap_or(Type::Unknown)
                } else if let Operand::Const(IrConst::Float(_)) = &right_op {
                    Type::F32
                } else {
                    Type::Unknown
                };

                let bin_ty = match op {
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::And
                    | BinOp::Or => Type::BOOL,
                    _ => {
                        if left_ty == Type::F32 || right_ty == Type::F32 {
                            Type::F32
                        } else if left_ty == Type::F64 || right_ty == Type::F64 {
                            Type::F64
                        } else if left_ty == Type::I64 || right_ty == Type::I64 {
                            Type::I64
                        } else if left_ty == Type::USIZE || right_ty == Type::USIZE {
                            Type::USIZE
                        } else {
                            Type::I32
                        }
                    }
                };

                let dest = self.new_var(bin_ty, None);
                self.emit(Instruction::Assign {
                    dest,
                    op: IrOp::Binary {
                        op,
                        left: left_op,
                        right: right_op,
                    },
                    span,
                });
                Operand::Var(dest)
            }
            ExprData::FieldAccess {
                target,
                field,
                span,
            } => {
                let target_op = self.lower_expr(target, ast);
                if let Operand::Var(target_var) = target_op {
                    let target_ty = self
                        .var_types
                        .get(target_var.0 as usize)
                        .cloned()
                        .unwrap_or(Type::Unknown);
                    let field_ty = match &target_ty {
                        Type::Struct(s_name) => {
                            if let Some(s_info) = self.ctx.structs.get(s_name) {
                                s_info
                                    .fields
                                    .get(&field)
                                    .map(|(t, _)| t.clone())
                                    .unwrap_or(Type::Unknown)
                            } else {
                                Type::Unknown
                            }
                        }
                        Type::Table(_) => {
                            let f_str = self.interner.resolve(&field);
                            if f_str == "len" || f_str == "capacity" {
                                Type::USIZE
                            } else {
                                Type::Unknown
                            }
                        }
                        _ => Type::Unknown,
                    };

                    let dest = self.new_var(field_ty, None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::GetField {
                            target: target_var,
                            field,
                        },
                        span,
                    });
                    Operand::Var(dest)
                } else {
                    Operand::Const(IrConst::Int(0))
                }
            }
            ExprData::IndexAccess {
                target,
                index,
                span,
            } => {
                let target_op = self.lower_expr(target, ast);
                let idx_op = self.lower_expr(index, ast);
                if let Operand::Var(target_var) = target_op {
                    let target_ty = self
                        .var_types
                        .get(target_var.0 as usize)
                        .cloned()
                        .unwrap_or(Type::Unknown);
                    let elem_ty = match target_ty {
                        Type::Table(elem_name) => {
                            if self.ctx.structs.contains_key(&elem_name) {
                                Type::Struct(elem_name)
                            } else {
                                match self.interner.resolve(&elem_name) {
                                    "i64" => Type::I64,
                                    "i32" => Type::I32,
                                    "f32" => Type::F32,
                                    "f64" => Type::F64,
                                    "usize" => Type::USIZE,
                                    _ => Type::Unknown,
                                }
                            }
                        }
                        _ => Type::Unknown,
                    };

                    let dest = self.new_var(elem_ty, None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::GetIndex {
                            target: target_var,
                            index: idx_op,
                            bounds_check: BoundsCheckMode::Checked, // pass will evaluate refinement
                        },
                        span,
                    });
                    Operand::Var(dest)
                } else {
                    Operand::Const(IrConst::Int(0))
                }
            }
            ExprData::Call {
                callee,
                args,
                span,
            } => {
                let args_vec: Vec<(CallIntent, ExprId)> = args.to_vec();
                let mut lowered_args = Vec::new();
                for (_, arg_expr) in args_vec {
                    lowered_args.push(self.lower_expr(arg_expr, ast));
                }

                if self.ctx.structs.contains_key(&callee) {
                    let struct_ty = Type::Struct(callee);
                    let byte_size = compute_type_size(&struct_ty, self.ctx);
                    let dest = self.new_var(struct_ty, None);
                    let mut fields = Vec::new();
                    if let Some(s_info) = self.ctx.structs.get(&callee) {
                        for (i, f_name) in s_info.field_order.iter().enumerate() {
                            if i < lowered_args.len() {
                                fields.push((*f_name, lowered_args[i].clone()));
                            }
                        }
                    }
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::Alloc {
                            type_name: callee,
                            byte_size,
                            fields,
                        },
                        span,
                    });
                    Operand::Var(dest)
                } else {
                    let ret_ty = self
                        .ctx
                        .functions
                        .get(&callee)
                        .map(|sig| sig.ret_type.clone())
                        .unwrap_or(Type::Unknown);
                    let dest = self.new_var(ret_ty, None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::Call {
                            func: callee,
                            args: lowered_args,
                        },
                        span,
                    });
                    Operand::Var(dest)
                }
            }
            ExprData::StaticMethodCall {
                target,
                method,
                args,
                span,
            } => {
                let args_vec: Vec<ExprId> = args.to_vec();
                let mut lowered_args = Vec::new();
                for arg_expr in args_vec {
                    lowered_args.push(self.lower_expr(arg_expr, ast));
                }

                let method_name = self.interner.resolve(&method);
                if method_name == "with_capacity" {
                    let elem_ty = Type::Struct(target);
                    let cap_op = lowered_args
                        .first()
                        .cloned()
                        .unwrap_or(Operand::Const(IrConst::Int(0)));
                    let dest = self.new_var(Type::Table(target), None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::AllocTable {
                            elem_type: elem_ty,
                            capacity: cap_op,
                        },
                        span,
                    });
                    Operand::Var(dest)
                } else {
                    let dest = self.new_var(Type::Unknown, None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::StaticMethodCall {
                            target,
                            method,
                            args: lowered_args,
                        },
                        span,
                    });
                    Operand::Var(dest)
                }
            }
            ExprData::MethodCall {
                target,
                method,
                args,
                span,
            } => {
                let target_op = self.lower_expr(target, ast);
                let args_vec: Vec<ExprId> = args.to_vec();
                let mut lowered_args = Vec::new();
                for arg_expr in args_vec {
                    lowered_args.push(self.lower_expr(arg_expr, ast));
                }
                if let Operand::Var(target_var) = target_op {
                    let dest_ty = if self.interner.resolve(&method) == "join" {
                        if let Some(Type::Job(inner_ty)) = self.var_types.get(target_var.0 as usize) {
                            (**inner_ty).clone()
                        } else {
                            Type::Unknown
                        }
                    } else {
                        Type::Unknown
                    };
                    let dest = self.new_var(dest_ty, None);
                    self.emit(Instruction::Assign {
                        dest,
                        op: IrOp::MethodCall {
                            target: target_var,
                            method,
                            args: lowered_args,
                        },
                        span,
                    });
                    Operand::Var(dest)
                } else {
                    Operand::Const(IrConst::Int(0))
                }
            }
            ExprData::Spawn { body, span } => {
                let mut used_idents = Vec::new();
                for &s in body {
                    collect_stmt_idents(s, &mut used_idents, ast);
                }
                let mut local_defs = HashSet::new();
                collect_locally_defined_idents(body, ast, &mut local_defs);

                let mut captured_vars = Vec::new();
                for spur in used_idents {
                    if !local_defs.contains(&spur) {
                        if let Some(&outer_var) = self.env.get(&spur) {
                            let var_ty = self.var_types[outer_var.0 as usize].clone();
                            if !captured_vars.iter().any(|(s, _, _)| *s == spur) {
                                captured_vars.push((spur, outer_var, var_ty));
                            }
                        }
                    }
                }

                let fiber_id = self.fiber_count;
                self.fiber_count += 1;
                let fiber_name = self
                    .interner
                    .get(format!("__axon_fiber_{}", fiber_id))
                    .unwrap_or_else(|| self.interner.get("spawn_fiber").unwrap_or_default());

                let mut fiber_builder = IrBuilder::new(self.ctx, self.interner);
                let mut fiber_params = Vec::new();
                for (name, _outer_var, ty) in &captured_vars {
                    let param_var = fiber_builder.new_var(ty.clone(), Some(*name));
                    fiber_params.push(IrParam {
                        name: *name,
                        var: param_var,
                        ty: ty.clone(),
                        intent: ParamIntent::Isolated,
                        refinement: None,
                    });
                }

                for &s in body {
                    fiber_builder.lower_stmt(s, ast);
                }

                let last_block_id = fiber_builder.current_block;
                let last_b = fiber_builder.blocks.iter_mut().find(|b| b.id == last_block_id).unwrap();
                if !matches!(last_b.terminator, Terminator::Return { .. }) {
                    last_b.terminator = Terminator::Return { value: None, span };
                }

                // Determine return type of body directly from lowered return terminator
                let mut ret_ty = Type::VOID;
                for b in &fiber_builder.blocks {
                    if let Terminator::Return { value: Some(op), .. } = &b.terminator {
                        match op {
                            Operand::Var(v) => {
                                ret_ty = fiber_builder.var_types[v.0 as usize].clone();
                            }
                            Operand::Const(IrConst::Int(_)) => ret_ty = Type::I32,
                            Operand::Const(IrConst::Float(_)) => ret_ty = Type::F32,
                            Operand::Const(IrConst::Bool(_)) => ret_ty = Type::BOOL,
                        }
                    }
                }

                let mut fiber_func = IrFunction {
                    name: fiber_name,
                    params: fiber_params,
                    ret_type: ret_ty.clone(),
                    blocks: fiber_builder.blocks,
                    var_types: fiber_builder.var_types,
                    var_names: fiber_builder.var_names,
                };

                crate::liveness::LivenessPass::new(self.ctx).run(&mut fiber_func);
                crate::reuse::MemoryReusePass::new().run(&mut fiber_func);
                crate::refinement::RefinementPass::new(self.interner).run(&mut fiber_func);

                self.outlined_functions.push(fiber_func);

                let mut spawn_args = Vec::new();
                for (_, outer_var, _) in captured_vars {
                    spawn_args.push(Operand::Var(outer_var));
                }

                let job_dest = self.new_var(Type::Job(Box::new(ret_ty)), None);
                self.emit(Instruction::Assign {
                    dest: job_dest,
                    op: IrOp::Spawn {
                        func_name: fiber_name,
                        args: spawn_args,
                    },
                    span,
                });
                Operand::Var(job_dest)
            }
        }
    }
}

fn collect_expr_idents(eid: ExprId, idents: &mut Vec<Spur>, ast: FrozenAst<'_, '_>) {
    match ast.get_expr(eid) {
        ExprData::Ident(s, _) => {
            if !idents.contains(s) {
                idents.push(*s);
            }
        }
        ExprData::Binary { left, right, .. } => {
            collect_expr_idents(*left, idents, ast);
            collect_expr_idents(*right, idents, ast);
        }
        ExprData::Call { args, .. } => {
            for (_, arg_id) in *args {
                collect_expr_idents(*arg_id, idents, ast);
            }
        }
        ExprData::MethodCall { target, args, .. } => {
            collect_expr_idents(*target, idents, ast);
            for arg_id in *args {
                collect_expr_idents(*arg_id, idents, ast);
            }
        }
        ExprData::StaticMethodCall { args, .. } => {
            for arg_id in *args {
                collect_expr_idents(*arg_id, idents, ast);
            }
        }
        ExprData::FieldAccess { target, .. } => {
            collect_expr_idents(*target, idents, ast);
        }
        ExprData::IndexAccess { target, index, .. } => {
            collect_expr_idents(*target, idents, ast);
            collect_expr_idents(*index, idents, ast);
        }
        ExprData::Spawn { body, .. } => {
            for &s in *body {
                collect_stmt_idents(s, idents, ast);
            }
        }
        _ => {}
    }
}

fn collect_stmt_idents(sid: StmtId, idents: &mut Vec<Spur>, ast: FrozenAst<'_, '_>) {
    match ast.get_stmt(sid) {
        StmtData::Let { init, .. } | StmtData::IsolatedLet { init, .. } => {
            collect_expr_idents(*init, idents, ast);
        }
        StmtData::Assign { target, value, .. } => {
            collect_expr_idents(*target, idents, ast);
            collect_expr_idents(*value, idents, ast);
        }
        StmtData::For { iter_target, body, .. } => {
            collect_expr_idents(*iter_target, idents, ast);
            for &s in *body {
                collect_stmt_idents(s, idents, ast);
            }
        }
        StmtData::While { cond, body, .. } => {
            collect_expr_idents(*cond, idents, ast);
            for &s in *body {
                collect_stmt_idents(s, idents, ast);
            }
        }
        StmtData::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_idents(*cond, idents, ast);
            for &s in *then_branch {
                collect_stmt_idents(s, idents, ast);
            }
            if let Some(eb) = else_branch {
                for &s in *eb {
                    collect_stmt_idents(s, idents, ast);
                }
            }
        }
        StmtData::Expr(eid) => {
            collect_expr_idents(*eid, idents, ast);
        }
        StmtData::Return { value, .. } => {
            if let Some(vid) = value {
                collect_expr_idents(*vid, idents, ast);
            }
        }
    }
}

fn collect_locally_defined_idents(body: &[StmtId], ast: FrozenAst<'_, '_>, locals: &mut HashSet<Spur>) {
    for &sid in body {
        match ast.get_stmt(sid) {
            StmtData::Let { name, .. } | StmtData::IsolatedLet { name, .. } => {
                locals.insert(*name);
            }
            StmtData::For { loop_var, body: inner, .. } => {
                locals.insert(*loop_var);
                collect_locally_defined_idents(inner, ast, locals);
            }
            StmtData::While { body: inner, .. } => {
                collect_locally_defined_idents(inner, ast, locals);
            }
            StmtData::If { then_branch, else_branch, .. } => {
                collect_locally_defined_idents(then_branch, ast, locals);
                if let Some(eb) = else_branch {
                    collect_locally_defined_idents(eb, ast, locals);
                }
            }
            _ => {}
        }
    }
}
