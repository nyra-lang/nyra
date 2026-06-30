//! Shared type-checking helpers (returns, assignability, logic ops).

use ast::*;

use super::{TypeChecker, TypeEnv};
use types::Type;

pub(super) fn block_has_return(block: &Block) -> bool {
    block.statements.iter().any(|s| matches!(s, Statement::Return(_)))
}

pub(super) fn collect_return_types_from_block(
    block: &Block,
    checker: &mut TypeChecker,
    env: &mut TypeEnv,
    out: &mut Vec<Type>,
) {
    for stmt in &block.statements {
        match stmt {
            Statement::Return(r) => {
                let ty = if let Some(v) = &r.value {
                    let checked = checker.check_expr(v, env);
                    if checked == Type::Unknown {
                        checker.infer_expr_type_hint(v, env).unwrap_or(checked)
                    } else {
                        checked
                    }
                } else {
                    Type::Void
                };
                out.push(ty);
            }
            Statement::Let(l) | Statement::Const(l) => {
                let value_ty = checker.check_expr(&l.value, env);
                let var_ty = if let Some(ref ann) = l.ty {
                    let dt = checker.type_from_ann(ann);
                    if value_ty != Type::Unknown && value_ty != dt {
                        value_ty
                    } else {
                        dt
                    }
                } else if value_ty != Type::Unknown {
                    value_ty
                } else {
                    checker.infer_expr_type_hint(&l.value, env).unwrap_or(Type::Unknown)
                };
                env.variables.insert(
                    l.name.clone(),
                    super::VarInfo {
                        ty: var_ty,
                        mutable: l.mutable,
                    },
                );
            }
            Statement::Assign(a) => {
                let value_ty = checker.check_expr(&a.value, env);
                if let Expression::Variable { name, .. } = &a.target {
                    let hinted = if value_ty == Type::Unknown {
                        checker
                            .infer_expr_type_hint(&a.value, env)
                            .unwrap_or(Type::Unknown)
                    } else {
                        value_ty
                    };
                    if hinted != Type::Unknown {
                        if let Some(info) = env.variables.get_mut(name) {
                            if info.mutable {
                                info.ty = hinted;
                            }
                        }
                    }
                }
            }
            Statement::If(i) => {
                checker.check_expr(&i.condition, env);
                let base = env.clone();
                let mut then_env = base.clone();
                collect_return_types_from_block(&i.then_block, checker, &mut then_env, out);
                if let Some(e) = &i.else_block {
                    let mut else_env = base.clone();
                    collect_return_types_from_block(e, checker, &mut else_env, out);
                    *env = merge_if_branch_envs(&base, &then_env, &else_env);
                } else {
                    *env = then_env;
                }
            }
            Statement::While(w) => {
                checker.check_expr(&w.condition, env);
                collect_return_types_from_block(&w.body, checker, env, out);
            }
            Statement::For(f) => {
                match &f.kind {
                    ForKind::Range { start, end } => {
                        checker.check_expr(start, env);
                        checker.check_expr(end, env);
                        env.variables.insert(
                            f.var.clone(),
                            super::VarInfo {
                                ty: Type::Integer(ast::IntKind::I32),
                                mutable: true,
                            },
                        );
                    }
                    ForKind::Iterable { iterable } => {
                        let iter_ty = checker.check_expr(iterable, env);
                        let elem_ty = match &iter_ty {
                            Type::Array { elem, len: Some(_) } => elem.as_ref().clone(),
                            Type::String => Type::Char,
                            Type::VecStr => Type::String,
                            _ => Type::Unknown,
                        };
                        env.variables.insert(
                            f.var.clone(),
                            super::VarInfo {
                                ty: elem_ty,
                                mutable: true,
                            },
                        );
                    }
                }
                collect_return_types_from_block(&f.body, checker, env, out);
            }
            Statement::Unsafe(b) | Statement::Spawn(b) | Statement::Benchmark(b) => {
                collect_return_types_from_block(b, checker, env, out);
            }
            _ => {}
        }
    }
}

pub(super) fn unify_return_types(types: &[Type]) -> Option<Type> {
    if types.is_empty() {
        return Some(Type::Void);
    }
    let mut acc = types[0].clone();
    for t in types.iter().skip(1) {
        if *t == Type::Unknown || acc == Type::Unknown {
            acc = if acc == Type::Unknown { t.clone() } else { acc };
            continue;
        }
        if *t != acc {
            return None;
        }
    }
    Some(acc)
}

/// Merge mutable bindings after `if` / `else` so sibling branches do not leak assignments.
pub(super) fn merge_if_branch_envs(base: &TypeEnv, then_env: &TypeEnv, else_env: &TypeEnv) -> TypeEnv {
    let mut out = base.clone();
    for (name, base_info) in &base.variables {
        if !base_info.mutable {
            continue;
        }
        let mut types = Vec::new();
        if let Some(info) = then_env.variables.get(name) {
            if info.ty != base_info.ty {
                types.push(info.ty.clone());
            }
        }
        if let Some(info) = else_env.variables.get(name) {
            if info.ty != base_info.ty {
                types.push(info.ty.clone());
            }
        }
        if types.is_empty() {
            continue;
        }
        types.push(base_info.ty.clone());
        if let Some(merged) = unify_return_types(&types) {
            if let Some(slot) = out.variables.get_mut(name) {
                slot.ty = merged;
            }
        }
    }
    out
}

pub(super) fn logic_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        _ => "?",
    }
}

pub(super) fn types_assignable(from: &Type, to: &Type) -> bool {
    if matches!(to, Type::Ptr) && matches!(from, Type::Struct(_)) {
        return true;
    }
    if matches!(from, Type::VecStr) && matches!(to, Type::Ptr) {
        return true;
    }
    if matches!(from, Type::Ptr) && matches!(to, Type::VecStr) {
        return true;
    }
    if from == to || *from == Type::Unknown || *to == Type::Unknown {
        return true;
    }
    if types::integer::integer_assignable(to, from) {
        return true;
    }
    if matches!(to, Type::F32 | Type::F64) && types::is_integer(from) {
        return true;
    }
    if let (
        Type::Array {
            elem: from_elem,
            len: from_len,
        },
        Type::Array {
            elem: to_elem,
            len: to_len,
        },
    ) = (from, to)
    {
        if types_assignable(from_elem, to_elem) {
            match (from_len, to_len) {
                (_, None) => return true,
                (Some(a), Some(b)) if a == b => return true,
                _ => {}
            }
        }
    }
    if matches!(to, Type::Integer(_)) && *from == Type::F64 {
        return true;
    }
    if let (
        Type::Ref {
            inner: from_inner,
            mutable: from_mut,
            ..
        },
        Type::Ref {
            inner: to_inner,
            mutable: to_mut,
            ..
        },
    ) = (from, to)
    {
        if from_mut == to_mut {
            return types_assignable(from_inner, to_inner);
        }
    }
    if let Type::Ref { inner, mutable: false, .. } = to {
        if !matches!(from, Type::Ref { .. }) && types_assignable(from, inner) {
            return true;
        }
    }
    // Immutable borrow coerces to owned value at call sites (`&string` → `string` for read-only ABI).
    if let Type::Ref { inner, mutable: false, .. } = from {
        if types_assignable(inner, to) {
            return true;
        }
    }
    if let Type::Ref { inner, mutable: true, .. } = to {
        if let Type::Ref { inner: from_inner, mutable: true, .. } = from {
            return types_assignable(from_inner, inner);
        }
    }
    matches!(to, Type::Ptr) && matches!(from, Type::FnPtr { .. })
}
