//! axon_types: Type Checker, Second-Class Reference Verifier & Local Reasoning Engine

pub mod types;

use axon_ast::*;
use axon_span::ByteSpan;
use lasso::{Rodeo, Spur};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub use types::Type;

#[derive(Error, Debug, Clone, PartialEq)]
#[error("Type error at {span:?}: {msg}")]
pub struct TypeError {
    pub span: ByteSpan,
    pub msg: String,
}

impl TypeError {
    pub fn new(span: ByteSpan, msg: impl Into<String>) -> Self {
        Self {
            span,
            msg: msg.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: Spur,
    pub fields: HashMap<Spur, (Type, ByteSpan)>,
    pub field_order: Vec<Spur>,
    pub span: ByteSpan,
}

#[derive(Debug, Clone)]
pub struct FuncSignature {
    pub name: Spur,
    pub params: Vec<(ParamIntent, Spur, Type)>,
    pub ret_type: Type,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalContext {
    pub structs: HashMap<Spur, StructInfo>,
    pub functions: HashMap<Spur, FuncSignature>,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve_ast_type(
        &self,
        ty_id: TypeId,
        ast: FrozenAst<'_, '_>,
        interner: &Rodeo,
    ) -> Result<Type, TypeError> {
        let ty_data = ast.get_type(ty_id);
        match ty_data {
            TypeData::Primitive(p, _) => Ok(Type::Primitive(*p)),
            TypeData::Named(spur, span) => {
                let name = interner.resolve(spur);
                if self.structs.contains_key(spur) {
                    Ok(Type::Struct(*spur))
                } else {
                    Err(TypeError::new(*span, format!("Unknown type '{}'", name)))
                }
            }
            TypeData::Table(elem, span) => {
                let elem_name = interner.resolve(elem);
                let is_prim = matches!(
                    elem_name,
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "f32" | "f64" | "bool"
                );
                if is_prim || self.structs.contains_key(elem) {
                    Ok(Type::Table(*elem))
                } else {
                    Err(TypeError::new(
                        *span,
                        format!("Unknown element type '{}' for Table", elem_name),
                    ))
                }
            }
            TypeData::Reference { inner, is_mut, .. } => {
                let inner_ty = self.resolve_ast_type(*inner, ast, interner)?;
                Ok(Type::Reference {
                    inner: Box::new(inner_ty),
                    is_mut: *is_mut,
                })
            }
            TypeData::Refined { base, .. } => {
                let base_ty = self.resolve_ast_type(*base, ast, interner)?;
                Ok(Type::Refined {
                    base: Box::new(base_ty),
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
struct VarState {
    intent: ParamIntent,
    ty: Type,
    moved: Option<ByteSpan>,
}

pub struct TypeChecker<'a> {
    arena: &'a AstArena<'a>,
    interner: &'a Rodeo,
    ctx: GlobalContext,
}

impl<'a> TypeChecker<'a> {
    pub fn new(arena: &'a AstArena<'a>, interner: &'a Rodeo) -> Self {
        Self {
            arena,
            interner,
            ctx: GlobalContext::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program<'a>) -> Result<(), Vec<TypeError>> {
        let mut errors = Vec::new();
        let frozen_ast = self.arena.freeze();

        // 0. Forward declare all struct names
        for s in program.structs.iter() {
            self.ctx.structs.insert(
                s.name,
                StructInfo {
                    name: s.name,
                    fields: HashMap::new(),
                    field_order: Vec::new(),
                    span: s.span,
                },
            );
        }

        // 1. Resolve struct field definitions
        for s in program.structs.iter() {
            let mut fields = HashMap::new();
            let mut field_order = Vec::new();

            for f in s.fields.iter() {
                let f_ty = match self.ctx.resolve_ast_type(f.ty, frozen_ast, self.interner) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        Type::Unknown
                    }
                };

                // Invariant 1: Second-Class References
                // Struct fields can NEVER be references
                if f_ty.is_reference() {
                    let s_name = self.interner.resolve(&s.name);
                    let f_name = self.interner.resolve(&f.name);
                    errors.push(TypeError::new(
                        f.span,
                        format!(
                            "Second-Class Reference Violation: struct '{}' cannot store reference field '{}'",
                            s_name, f_name
                        ),
                    ));
                }

                fields.insert(f.name, (f_ty, f.span));
                field_order.push(f.name);
            }

            self.ctx.structs.insert(
                s.name,
                StructInfo {
                    name: s.name,
                    fields,
                    field_order,
                    span: s.span,
                },
            );
        }

        // 2. Collect function signatures
        for f in program.functions.iter() {
            let mut params = Vec::new();
            for p in f.params.iter() {
                let p_ty = match self.ctx.resolve_ast_type(p.ty, frozen_ast, self.interner) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        Type::Unknown
                    }
                };
                params.push((p.intent, p.name, p_ty));
            }

            let ret_ty = if let Some(rt) = f.ret_type {
                match self.ctx.resolve_ast_type(rt, frozen_ast, self.interner) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        Type::Unknown
                    }
                }
            } else {
                Type::VOID
            };

            // Invariant 1: Second-Class References
            // Functions can NEVER return a reference
            if ret_ty.is_reference() {
                let f_name = self.interner.resolve(&f.name);
                errors.push(TypeError::new(
                    f.span,
                    format!(
                        "Second-Class Reference Violation: function '{}' cannot return a reference",
                        f_name
                    ),
                ));
            }

            self.ctx.functions.insert(
                f.name,
                FuncSignature {
                    name: f.name,
                    params,
                    ret_type: ret_ty,
                    span: f.span,
                },
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // 3. Local Reasoning: Check every function independently in parallel via Rayon
        let ctx = &self.ctx;
        let interner = self.interner;

        let results: Vec<Result<(), Vec<TypeError>>> = program
            .functions
            .par_iter()
            .map(|f| Self::check_function(f, ctx, frozen_ast, interner))
            .collect();

        for res in results {
            if let Err(mut errs) = res {
                errors.append(&mut errs);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_function<'b>(
        f: &FuncDef<'a>,
        ctx: &GlobalContext,
        ast: FrozenAst<'b, 'a>,
        interner: &Rodeo,
    ) -> Result<(), Vec<TypeError>> {
        let sig = &ctx.functions[&f.name];
        let mut env: HashMap<Spur, VarState> = HashMap::new();
        let mut errors = Vec::new();

        for (intent, name, ty) in &sig.params {
            env.insert(
                *name,
                VarState {
                    intent: *intent,
                    ty: ty.clone(),
                    moved: None,
                },
            );
        }

        for &stmt_id in f.body {
            if let Err(e) = Self::check_stmt(stmt_id, &mut env, sig, ctx, ast, interner) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_stmt<'b>(
        stmt_id: StmtId,
        env: &mut HashMap<Spur, VarState>,
        sig: &FuncSignature,
        ctx: &GlobalContext,
        ast: FrozenAst<'b, 'a>,
        interner: &Rodeo,
    ) -> Result<(), TypeError> {
        let stmt = ast.get_stmt(stmt_id);
        match stmt {
            StmtData::Let {
                name,
                ty,
                init,
                span: _,
            } => {
                let init_ty = Self::check_expr(*init, env, ctx, ast, interner)?;
                let decl_ty = if let Some(t_id) = ty {
                    let t = ctx.resolve_ast_type(*t_id, ast, interner)?;
                    if t != init_ty && init_ty != Type::Unknown {
                        return Err(TypeError::new(
                            ast.get_expr(*init).span(),
                            format!(
                                "Type mismatch: variable '{}' declared with {:?}, but initialized with {:?}",
                                interner.resolve(name),
                                t,
                                init_ty
                            ),
                        ));
                    }
                    t
                } else {
                    init_ty
                };

                env.insert(
                    *name,
                    VarState {
                        intent: ParamIntent::Let,
                        ty: decl_ty,
                        moved: None,
                    },
                );
                Ok(())
            }

            StmtData::IsolatedLet {
                name,
                ty,
                init,
                span: _,
            } => {
                let init_ty = Self::check_expr(*init, env, ctx, ast, interner)?;
                let decl_ty = if let Some(t_id) = ty {
                    ctx.resolve_ast_type(*t_id, ast, interner)?
                } else {
                    init_ty
                };

                env.insert(
                    *name,
                    VarState {
                        intent: ParamIntent::Isolated,
                        ty: decl_ty,
                        moved: None,
                    },
                );
                Ok(())
            }

            StmtData::Assign {
                target,
                op: _,
                value,
                span: _,
            } => {
                let target_expr = ast.get_expr(*target);
                match target_expr {
                    ExprData::Ident(spur, span) => {
                        let var_state = env.get(spur).ok_or_else(|| {
                            TypeError::new(*span, format!("Undefined variable '{}'", interner.resolve(spur)))
                        })?;

                        // Check Use-After-Move
                        if let Some(move_span) = var_state.moved {
                            return Err(TypeError::new(
                                *span,
                                format!(
                                    "Linear Isolation Violation: cannot assign to moved variable '{}' (previously moved at {:?})",
                                    interner.resolve(spur),
                                    move_span
                                ),
                            ));
                        }

                        // Check Mutability
                        if var_state.intent == ParamIntent::Let {
                            return Err(TypeError::new(
                                *span,
                                format!(
                                    "Cannot mutate immutable 'let' variable '{}'",
                                    interner.resolve(spur)
                                ),
                            ));
                        }
                    }
                    ExprData::FieldAccess { target: base, field: _, span } => {
                        let _base_ty = Self::check_expr(*base, env, ctx, ast, interner)?;
                        if let ExprData::Ident(spur, _) = ast.get_expr(*base) {
                            if let Some(var_state) = env.get(spur) {
                                if let Some(move_span) = var_state.moved {
                                    return Err(TypeError::new(
                                        *span,
                                        format!(
                                            "Linear Isolation Violation: cannot access field of moved variable '{}' (previously moved at {:?})",
                                            interner.resolve(spur),
                                            move_span
                                        ),
                                    ));
                                }
                                if var_state.intent == ParamIntent::Let {
                                    return Err(TypeError::new(
                                        *span,
                                        format!(
                                            "Cannot mutate field of immutable 'let' variable '{}'",
                                            interner.resolve(spur)
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }

                Self::check_expr(*value, env, ctx, ast, interner)?;
                Ok(())
            }

            StmtData::For {
                loop_var,
                iter_target,
                body,
                span: _,
            } => {
                let iter_ty = Self::check_expr(*iter_target, env, ctx, ast, interner)?;
                let elem_ty = match iter_ty {
                    Type::Table(elem_spur) => {
                        if ctx.structs.contains_key(&elem_spur) {
                            Type::Struct(elem_spur)
                        } else {
                            Type::Primitive(PrimitiveType::F32)
                        }
                    }
                    _ => Type::Unknown,
                };

                let mut sub_env = env.clone();
                sub_env.insert(
                    *loop_var,
                    VarState {
                        intent: ParamIntent::Mut,
                        ty: elem_ty,
                        moved: None,
                    },
                );

                for &s in *body {
                    Self::check_stmt(s, &mut sub_env, sig, ctx, ast, interner)?;
                }
                Ok(())
            }

            StmtData::While { cond, body, span: _ } => {
                let cond_ty = Self::check_expr(*cond, env, ctx, ast, interner)?;
                if cond_ty != Type::BOOL && cond_ty != Type::Unknown {
                    return Err(TypeError::new(
                        ast.get_expr(*cond).span(),
                        format!("Condition in while loop must be bool, found {:?}", cond_ty),
                    ));
                }
                for &s in *body {
                    Self::check_stmt(s, env, sig, ctx, ast, interner)?;
                }
                Ok(())
            }

            StmtData::If {
                cond,
                then_branch,
                else_branch,
                span: _,
            } => {
                let cond_ty = Self::check_expr(*cond, env, ctx, ast, interner)?;
                if cond_ty != Type::BOOL && cond_ty != Type::Unknown {
                    return Err(TypeError::new(
                        ast.get_expr(*cond).span(),
                        format!("Condition in if statement must be bool, found {:?}", cond_ty),
                    ));
                }

                let mut then_env = env.clone();
                for &s in *then_branch {
                    Self::check_stmt(s, &mut then_env, sig, ctx, ast, interner)?;
                }

                if let Some(eb) = else_branch {
                    let mut else_env = env.clone();
                    for &s in *eb {
                        Self::check_stmt(s, &mut else_env, sig, ctx, ast, interner)?;
                    }
                }
                Ok(())
            }

            StmtData::Return {
                is_move,
                value,
                span,
            } => {
                if let Some(val_id) = value {
                    let ret_ty = Self::check_expr(*val_id, env, ctx, ast, interner)?;

                    if *is_move {
                        if let ExprData::Ident(spur, v_span) = ast.get_expr(*val_id) {
                            let var_state = env.get_mut(spur).ok_or_else(|| {
                                TypeError::new(*v_span, format!("Undefined variable '{}'", interner.resolve(spur)))
                            })?;

                            if let Some(m_span) = var_state.moved {
                                return Err(TypeError::new(
                                    *v_span,
                                    format!(
                                        "Linear Isolation Violation: cannot move already-moved variable '{}' (moved at {:?})",
                                        interner.resolve(spur),
                                        m_span
                                    ),
                                ));
                            }

                            var_state.moved = Some(*span);
                        }
                    }

                    if sig.ret_type != Type::Unknown && ret_ty != Type::Unknown && sig.ret_type != ret_ty {
                        return Err(TypeError::new(
                            *span,
                            format!(
                                "Return type mismatch: function expected {:?}, returned {:?}",
                                sig.ret_type, ret_ty
                            ),
                        ));
                    }
                }
                Ok(())
            }

            StmtData::Expr(expr_id) => {
                Self::check_expr(*expr_id, env, ctx, ast, interner)?;
                Ok(())
            }
        }
    }

    fn check_expr<'b>(
        expr_id: ExprId,
        env: &mut HashMap<Spur, VarState>,
        ctx: &GlobalContext,
        ast: FrozenAst<'b, 'a>,
        interner: &Rodeo,
    ) -> Result<Type, TypeError> {
        let expr = ast.get_expr(expr_id);
        match expr {
            ExprData::Int(_, _) => Ok(Type::I32),
            ExprData::Float(_, _) => Ok(Type::F32),
            ExprData::Bool(_, _) => Ok(Type::BOOL),

            ExprData::Ident(spur, span) => {
                let var_state = env.get(spur).ok_or_else(|| {
                    TypeError::new(*span, format!("Undefined variable '{}'", interner.resolve(spur)))
                })?;

                if let Some(m_span) = var_state.moved {
                    return Err(TypeError::new(
                        *span,
                        format!(
                            "Linear Isolation Violation: use of moved variable '{}' (previously moved at {:?})",
                            interner.resolve(spur),
                            m_span
                        ),
                    ));
                }

                Ok(var_state.ty.clone())
            }

            ExprData::Binary {
                op,
                left,
                right,
                span: _,
            } => {
                let l_ty = Self::check_expr(*left, env, ctx, ast, interner)?;
                let r_ty = Self::check_expr(*right, env, ctx, ast, interner)?;

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
                    | BinOp::AddWrap | BinOp::SubWrap | BinOp::MulWrap
                    | BinOp::AddSat | BinOp::SubSat | BinOp::MulSat => {
                        if l_ty.is_numeric() && r_ty.is_numeric() {
                            Ok(l_ty)
                        } else {
                            Ok(Type::Unknown)
                        }
                    }
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                        Ok(Type::BOOL)
                    }
                    BinOp::And | BinOp::Or => Ok(Type::BOOL),
                }
            }

            ExprData::FieldAccess {
                target,
                field,
                span,
            } => {
                let target_ty = Self::check_expr(*target, env, ctx, ast, interner)?;
                match target_ty {
                    Type::Struct(s_spur) => {
                        let s_info = ctx.structs.get(&s_spur).ok_or_else(|| {
                            TypeError::new(*span, format!("Unknown struct '{}'", interner.resolve(&s_spur)))
                        })?;
                        let (f_ty, _) = s_info.fields.get(field).ok_or_else(|| {
                            TypeError::new(
                                *span,
                                format!(
                                    "Struct '{}' has no field '{}'",
                                    interner.resolve(&s_spur),
                                    interner.resolve(field)
                                ),
                            )
                        })?;
                        Ok(f_ty.clone())
                    }
                    Type::Table(elem_spur) => {
                        if let Some(s_info) = ctx.structs.get(&elem_spur) {
                            if let Some((f_ty, _)) = s_info.fields.get(field) {
                                return Ok(f_ty.clone());
                            }
                        }
                        if interner.resolve(field) == "len" {
                            Ok(Type::USIZE)
                        } else {
                            Ok(Type::F32)
                        }
                    }
                    _ => Ok(Type::Unknown),
                }
            }

            ExprData::IndexAccess {
                target,
                index,
                span: _,
            } => {
                let target_ty = Self::check_expr(*target, env, ctx, ast, interner)?;
                let _ = Self::check_expr(*index, env, ctx, ast, interner)?;
                match target_ty {
                    Type::Table(elem) => {
                        if ctx.structs.contains_key(&elem) {
                            Ok(Type::Struct(elem))
                        } else {
                            let elem_name = interner.resolve(&elem);
                            match elem_name {
                                "i64" => Ok(Type::I64),
                                "i32" => Ok(Type::I32),
                                "f32" => Ok(Type::F32),
                                "f64" => Ok(Type::F64),
                                _ => Ok(Type::Unknown),
                            }
                        }
                    }
                    _ => Ok(Type::Unknown),
                }
            }

            ExprData::Call {
                callee,
                args,
                span,
            } => {
                for (intent, arg_expr) in *args {
                    Self::check_expr(*arg_expr, env, ctx, ast, interner)?;

                    if *intent == CallIntent::Move {
                        if let ExprData::Ident(spur, a_span) = ast.get_expr(*arg_expr) {
                            let var_state = env.get_mut(spur).ok_or_else(|| {
                                TypeError::new(*a_span, format!("Undefined variable '{}'", interner.resolve(spur)))
                            })?;

                            if let Some(m_span) = var_state.moved {
                                return Err(TypeError::new(
                                    *a_span,
                                    format!(
                                        "Linear Isolation Violation: cannot move already-moved variable '{}' (moved at {:?})",
                                        interner.resolve(spur),
                                        m_span
                                    ),
                                ));
                            }

                            var_state.moved = Some(*span);
                        }
                    }
                }

                if let Some(sig) = ctx.functions.get(callee) {
                    Ok(sig.ret_type.clone())
                } else {
                    Ok(Type::Unknown)
                }
            }

            ExprData::StaticMethodCall {
                target,
                method: _,
                args,
                span: _,
            } => {
                for arg in *args {
                    Self::check_expr(*arg, env, ctx, ast, interner)?;
                }
                // Table[T]::with_capacity returns Table[T]
                Ok(Type::Table(*target))
            }

            ExprData::MethodCall {
                target,
                method: _,
                args,
                span: _,
            } => {
                let _ = Self::check_expr(*target, env, ctx, ast, interner)?;
                for arg in *args {
                    Self::check_expr(*arg, env, ctx, ast, interner)?;
                }
                Ok(Type::Unknown)
            }

            ExprData::Spawn { body, span } => {
                let mut captured_moved = HashSet::new();
                for &stmt_id in *body {
                    Self::collect_moved_vars(stmt_id, &mut captured_moved, ast);
                }

                let mut sub_env = env.clone();
                for spur in &captured_moved {
                    if let Some(var_state) = sub_env.get_mut(spur) {
                        var_state.moved = None;
                    }
                }

                let dummy_sig = FuncSignature {
                    name: Spur::default(),
                    params: Vec::new(),
                    ret_type: Type::Unknown,
                    span: *span,
                };

                for &s in *body {
                    Self::check_stmt(s, &mut sub_env, &dummy_sig, ctx, ast, interner)?;
                }

                // In parent scope: mark all variables moved into spawn as MOVED
                for spur in captured_moved {
                    if let Some(var_state) = env.get_mut(&spur) {
                        var_state.moved = Some(*span);
                    }
                }

                Ok(Type::Unknown)
            }
        }
    }

    fn collect_moved_vars<'b>(stmt_id: StmtId, captured: &mut HashSet<Spur>, ast: FrozenAst<'b, 'a>) {
        let stmt = ast.get_stmt(stmt_id);
        match stmt {
            StmtData::Return { is_move, value, .. } => {
                if *is_move {
                    if let Some(vid) = value {
                        if let ExprData::Ident(spur, _) = ast.get_expr(*vid) {
                            captured.insert(*spur);
                        }
                    }
                }
            }
            StmtData::Expr(eid) => {
                if let ExprData::Call { args, .. } = ast.get_expr(*eid) {
                    for (intent, arg_id) in *args {
                        if *intent == CallIntent::Move {
                            if let ExprData::Ident(spur, _) = ast.get_expr(*arg_id) {
                                captured.insert(*spur);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
