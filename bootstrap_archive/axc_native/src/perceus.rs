//! Axon v5.0 Static Perceus Memory Manager & FBIP In-Place Reuse Analyzer (`perceus.rs`)
//! Zero Garbage Collector (0 GC), Zero `'a` Lifetimes.
//! Computes exact variable liveness per function, inserts deterministic `Drop` (`VirtualFree`)
//! for owned allocations (`Table[T]`, `ByteBuf`, `IntBuf`), and pairs dying allocations with
//! subsequent allocations of the same class into zero-syscall FBIP In-Place Reuse tokens!

use crate::ast::*;
use crate::sema::{CheckedProgram, Ty, resolve_typespec};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocKind {
    Table(String),
    ByteBuf,
    IntBuf,
}

#[derive(Debug, Clone)]
pub struct PerceusFnPlan {
    /// Statement indices after which an owned variable can be deterministically dropped (`VirtualFree`)
    pub drops_after_stmt: HashMap<usize, Vec<(String, AllocKind)>>,
    /// Variables remaining live until function return (dropped at epilogue unless returned/moved)
    pub epilogue_drops: Vec<(String, AllocKind)>,
    /// Map from a new `VarDecl` variable name -> dying variable name whose memory block is reused in-place (0 alloc!)
    pub fbip_reuse_from: HashMap<String, String>,
}

pub fn analyze_program_perceus(
    prog: &Program,
    checked: &CheckedProgram,
) -> HashMap<String, PerceusFnPlan> {
    let mut plans = HashMap::new();
    for f in &prog.funcs {
        plans.insert(f.name.clone(), analyze_fn_perceus(f, checked));
    }
    plans
}

fn classify_alloc_expr(
    expr: &Expr,
    ty_hint: Option<&TypeSpec>,
    checked: &CheckedProgram,
) -> Option<AllocKind> {
    match expr {
        Expr::TableAlloc { elem_ty, .. } => return Some(AllocKind::Table(elem_ty.clone())),
        Expr::Call { callee, .. } => {
            if callee == "buf_new" {
                return Some(AllocKind::ByteBuf);
            }
            if callee == "ibuf_new" {
                return Some(AllocKind::IntBuf);
            }
        }
        _ => {}
    }
    if let Some(ts) = ty_hint {
        match resolve_typespec(ts, &checked.structs, &checked.interfaces) {
            Ty::Table(s) => return Some(AllocKind::Table(s)),
            Ty::Struct(s) if s == "ByteBuf" => return Some(AllocKind::ByteBuf),
            Ty::Struct(s) if s == "IntBuf" => return Some(AllocKind::IntBuf),
            _ => {}
        }
    }
    None
}

