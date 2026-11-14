//! Axon v6.0 Static Type, Perceus Ownership, Loop Safety & Capability Verifier (`sema.rs`)

use crate::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Dec, // Exact 64-bit scaled financial decimal (10_000 scale)
    Bool,
    Str,
    Struct(String),
    Tuple(Vec<Ty>),
    Table(String),
    Result(Box<Ty>, Box<Ty>),
    Job(Box<Ty>),
    Interface(String),
    Unit,
    Any,
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub code: &'static str,
    pub msg: String,
    pub line: usize,
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
struct VarInfo {
    ty: Ty,
    mutable: bool,
    isolated: bool,
    moved_at: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct StructLayout {
    pub fields: Vec<(String, Ty)>,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<(String, ParamMode, Ty, bool)>,
    pub ret: Ty,
}

pub struct CheckedProgram {
    pub structs: HashMap<String, StructLayout>,
    pub interfaces: HashMap<String, InterfaceDecl>,
    pub fn_sigs: HashMap<String, FnSig>,
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "isize"
            | "byte"
            | "f16"
            | "bf16"
            | "f32"
            | "f64"
            | "mxfp8"
            | "mxfp4"
            | "nvfp4"
            | "dec"
            | "dec128"
            | "bool"
            | "str"
            | "void"
            | "ByteBuf"
            | "IntBuf"
            | "RootCap"
            | "FsCap"
            | "NetCap"
    )
}

fn validate_typespec(
    ts: &TypeSpec,
    line: usize,
    structs: &HashMap<String, StructLayout>,
    interfaces: &HashMap<String, InterfaceDecl>,
    errors: &mut Vec<SemanticError>,
) {
    match ts {
        TypeSpec::Unit => {}
        TypeSpec::Ref(inner) | TypeSpec::Refined(inner, _) => {
            validate_typespec(inner, line, structs, interfaces, errors);
        }
        TypeSpec::Tuple(elems) => {
            for e in elems {
                validate_typespec(e, line, structs, interfaces, errors);
            }
        }
        TypeSpec::Result(ok, err) => {
            validate_typespec(ok, line, structs, interfaces, errors);
            validate_typespec(err, line, structs, interfaces, errors);
        }
        TypeSpec::Generic(name, args) => {
            if name != "Table" && name != "Finite" && name != "Job" {
                errors.push(SemanticError {
                    code: "E0403",
                    msg: format!("Undefined generic type `{}`", name),
                    line,
                    hint: Some("Use built-in `Table[T]` or a declared struct/interface.".into()),
                });
            }
            for a in args {
                validate_typespec(a, line, structs, interfaces, errors);
            }
        }
        TypeSpec::Named(name) => {
            if !is_builtin_type_name(name)
                && !structs.contains_key(name)
                && !interfaces.contains_key(name)
            {
                errors.push(SemanticError {
                    code: "E0403",
                    msg: format!("Undefined type `{}`", name),
                    line,
                    hint: Some("Declare the struct/interface or import its module with `use`.".into()),
                });
            }
        }
    }
}

pub fn check_program(prog: &Program) -> Result<CheckedProgram, Vec<SemanticError>> {
    let mut errors = Vec::new();
    let mut structs = HashMap::new();
    let mut interfaces = HashMap::new();
    let mut fn_sigs = HashMap::new();

    for iface in &prog.interfaces {
        interfaces.insert(iface.name.clone(), iface.clone());
    }

    // Pre-register all struct names first so cross-referencing structs resolve cleanly
    for s in &prog.structs {
        structs.insert(s.name.clone(), StructLayout { fields: Vec::new() });
    }

    // 1. Validate Structs (Enforce Second-Class Reference Law: NO `&T` inside structs!)
    for s in &prog.structs {
        let mut fields = Vec::new();
        for f in &s.fields {
            if contains_ref(&f.ty) {
                errors.push(SemanticError {
                    code: "E0101",
                    msg: format!(
                        "Second-Class Reference Violation: cannot store reference `&T` in field `{}.{}`",
                        s.name, f.name
                    ),
                    line: f.line,
                    hint: Some("Store value directly, use `Table[T]` index handle, or pass `mut` parameter.".into()),
                });
            }
            validate_typespec(&f.ty, f.line, &structs, &interfaces, &mut errors);
            let fty = resolve_typespec(&f.ty, &structs, &interfaces);
            fields.push((f.name.clone(), fty));
        }
        structs.insert(s.name.clone(), StructLayout { fields });
    }

    // 2. Register Function Signatures & Check Return Reference Law (`08_return_reference.ax`)
    for f in &prog.funcs {
        if contains_ref(&f.ret_ty) {
            errors.push(SemanticError {
                code: "E0102",
                msg: format!(
                    "Reference Escape Violation: function `{}` cannot return a borrowed reference",
                    f.name
                ),
                line: f.line,
                hint: Some("Return an owned value (`-> Node`) so Perceus FBIP reuses memory in-place.".into()),
            });
        }
        validate_typespec(&f.ret_ty, f.line, &structs, &interfaces, &mut errors);
        let mut p_list = Vec::new();
        for p in &f.params {
            validate_typespec(&p.ty, p.line, &structs, &interfaces, &mut errors);
            let pty = resolve_typespec(&p.ty, &structs, &interfaces);
            p_list.push((p.name.clone(), p.mode, pty, p.default_val.is_some()));
        }
        let ret = resolve_typespec(&f.ret_ty, &structs, &interfaces);
        fn_sigs.insert(f.name.clone(), FnSig { params: p_list, ret });
    }

    // 3. Local Reasoning Function-by-Function Verification
    for f in &prog.funcs {
        let ret_ty = fn_sigs
            .get(&f.name)
            .map(|s| s.ret.clone())
            .unwrap_or(Ty::Unit);

        if ret_ty != Ty::Unit && !f.body.is_empty() && !block_always_returns(&f.body) {
            errors.push(SemanticError {
                code: "E0405",
                msg: format!(
                    "Missing return statement: function `{}` does not return `{:?}` on all control-flow paths",
                    f.name, ret_ty
                ),
                line: f.line,
                hint: Some("Ensure every branch returns a value or add a trailing `return`.".into()),
            });
        }

        let mut checker = FnChecker {
            fn_name: &f.name,
            ret_ty,
            structs: &structs,
            interfaces: &interfaces,
            fn_sigs: &fn_sigs,
            vars: HashMap::new(),
            in_spawn: false,
            outer_vars_before_spawn: HashSet::new(),
            spawn_moved_vars: HashSet::new(),
            loop_depth: 0,
            outer_vars_before_loop: HashSet::new(),
            errors: &mut errors,
        };

        for p in &f.params {
            if let Some(def_e) = &p.default_val {
                let _ = checker.check_expr(def_e);
            }
            let pty = resolve_typespec(&p.ty, &structs, &interfaces);
            let mutable = matches!(p.mode, ParamMode::Mut);
            let isolated = matches!(p.mode, ParamMode::Isolated | ParamMode::Move);
            checker.vars.insert(
                p.name.clone(),
                VarInfo {
                    ty: pty,
                    mutable,
                    isolated,
                    moved_at: None,
                },
            );
        }

        checker.check_block(&f.body);
    }

    if errors.is_empty() {
        Ok(CheckedProgram {
            structs,
            interfaces,
            fn_sigs,
        })
    } else {
        Err(errors)
    }
}

