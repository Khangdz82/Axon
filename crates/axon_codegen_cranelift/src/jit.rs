//! Cranelift JIT Engine for Axon Dev Mode
//!
//! Provides sub-10ms compilation and in-memory execution of Axon Perceus IR.

use crate::types::map_type;
use axon_ast::BinOp;
use axon_perceus::ir::{
    BlockId, BoundsCheckMode, Instruction, IrConst, IrFunction, IrOp, IrProgram,
    IrVar, Operand, Terminator,
};
use axon_types::Type;
use cranelift::codegen::ir::{FuncRef, TrapCode, UserFuncName};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use lasso::{Rodeo, Spur};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};

extern "C" fn axon_jit_alloc(size: usize, align: usize) -> *mut u8 {
    unsafe {
        let align = align.max(8);
        let size = size.max(1);
        let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
        std::alloc::alloc_zeroed(layout)
    }
}

extern "C" fn axon_jit_free(ptr: *mut u8, size: usize, align: usize) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            let align = align.max(8);
            let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
            std::alloc::dealloc(ptr, layout);
        }
    }
}

static PANIC_TRIGGERED: AtomicI32 = AtomicI32::new(0);

extern "C" fn axon_jit_assert(cond: i32, code: i32) {
    if cond == 0 {
        PANIC_TRIGGERED.store(code, Ordering::SeqCst);
        eprintln!("[Axon JIT Panic] Runtime check failed (code: {})", code);
    }
}

pub struct CraneliftJit {
    module: JITModule,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
}

