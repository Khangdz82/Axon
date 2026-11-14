//! axon_codegen_c: C23 Transpiler Backend for Axon
//!
//! Emits clean, standard C23 / C11 code implementing Native Structure-of-Arrays (SoA)
//! for `Table[T]`, SIMD vectorization pragmas, Perceus in-place memory reuse,
//! and elided bounds checks.

pub mod runtime_header;
pub mod soa;
pub mod types;

use axon_ast::BinOp;
use axon_perceus::*;
use axon_types::Type;
use lasso::Rodeo;
use runtime_header::RUNTIME_HEADER;
use soa::generate_struct_and_soa;
use std::collections::{HashMap, HashSet};
use types::to_c_type;

pub struct CCodeGen<'a> {
    ir: &'a IrProgram,
    interner: &'a Rodeo,
}

impl<'a> CCodeGen<'a> {
    pub fn new(ir: &'a IrProgram, interner: &'a Rodeo) -> Self {
        Self { ir, interner }
    }

    pub fn generate(&self) -> String {
        let mut out = String::new();

        // 1. Runtime Header
        out.push_str(RUNTIME_HEADER);
        out.push('\n');

        // 2. Struct Declarations and Native SoA Table Definitions
        out.push_str("// === NATIVE SoA & STRUCT DEFINITIONS ===\n");
        for s_info in self.ir.ctx.structs.values() {
            out.push_str(&generate_struct_and_soa(s_info, &self.ir.ctx, self.interner));
        }

        // 3. Forward Declarations of all Functions
        out.push_str("// === FUNCTION FORWARD DECLARATIONS ===\n");
        for func in &self.ir.functions {
            let fn_name = self.interner.resolve(&func.name);
            let ret_c = to_c_type(&func.ret_type, self.interner);

            if fn_name == "main" {
                out.push_str("int main(void);\n");
            } else {
                out.push_str(&format!("{} Axon_{}(", ret_c, fn_name));
                for (i, p) in func.params.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let p_c_ty = to_c_type(&p.ty, self.interner);
                    out.push_str(&format!("{} v{}", p_c_ty, p.var.0));
                }
                out.push_str(");\n");

                if fn_name.starts_with("__axon_fiber_") {
                    out.push_str(&format!("typedef struct Axon_Ctx_{} {{\n", fn_name));
                    if func.params.is_empty() {
                        out.push_str("    char __empty;\n");
                    } else {
                        for (i, p) in func.params.iter().enumerate() {
                            let p_c_ty = to_c_type(&p.ty, self.interner);
                            out.push_str(&format!("    {} v{};\n", p_c_ty, i));
                        }
                    }
                    out.push_str(&format!("}} Axon_Ctx_{};\n", fn_name));
                    out.push_str(&format!("void* Axon_thunk_{}(void* __raw_ctx);\n", fn_name));
                }
            }
        }
        out.push('\n');

        // 4. Function Implementations
        out.push_str("// === FUNCTION IMPLEMENTATIONS ===\n");
        for func in &self.ir.functions {
            let fn_name = self.interner.resolve(&func.name);
            out.push_str(&self.generate_function(func));
            out.push('\n');

            if fn_name.starts_with("__axon_fiber_") {
                out.push_str(&format!("void* Axon_thunk_{}(void* __raw_ctx) {{\n", fn_name));
                out.push_str(&format!("    Axon_Ctx_{0}* __ctx = (Axon_Ctx_{0}*)__raw_ctx;\n", fn_name));
                let mut call_args = String::new();
                for (i, _) in func.params.iter().enumerate() {
                    if i > 0 {
                        call_args.push_str(", ");
                    }
                    call_args.push_str(&format!("__ctx->v{}", i));
                }
                if func.ret_type == Type::VOID {
                    out.push_str(&format!("    Axon_{}({});\n", fn_name, call_args));
                    out.push_str("    free(__ctx);\n");
                    out.push_str("    return NULL;\n");
                } else if matches!(func.ret_type, Type::Table(_) | Type::Struct(_)) {
                    let ret_c = to_c_type(&func.ret_type, self.interner);
                    out.push_str(&format!("    {0} __res = Axon_{1}({2});\n", ret_c, fn_name, call_args));
                    out.push_str("    free(__ctx);\n");
                    out.push_str(&format!("    {0}* __boxed = ({0}*)malloc(sizeof({0}));\n", ret_c));
                    out.push_str("    *__boxed = __res;\n");
                    out.push_str("    return (void*)__boxed;\n");
                } else {
                    let ret_c = to_c_type(&func.ret_type, self.interner);
                    out.push_str(&format!("    {0} __res = Axon_{1}({2});\n", ret_c, fn_name, call_args));
                    out.push_str("    free(__ctx);\n");
                    out.push_str("    return (void*)(intptr_t)__res;\n");
                }
                out.push_str("}\n\n");
            }
        }