fn stmt_diverges(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::ExprStmt {
            expr: Expr::Call { callee, .. },
            ..
        } if callee == "exit_process" => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => !else_body.is_empty() && block_diverges(then_body) && block_diverges(else_body),
        Stmt::With { body, .. } => block_diverges(body),
        _ => false,
    }
}

fn block_diverges(stmts: &[Stmt]) -> bool {
    stmts.iter().any(stmt_diverges)
}

fn block_has_break(stmts: &[Stmt]) -> bool {
    for s in stmts {
        match s {
            Stmt::Break { .. } => return true,
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                if block_has_break(then_body) || block_has_break(else_body) {
                    return true;
                }
            }
            Stmt::With { body, .. } => {
                if block_has_break(body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn stmt_always_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::ExprStmt {
            expr: Expr::Call { callee, .. },
            ..
        } if callee == "exit_process" => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            !else_body.is_empty()
                && block_always_returns(then_body)
                && block_always_returns(else_body)
        }
        Stmt::With { body, .. } => block_always_returns(body),
        Stmt::While {
            cond: Expr::Bool(true, _),
            body,
            ..
        } => !block_has_break(body),
        _ => false,
    }
}

fn block_always_returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(stmt_always_returns)
}

fn eval_const_i64(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(v, _) => Some(*v),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr: inner,
            ..
        } => eval_const_i64(inner).map(|v| v.wrapping_neg()),
        Expr::Binary {
            op, left, right, ..
        } => {
            let l = eval_const_i64(left)?;
            let r = eval_const_i64(right)?;
            match op {
                BinOp::Add | BinOp::WrapAdd => Some(l.wrapping_add(r)),
                BinOp::Sub | BinOp::WrapSub => Some(l.wrapping_sub(r)),
                BinOp::Mul | BinOp::WrapMul => Some(l.wrapping_mul(r)),
                BinOp::BitAnd => Some(l & r),
                BinOp::BitOr => Some(l | r),
                BinOp::BitXor => Some(l ^ r),
                BinOp::Shl => Some(l.wrapping_shl((r & 63) as u32)),
                BinOp::Shr => Some(l.wrapping_shr((r & 63) as u32)),
                BinOp::Div if r != 0 => Some(l.wrapping_div(r)),
                BinOp::Mod if r != 0 => Some(l.wrapping_rem(r)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn contains_ref(ts: &TypeSpec) -> bool {
    match ts {
        TypeSpec::Ref(_) => true,
        TypeSpec::Generic(_, args) => args.iter().any(contains_ref),
        TypeSpec::Tuple(elems) => elems.iter().any(contains_ref),
        TypeSpec::Result(ok, err) => contains_ref(ok) || contains_ref(err),
        TypeSpec::Refined(inner, _) => contains_ref(inner),
        _ => false,
    }
}

pub fn resolve_typespec(
    ts: &TypeSpec,
    structs: &HashMap<String, StructLayout>,
    interfaces: &HashMap<String, InterfaceDecl>,
) -> Ty {
    match ts {
        TypeSpec::Unit => Ty::Unit,
        TypeSpec::Ref(inner) => resolve_typespec(inner, structs, interfaces),
        TypeSpec::Refined(inner, _) => resolve_typespec(inner, structs, interfaces),
        TypeSpec::Tuple(elems) => Ty::Tuple(
            elems
                .iter()
                .map(|e| resolve_typespec(e, structs, interfaces))
                .collect(),
        ),
        TypeSpec::Result(ok, err) => Ty::Result(
            Box::new(resolve_typespec(ok, structs, interfaces)),
            Box::new(resolve_typespec(err, structs, interfaces)),
        ),
        TypeSpec::Generic(name, args) => {
            if name == "Table" {
                let elem = args
                    .first()
                    .map(|a| match a {
                        TypeSpec::Named(n) => n.clone(),
                        _ => "i64".into(),
                    })
                    .unwrap_or_else(|| "i64".into());
                Ty::Table(elem)
            } else if name == "Finite" {
                Ty::Float
            } else {
                Ty::Any
            }
        }
        TypeSpec::Named(name) => match name.as_str() {
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize"
            | "byte" => Ty::Int,
            "f16" | "bf16" | "f32" | "f64" | "mxfp8" | "mxfp4" | "nvfp4" => Ty::Float,
            "dec" | "dec128" => Ty::Dec,
            "bool" => Ty::Bool,
            "str" => Ty::Str,
            "void" => Ty::Unit,
            other => {
                if interfaces.contains_key(other) {
                    Ty::Interface(other.to_string())
                } else {
                    Ty::Struct(other.to_string())
                }
            }
        },
    }
}

struct FnChecker<'a> {
    fn_name: &'a str,
    ret_ty: Ty,
    structs: &'a HashMap<String, StructLayout>,
    interfaces: &'a HashMap<String, InterfaceDecl>,
    fn_sigs: &'a HashMap<String, FnSig>,
    vars: HashMap<String, VarInfo>,
    in_spawn: bool,
    outer_vars_before_spawn: HashSet<String>,
    spawn_moved_vars: HashSet<String>,
    loop_depth: usize,
    outer_vars_before_loop: HashSet<String>,
    errors: &'a mut Vec<SemanticError>,
}

impl<'a> FnChecker<'a> {
    fn has_capability(&self, cap_type: &str) -> bool {
        self.vars.values().any(|v| match &v.ty {
            Ty::Struct(s) => s == cap_type || s == "RootCap",
            _ => false,
        })
    }

    fn enter_loop(&mut self) -> HashSet<String> {
        let prev = self.outer_vars_before_loop.clone();
        if self.loop_depth == 0 {
            self.outer_vars_before_loop = self.vars.keys().cloned().collect();
        }
        self.loop_depth += 1;
        prev
    }

    fn exit_loop(&mut self, prev: HashSet<String>) {
        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.outer_vars_before_loop.clear();
        } else {
            self.outer_vars_before_loop = prev;
        }
    }

    fn check_block(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name,
                kind,
                ty,
                init,
                line,
            } => {
                let init_ty = self.check_expr(init);
                let final_ty = if let Some(ts) = ty {
                    validate_typespec(ts, *line, self.structs, self.interfaces, self.errors);
                    let declared = resolve_typespec(ts, self.structs, self.interfaces);
                    if !types_compatible(
                        &declared,
                        &init_ty,
                        self.structs,
                        self.interfaces,
                        self.fn_sigs,
                    ) {
                        self.errors.push(SemanticError {
                            code: "E0301",
                            msg: format!(
                                "Type mismatch in `{}`: variable `{}` declared as `{:?}`, but initialized with `{:?}`",
                                self.fn_name, name, declared, init_ty
                            ),
                            line: *line,
                            hint: Some(format!("Use `{name} := ...` for automatic type inference or match the declared type.")),
                        });
                    }
                    declared
                } else {
                    init_ty
                };

                self.vars.insert(
                    name.clone(),
                    VarInfo {
                        ty: final_ty,
                        mutable: matches!(kind, VarKind::Mut | VarKind::Isolated),
                        isolated: matches!(kind, VarKind::Isolated),
                        moved_at: None,
                    },
                );
            }
            Stmt::TupleDestruct {
                names,
                kind,
                init,
                line: _,
            } => {
                let init_ty = self.check_expr(init);
                let elem_tys = match init_ty {
                    Ty::Tuple(v) => v,
                    _ => vec![Ty::Any; names.len()],
                };
                for (i, n) in names.iter().enumerate() {
                    let ety = elem_tys.get(i).cloned().unwrap_or(Ty::Any);
                    self.vars.insert(
                        n.clone(),
                        VarInfo {
                            ty: ety,
                            mutable: matches!(kind, VarKind::Mut),
                            isolated: false,
                            moved_at: None,
                        },
                    );
                }
            }
            Stmt::Assign {
                target,
                op: _,
                val,
                line,
            } => {
                let root_var = extract_root_ident(target);
                if let Some(vname) = root_var {
                    if let Some(vinfo) = self.vars.get(&vname).cloned() {
                        if let Some(moved_line) = vinfo.moved_at {
                            self.errors.push(SemanticError {
                                code: "E0201",
                                msg: format!(
                                    "Use of moved value `{}` (previously moved at line {})",
                                    vname, moved_line
                                ),
                                line: *line,
                                hint: None,
                            });
                        }
                        if !vinfo.mutable {
                            self.errors.push(SemanticError {
                                code: "E0104",
                                msg: format!(
                                    "Cannot mutate immutable binding `{}` in `{}`",
                                    vname, self.fn_name
                                ),
                                line: *line,
                                hint: Some(format!(
                                    "Declare `{}` as mutable: `mut {} := ...` or `mut {}: Type`.",
                                    vname, vname, vname
                                )),
                            });
                        }
                    }
                }
                let lhs_ty = self.check_expr(target);
                let rhs_ty = self.check_expr(val);
                if !types_compatible(&lhs_ty, &rhs_ty, self.structs, self.interfaces, self.fn_sigs)
                {
                    self.errors.push(SemanticError {
                        code: "E0302",
                        msg: format!(
                            "Cannot assign `{:?}` to target of type `{:?}`",
                            rhs_ty, lhs_ty
                        ),
                        line: *line,
                        hint: None,
                    });
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let _ = self.check_expr(cond);
                let saved = self.vars.clone();
                self.check_block(then_body);
                let after_then = self.vars.clone();
                let then_div = block_diverges(then_body);
                self.vars = saved.clone();
                self.check_block(else_body);
                let after_else = self.vars.clone();
                let else_div = !else_body.is_empty() && block_diverges(else_body);
                self.vars = saved;
                for (k, cur) in self.vars.iter_mut() {
                    if then_div && else_div {
                        continue;
                    }
                    if !then_div {
                        if let Some(vt) = after_then.get(k) {
                            if vt.moved_at.is_some() {
                                cur.moved_at = vt.moved_at;
                            }
                        }
                    }
                    if !else_div {
                        if let Some(ve) = after_else.get(k) {
                            if ve.moved_at.is_some() {
                                cur.moved_at = ve.moved_at;
                            }
                        }
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                let pre_moves: HashMap<String, Option<usize>> =
                    self.vars.iter().map(|(k, v)| (k.clone(), v.moved_at)).collect();
                let prev = self.enter_loop();
                let _ = self.check_expr(cond);
                let saved_keys: HashSet<String> = self.vars.keys().cloned().collect();
                self.check_block(body);
                self.vars.retain(|k, _| saved_keys.contains(k));
                self.exit_loop(prev);
                if !block_diverges(body) {
                    for (k, pre_mv) in pre_moves {
                        if pre_mv.is_none() {
                            if let Some(Some(mv_ln)) = self.vars.get(&k).map(|v| v.moved_at) {
                                self.errors.push(SemanticError {
                                    code: "E0203",
                                    msg: format!(
                                        "Linear Loop Violation: moving outer variable `{}` inside a loop in `{}` would double-move on the next iteration",
                                        k, self.fn_name
                                    ),
                                    line: mv_ln,
                                    hint: Some("Allocate or rebind the buffer inside the loop (Perceus Loop Self-FBIP reuses it in-place) or move it after the loop.".into()),
                                });
                            }
                        }
                    }
                }
            }
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                ..
            } => {
                let _ = self.check_expr(start);
                let _ = self.check_expr(end);
                let pre_moves: HashMap<String, Option<usize>> =
                    self.vars.iter().map(|(k, v)| (k.clone(), v.moved_at)).collect();
                let prev = self.enter_loop();
                let saved_var = self.vars.get(var).cloned();
                let saved_keys: HashSet<String> = self.vars.keys().cloned().collect();
                self.vars.insert(
                    var.clone(),
                    VarInfo {
                        ty: Ty::Int,
                        mutable: false,
                        isolated: false,
                        moved_at: None,
                    },
                );
                self.check_block(body);
                self.vars.retain(|k, _| saved_keys.contains(k));
                if let Some(sv) = saved_var {
                    self.vars.insert(var.clone(), sv);
                }
                self.exit_loop(prev);
                if !block_diverges(body) {
                    for (k, pre_mv) in pre_moves {
                        if pre_mv.is_none() {
                            if let Some(Some(mv_ln)) = self.vars.get(&k).map(|v| v.moved_at) {
                                self.errors.push(SemanticError {
                                    code: "E0203",
                                    msg: format!(
                                        "Linear Loop Violation: moving outer variable `{}` inside a `for` loop in `{}` would double-move on the next iteration",
                                        k, self.fn_name
                                    ),
                                    line: mv_ln,
                                    hint: Some("Allocate or rebind inside the loop or move after the loop.".into()),
                                });
                            }
                        }
                    }
                }
            }
            Stmt::ForEach {
                var,
                collection,
                body,
                ..
            } => {
                let col_ty = self.check_expr(collection);
                let elem_ty = match col_ty {
                    Ty::Table(s) => Ty::Struct(s),
                    _ => Ty::Any,
                };
                let pre_moves: HashMap<String, Option<usize>> =
                    self.vars.iter().map(|(k, v)| (k.clone(), v.moved_at)).collect();
                let prev = self.enter_loop();
                let saved_var = self.vars.get(var).cloned();
                let saved_keys: HashSet<String> = self.vars.keys().cloned().collect();
                self.vars.insert(
                    var.clone(),
                    VarInfo {
                        ty: elem_ty,
                        mutable: true,
                        isolated: false,
                        moved_at: None,
                    },
                );
                self.check_block(body);
                self.vars.retain(|k, _| saved_keys.contains(k));
                if let Some(sv) = saved_var {
                    self.vars.insert(var.clone(), sv);
                }
                self.exit_loop(prev);
                if !block_diverges(body) {
                    for (k, pre_mv) in pre_moves {
                        if pre_mv.is_none() {
                            if let Some(Some(mv_ln)) = self.vars.get(&k).map(|v| v.moved_at) {
                                self.errors.push(SemanticError {
                                    code: "E0203",
                                    msg: format!(
                                        "Linear Loop Violation: moving outer variable `{}` inside a `for` loop in `{}` would double-move on the next iteration",
                                        k, self.fn_name
                                    ),
                                    line: mv_ln,
                                    hint: None,
                                });
                            }
                        }
                    }
                }
            }
            Stmt::With {
                var, init, body, ..
            } => {
                let ity = self.check_expr(init);
                let saved_var = self.vars.get(var).cloned();
                let saved_keys: HashSet<String> = self.vars.keys().cloned().collect();
                self.vars.insert(
                    var.clone(),
                    VarInfo {
                        ty: ity,
                        mutable: true,
                        isolated: false,
                        moved_at: None,
                    },
                );
                self.check_block(body);
                self.vars.retain(|k, _| saved_keys.contains(k));
                if let Some(sv) = saved_var {
                    self.vars.insert(var.clone(), sv);
                }
            }
            Stmt::Return { val, line } => {
                if self.in_spawn {
                    if let Some(e) = val {
                        let _ = self.check_expr(e);
                    }
                    return;
                }
                let actual = match val {
                    Some(e) => self.check_expr(e),
                    None => Ty::Unit,
                };
                if !types_compatible(
                    &self.ret_ty,
                    &actual,
                    self.structs,
                    self.interfaces,
                    self.fn_sigs,
                ) {
                    self.errors.push(SemanticError {
                        code: "E0303",
                        msg: format!(
                            "Return type mismatch in `{}`: expected `{:?}`, found `{:?}`",
                            self.fn_name, self.ret_ty, actual
                        ),
                        line: *line,
                        hint: None,
                    });
                }
            }
            Stmt::Break { line } => {
                if self.loop_depth == 0 {
                    self.errors.push(SemanticError {
                        code: "E0407",
                        msg: format!("`break` used outside of loop in `{}`", self.fn_name),
                        line: *line,
                        hint: Some("Place `break` inside a `while` or `for` loop.".into()),
                    });
                }
            }
            Stmt::Continue { line } => {
                if self.loop_depth == 0 {
                    self.errors.push(SemanticError {
                        code: "E0407",
                        msg: format!("`continue` used outside of loop in `{}`", self.fn_name),
                        line: *line,
                        hint: Some("Place `continue` inside a `while` or `for` loop.".into()),
                    });
                }
            }
            Stmt::ExprStmt { expr, .. } => {
                let _ = self.check_expr(expr);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Ty {
        match expr {
            Expr::Int(_, _) => Ty::Int,
            Expr::Float(_, _) => Ty::Float,
            Expr::Dec(_, _) => Ty::Dec,
            Expr::Bool(_, _) => Ty::Bool,
            Expr::Str(_, _) | Expr::SecretStr(_, _) => Ty::Str,
            Expr::FStr(parts, _) => {
                for p in parts {
                    if let FStrPart::Expr(e) = p {
                        let _ = self.check_expr(e);
                    }
                }
                Ty::Str
            }
            Expr::Ident(name, line) => {
                if let Some(v) = self.vars.get(name).cloned() {
                    if let Some(moved_line) = v.moved_at {
                        self.errors.push(SemanticError {
                            code: "E0201",
                            msg: format!(
                                "Linear Ownership Violation: use of `{}` after it was moved at line {}",
                                name, moved_line
                            ),
                            line: *line,
                            hint: Some("Values moved into a fiber or `move` parameter cannot be accessed again.".into()),
                        });
                    }
                    if self.in_spawn
                        && self.outer_vars_before_spawn.contains(name)
                        && v.isolated
                        && !self.spawn_moved_vars.contains(name)
                    {
                        self.errors.push(SemanticError {
                            code: "E0202",
                            msg: format!(
                                "Colorless Concurrency Violation: capturing isolated `{}` inside `spawn` without `move`",
                                name
                            ),
                            line: *line,
                            hint: Some(format!("Transfer linear ownership into the fiber using `move {}`.", name)),
                        });
                    }
                    v.ty
                } else {
                    self.errors.push(SemanticError {
                        code: "E0403",
                        msg: format!("Undefined identifier `{}` in `{}`", name, self.fn_name),
                        line: *line,
                        hint: Some(format!("Declare `{}` with `{} := ...` before using it.", name, name)),
                    });
                    Ty::Any
                }
            }
            Expr::MoveIdent(name, line) => {
                if let Some(v) = self.vars.get_mut(name) {
                    if let Some(moved_line) = v.moved_at {
                        self.errors.push(SemanticError {
                            code: "E0201",
                            msg: format!(
                                "Linear Ownership Violation: `{}` was already moved at line {}",
                                name, moved_line
                            ),
                            line: *line,
                            hint: None,
                        });
                    } else {
                        v.moved_at = Some(*line);
                    }
                    if self.in_spawn {
                        self.spawn_moved_vars.insert(name.clone());
                    }
                    v.ty.clone()
                } else {
                    self.errors.push(SemanticError {
                        code: "E0403",
                        msg: format!("Undefined identifier `{}` in `move` expression", name),
                        line: *line,
                        hint: None,
                    });
                    Ty::Any
                }
            }
            Expr::Tuple(elems, _) => Ty::Tuple(elems.iter().map(|e| self.check_expr(e)).collect()),
            Expr::Unary {
                op, expr: inner, ..
            } => {
                let t = self.check_expr(inner);
                match op {
                    UnaryOp::Neg => t,
                    UnaryOp::Not => Ty::Bool,
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                line,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                if matches!(op, BinOp::Div | BinOp::Mod) && eval_const_i64(right) == Some(0) {
                    self.errors.push(SemanticError {
                        code: "E0406",
                        msg: format!(
                            "Constant Arithmetic Violation: division or modulo by zero in `{}`",
                            self.fn_name
                        ),
                        line: *line,
                        hint: Some("Use a non-zero divisor.".into()),
                    });
                }
                if *op == BinOp::Or {
                    if let Ty::Result(ok_ty, _) = lt {
                        return *ok_ty;
                    }
                    return Ty::Bool;
                }
                match op {
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::And => Ty::Bool,
                    _ => {
                        if lt == Ty::Dec || rt == Ty::Dec {
                            Ty::Dec
                        } else if lt == Ty::Float || rt == Ty::Float {
                            Ty::Float
                        } else {
                            Ty::Int
                        }
                    }
                }
            }
            Expr::OrReturn { lhs, ret_val, .. } => {
                let lt = self.check_expr(lhs);
                if let Some(rv) = ret_val {
                    let _ = self.check_expr(rv);
                }
                if let Ty::Result(ok_ty, _) = lt {
                    *ok_ty
                } else {
                    lt
                }
            }
            Expr::Cast {
                expr: inner,
                ty,
                line,
            } => {
                let _ = self.check_expr(inner);
                validate_typespec(ty, *line, self.structs, self.interfaces, self.errors);
                resolve_typespec(ty, self.structs, self.interfaces)
            }
            Expr::Field { base, field, line } => {
                let bt = self.check_expr(base);
                if field == "len" {
                    return Ty::Int;
                }
                match bt {
                    Ty::Any => Ty::Any,
                    Ty::Struct(sname) => {
                        if let Some(layout) = self.structs.get(&sname) {
                            for (fname, fty) in &layout.fields {
                                if fname == field {
                                    return fty.clone();
                                }
                            }
                            self.errors.push(SemanticError {
                                code: "E0403",
                                msg: format!("Struct `{}` has no field `{}`", sname, field),
                                line: *line,
                                hint: None,
                            });
                            Ty::Any
                        } else {
                            self.errors.push(SemanticError {
                                code: "E0403",
                                msg: format!("Cannot access field `{}` on `{}`", field, sname),
                                line: *line,
                                hint: None,
                            });
                            Ty::Any
                        }
                    }
                    other => {
                        self.errors.push(SemanticError {
                            code: "E0403",
                            msg: format!("Type `{:?}` has no field `{}`", other, field),
                            line: *line,
                            hint: None,
                        });
                        Ty::Any
                    }
                }
            }
            Expr::TupleIdx { base, idx, .. } => {
                let bt = self.check_expr(base);
                if let Ty::Tuple(elems) = bt {
                    elems.get(*idx).cloned().unwrap_or(Ty::Any)
                } else {
                    Ty::Any
                }
            }
            Expr::Index { base, idx, .. } => {
                let bt = self.check_expr(base);
                let _ = self.check_expr(idx);
                match bt {
                    Ty::Table(elem) => {
                        if elem == "i64" || elem == "i32" || elem == "usize" {
                            Ty::Int
                        } else if elem == "f32" || elem == "f64" {
                            Ty::Float
                        } else {
                            Ty::Struct(elem)
                        }
                    }
                    _ => Ty::Int,
                }
            }
            Expr::TableAlloc {
                elem_ty, cap, line, ..
            } => {
                if !is_builtin_type_name(elem_ty) && !self.structs.contains_key(elem_ty) {
                    self.errors.push(SemanticError {
                        code: "E0403",
                        msg: format!("Undefined element type `{}` in `Table[{}]`", elem_ty, elem_ty),
                        line: *line,
                        hint: None,
                    });
                }
                let _ = self.check_expr(cap);
                Ty::Table(elem_ty.clone())
            }
            Expr::Call {
                callee, args, line, ..
            } => {
                // Object-Capability Security Enforcement on Privileged Raw Intrinsics
                if (callee == "sys_fs_write" || callee == "sys_fs_read")
                    && !self.has_capability("FsCap")
                {
                    self.errors.push(SemanticError {
                        code: "E0401",
                        msg: format!(
                            "Object-Capability Security Violation: `{}` calls `{}` without holding an `FsCap` or `RootCap` token",
                            self.fn_name, callee
                        ),
                        line: *line,
                        hint: Some("Pass `cap: FsCap` from `main(root: RootCap)` to authorize filesystem access.".into()),
                    });
                }
                if callee == "sys_net_connect" && !self.has_capability("NetCap") {
                    self.errors.push(SemanticError {
                        code: "E0402",
                        msg: format!(
                            "Object-Capability Security Violation: `{}` calls `{}` without holding a `NetCap` or `RootCap` token",
                            self.fn_name, callee
                        ),
                        line: *line,
                        hint: Some("Pass `net: NetCap` attenuated via `net_allow_host(...)` to authorize network access.".into()),
                    });
                }

                // Built-in constructors & intrinsics with arity verification
                if let Some((expected_arity, ret_ty)) = self.check_builtin_call(callee, args, *line)
                {
                    if let Some(exp) = expected_arity {
                        if args.len() != exp {
                            self.errors.push(SemanticError {
                                code: "E0404",
                                msg: format!(
                                    "Intrinsic `{}` expects {} argument(s), found {}",
                                    callee,
                                    exp,
                                    args.len()
                                ),
                                line: *line,
                                hint: None,
                            });
                        }
                    }
                    return ret_ty;
                }

                // Check struct constructor `StructName(...)`
                if let Some(layout) = self.structs.get(callee).cloned() {
                    if args.len() > layout.fields.len() {
                        self.errors.push(SemanticError {
                            code: "E0404",
                            msg: format!(
                                "Struct constructor `{}` expects {} field(s), found {}",
                                callee,
                                layout.fields.len(),
                                args.len()
                            ),
                            line: *line,
                            hint: None,
                        });
                    }
                    let mut filled = vec![false; layout.fields.len()];
                    for (idx, a) in args.iter().enumerate() {
                        let arg_ty = if a.mode == ArgPassMode::Move {
                            if let Expr::Ident(vname, ln) = &a.expr {
                                self.check_expr(&Expr::MoveIdent(vname.clone(), *ln))
                            } else {
                                self.check_expr(&a.expr)
                            }
                        } else {
                            self.check_expr(&a.expr)
                        };
                        let fidx_opt = if let Some(aname) = &a.name {
                            let found = layout.fields.iter().position(|(fnm, _)| fnm == aname);
                            if found.is_none() {
                                self.errors.push(SemanticError {
                                    code: "E0404",
                                    msg: format!(
                                        "Unknown field `{}` in constructor `{}`",
                                        aname, callee
                                    ),
                                    line: *line,
                                    hint: None,
                                });
                            }
                            found
                        } else if idx < layout.fields.len() {
                            Some(idx)
                        } else {
                            None
                        };
                        if let Some(fidx) = fidx_opt {
                            filled[fidx] = true;
                            let expected_ty = &layout.fields[fidx].1;
                            if !types_compatible(
                                expected_ty,
                                &arg_ty,
                                self.structs,
                                self.interfaces,
                                self.fn_sigs,
                            ) {
                                self.errors.push(SemanticError {
                                    code: "E0304",
                                    msg: format!(
                                        "Field `{}` of `{}` expected `{:?}`, found `{:?}`",
                                        layout.fields[fidx].0, callee, expected_ty, arg_ty
                                    ),
                                    line: *line,
                                    hint: None,
                                });
                            }
                        }
                    }
                    if filled.iter().any(|&ok| !ok) && args.len() < layout.fields.len() {
                        self.errors.push(SemanticError {
                            code: "E0404",
                            msg: format!(
                                "Missing field argument(s) in struct constructor `{}` (expected {}, found {})",
                                callee,
                                layout.fields.len(),
                                args.len()
                            ),
                            line: *line,
                            hint: None,
                        });
                    }
                    return Ty::Struct(callee.clone());
                }

                // Evaluate arguments once while tracking types & pass modes
                let mut arg_tys = Vec::with_capacity(args.len());
                for a in args {
                    if a.mode == ArgPassMode::Mut {
                        if let Some(vname) = extract_root_ident(&a.expr) {
                            if self.in_spawn
                                && self.outer_vars_before_spawn.contains(&vname)
                                && !self.spawn_moved_vars.contains(&vname)
                            {
                                self.errors.push(SemanticError {
                                    code: "E0202",
                                    msg: format!(
                                        "Colorless Concurrency Violation: mutating outer `{}` inside `spawn` without moving ownership",
                                        vname
                                    ),
                                    line: *line,
                                    hint: Some(format!("Move `{}` into the `spawn` block explicitly.", vname)),
                                });
                            }
                            if let Some(vi) = self.vars.get(&vname) {
                                if !vi.mutable {
                                    self.errors.push(SemanticError {
                                        code: "E0104",
                                        msg: format!(
                                            "Cannot pass immutable binding `{}` as `mut` argument to `{}`",
                                            vname, callee
                                        ),
                                        line: *line,
                                        hint: Some(format!("Declare `mut {} := ...`.", vname)),
                                    });
                                }
                            }
                        } else {
                            self.errors.push(SemanticError {
                                code: "E0404",
                                msg: format!("`mut` argument to `{}` must be a mutable variable or field", callee),
                                line: *line,
                                hint: None,
                            });
                        }
                    }
                    let aty = if a.mode == ArgPassMode::Move {
                        if let Expr::Ident(vname, ln) = &a.expr {
                            self.check_expr(&Expr::MoveIdent(vname.clone(), *ln))
                        } else {
                            self.check_expr(&a.expr)
                        }
                    } else {
                        self.check_expr(&a.expr)
                    };
                    arg_tys.push(aty);
                }

                let mut sig_opt = self.fn_sigs.get(callee).cloned();
                if sig_opt.is_none() && !arg_tys.is_empty() {
                    if let Ty::Struct(sname) = &arg_tys[0] {
                        let mangled = format!("{}__{}", sname, callee);
                        sig_opt = self.fn_sigs.get(&mangled).cloned();
                    }
                }

                let Some(sig) = sig_opt else {
                    self.errors.push(SemanticError {
                        code: "E0403",
                        msg: format!("Undefined function `{}` called in `{}`", callee, self.fn_name),
                        line: *line,
                        hint: Some("Check spelling or import the module with `use`.".into()),
                    });
                    return Ty::Any;
                };

                if args.len() > sig.params.len() {
                    self.errors.push(SemanticError {
                        code: "E0404",
                        msg: format!(
                            "Too many arguments to `{}`: expected at most {}, found {}",
                            callee,
                            sig.params.len(),
                            args.len()
                        ),
                        line: *line,
                        hint: None,
                    });
                }

                let mut supplied = vec![false; sig.params.len()];
                for (idx, a) in args.iter().enumerate() {
                    let target_opt = if let Some(aname) = &a.name {
                        let found = sig.params.iter().position(|(pn, _, _, _)| pn == aname);
                        if found.is_none() {
                            self.errors.push(SemanticError {
                                code: "E0404",
                                msg: format!("Unknown named argument `{}` in call to `{}`", aname, callee),
                                line: *line,
                                hint: None,
                            });
                        }
                        found
                    } else if idx < sig.params.len() {
                        Some(idx)
                    } else {
                        None
                    };

                    if let Some(target_idx) = target_opt {
                        supplied[target_idx] = true;
                        let (pname, pmode, expected_ty, _) = &sig.params[target_idx];
                        if *pmode == ParamMode::Mut && a.mode != ArgPassMode::Mut {
                            self.errors.push(SemanticError {
                                code: "E0404",
                                msg: format!(
                                    "Parameter `{}` of `{}` requires explicit `mut` pass mode at call site",
                                    pname, callee
                                ),
                                line: *line,
                                hint: Some("Pass the argument with `mut x`.".into()),
                            });
                        } else if *pmode == ParamMode::Move && a.mode != ArgPassMode::Move {
                            // Note: `Expr::MoveIdent` parsed inside arg expression also counts as move
                            if !matches!(a.expr, Expr::MoveIdent(_, _)) {
                                self.errors.push(SemanticError {
                                    code: "E0404",
                                    msg: format!(
                                        "Parameter `{}` of `{}` requires explicit `move` ownership transfer",
                                        pname, callee
                                    ),
                                    line: *line,
                                    hint: Some("Pass the argument with `move x`.".into()),
                                });
                            }
                        } else if matches!(pmode, ParamMode::Immutable | ParamMode::LetExplicit)
                            && a.mode == ArgPassMode::Mut
                        {
                            self.errors.push(SemanticError {
                                code: "E0404",
                                msg: format!(
                                    "Argument for immutable parameter `{}` of `{}` cannot be passed as `mut`",
                                    pname, callee
                                ),
                                line: *line,
                                hint: None,
                            });
                        }
                        if !types_compatible(
                            expected_ty,
                            &arg_tys[idx],
                            self.structs,
                            self.interfaces,
                            self.fn_sigs,
                        ) {
                            self.errors.push(SemanticError {
                                code: "E0304",
                                msg: format!(
                                    "Argument `{}` to `{}` expected `{:?}`, found `{:?}`",
                                    pname, callee, expected_ty, arg_tys[idx]
                                ),
                                line: *line,
                                hint: None,
                            });
                        }
                    }
                }

                for (pi, (pname, _, _, has_default)) in sig.params.iter().enumerate() {
                    if !supplied[pi] && !has_default {
                        self.errors.push(SemanticError {
                            code: "E0404",
                            msg: format!(
                                "Missing required argument `{}` in call to `{}`",
                                pname, callee
                            ),
                            line: *line,
                            hint: None,
                        });
                    }
                }

                sig.ret
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                line,
            } => {
                let rty = self.check_expr(receiver);
                let mut arg_tys = Vec::with_capacity(args.len());
                for a in args {
                    if a.mode == ArgPassMode::Move {
                        if let Expr::Ident(vname, ln) = &a.expr {
                            arg_tys.push(self.check_expr(&Expr::MoveIdent(vname.clone(), *ln)));
                        } else {
                            arg_tys.push(self.check_expr(&a.expr));
                        }
                    } else {
                        arg_tys.push(self.check_expr(&a.expr));
                    }
                }
                if method == "join" {
                    if let Ty::Job(inner) = rty {
                        return *inner;
                    }
                }
                match rty {
                    Ty::Any => Ty::Any,
                    Ty::Struct(sname) => {
                        let mangled = format!("{}__{}", sname, method);
                        if let Some(sig) = self
                            .fn_sigs
                            .get(&mangled)
                            .or_else(|| self.fn_sigs.get(method))
                            .cloned()
                        {
                            let expected_extra = sig.params.len().saturating_sub(1);
                            let min_extra = sig
                                .params
                                .iter()
                                .skip(1)
                                .filter(|(_, _, _, has_def)| !has_def)
                                .count();
                            if args.len() < min_extra || args.len() > expected_extra {
                                self.errors.push(SemanticError {
                                    code: "E0404",
                                    msg: format!(
                                        "Method `{}.{}` expected {}..={} argument(s), found {}",
                                        sname,
                                        method,
                                        min_extra,
                                        expected_extra,
                                        args.len()
                                    ),
                                    line: *line,
                                    hint: None,
                                });
                            }
                            return sig.ret;
                        }
                        self.errors.push(SemanticError {
                            code: "E0403",
                            msg: format!("Undefined method `{}` on struct `{}`", method, sname),
                            line: *line,
                            hint: None,
                        });
                        Ty::Any
                    }
                    Ty::Interface(iname) => {
                        if let Some(iface) = self.interfaces.get(&iname) {
                            if let Some(msig) = iface.methods.iter().find(|m| m.name == *method) {
                                return resolve_typespec(&msig.ret_ty, self.structs, self.interfaces);
                            }
                        }
                        self.errors.push(SemanticError {
                            code: "E0403",
                            msg: format!("Undefined method `{}` on interface `{}`", method, iname),
                            line: *line,
                            hint: None,
                        });
                        Ty::Any
                    }
                    other => {
                        self.errors.push(SemanticError {
                            code: "E0403",
                            msg: format!("Undefined method `{}` on type `{:?}`", method, other),
                            line: *line,
                            hint: None,
                        });
                        Ty::Any
                    }
                }
            }
            Expr::IfExpr {
                cond,
                then_body,
                then_tail,
                else_body,
                else_tail,
                ..
            } => {
                let _ = self.check_expr(cond);
                self.check_block(then_body);
                let t1 = self.check_expr(then_tail);
                self.check_block(else_body);
                let _t2 = self.check_expr(else_tail);
                t1
            }
            Expr::Spawn { body, .. } => {
                let prev_in_spawn = self.in_spawn;
                let prev_outer = self.outer_vars_before_spawn.clone();
                let prev_moved = self.spawn_moved_vars.clone();

                self.in_spawn = true;
                self.outer_vars_before_spawn = self.vars.keys().cloned().collect();
                let mut explicit_moves = HashSet::new();
                collect_explicit_moves_in_stmts(body, &mut explicit_moves);
                self.spawn_moved_vars = explicit_moves;

                self.check_block(body);

                self.in_spawn = prev_in_spawn;
                self.outer_vars_before_spawn = prev_outer;
                self.spawn_moved_vars = prev_moved;
                Ty::Job(Box::new(Ty::Any))
            }
        }
    }

    fn check_builtin_call(
        &mut self,
        callee: &str,
        args: &[CallArg],
        _line: usize,
    ) -> Option<(Option<usize>, Ty)> {
        let check_all_args = |this: &mut Self| {
            for a in args {
                if a.mode == ArgPassMode::Move {
                    if let Expr::Ident(vname, ln) = &a.expr {
                        let _ = this.check_expr(&Expr::MoveIdent(vname.clone(), *ln));
                        continue;
                    }
                }
                let _ = this.check_expr(&a.expr);
            }
        };

        match callee {
            "ok" => {
                let inner = args
                    .first()
                    .map(|a| self.check_expr(&a.expr))
                    .unwrap_or(Ty::Int);
                Some((
                    Some(1),
                    Ty::Result(Box::new(inner), Box::new(Ty::Int)),
                ))
            }
            "err" => {
                let e = args
                    .first()
                    .map(|a| self.check_expr(&a.expr))
                    .unwrap_or(Ty::Int);
                Some((Some(1), Ty::Result(Box::new(Ty::Any), Box::new(e))))
            }
            "print" | "println" => {
                check_all_args(self);
                Some((None, Ty::Unit))
            }
            "clock_ns" | "time_ns" | "args_count" => {
                check_all_args(self);
                Some((Some(0), Ty::Int))
            }
            "buf_new" => {
                check_all_args(self);
                Some((Some(1), Ty::Struct("ByteBuf".into())))
            }
            "ibuf_new" => {
                check_all_args(self);
                Some((Some(1), Ty::Struct("IntBuf".into())))
            }
            "buf_free" | "exit_process" => {
                check_all_args(self);
                Some((Some(1), Ty::Unit))
            }
            "buf_push" | "buf_push_u16" | "buf_push_u32" | "buf_push_u64" | "buf_push_str"
            | "ibuf_push" => {
                check_all_args(self);
                Some((Some(2), Ty::Unit))
            }
            "buf_set" | "buf_put_u32" | "buf_put_u64" | "ibuf_set" | "table_fill_f32" => {
                check_all_args(self);
                Some((Some(3), Ty::Unit))
            }
            "table_get_f32" => {
                check_all_args(self);
                Some((Some(3), Ty::Float))
            }
            "buf_len" | "ibuf_len" | "str_len" => {
                check_all_args(self);
                Some((Some(1), Ty::Int))
            }
            "buf_get" | "ibuf_get" | "str_byte" | "whitebox_sbox" | "ushr64" => {
                check_all_args(self);
                Some((Some(2), Ty::Int))
            }
            "str_eq" => {
                check_all_args(self);
                Some((Some(2), Ty::Bool))
            }
            "args_get" => {
                check_all_args(self);
                Some((Some(1), Ty::Str))
            }
            "str_slice" | "buf_slice_str" => {
                check_all_args(self);
                Some((Some(3), Ty::Str))
            }
            "fs_read_file" | "sys_fs_read" => {
                check_all_args(self);
                Some((
                    Some(1),
                    Ty::Result(Box::new(Ty::Struct("ByteBuf".into())), Box::new(Ty::Int)),
                ))
            }
            "fs_write_file" | "sys_fs_write" => {
                check_all_args(self);
                Some((
                    Some(2),
                    Ty::Result(Box::new(Ty::Int), Box::new(Ty::Int)),
                ))
            }
            "sys_net_connect" => {
                check_all_args(self);
                Some((
                    Some(2),
                    Ty::Result(Box::new(Ty::Int), Box::new(Ty::Int)),
                ))
            }
            _ => None,
        }
    }
}

fn collect_explicit_moves_in_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Stmt::Return { val: Some(e), .. } | Stmt::ExprStmt { expr: e, .. } => {
                collect_explicit_moves_in_expr(e, out);
            }
            Stmt::VarDecl { init, .. } => collect_explicit_moves_in_expr(init, out),
            _ => {}
        }
    }
}

fn collect_explicit_moves_in_expr(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::MoveIdent(name, _) => {
            out.insert(name.clone());
        }
        Expr::Call { args, .. } => {
            for a in args {
                if a.mode == ArgPassMode::Move {
                    if let Expr::Ident(n, _) = &a.expr {
                        out.insert(n.clone());
                    }
                }
                collect_explicit_moves_in_expr(&a.expr, out);
            }
        }
        _ => {}
    }
}

fn extract_root_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::Ident(n, _) => Some(n.clone()),
        Expr::Field { base, .. } | Expr::Index { base, .. } | Expr::TupleIdx { base, .. } => {
            extract_root_ident(base)
        }
        _ => None,
    }
}

fn types_compatible(
    expected: &Ty,
    actual: &Ty,
    _structs: &HashMap<String, StructLayout>,
    interfaces: &HashMap<String, InterfaceDecl>,
    fn_sigs: &HashMap<String, FnSig>,
) -> bool {
    if expected == actual || *expected == Ty::Any || *actual == Ty::Any {
        return true;
    }
    match (expected, actual) {
        (Ty::Result(ok1, err1), Ty::Result(ok2, err2)) => {
            types_compatible(ok1, ok2, _structs, interfaces, fn_sigs)
                && types_compatible(err1, err2, _structs, interfaces, fn_sigs)
        }
        (Ty::Interface(iname), Ty::Struct(sname)) => {
            if let Some(iface) = interfaces.get(iname) {
                iface.methods.iter().all(|m| {
                    let impl_method = format!("{}__{}", sname, m.name);
                    fn_sigs.contains_key(&impl_method) || fn_sigs.contains_key(&m.name)
                })
            } else {
                true
            }
        }
        _ => false,
    }
}