fn analyze_fn_perceus(f: &FnDecl, checked: &CheckedProgram) -> PerceusFnPlan {
    // 1. Collect owned allocations introduced at top-level statements & track last-use index + escape set
    let mut owned_decls: Vec<(usize, String, AllocKind)> = Vec::new();
    let mut owned_map: HashMap<String, AllocKind> = HashMap::new();

    // Parameters passed with `Move` or `Isolated` of owned buffer/table types are also owned by this function
    for p in &f.params {
        if matches!(p.mode, ParamMode::Move | ParamMode::Isolated) {
            let pty = resolve_typespec(&p.ty, &checked.structs, &checked.interfaces);
            let kind = match pty {
                Ty::Table(s) => Some(AllocKind::Table(s)),
                Ty::Struct(s) if s == "ByteBuf" => Some(AllocKind::ByteBuf),
                Ty::Struct(s) if s == "IntBuf" => Some(AllocKind::IntBuf),
                _ => None,
            };
            if let Some(k) = kind {
                owned_map.insert(p.name.clone(), k);
            }
        }
    }

    for (idx, stmt) in f.body.iter().enumerate() {
        if let Stmt::VarDecl {
            name, ty, init, ..
        } = stmt
        {
            if let Some(kind) = classify_alloc_expr(init, ty.as_ref(), checked) {
                owned_decls.push((idx, name.clone(), kind.clone()));
                owned_map.insert(name.clone(), kind);
            }
        }
    }

    if owned_map.is_empty() {
        return PerceusFnPlan {
            drops_after_stmt: HashMap::new(),
            epilogue_drops: Vec::new(),
            fbip_reuse_from: HashMap::new(),
        };
    }

    // 2. Scan last-use statement index and escaping variables (`return x`, `move x`, or `buf_free(move x)`)
    let mut last_use: HashMap<String, usize> = HashMap::new();
    let mut escaped: HashSet<String> = HashSet::new();

    for (idx, stmt) in f.body.iter().enumerate() {
        let mut used = HashSet::new();
        collect_stmt_uses(stmt, &mut used, &mut escaped);
        for u in used {
            if owned_map.contains_key(&u) {
                last_use.insert(u, idx);
            }
        }
    }

    // 3. FBIP In-Place Reuse Pairing:
    // If owned variable `old_v` has its last use BEFORE statement `decl_idx` (and didn't escape),
    // and statement `decl_idx` allocates a new variable `new_v` of the exact same `AllocKind`,
    // reuse `old_v`'s memory block in-place for `new_v` instead of freeing and re-allocating!
    let mut fbip_reuse_from: HashMap<String, String> = HashMap::new();
    let mut consumed_by_reuse: HashSet<String> = HashSet::new();

    for (decl_idx, new_name, new_kind) in &owned_decls {
        // Look for an already-dead, non-escaped allocation of matching `AllocKind`
        let mut candidate: Option<String> = None;
        for (old_name, old_kind) in &owned_map {
            if old_name == new_name
                || old_kind != new_kind
                || escaped.contains(old_name)
                || consumed_by_reuse.contains(old_name)
            {
                continue;
            }
            // Check old_name was declared before `decl_idx` and its last use is `< decl_idx`
            let old_decl_idx = owned_decls
                .iter()
                .find(|(_, n, _)| n == old_name)
                .map(|(i, _, _)| *i)
                .unwrap_or(0);
            let old_last = last_use.get(old_name).copied().unwrap_or(old_decl_idx);
            if old_decl_idx < *decl_idx && old_last < *decl_idx {
                candidate = Some(old_name.clone());
                break;
            }
        }
        if let Some(donor) = candidate {
            consumed_by_reuse.insert(donor.clone());
            fbip_reuse_from.insert(new_name.clone(), donor);
        }
    }

    // 4. Schedule deterministic Perceus `Drop` (`VirtualFree`) for remaining non-escaped allocations
    let mut drops_after_stmt: HashMap<usize, Vec<(String, AllocKind)>> = HashMap::new();
    let mut epilogue_drops: Vec<(String, AllocKind)> = Vec::new();

    for (var_name, kind) in &owned_map {
        if escaped.contains(var_name) || consumed_by_reuse.contains(var_name) {
            continue;
        }
        if let Some(&lu_idx) = last_use.get(var_name) {
            // If last use is not a return/control-flow tail, drop right after last use; else at epilogue
            if lu_idx + 1 < f.body.len() && !stmt_contains_return(&f.body[lu_idx]) {
                drops_after_stmt
                    .entry(lu_idx)
                    .or_default()
                    .push((var_name.clone(), kind.clone()));
            } else {
                epilogue_drops.push((var_name.clone(), kind.clone()));
            }
        } else {
            epilogue_drops.push((var_name.clone(), kind.clone()));
        }
    }

    PerceusFnPlan {
        drops_after_stmt,
        epilogue_drops,
        fbip_reuse_from,
    }
}

fn stmt_contains_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => then_body.iter().any(stmt_contains_return) || else_body.iter().any(stmt_contains_return),
        Stmt::While { body, .. }
        | Stmt::ForRange { body, .. }
        | Stmt::ForEach { body, .. }
        | Stmt::With { body, .. } => body.iter().any(stmt_contains_return),
        _ => false,
    }
}

