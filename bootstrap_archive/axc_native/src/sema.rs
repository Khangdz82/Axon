//! Axon v5.0 Static Type, Perceus Ownership, Duck-Interface & Capability Security Verifier (`sema.rs`)

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

pub fn check_program(prog: &Program) -> Result<CheckedProgram, Vec<SemanticError>> {
    let mut errors = Vec::new();
    let mut structs = HashMap::new();
    let mut interfaces = HashMap::new();
    let mut fn_sigs = HashMap::new();

    for iface in &prog.interfaces {
        interfaces.insert(iface.name.clone(), iface.clone());
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
            let fty = resolve_typespec(&f.ty, &structs, &interfaces);
            fields.push((f.name.clone(), fty));
        }
        structs.insert(s.name.clone(), StructLayout { fields });
    }

    // Re-resolve struct field types now that all struct names are registered
    for s in &prog.structs {
        let mut fields = Vec::new();
        for f in &s.fields {
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
                hint: Some("Return an owned value (`-> Node`) — Perceus FBIP reuses memory in-place with zero copy!".into()),
            });
        }
        let mut p_list = Vec::new();
        for p in &f.params {
            let pty = resolve_typespec(&p.ty, &structs, &interfaces);
            p_list.push((p.name.clone(), p.mode, pty, p.default_val.is_some()));
        }
        let ret = resolve_typespec(&f.ret_ty, &structs, &interfaces);
        fn_sigs.insert(f.name.clone(), FnSig { params: p_list, ret });
    }

    // 3. Local Reasoning Function-by-Function Verification
    for f in &prog.funcs {
        let mut checker = FnChecker {
            fn_name: &f.name,
            ret_ty: fn_sigs.get(&f.name).map(|s| s.ret.clone()).unwrap_or(Ty::Unit),
            structs: &structs,
            interfaces: &interfaces,
            fn_sigs: &fn_sigs,
            vars: HashMap::new(),
            in_spawn: false,
            outer_vars_before_spawn: HashSet::new(),
            spawn_moved_vars: HashSet::new(),
            errors: &mut errors,
        };

        for p in &f.params {
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
                } else if structs.contains_key(other) {
                    Ty::Struct(other.to_string())
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
    errors: &'a mut Vec<SemanticError>,
}

impl<'a> FnChecker<'a> {
    fn has_capability(&self, cap_type: &str) -> bool {
        self.vars.values().any(|v| match &v.ty {
            Ty::Struct(s) => s == cap_type || s == "RootCap",
            _ => false,
        })
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
                    let declared = resolve_typespec(ts, self.structs, self.interfaces);
                    if !types_compatible(&declared, &init_ty, self.structs, self.interfaces, self.fn_sigs) {
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
                if !types_compatible(&lhs_ty, &rhs_ty, self.structs, self.interfaces, self.fn_sigs) {
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
                self.vars = saved;
                self.check_block(else_body);
                for (k, v) in after_then {
                    if v.moved_at.is_some() {
                        if let Some(cur) = self.vars.get_mut(&k) {
                            cur.moved_at = v.moved_at;
                        }
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                let _ = self.check_expr(cond);
                self.check_block(body);
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
            }
            Stmt::With {
                var, init, body, ..
            } => {
                let ity = self.check_expr(init);
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
                if !types_compatible(&self.ret_ty, &actual, self.structs, self.interfaces, self.fn_sigs) {
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
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
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
                    Ty::Any
                }
            }
            Expr::Tuple(elems, _) => {
                Ty::Tuple(elems.iter().map(|e| self.check_expr(e)).collect())
            }
            Expr::Unary { op, expr: inner, .. } => {
                let t = self.check_expr(inner);
                match op {
                    UnaryOp::Neg => t,
                    UnaryOp::Not => Ty::Bool,
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                // Result fallback: `res or fallback_value`!
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
            Expr::Cast { expr: inner, ty, .. } => {
                let _ = self.check_expr(inner);
                resolve_typespec(ty, self.structs, self.interfaces)
            }
            Expr::Field { base, field, .. } => {
                let bt = self.check_expr(base);
                if field == "len" {
                    return Ty::Int;
                }
                if let Ty::Struct(sname) = bt {
                    if let Some(layout) = self.structs.get(&sname) {
                        for (fname, fty) in &layout.fields {
                            if fname == field {
                                return fty.clone();
                            }
                        }
                    }
                }
                Ty::Any
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
            Expr::TableAlloc { elem_ty, cap, .. } => {
                let _ = self.check_expr(cap);
                Ty::Table(elem_ty.clone())
            }
            Expr::Call { callee, args, line, .. } => {
                // Object-Capability Security Enforcement on Privileged Raw Intrinsics!
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

                // Built-in constructors & intrinsics
                if callee == "ok" {
                    let inner = args.first().map(|a| self.check_expr(&a.expr)).unwrap_or(Ty::Int);
                    return Ty::Result(Box::new(inner), Box::new(Ty::Int));
                }
                if callee == "err" {
                    let e = args.first().map(|a| self.check_expr(&a.expr)).unwrap_or(Ty::Int);
                    return Ty::Result(Box::new(Ty::Any), Box::new(e));
                }
                if callee == "print" || callee == "println" {
                    for a in args {
                        let _ = self.check_expr(&a.expr);
                    }
                    return Ty::Unit;
                }
                if callee == "clock_ns" || callee == "time_ns" {
                    return Ty::Int;
                }
                if callee == "buf_new" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Struct("ByteBuf".into());
                }
                if callee == "ibuf_new" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Struct("IntBuf".into());
                }
                if matches!(
                    callee.as_str(),
                    "buf_push" | "buf_push_u16" | "buf_push_u32" | "buf_push_u64"
                    | "buf_push_str" | "buf_set" | "buf_put_u32" | "buf_put_u64"
                    | "buf_free" | "ibuf_push" | "ibuf_set" | "table_fill_f32"
                ) {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Unit;
                }
                if callee == "table_get_f32" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Float;
                }
                if matches!(
                    callee.as_str(),
                    "buf_get" | "buf_len" | "ibuf_get" | "ibuf_len"
                    | "str_byte" | "str_len" | "args_count" | "whitebox_sbox"
                ) {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Int;
                }
                if callee == "str_eq" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Bool;
                }
                if matches!(callee.as_str(), "str_slice" | "buf_slice_str" | "args_get") {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Str;
                }
                if callee == "fs_read_file" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Result(Box::new(Ty::Struct("ByteBuf".into())), Box::new(Ty::Int));
                }
                if callee == "fs_write_file" {
                    for a in args { let _ = self.check_expr(&a.expr); }
                    return Ty::Result(Box::new(Ty::Int), Box::new(Ty::Int));
                }

                // Check struct constructor `StructName(...)`
                if self.structs.contains_key(callee) {
                    for a in args {
                        let _ = self.check_expr(&a.expr);
                    }
                    return Ty::Struct(callee.clone());
                }

                // Check user/stdlib function call
                let sig_opt = self.fn_sigs.get(callee).cloned();
                for (idx, a) in args.iter().enumerate() {
                    if a.mode == ArgPassMode::Mut {
                        if let Expr::Ident(vname, _) = &a.expr {
                            if self.in_spawn
                                && self.outer_vars_before_spawn.contains(vname)
                                && !self.spawn_moved_vars.contains(vname)
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
                        }
                    }
                    let arg_ty = if a.mode == ArgPassMode::Move {
                        if let Expr::Ident(vname, ln) = &a.expr {
                            self.check_expr(&Expr::MoveIdent(vname.clone(), *ln))
                        } else {
                            self.check_expr(&a.expr)
                        }
                    } else {
                        self.check_expr(&a.expr)
                    };
                    if let Some(sig) = &sig_opt {
                        let target_idx = if let Some(aname) = &a.name {
                            sig.params
                                .iter()
                                .position(|(pn, _, _, _)| pn == aname)
                                .unwrap_or(idx)
                        } else {
                            idx
                        };
                        if let Some((_, _, expected_ty, _)) = sig.params.get(target_idx) {
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
                                        "Argument {} to `{}` expected `{:?}`, found `{:?}`",
                                        target_idx + 1,
                                        callee,
                                        expected_ty,
                                        arg_ty
                                    ),
                                    line: *line,
                                    hint: None,
                                });
                            }
                        }
                    }
                }
                sig_opt.map(|s| s.ret).unwrap_or(Ty::Any)
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                let rty = self.check_expr(receiver);
                for a in args {
                    let _ = self.check_expr(&a.expr);
                }
                if method == "join" {
                    if let Ty::Job(inner) = rty {
                        return *inner;
                    }
                }
                if let Ty::Struct(sname) = rty {
                    let mangled = format!("{}__{}", sname, method);
                    if let Some(sig) = self.fn_sigs.get(&mangled) {
                        return sig.ret.clone();
                    }
                }
                Ty::Any
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
                // Pre-scan explicit `move var` inside spawn body so valid moves before/at return are recognized
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
        // Go-style Duck-Typed Interface compatibility!
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