impl CraneliftJit {
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| e.to_string())?;
        flag_builder
            .set("is_pic", "false")
            .map_err(|e| e.to_string())?;
        flag_builder
            .set("opt_level", "none")
            .map_err(|e| e.to_string())?;

        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).map_err(|e| e.to_string())?;

        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        jit_builder.symbol("axon_jit_alloc", axon_jit_alloc as *const u8);
        jit_builder.symbol("axon_jit_free", axon_jit_free as *const u8);
        jit_builder.symbol("axon_jit_assert", axon_jit_assert as *const u8);

        let module = JITModule::new(jit_builder);
        let builder_context = FunctionBuilderContext::new();
        let ctx = module.make_context();

        Ok(Self {
            module,
            builder_context,
            ctx,
        })
    }

    pub fn compile_and_run(
        mut self,
        ir_prog: &IrProgram,
        interner: &Rodeo,
    ) -> Result<i64, String> {
        let ptr_type = self.module.target_config().pointer_type();

        // 1. Declare all functions in JITModule
        let mut func_ids: HashMap<Spur, FuncId> = HashMap::new();
        for func in &ir_prog.functions {
            let fn_name = interner.resolve(&func.name);
            let mut sig = self.module.make_signature();
            for p in &func.params {
                if let Some(cl_ty) = map_type(&p.ty, ptr_type) {
                    sig.params.push(AbiParam::new(cl_ty));
                }
            }
            if let Some(cl_ty) = map_type(&func.ret_type, ptr_type) {
                sig.returns.push(AbiParam::new(cl_ty));
            }

            let func_id = self
                .module
                .declare_function(fn_name, Linkage::Export, &sig)
                .map_err(|e| format!("Declare function error: {}", e))?;
            func_ids.insert(func.name, func_id);
        }

        // 2. Compile each function
        for func in &ir_prog.functions {
            let func_id = *func_ids.get(&func.name).unwrap();
            self.compile_function(func, func_id, &func_ids, ir_prog, interner, ptr_type)?;
        }

        // 3. Finalize all definitions in memory
        self.module
            .finalize_definitions()
            .map_err(|e| format!("JIT finalization error: {}", e))?;

        // 4. Locate main function
        let main_spur = interner
            .get("main")
            .ok_or_else(|| "No 'main' function found in program".to_string())?;
        let main_func_id = func_ids
            .get(&main_spur)
            .ok_or_else(|| "No 'main' function compiled".to_string())?;

        let main_fn_ptr = self.module.get_finalized_function(*main_func_id);

        let main_func_info = ir_prog.functions.iter().find(|f| f.name == main_spur);
        let ret_ty = main_func_info.map(|f| &f.ret_type).unwrap_or(&Type::VOID);

        // 5. Execute in-memory
        PANIC_TRIGGERED.store(0, Ordering::SeqCst);

        let exit_code = if *ret_ty == Type::VOID {
            let main_fn: extern "C" fn() = unsafe { std::mem::transmute(main_fn_ptr) };
            main_fn();
            0
        } else if *ret_ty == Type::I32 {
            let main_fn: extern "C" fn() -> i32 = unsafe { std::mem::transmute(main_fn_ptr) };
            main_fn() as i64
        } else {
            let main_fn: extern "C" fn() -> i64 = unsafe { std::mem::transmute(main_fn_ptr) };
            main_fn()
        };

        let panic_code = PANIC_TRIGGERED.load(Ordering::SeqCst);
        if panic_code != 0 {
            return Err(format!("JIT runtime panic occurred (code: {})", panic_code));
        }

        Ok(exit_code)
    }

    fn compile_function(
        &mut self,
        func: &IrFunction,
        func_id: FuncId,
        func_ids: &HashMap<Spur, FuncId>,
        prog: &IrProgram,
        interner: &Rodeo,
        ptr_type: types::Type,
    ) -> Result<(), String> {
        let mut sig = self.module.make_signature();
        for p in &func.params {
            if let Some(cl_ty) = map_type(&p.ty, ptr_type) {
                sig.params.push(AbiParam::new(cl_ty));
            }
        }
        if let Some(cl_ty) = map_type(&func.ret_type, ptr_type) {
            sig.returns.push(AbiParam::new(cl_ty));
        }

        self.ctx.func.signature = sig;
        self.ctx.func.name = UserFuncName::user(0, func_id.as_u32());

        // Index tracking for SoA column write-backs (synthetic view proxy)
        let mut index_source_map: HashMap<IrVar, (IrVar, Operand)> = HashMap::new();
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Instruction::Assign {
                    dest,
                    op: IrOp::GetIndex { target, index, .. },
                    ..
                } = inst
                {
                    index_source_map.insert(*dest, (*target, index.clone()));
                }
            }
        }

        let mut compiler = FuncCompiler {
            module: &mut self.module,
            func_ids,
            prog,
            interner,
            ptr_type,
            index_source_map: &index_source_map,
        };

        // Compute reachable blocks starting from BlockId(0)
        let mut reachable: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
        let mut queue = vec![BlockId(0)];
        reachable.insert(BlockId(0));
        while let Some(curr) = queue.pop() {
            if let Some(b) = func.blocks.iter().find(|b| b.id == curr) {
                match &b.terminator {
                    Terminator::Branch { target, .. } => {
                        if reachable.insert(*target) {
                            queue.push(*target);
                        }
                    }
                    Terminator::CondBranch {
                        then_block,
                        else_block,
                        ..
                    } => {
                        if reachable.insert(*then_block) {
                            queue.push(*then_block);
                        }
                        if reachable.insert(*else_block) {
                            queue.push(*else_block);
                        }
                    }
                    Terminator::Return { .. } => {}
                }
            }
        }

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        // Create Cranelift blocks ONLY for reachable IR blocks
        let mut cl_blocks: HashMap<BlockId, Block> = HashMap::new();
        for block in &func.blocks {
            if reachable.contains(&block.id) {
                cl_blocks.insert(block.id, builder.create_block());
            }
        }

        // Declare variables
        for (i, ty) in func.var_types.iter().enumerate() {
            if let Some(cl_ty) = map_type(ty, ptr_type) {
                let var = Variable::new(i);
                builder.declare_var(var, cl_ty);
            }
        }

        // Entry block parameters
        if let Some(entry_block) = cl_blocks.get(&BlockId(0)) {
            builder.append_block_params_for_function_params(*entry_block);
            builder.switch_to_block(*entry_block);

            // Bind function parameters to their IrVars
            let mut param_idx = 0;
            for p in &func.params {
                if map_type(&p.ty, ptr_type).is_some() {
                    let val = builder.block_params(*entry_block)[param_idx];
                    builder.def_var(Variable::new(p.var.0 as usize), val);
                    param_idx += 1;
                }
            }
        }

        // Compile each reachable block
        for block in &func.blocks {
            if !reachable.contains(&block.id) {
                continue;
            }
            let cl_block = cl_blocks[&block.id];
            if block.id != BlockId(0) {
                builder.switch_to_block(cl_block);
            }

            for inst in &block.instructions {
                compiler.compile_instruction(&mut builder, inst, func)?;
            }

            // Compile terminator
            match &block.terminator {
                Terminator::Return { value, .. } => {
                    if let Some(op) = value {
                        let ret_cl_ty = map_type(&func.ret_type, compiler.ptr_type);
                        if let Some(val) = compiler.eval_operand(&mut builder, op, func, ret_cl_ty) {
                            builder.ins().return_(&[val]);
                        } else {
                            builder.ins().return_(&[]);
                        }
                    } else if map_type(&func.ret_type, compiler.ptr_type).is_some() {
                        builder.ins().trap(TrapCode::unwrap_user(1));
                    } else {
                        builder.ins().return_(&[]);
                    }
                }
                Terminator::Branch { target, .. } => {
                    builder.ins().jump(cl_blocks[target], &[]);
                }
                Terminator::CondBranch {
                    cond,
                    then_block,
                    else_block,
                    ..
                } => {
                    let cond_val = compiler
                        .eval_operand(&mut builder, cond, func, Some(types::I8))
                        .ok_or_else(|| "Condition operand evaluated to void".to_string())?;
                    builder.ins().brif(
                        cond_val,
                        cl_blocks[then_block],
                        &[],
                        cl_blocks[else_block],
                        &[],
                    );
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Define function error: {:#?}\nFunc IR:\n{}", e, self.ctx.func.display()))?;
        self.module.clear_context(&mut self.ctx);

        Ok(())
    }
}

struct FuncCompiler<'a> {
    module: &'a mut JITModule,
    func_ids: &'a HashMap<Spur, FuncId>,
    prog: &'a IrProgram,
    interner: &'a Rodeo,
    ptr_type: types::Type,
    index_source_map: &'a HashMap<IrVar, (IrVar, Operand)>,
}

impl<'a> FuncCompiler<'a> {
    fn compile_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        inst: &Instruction,
        func: &IrFunction,
    ) -> Result<(), String> {
        match inst {
            Instruction::Assign { dest, op, .. } => {
                let dest_cl_ty = func
                    .var_types
                    .get(dest.0 as usize)
                    .and_then(|t| map_type(t, self.ptr_type));
                if let Some(mut val) = self.compile_op(builder, op, func, dest_cl_ty)? {
                    if let Some(target_cl_ty) = dest_cl_ty {
                        let actual_ty = builder.func.dfg.value_type(val);
                        if actual_ty != target_cl_ty {
                            if actual_ty.is_int() && target_cl_ty.is_int() {
                                if actual_ty.bits() < target_cl_ty.bits() {
                                    val = builder.ins().sextend(target_cl_ty, val);
                                } else if actual_ty.bits() > target_cl_ty.bits() {
                                    val = builder.ins().ireduce(target_cl_ty, val);
                                }
                            } else if actual_ty.is_float() && target_cl_ty.is_float() {
                                if actual_ty.bits() < target_cl_ty.bits() {
                                    val = builder.ins().fpromote(target_cl_ty, val);
                                } else if actual_ty.bits() > target_cl_ty.bits() {
                                    val = builder.ins().fdemote(target_cl_ty, val);
                                }
                            } else if actual_ty.is_int() && target_cl_ty.is_float() {
                                val = builder.ins().fcvt_from_sint(target_cl_ty, val);
                            } else if actual_ty.is_float() && target_cl_ty.is_int() {
                                val = builder.ins().fcvt_to_sint(target_cl_ty, val);
                            }
                        }
                    }
                    builder.def_var(Variable::new(dest.0 as usize), val);
                }
            }
            Instruction::Drop { .. } => {
                // In Dev mode JIT, Drop is handled by FBIP reuse tokens.
            }
            Instruction::ReuseToken { dest, source, .. } => {
                let src_val = builder.use_var(Variable::new(source.0 as usize));
                builder.def_var(Variable::new(dest.0 as usize), src_val);
            }
            Instruction::SetField {
                target,
                field,
                value,
                ..
            } => {
                let target_ty = func.var_types.get(target.0 as usize).unwrap_or(&Type::Unknown);
                if let Type::Struct(s_name) = target_ty {
                    if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                        let (f_ty, _) = &s_info.fields[field];
                        let f_cl_ty = map_type(f_ty, self.ptr_type);
                        let val = self
                            .eval_operand(builder, value, func, f_cl_ty)
                            .ok_or_else(|| "Cannot set void field value".to_string())?;

                        let offset = Self::struct_field_offset(s_info, *field, &self.prog.ctx)?;
                        let target_ptr = builder.use_var(Variable::new(target.0 as usize));
                        builder.ins().store(MemFlags::trusted(), val, target_ptr, offset as i32);
                    }
                }

                // If target came from an SoA table column view, write back to table column array!
                if let Some((tbl_var, idx_op)) = self.index_source_map.get(target) {
                    let tbl_ty = func.var_types.get(tbl_var.0 as usize).unwrap_or(&Type::Unknown);
                    if let Type::Table(s_name) = tbl_ty {
                        if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                            let col_idx = s_info.field_order.iter().position(|f| f == field);
                            if let Some(col_i) = col_idx {
                                let (elem_field_ty, _) = &s_info.fields[field];
                                let f_cl_ty = map_type(elem_field_ty, self.ptr_type);
                                let val = self
                                    .eval_operand(builder, value, func, f_cl_ty)
                                    .ok_or_else(|| "Cannot set void field value".to_string())?;

                                let tbl_ptr = builder.use_var(Variable::new(tbl_var.0 as usize));
                                let col_ptr_offset = 16 + (col_i as i32 * 8);
                                let col_array_ptr = builder.ins().load(
                                    self.ptr_type,
                                    MemFlags::trusted(),
                                    tbl_ptr,
                                    col_ptr_offset,
                                );

                                let idx_val = self
                                    .eval_operand(builder, idx_op, func, Some(self.ptr_type))
                                    .ok_or_else(|| "Index is void".to_string())?;

                                let elem_size = axon_perceus::size::compute_scalar_size(elem_field_ty, &self.prog.ctx);
                                let elem_size_val = builder.ins().iconst(self.ptr_type, elem_size as i64);
                                let byte_offset = builder.ins().imul(idx_val, elem_size_val);
                                let elem_addr = builder.ins().iadd(col_array_ptr, byte_offset);

                                builder.ins().store(MemFlags::trusted(), val, elem_addr, 0);
                            }
                        }
                    }
                }
            }
            Instruction::SetIndex {
                target,
                index,
                value,
                bounds_check,
                ..
            } => {
                let target_ptr = builder.use_var(Variable::new(target.0 as usize));
                let idx_val = self
                    .eval_operand(builder, index, func, Some(self.ptr_type))
                    .ok_or_else(|| "Index is void".to_string())?;
                let _val = self
                    .eval_operand(builder, value, func, None)
                    .ok_or_else(|| "Value is void".to_string())?;

                if *bounds_check == BoundsCheckMode::Checked {
                    let len_val = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, 0);
                    let ok = builder.ins().icmp(IntCC::UnsignedLessThan, idx_val, len_val);
                    let ok_i32 = builder.ins().uextend(types::I32, ok);
                    let code_val = builder.ins().iconst(types::I32, 101);
                    let assert_callee = self.get_or_declare_host_func("axon_jit_assert", &[types::I32, types::I32], None, builder)?;
                    builder.ins().call(assert_callee, &[ok_i32, code_val]);
                }
            }
            Instruction::Eval { op, .. } => {
                self.compile_op(builder, op, func, None)?;
            }
        }
        Ok(())
    }

    fn compile_op(
        &mut self,
        builder: &mut FunctionBuilder,
        op: &IrOp,
        func: &IrFunction,
        dest_cl_ty: Option<types::Type>,
    ) -> Result<Option<Value>, String> {
        match op {
            IrOp::Copy(operand) => Ok(self.eval_operand(builder, operand, func, dest_cl_ty)),
            IrOp::Binary { op, left, right } => {
                let is_cmp = matches!(
                    op,
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq
                );

                let operand_ty = if is_cmp {
                    let left_ty = match left {
                        Operand::Var(v) => func.var_types.get(v.0 as usize).and_then(|t| map_type(t, self.ptr_type)),
                        Operand::Const(IrConst::Float(_)) => Some(types::F32),
                        Operand::Const(IrConst::Bool(_)) => Some(types::I8),
                        Operand::Const(IrConst::Int(_)) => match right {
                            Operand::Var(rv) => func.var_types.get(rv.0 as usize).and_then(|t| map_type(t, self.ptr_type)),
                            _ => Some(types::I32),
                        },
                    };
                    left_ty.unwrap_or(types::I32)
                } else {
                    dest_cl_ty.unwrap_or(types::I32)
                };

                let lhs = self
                    .eval_operand(builder, left, func, Some(operand_ty))
                    .ok_or_else(|| "LHS is void".to_string())?;
                let rhs = self
                    .eval_operand(builder, right, func, Some(operand_ty))
                    .ok_or_else(|| "RHS is void".to_string())?;

                let is_float = operand_ty.is_float();

                let res = if is_float {
                    match op {
                        BinOp::Add | BinOp::AddWrap | BinOp::AddSat => builder.ins().fadd(lhs, rhs),
                        BinOp::Sub | BinOp::SubWrap | BinOp::SubSat => builder.ins().fsub(lhs, rhs),
                        BinOp::Mul | BinOp::MulWrap | BinOp::MulSat => builder.ins().fmul(lhs, rhs),
                        BinOp::Div => builder.ins().fdiv(lhs, rhs),
                        BinOp::Eq => builder.ins().fcmp(FloatCC::Equal, lhs, rhs),
                        BinOp::NotEq => builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs),
                        BinOp::Lt => builder.ins().fcmp(FloatCC::LessThan, lhs, rhs),
                        BinOp::LtEq => builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs),
                        BinOp::Gt => builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs),
                        BinOp::GtEq => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs),
                        _ => return Err(format!("Unsupported float operator {:?}", op)),
                    }
                } else {
                    match op {
                        BinOp::Add | BinOp::AddWrap | BinOp::AddSat => builder.ins().iadd(lhs, rhs),
                        BinOp::Sub | BinOp::SubWrap | BinOp::SubSat => builder.ins().isub(lhs, rhs),
                        BinOp::Mul | BinOp::MulWrap => builder.ins().imul(lhs, rhs),
                        BinOp::MulSat => builder.ins().imul(lhs, rhs),
                        BinOp::Div => builder.ins().sdiv(lhs, rhs),
                        BinOp::Mod => builder.ins().srem(lhs, rhs),
                        BinOp::Eq => builder.ins().icmp(IntCC::Equal, lhs, rhs),
                        BinOp::NotEq => builder.ins().icmp(IntCC::NotEqual, lhs, rhs),
                        BinOp::Lt => builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs),
                        BinOp::LtEq => builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs),
                        BinOp::Gt => builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs),
                        BinOp::GtEq => builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs),
                        BinOp::And => builder.ins().band(lhs, rhs),
                        BinOp::Or => builder.ins().bor(lhs, rhs),
                    }
                };
                Ok(Some(res))
            }
            IrOp::Alloc {
                type_name,
                byte_size,
                fields,
            } => {
                let size_val = builder.ins().iconst(self.ptr_type, *byte_size as i64);
                let align_val = builder.ins().iconst(self.ptr_type, 64);
                let alloc_callee = self.get_or_declare_host_func(
                    "axon_jit_alloc",
                    &[self.ptr_type, self.ptr_type],
                    Some(self.ptr_type),
                    builder,
                )?;
                let call_inst = builder.ins().call(alloc_callee, &[size_val, align_val]);
                let ptr = builder.inst_results(call_inst)[0];

                if let Some(s_info) = self.prog.ctx.structs.get(type_name) {
                    for (f_name, f_op) in fields {
                        let (f_ty, _) = &s_info.fields[f_name];
                        let f_cl_ty = map_type(f_ty, self.ptr_type);
                        if let Some(f_val) = self.eval_operand(builder, f_op, func, f_cl_ty) {
                            let offset = Self::struct_field_offset(s_info, *f_name, &self.prog.ctx)?;
                            builder
                                .ins()
                                .store(MemFlags::trusted(), f_val, ptr, offset as i32);
                        }
                    }
                }
                Ok(Some(ptr))
            }
            IrOp::AllocInPlace {
                token,
                type_name,
                fields,
                ..
            } => {
                let ptr = builder.use_var(Variable::new(token.0 as usize));
                if let Some(s_info) = self.prog.ctx.structs.get(type_name) {
                    for (f_name, f_op) in fields {
                        let (f_ty, _) = &s_info.fields[f_name];
                        let f_cl_ty = map_type(f_ty, self.ptr_type);
                        if let Some(f_val) = self.eval_operand(builder, f_op, func, f_cl_ty) {
                            let offset = Self::struct_field_offset(s_info, *f_name, &self.prog.ctx)?;
                            builder
                                .ins()
                                .store(MemFlags::trusted(), f_val, ptr, offset as i32);
                        }
                    }
                }
                Ok(Some(ptr))
            }
            IrOp::AllocTable {
                elem_type,
                capacity,
            } => {
                let cap_val = self
                    .eval_operand(builder, capacity, func, Some(self.ptr_type))
                    .ok_or_else(|| "Capacity is void".to_string())?;

                let col_count = match elem_type {
                    Type::Struct(s_name) => self
                        .prog
                        .ctx
                        .structs
                        .get(s_name)
                        .map(|s| s.field_order.len())
                        .unwrap_or(1),
                    _ => 1,
                };

                let header_size = 16 + 8 * col_count;
                let header_size_val = builder.ins().iconst(self.ptr_type, header_size as i64);
                let align_val = builder.ins().iconst(self.ptr_type, 64);
                let alloc_callee = self.get_or_declare_host_func(
                    "axon_jit_alloc",
                    &[self.ptr_type, self.ptr_type],
                    Some(self.ptr_type),
                    builder,
                )?;
                let call_inst = builder.ins().call(alloc_callee, &[header_size_val, align_val]);
                let tbl_ptr = builder.inst_results(call_inst)[0];

                let zero_val = builder.ins().iconst(self.ptr_type, 0);
                builder.ins().store(MemFlags::trusted(), zero_val, tbl_ptr, 0);
                builder.ins().store(MemFlags::trusted(), cap_val, tbl_ptr, 8);

                if let Type::Struct(s_name) = elem_type {
                    if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                        for (i, f_name) in s_info.field_order.iter().enumerate() {
                            let (f_ty, _) = &s_info.fields[f_name];
                            let elem_size =
                                axon_perceus::size::compute_scalar_size(f_ty, &self.prog.ctx);
                            let elem_size_val = builder.ins().iconst(self.ptr_type, elem_size as i64);
                            let col_bytes = builder.ins().imul(cap_val, elem_size_val);

                            let col_call =
                                builder.ins().call(alloc_callee, &[col_bytes, align_val]);
                            let col_buf = builder.inst_results(col_call)[0];

                            let col_slot_offset = 16 + (i as i32 * 8);
                            builder.ins().store(
                                MemFlags::trusted(),
                                col_buf,
                                tbl_ptr,
                                col_slot_offset,
                            );
                        }
                    }
                }
                Ok(Some(tbl_ptr))
            }
            IrOp::GetField { target, field } => {
                let target_ty = func.var_types.get(target.0 as usize).unwrap_or(&Type::Unknown);
                let target_ptr = builder.use_var(Variable::new(target.0 as usize));

                match target_ty {
                    Type::Table(s_name) => {
                        let field_str = self.interner.resolve(field);
                        if field_str == "len" {
                            let val = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, 0);
                            Ok(Some(val))
                        } else if field_str == "capacity" {
                            let val = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, 8);
                            Ok(Some(val))
                        } else if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                            let col_idx = s_info.field_order.iter().position(|f| f == field);
                            if let Some(col_i) = col_idx {
                                let col_offset = 16 + (col_i as i32 * 8);
                                let val = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, col_offset);
                                Ok(Some(val))
                            } else {
                                Err(format!("Unknown field '{}' on Table", field_str))
                            }
                        } else {
                            Err(format!("Unknown Table struct {:?}", s_name))
                        }
                    }
                    Type::Struct(s_name) => {
                        if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                            let (f_ty, _) = s_info
                                .fields
                                .get(field)
                                .ok_or_else(|| format!("Unknown struct field {:?}", field))?;
                            let cl_ty = map_type(f_ty, self.ptr_type)
                                .ok_or_else(|| "Field type is void".to_string())?;
                            let offset = Self::struct_field_offset(s_info, *field, &self.prog.ctx)?;
                            let val = builder.ins().load(
                                cl_ty,
                                MemFlags::trusted(),
                                target_ptr,
                                offset as i32,
                            );
                            Ok(Some(val))
                        } else {
                            Err(format!("Unknown struct {:?}", s_name))
                        }
                    }
                    _ => Err(format!("GetField on non-struct/table type {:?}", target_ty)),
                }
            }
            IrOp::GetIndex {
                target,
                index,
                bounds_check,
            } => {
                let target_ty = func.var_types.get(target.0 as usize).unwrap_or(&Type::Unknown);
                let target_ptr = builder.use_var(Variable::new(target.0 as usize));
                let idx_val = self
                    .eval_operand(builder, index, func, Some(self.ptr_type))
                    .ok_or_else(|| "Index is void".to_string())?;

                if *bounds_check == BoundsCheckMode::Checked {
                    let len_val = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, 0);
                    let ok = builder.ins().icmp(IntCC::UnsignedLessThan, idx_val, len_val);
                    let ok_i32 = builder.ins().uextend(types::I32, ok);
                    let code_val = builder.ins().iconst(types::I32, 101);
                    let assert_callee = self.get_or_declare_host_func("axon_jit_assert", &[types::I32, types::I32], None, builder)?;
                    builder.ins().call(assert_callee, &[ok_i32, code_val]);
                }

                if let Type::Table(s_name) = target_ty {
                    if let Some(s_info) = self.prog.ctx.structs.get(s_name) {
                        let total_size: usize = s_info
                            .field_order
                            .iter()
                            .map(|f| axon_perceus::size::compute_scalar_size(&s_info.fields[f].0, &self.prog.ctx))
                            .sum();

                        let size_val = builder.ins().iconst(self.ptr_type, total_size as i64);
                        let align_val = builder.ins().iconst(self.ptr_type, 64);
                        let alloc_callee = self.get_or_declare_host_func(
                            "axon_jit_alloc",
                            &[self.ptr_type, self.ptr_type],
                            Some(self.ptr_type),
                            builder,
                        )?;
                        let call_inst = builder.ins().call(alloc_callee, &[size_val, align_val]);
                        let item_ptr = builder.inst_results(call_inst)[0];

                        // Copy each column element to synthetic struct
                        for (col_i, f_name) in s_info.field_order.iter().enumerate() {
                            let (f_ty, _) = &s_info.fields[f_name];
                            let cl_ty = map_type(f_ty, self.ptr_type).unwrap();
                            let elem_size = axon_perceus::size::compute_scalar_size(f_ty, &self.prog.ctx);

                            let col_offset = 16 + (col_i as i32 * 8);
                            let col_ptr = builder.ins().load(self.ptr_type, MemFlags::trusted(), target_ptr, col_offset);

                            let elem_size_val = builder.ins().iconst(self.ptr_type, elem_size as i64);
                            let byte_offset = builder.ins().imul(idx_val, elem_size_val);
                            let elem_addr = builder.ins().iadd(col_ptr, byte_offset);

                            let field_val = builder.ins().load(cl_ty, MemFlags::trusted(), elem_addr, 0);
                            let struct_offset = Self::struct_field_offset(s_info, *f_name, &self.prog.ctx)?;
                            builder.ins().store(MemFlags::trusted(), field_val, item_ptr, struct_offset as i32);
                        }
                        return Ok(Some(item_ptr));
                    }
                }

                Ok(Some(target_ptr))
            }
            IrOp::Call { func: callee, args } => {
                let target_func_id = self
                    .func_ids
                    .get(callee)
                    .ok_or_else(|| format!("Call to undeclared function {:?}", callee))?;

                let local_func = self
                    .module
                    .declare_func_in_func(*target_func_id, builder.func);

                let target_func_info = self.prog.functions.iter().find(|f| f.name == *callee);

                let mut arg_vals = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let expected_param_cl_ty = target_func_info
                        .and_then(|fi| fi.params.get(i))
                        .and_then(|p| map_type(&p.ty, self.ptr_type));
                    if let Some(v) = self.eval_operand(builder, a, func, expected_param_cl_ty) {
                        arg_vals.push(v);
                    }
                }

                let call_inst = builder.ins().call(local_func, &arg_vals);
                let results = builder.inst_results(call_inst);
                if results.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(results[0]))
                }
            }
            IrOp::StaticMethodCall {
                target,
                method,
                args,
            } => {
                let m_name = self.interner.resolve(method);
                if m_name == "with_capacity" {
                    let cap_op = args.first().cloned().unwrap_or(Operand::Const(IrConst::Int(0)));
                    let elem_ty = Type::Struct(*target);
                    return self.compile_op(
                        builder,
                        &IrOp::AllocTable {
                            elem_type: elem_ty,
                            capacity: cap_op,
                        },
                        func,
                        dest_cl_ty,
                    );
                }
                Ok(None)
            }
            IrOp::MethodCall { target, method, .. } => {
                let m_name = self.interner.resolve(method);
                if m_name == "join" {
                    // In Dev JIT mode, single-thread or cooperative fiber completes immediately
                    let target_val = builder.use_var(Variable::new(target.0 as usize));
                    Ok(Some(target_val))
                } else {
                    Ok(None)
                }
            }
            IrOp::Spawn { func_name, args } => {
                let target_func_id = self
                    .func_ids
                    .get(func_name)
                    .ok_or_else(|| format!("Spawn of undeclared fiber {:?}", func_name))?;
                let local_func = self
                    .module
                    .declare_func_in_func(*target_func_id, builder.func);
                let mut arg_vals = Vec::new();
                for a in args {
                    if let Some(v) = self.eval_operand(builder, a, func, None) {
                        arg_vals.push(v);
                    }
                }
                let call_inst = builder.ins().call(local_func, &arg_vals);
                let results = builder.inst_results(call_inst);
                if results.is_empty() {
                    let zero = builder.ins().iconst(self.ptr_type, 0);
                    Ok(Some(zero))
                } else {
                    Ok(Some(results[0]))
                }
            }
        }
    }

    fn eval_operand(
        &self,
        builder: &mut FunctionBuilder,
        op: &Operand,
        func: &IrFunction,
        expected_ty: Option<types::Type>,
    ) -> Option<Value> {
        let val = match op {
            Operand::Var(v) => {
                let ty = func.var_types.get(v.0 as usize)?;
                if *ty == Type::VOID {
                    return None;
                }
                builder.use_var(Variable::new(v.0 as usize))
            }
            Operand::Const(c) => match c {
                IrConst::Int(i) => match expected_ty {
                    Some(types::I32) => builder.ins().iconst(types::I32, *i as i32 as i64),
                    Some(types::I16) => builder.ins().iconst(types::I16, *i as i16 as i64),
                    Some(types::I8) => builder.ins().iconst(types::I8, *i as i8 as i64),
                    Some(types::F32) => builder.ins().f32const(*i as f32),
                    Some(types::F64) => builder.ins().f64const(*i as f64),
                    _ => builder.ins().iconst(types::I64, *i),
                },
                IrConst::Float(fl) => match expected_ty {
                    Some(types::F64) => builder.ins().f64const(*fl),
                    _ => builder.ins().f32const(*fl as f32),
                },
                IrConst::Bool(b) => {
                    let val = if *b { 1 } else { 0 };
                    builder.ins().iconst(types::I8, val)
                }
            },
        };

        if let Some(target_cl_ty) = expected_ty {
            let actual_ty = builder.func.dfg.value_type(val);
            if actual_ty != target_cl_ty {
                if actual_ty.is_int() && target_cl_ty.is_int() {
                    if actual_ty.bits() < target_cl_ty.bits() {
                        return Some(builder.ins().sextend(target_cl_ty, val));
                    } else if actual_ty.bits() > target_cl_ty.bits() {
                        return Some(builder.ins().ireduce(target_cl_ty, val));
                    }
                } else if actual_ty.is_float() && target_cl_ty.is_float() {
                    if actual_ty.bits() < target_cl_ty.bits() {
                        return Some(builder.ins().fpromote(target_cl_ty, val));
                    } else if actual_ty.bits() > target_cl_ty.bits() {
                        return Some(builder.ins().fdemote(target_cl_ty, val));
                    }
                }
            }
        }

        Some(val)
    }

    fn struct_field_offset(
        s_info: &axon_types::StructInfo,
        field: Spur,
        ctx: &axon_types::GlobalContext,
    ) -> Result<usize, String> {
        let mut offset = 0;
        for f_name in &s_info.field_order {
            if *f_name == field {
                return Ok(offset);
            }
            let (f_ty, _) = &s_info.fields[f_name];
            offset += axon_perceus::size::compute_scalar_size(f_ty, ctx);
        }
        Err(format!("Field {:?} not in struct {:?}", field, s_info.name))
    }

    fn get_or_declare_host_func(
        &mut self,
        name: &str,
        param_tys: &[types::Type],
        ret_ty: Option<types::Type>,
        builder: &mut FunctionBuilder,
    ) -> Result<FuncRef, String> {
        let mut sig = self.module.make_signature();
        for p in param_tys {
            sig.params.push(AbiParam::new(*p));
        }
        if let Some(r) = ret_ty {
            sig.returns.push(AbiParam::new(r));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| e.to_string())?;

        Ok(self.module.declare_func_in_func(func_id, builder.func))
    }
}