fn collect_stmt_uses(stmt: &Stmt, used: &mut HashSet<String>, escaped: &mut HashSet<String>) {
    match stmt {
        Stmt::VarDecl { init, .. } | Stmt::TupleDestruct { init, .. } => {
            // Aliasing `b2 := b1` marks `b1` as escaped so we don't double-free
            if let Expr::Ident(id, _) | Expr::MoveIdent(id, _) = init {
                escaped.insert(id.clone());
            }
            collect_expr_uses(init, used, escaped, false);
        }
        Stmt::Assign { target, val, .. } => {
            collect_expr_uses(target, used, escaped, false);
            collect_expr_uses(val, used, escaped, false);
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            collect_expr_uses(cond, used, escaped, false);
            for s in then_body {
                collect_stmt_uses(s, used, escaped);
            }
            for s in else_body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        Stmt::While { cond, body, .. } => {
            collect_expr_uses(cond, used, escaped, false);
            for s in body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            collect_expr_uses(start, used, escaped, false);
            collect_expr_uses(end, used, escaped, false);
            for s in body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        Stmt::ForEach {
            collection, body, ..
        } => {
            collect_expr_uses(collection, used, escaped, false);
            for s in body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        Stmt::With { init, body, .. } => {
            collect_expr_uses(init, used, escaped, false);
            for s in body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        Stmt::Return { val: Some(e), .. } => {
            collect_expr_uses(e, used, escaped, true);
        }
        Stmt::ExprStmt { expr, .. } => {
            collect_expr_uses(expr, used, escaped, false);
        }
        _ => {}
    }
}

fn collect_expr_uses(
    expr: &Expr,
    used: &mut HashSet<String>,
    escaped: &mut HashSet<String>,
    in_return: bool,
) {
    match expr {
        Expr::Ident(name, _) => {
            used.insert(name.clone());
            if in_return {
                escaped.insert(name.clone());
            }
        }
        Expr::MoveIdent(name, _) => {
            used.insert(name.clone());
            escaped.insert(name.clone());
        }
        Expr::Tuple(elems, _) => {
            for e in elems {
                collect_expr_uses(e, used, escaped, in_return);
            }
        }
        Expr::FStr(parts, _) => {
            for p in parts {
                if let FStrPart::Expr(e) = p {
                    collect_expr_uses(e, used, escaped, false);
                }
            }
        }
        Expr::Unary { expr: inner, .. } | Expr::Cast { expr: inner, .. } => {
            collect_expr_uses(inner, used, escaped, in_return);
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_uses(left, used, escaped, false);
            collect_expr_uses(right, used, escaped, false);
        }
        Expr::OrReturn { lhs, ret_val, .. } => {
            collect_expr_uses(lhs, used, escaped, in_return);
            if let Some(rv) = ret_val {
                collect_expr_uses(rv, used, escaped, true);
            }
        }
        Expr::Field { base, .. } | Expr::TupleIdx { base, .. } => {
            collect_expr_uses(base, used, escaped, false);
        }
        Expr::Index { base, idx, .. } => {
            collect_expr_uses(base, used, escaped, false);
            collect_expr_uses(idx, used, escaped, false);
        }
        Expr::TableAlloc { cap, .. } => {
            collect_expr_uses(cap, used, escaped, false);
        }
        Expr::Call { callee, args, .. } => {
            let is_struct_ctor = callee.chars().next().is_some_and(|c| c.is_ascii_uppercase());
            let is_ctor = callee == "ok" || callee == "err" || is_struct_ctor;
            for a in args {
                if a.mode == ArgPassMode::Move || callee == "buf_free" || is_struct_ctor {
                    if let Expr::Ident(n, _) = &a.expr {
                        escaped.insert(n.clone());
                    }
                }
                collect_expr_uses(&a.expr, used, escaped, in_return && is_ctor);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            collect_expr_uses(receiver, used, escaped, in_return);
            for a in args {
                collect_expr_uses(&a.expr, used, escaped, false);
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
            collect_expr_uses(cond, used, escaped, false);
            for s in then_body {
                collect_stmt_uses(s, used, escaped);
            }
            collect_expr_uses(then_tail, used, escaped, in_return);
            for s in else_body {
                collect_stmt_uses(s, used, escaped);
            }
            collect_expr_uses(else_tail, used, escaped, in_return);
        }
        Expr::Spawn { body, .. } => {
            for s in body {
                collect_stmt_uses(s, used, escaped);
            }
        }
        _ => {}
    }
}