        out
    }

    fn generate_function(&self, func: &IrFunction) -> String {
        let mut out = String::new();
        let fn_name = self.interner.resolve(&func.name);
        let ret_c = to_c_type(&func.ret_type, self.interner);

        if fn_name == "main" {
            out.push_str("int main(void) {\n");
            if self.program_has_spawn() {
                out.push_str("    axon_scheduler_init(0);\n");
            }
        } else {
            out.push_str(&format!("{} Axon_{}(", ret_c, fn_name));
            for (i, p) in func.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let p_c_ty = to_c_type(&p.ty, self.interner);
                out.push_str(&format!("{} v{}", p_c_ty, p.var.0));
            }
            out.push_str(") {\n");
        }

        // Identify spawn variables, reuse tokens, and table indexing sources
        let mut spawn_vars = HashSet::new();
        let mut reuse_token_vars = HashSet::new();
        let mut index_source_map: HashMap<IrVar, (IrVar, Operand)> = HashMap::new();

        for block in &func.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::ReuseToken { dest, .. } => {
                        reuse_token_vars.insert(*dest);
                    }
                    Instruction::Assign {
                        dest,
                        op: IrOp::Spawn { .. },
                        ..
                    } => {
                        spawn_vars.insert(*dest);
                    }
                    Instruction::Assign {
                        dest,
                        op: IrOp::Copy(Operand::Var(src)),
                        ..
                    } => {
                        if spawn_vars.contains(src) {
                            spawn_vars.insert(*dest);
                        }
                    }
                    Instruction::Assign {
                        dest,
                        op:
                            IrOp::GetIndex {
                                target,
                                index,
                                ..
                            },
                        ..
                    } => {
                        index_source_map.insert(*dest, (*target, index.clone()));
                    }
                    _ => {}
                }
            }
        }

        // Declare local variables at top of function
        let mut param_vars = HashSet::new();
        for p in &func.params {
            param_vars.insert(p.var);
        }

        for (id, ty) in func.var_types.iter().enumerate() {
            let v = IrVar(id as u32);
            if !param_vars.contains(&v) {
                if *ty == Type::VOID {
                    continue; // Skip declaring void variables in C
                }
                if reuse_token_vars.contains(&v) {
                    out.push_str(&format!("    void* v{} = NULL;\n", id));
                } else if spawn_vars.contains(&v) || matches!(ty, Type::Job(_)) {
                    out.push_str(&format!("    Axon_Job* v{} = NULL;\n", id));
                } else if matches!(ty, Type::Struct(_) | Type::Table(_)) {
                    let c_ty = to_c_type(ty, self.interner);
                    out.push_str(&format!("    {} v{} = {{0}};\n", c_ty, id));
                } else {
                    let c_ty = to_c_type(ty, self.interner);
                    out.push_str(&format!("    {} v{} = 0;\n", c_ty, id));
                }
            }
        }
        out.push('\n');

        // Generate Basic Blocks
        for block in &func.blocks {
            out.push_str(&format!("  {}: ;\n", block.id));

            for inst in &block.instructions {
                self.generate_instruction(&mut out, inst, func, &index_source_map);
            }

            match &block.terminator {
                Terminator::Return { value: Some(op), .. } => {
                    if fn_name == "main" && self.program_has_spawn() {
                        out.push_str("    axon_scheduler_shutdown();\n");
                    }
                    out.push_str(&format!("    return {};\n", self.operand_to_c(op)));
                }
                Terminator::Return { value: None, .. } => {
                    if fn_name == "main" {
                        if self.program_has_spawn() {
                            out.push_str("    axon_scheduler_shutdown();\n");
                        }
                        out.push_str("    return 0;\n");
                    } else {
                        out.push_str("    return;\n");
                    }
                }
                Terminator::Branch { target, .. } => {
                    out.push_str(&format!("    goto {};\n", target));
                }
                Terminator::CondBranch {
                    cond,
                    then_block,
                    else_block,
                    ..
                } => {
                    out.push_str(&format!(
                        "    if ({}) goto {}; else goto {};\n",
                        self.operand_to_c(cond),
                        then_block,
                        else_block
                    ));
                }
            }
        }

        out.push_str("}\n");
        out
    }

    fn generate_instruction(
        &self,
        out: &mut String,
        inst: &Instruction,
        func: &IrFunction,
        index_source_map: &HashMap<IrVar, (IrVar, Operand)>,
    ) {
        match inst {
            Instruction::Assign { dest, op, .. } => {
                let dest_ty = func.var_types.get(dest.0 as usize).unwrap_or(&Type::Unknown);
                if let IrOp::Spawn { func_name, args } = op {
                    let fn_str = self.interner.resolve(func_name);
                    out.push_str(&format!(
                        "    Axon_Ctx_{0}* __ctx_{1} = (Axon_Ctx_{0}*)malloc(sizeof(Axon_Ctx_{0}));\n",
                        fn_str, dest.0
                    ));
                    for (i, a) in args.iter().enumerate() {
                        out.push_str(&format!(
                            "    __ctx_{0}->v{1} = {2};\n",
                            dest.0,
                            i,
                            self.operand_to_c(a)
                        ));
                    }
                    out.push_str(&format!(
                        "    v{0} = axon_spawn(&Axon_thunk_{1}, __ctx_{0});\n",
                        dest.0, fn_str
                    ));
                    return;
                }

                if let IrOp::MethodCall { target, method, .. } = op {
                    let m_str = self.interner.resolve(method);
                    if m_str == "join" {
                        if *dest_ty == Type::VOID {
                            out.push_str(&format!("    axon_join(v{});\n", target.0));
                        } else if matches!(dest_ty, Type::Table(_) | Type::Struct(_)) {
                            let c_ty = to_c_type(dest_ty, self.interner);
                            out.push_str(&format!(
                                "    void* __res_ptr_{0} = axon_join(v{1});\n",
                                dest.0, target.0
                            ));
                            out.push_str(&format!(
                                "    v{0} = *({1}*)__res_ptr_{0};\n",
                                dest.0, c_ty
                            ));
                            out.push_str(&format!("    free(__res_ptr_{});\n", dest.0));
                        } else {
                            let c_ty = to_c_type(dest_ty, self.interner);
                            out.push_str(&format!(
                                "    v{0} = ({1})(intptr_t)axon_join(v{2});\n",
                                dest.0, c_ty, target.0
                            ));
                        }
                        return;
                    }
                }

                if *dest_ty == Type::VOID {
                    out.push_str("    ");
                    self.generate_op(out, op, func);
                    out.push_str(";\n");
                } else {
                    out.push_str(&format!("    {} = ", dest));
                    self.generate_op(out, op, func);
                    out.push_str(";\n");
                }
            }
            Instruction::Drop { var, byte_size, .. } => {
                let var_ty = func.var_types.get(var.0 as usize).unwrap_or(&Type::Unknown);
                if let Type::Table(s_name) = var_ty {
                    if self.ir.ctx.structs.contains_key(s_name) {
                        out.push_str(&format!(
                            "    Axon_Table_{}_free(&{});\n",
                            self.interner.resolve(s_name),
                            var
                        ));
                    }
                } else if *byte_size > 0 {
                    out.push_str(&format!("    // drop {} (size: {}B)\n", var, byte_size));
                }
            }
            Instruction::ReuseToken {
                dest,
                source,
                byte_size,
                ..
            } => {
                out.push_str(&format!(
                    "    {} = (void*)(&{}); // FBIP reuse token (size: {}B)\n",
                    dest, source, byte_size
                ));
            }
            Instruction::SetField {
                target,
                field,
                value,
                ..
            } => {
                let f_str = self.interner.resolve(field);
                out.push_str(&format!(
                    "    {}.{} = {};\n",
                    target,
                    f_str,
                    self.operand_to_c(value)
                ));
                // If target was loaded from an SoA table column, write back to column!
                if let Some((tbl_var, idx_op)) = index_source_map.get(target) {
                    out.push_str(&format!(
                        "    {}.{}[{}] = {};\n",
                        tbl_var,
                        f_str,
                        self.operand_to_c(idx_op),
                        self.operand_to_c(value)
                    ));
                }
            }
            Instruction::SetIndex {
                target,
                index,
                value,
                bounds_check,
                ..
            } => {
                let target_ty = func.var_types.get(target.0 as usize).unwrap_or(&Type::Unknown);
                if *bounds_check == BoundsCheckMode::Checked {
                    out.push_str(&format!(
                        "    assert({0} < {1}.len && \"Index out of bounds\");\n",
                        self.operand_to_c(index),
                        target
                    ));
                }
                if let Type::Table(s_name) = target_ty {
                    if !self.ir.ctx.structs.contains_key(s_name) {
                        out.push_str(&format!(
                            "    {}.data[{}] = {};\n",
                            target,
                            self.operand_to_c(index),
                            self.operand_to_c(value)
                        ));
                        return;
                    }
                }
                out.push_str(&format!(
                    "    {}[{}] = {};\n",
                    target,
                    self.operand_to_c(index),
                    self.operand_to_c(value)
                ));
            }
            Instruction::Eval { op, .. } => {
                if let IrOp::Spawn { func_name, args } = op {
                    let fn_str = self.interner.resolve(func_name);
                    out.push_str(&format!(
                        "    Axon_Ctx_{0}* __ctx_eval = (Axon_Ctx_{0}*)malloc(sizeof(Axon_Ctx_{0}));\n",
                        fn_str
                    ));
                    for (i, a) in args.iter().enumerate() {
                        out.push_str(&format!(
                            "    __ctx_eval->v{0} = {1};\n",
                            i,
                            self.operand_to_c(a)
                        ));
                    }
                    out.push_str(&format!(
                        "    axon_spawn(&Axon_thunk_{}, __ctx_eval);\n",
                        fn_str
                    ));
                    return;
                }
                out.push_str("    ");
                self.generate_op(out, op, func);
                out.push_str(";\n");
            }
        }
    }

    fn generate_op(&self, out: &mut String, op: &IrOp, func: &IrFunction) {
        match op {
            IrOp::Copy(op) => {
                out.push_str(&self.operand_to_c(op));
            }
            IrOp::Binary { op, left, right } => {
                let l = self.operand_to_c(left);
                let r = self.operand_to_c(right);
                match op {
                    BinOp::Add => out.push_str(&format!("{} + {}", l, r)),
                    BinOp::AddWrap => out.push_str(&format!("axon_wrap_add_i32({}, {})", l, r)),
                    BinOp::AddSat => out.push_str(&format!("axon_sat_add_i32({}, {})", l, r)),
                    BinOp::Sub => out.push_str(&format!("{} - {}", l, r)),
                    BinOp::SubWrap => out.push_str(&format!("axon_wrap_sub_i32({}, {})", l, r)),
                    BinOp::SubSat => out.push_str(&format!("axon_sat_sub_i32({}, {})", l, r)),
                    BinOp::Mul => out.push_str(&format!("{} * {}", l, r)),
                    BinOp::MulWrap => out.push_str(&format!("axon_wrap_mul_i32({}, {})", l, r)),
                    BinOp::MulSat => out.push_str(&format!("axon_sat_mul_i32({}, {})", l, r)),
                    BinOp::Div => out.push_str(&format!("{} / {}", l, r)),
                    BinOp::Mod => out.push_str(&format!("{} % {}", l, r)),
                    BinOp::Eq => out.push_str(&format!("{} == {}", l, r)),
                    BinOp::NotEq => out.push_str(&format!("{} != {}", l, r)),
                    BinOp::Lt => out.push_str(&format!("{} < {}", l, r)),
                    BinOp::LtEq => out.push_str(&format!("{} <= {}", l, r)),
                    BinOp::Gt => out.push_str(&format!("{} > {}", l, r)),
                    BinOp::GtEq => out.push_str(&format!("{} >= {}", l, r)),
                    BinOp::And => out.push_str(&format!("{} && {}", l, r)),
                    BinOp::Or => out.push_str(&format!("{} || {}", l, r)),
                }
            }
            IrOp::Alloc {
                type_name, fields, ..
            } => {
                let s_name = self.interner.resolve(type_name);
                out.push_str(&format!("(Axon_{}){{", s_name));
                for (i, (f_name, f_val)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!(
                        ".{} = {}",
                        self.interner.resolve(f_name),
                        self.operand_to_c(f_val)
                    ));
                }
                out.push('}');
            }
            IrOp::AllocInPlace {
                token,
                type_name,
                fields,
                ..
            } => {
                let s_name = self.interner.resolve(type_name);
                out.push_str(&format!(
                    "(*((Axon_{0}*){1}) = (Axon_{0}){{",
                    s_name, token
                ));
                for (i, (f_name, f_val)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!(
                        ".{} = {}",
                        self.interner.resolve(f_name),
                        self.operand_to_c(f_val)
                    ));
                }
                out.push_str("})");
            }
            IrOp::AllocTable { elem_type, capacity } => {
                if let Type::Struct(s_name) = elem_type {
                    out.push_str(&format!(
                        "Axon_Table_{}_with_capacity({})",
                        self.interner.resolve(s_name),
                        self.operand_to_c(capacity)
                    ));
                } else {
                    out.push('0');
                }
            }
            IrOp::GetField { target, field } => {
                out.push_str(&format!("{}.{}", target, self.interner.resolve(field)));
            }
            IrOp::GetIndex {
                target,
                index,
                bounds_check,
            } => {
                let target_ty = func.var_types.get(target.0 as usize).unwrap_or(&Type::Unknown);
                if *bounds_check == BoundsCheckMode::Checked {
                    out.push_str(&format!(
                        "({0} < {1}.len ? 0 : (assert(false && \"Index out of bounds\"), 0)), ",
                        self.operand_to_c(index),
                        target
                    ));
                }
                if let Type::Table(s_name) = target_ty {
                    if let Some(s_info) = self.ir.ctx.structs.get(s_name) {
                        out.push_str(&format!("(Axon_{}){{", self.interner.resolve(s_name)));
                        for (i, f_name) in s_info.field_order.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            let f_str = self.interner.resolve(f_name);
                            out.push_str(&format!(
                                ".{0} = {1}.{0}[{2}]",
                                f_str,
                                target,
                                self.operand_to_c(index)
                            ));
                        }
                        out.push('}');
                        return;
                    } else {
                        // Primitive table data array
                        out.push_str(&format!("{}.data[{}]", target, self.operand_to_c(index)));
                        return;
                    }
                }
                out.push_str(&format!("{}[{}]", target, self.operand_to_c(index)));
            }
            IrOp::Call { func, args } => {
                out.push_str(&format!("Axon_{}(", self.interner.resolve(func)));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&self.operand_to_c(a));
                }
                out.push(')');
            }
            IrOp::StaticMethodCall { target, method, args } => {
                out.push_str(&format!(
                    "Axon_{}_{}(",
                    self.interner.resolve(target),
                    self.interner.resolve(method)
                ));
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&self.operand_to_c(a));
                }
                out.push(')');
            }
            IrOp::MethodCall { target, method, .. } => {
                let m_name = self.interner.resolve(method);
                if m_name == "join" {
                    out.push_str(&format!("(int64_t)(intptr_t)axon_join({})", target));
                } else {
                    out.push_str(&format!("{}.{}()", target, m_name));
                }
            }
            IrOp::Spawn { func_name, .. } => {
                let fn_str = self.interner.resolve(func_name);
                out.push_str(&format!("axon_spawn(&Axon_thunk_{}, NULL)", fn_str));
            }
        }
    }

    fn program_has_spawn(&self) -> bool {
        self.ir.functions.iter().any(|f| {
            f.blocks.iter().any(|b| {
                b.instructions.iter().any(|inst| {
                    matches!(
                        inst,
                        Instruction::Assign {
                            op: IrOp::Spawn { .. },
                            ..
                        } | Instruction::Eval {
                            op: IrOp::Spawn { .. },
                            ..
                        }
                    )
                })
            })
        })
    }

    fn operand_to_c(&self, op: &Operand) -> String {
        match op {
            Operand::Var(v) => format!("v{}", v.0),
            Operand::Const(IrConst::Int(i)) => format!("{}", i),
            Operand::Const(IrConst::Float(f)) => {
                if f.fract() == 0.0 {
                    format!("{:.1}f", f)
                } else {
                    format!("{}f", f)
                }
            }
            Operand::Const(IrConst::Bool(b)) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
        }
    }
}

pub fn generate_c_program(ir: &IrProgram, interner: &Rodeo) -> String {
    let codegen = CCodeGen::new(ir, interner);
    codegen.generate()
}
