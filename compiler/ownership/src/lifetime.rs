use std::collections::HashMap;

use ast::*;
use ast::expr_span;
use errors::NyraError;

use crate::context::OwnershipCtx;
use crate::diag;
use crate::subtype;
use types::Type;

#[derive(Debug, Clone)]
struct ResolvedSig {
    lifetime_params: Vec<String>,
    param_lifetimes: Vec<Option<String>>,
    return_lifetime: Option<String>,
}

pub fn check_program(program: &Program, _ctx: &OwnershipCtx, errors: &mut Vec<NyraError>) {
    for func in &program.functions {
        if !func.type_params.is_empty() {
            continue;
        }
        check_function(func, errors);
    }
    for imp in &program.impls {
        for method in &imp.methods {
            check_function(method, errors);
        }
    }
    for ti in &program.trait_impls {
        for method in &ti.methods {
            check_function(method, errors);
        }
    }
    check_hrtb_passes(program, errors);
}

fn check_hrtb_passes(program: &Program, errors: &mut Vec<NyraError>) {
    let mut sigs: HashMap<String, Vec<Type>> = HashMap::new();
    for f in &program.functions {
        sigs.insert(
            f.name.clone(),
            f.params.iter().map(|p| Type::from(p.ty.clone())).collect(),
        );
    }
    for func in &program.functions {
        walk_hrtb_calls(&func.body, &sigs, errors);
    }
}

fn walk_hrtb_calls(block: &Block, sigs: &HashMap<String, Vec<Type>>, errors: &mut Vec<NyraError>) {
    for stmt in &block.statements {
        match stmt {
            Statement::Expression(Expression::Call(c)) => {
                if let Some(params) = sigs.get(&c.callee) {
                    for (arg, param_ty) in c.args.iter().zip(params.iter()) {
                        check_hrtb_arg(arg, param_ty, errors);
                    }
                }
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    walk_expr_hrtb(arg, sigs, errors);
                }
                if let Some(c) = &p.color {
                    walk_expr_hrtb(c, sigs, errors);
                }
            }
            Statement::If(i) => {
                walk_expr_hrtb(&i.condition, sigs, errors);
                walk_hrtb_calls(&i.then_block, sigs, errors);
                if let Some(e) = &i.else_block {
                    walk_hrtb_calls(e, sigs, errors);
                }
            }
            Statement::While(w) => {
                walk_expr_hrtb(&w.condition, sigs, errors);
                walk_hrtb_calls(&w.body, sigs, errors);
            }
            Statement::Spawn(s) => walk_hrtb_calls(&s.body, sigs, errors),
            Statement::Benchmark(b) => walk_hrtb_calls(b, sigs, errors),
            _ => {}
        }
    }
}

fn walk_expr_hrtb(expr: &Expression, sigs: &HashMap<String, Vec<Type>>, errors: &mut Vec<NyraError>) {
    if let Expression::Call(c) = expr {
        if let Some(params) = sigs.get(&c.callee) {
            for (arg, param_ty) in c.args.iter().zip(params.iter()) {
                check_hrtb_arg(arg, param_ty, errors);
            }
        }
    }
}

fn check_hrtb_arg(arg: &Expression, param_ty: &Type, errors: &mut Vec<NyraError>) {
    let Type::ForAll { lifetimes, inner } = param_ty else {
        return;
    };
    let Expression::Variable { name, .. } = arg else {
        return;
    };
    if lifetimes.is_empty() {
        return;
    }
    if let Type::FnPtr { lifetime_params, .. } = inner.as_ref() {
        if !lifetime_params.is_empty() && lifetime_params.len() != lifetimes.len() {
            errors.push(diag::hrtb_lifetime_arity_mismatch(name, expr_span(arg)));
        }
    }
}

fn check_function(func: &Function, errors: &mut Vec<NyraError>) {
    let sig = resolve_lifetimes(func);
    validate_lifetime_params(func, &sig, errors);
    check_returns(func, &sig, errors);
}

fn resolve_lifetimes(func: &Function) -> ResolvedSig {
    let mut elided_counter = 0usize;
    let mut fresh = || {
        let name = format!("'elided{elided_counter}");
        elided_counter += 1;
        name
    };

    let lifetime_params: Vec<String> = func.lifetime_params.clone();

    let mut param_lifetimes = Vec::new();
    let mut input_ref_lifetimes = Vec::new();

    for p in &func.params {
        let lt = ref_lifetime_from_ann(&p.ty, &lifetime_params, &mut fresh);
        if let Some(ref l) = lt {
            input_ref_lifetimes.push(l.clone());
        }
        param_lifetimes.push(lt);
    }

    let return_is_elided_ref = func.return_type.as_ref().is_some_and(|t| {
        matches!(
            t,
            TypeAnnotation::Ref {
                lifetime: None,
                ..
            }
        )
    });

    let mut return_lifetime = func.return_type.as_ref().and_then(|t| match t {
        TypeAnnotation::Ref {
            lifetime: Some(lt),
            ..
        } => Some(lt.clone()),
        _ => None,
    });

    // Elision rule: single input ref lifetime → output ref gets same lifetime.
    if return_is_elided_ref {
        if input_ref_lifetimes.len() == 1 {
            return_lifetime = Some(input_ref_lifetimes[0].clone());
        } else if input_ref_lifetimes.is_empty() {
            return_lifetime = Some(fresh());
        }
    }

    ResolvedSig {
        lifetime_params,
        param_lifetimes,
        return_lifetime,
    }
}

#[allow(clippy::only_used_in_recursion)]
fn ref_lifetime_from_ann(
    ann: &TypeAnnotation,
    lifetime_params: &[String],
    fresh: &mut dyn FnMut() -> String,
) -> Option<String> {
    match ann {
        TypeAnnotation::Ref { lifetime, .. } => {
            if let Some(lt) = lifetime {
                Some(lt.clone())
            } else {
                Some(fresh())
            }
        }
        TypeAnnotation::Array { elem, .. } => ref_lifetime_from_ann(elem, lifetime_params, fresh),
        _ => None,
    }
}

fn validate_lifetime_params(func: &Function, sig: &ResolvedSig, errors: &mut Vec<NyraError>) {
    let mut used = HashMap::<String, usize>::new();

    for p in &func.params {
        collect_lifetime_names_in_ann(&p.ty, &mut used);
    }
    if let Some(rt) = &func.return_type {
        collect_lifetime_names_in_ann(rt, &mut used);
    }

    for lt in used.keys() {
        if !sig.lifetime_params.iter().any(|p| p == lt) && !lt.starts_with("'elided") {
            errors.push(diag::undeclared_lifetime(lt, &func.name, func.span.clone()));
        }
    }

    if func.return_type.as_ref().is_some_and(|t| {
        matches!(
            t,
            TypeAnnotation::Ref {
                lifetime: None,
                ..
            }
        )
    }) && sig.return_lifetime.is_none()
    {
        let ref_inputs = sig
            .param_lifetimes
            .iter()
            .filter(|l| l.is_some())
            .count();
        if ref_inputs > 1 {
            errors.push(diag::lifetime_elision_ambiguous(&func.name, func.span.clone()));
        }
    }
}

fn collect_lifetime_names_in_ann(ann: &TypeAnnotation, used: &mut HashMap<String, usize>) {
    match ann {
        TypeAnnotation::Ref { lifetime, inner, .. } => {
            if let Some(lt) = lifetime {
                *used.entry(lt.clone()).or_default() += 1;
            }
            collect_lifetime_names_in_ann(inner, used);
        }
        TypeAnnotation::Array { elem, .. } => collect_lifetime_names_in_ann(elem, used),
        _ => {}
    }
}

fn check_returns(func: &Function, sig: &ResolvedSig, errors: &mut Vec<NyraError>) {
    let ret_is_ref = func
        .return_type
        .as_ref()
        .map(|t| matches!(Type::from(t.clone()), Type::Ref { .. }))
        .unwrap_or(false);
    if !ret_is_ref {
        return;
    }

    let Some(ret_expr) = find_return_expr(&func.body) else {
        return;
    };

    if returns_ref_to_local(&ret_expr) {
        errors.push(diag::return_ref_to_local(func.span.clone()));
        return;
    }

    if let Some(source_lt) = lifetime_of_returned_ref(&ret_expr, func, sig) {
        if let Some(expected) = &sig.return_lifetime {
            if !lifetime_outlives(&source_lt, expected) && source_lt != *expected {
                errors.push(diag::returned_lifetime_too_short(
                    &source_lt,
                    expected,
                    func.span.clone(),
                ));
            }
        }
    }
}

fn lifetime_of_returned_ref(expr: &Expression, func: &Function, sig: &ResolvedSig) -> Option<String> {
    match expr {
        Expression::Unary(u) if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) => {
            if let Expression::Variable { name, .. } = &u.operand {
                for (p, lt) in func.params.iter().zip(sig.param_lifetimes.iter()) {
                    if &p.name == name {
                        return lt.clone();
                    }
                }
                return Some("'local".into());
            }
            None
        }
        Expression::Variable { name, .. } => {
            for (p, lt) in func.params.iter().zip(sig.param_lifetimes.iter()) {
                if &p.name == name {
                    return lt.clone();
                }
            }
            Some("'local".into())
        }
        Expression::Grouped(inner) => lifetime_of_returned_ref(inner, func, sig),
        _ => None,
    }
}

fn lifetime_outlives(source: &str, required: &str) -> bool {
    subtype::lifetime_outlives(source, required)
}

fn find_return_expr(block: &Block) -> Option<Expression> {
    for stmt in &block.statements {
        match stmt {
            Statement::Return(r) => return r.value.clone(),
            Statement::If(i) => {
                if let Some(e) = find_return_expr(&i.then_block) {
                    return Some(e);
                }
                if let Some(else_b) = &i.else_block {
                    if let Some(e) = find_return_expr(else_b) {
                        return Some(e);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn returns_ref_to_local(expr: &Expression) -> bool {
    match expr {
        Expression::Unary(u) if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) => {
            matches!(&u.operand, Expression::Variable { .. })
        }
        Expression::Grouped(inner) => returns_ref_to_local(inner),
        _ => false,
    }
}
