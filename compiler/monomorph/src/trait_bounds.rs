//! Validate `T: Trait` bounds at generic monomorph sites.

use std::collections::HashSet;

use ast::*;
use errors::{ErrorKind, NyraError, Span};

fn mangle_type_ann(base: &str, args: &[TypeAnnotation]) -> String {
    if args.is_empty() {
        return base.to_string();
    }
    let suffix: String = args
        .iter()
        .map(|a| match a {
            TypeAnnotation::Integer(k) => format!("{k:?}"),
            TypeAnnotation::Struct(n) => n.clone(),
            TypeAnnotation::Generic(n) => n.clone(),
            TypeAnnotation::Applied { base, args } => mangle_type_ann(base, args),
            TypeAnnotation::String => "string".into(),
            TypeAnnotation::Bool => "bool".into(),
            TypeAnnotation::F32 => "f32".into(),
            TypeAnnotation::F64 => "f64".into(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join("_");
    format!("{base}_{suffix}")
}

pub fn concrete_type_name(ann: &TypeAnnotation) -> Option<String> {
    match ann {
        TypeAnnotation::Struct(n) => Some(n.clone()),
        TypeAnnotation::Applied { base, args } => Some(mangle_type_ann(base, args)),
        TypeAnnotation::Integer(_) | TypeAnnotation::String | TypeAnnotation::Bool
        | TypeAnnotation::F32 | TypeAnnotation::F64 | TypeAnnotation::Char => None,
        TypeAnnotation::Generic(_) => None,
        _ => None,
    }
}

fn trait_impl_exists(pairs: &[(String, String)], type_name: &str, trait_name: &str) -> bool {
    pairs
        .iter()
        .any(|(ty, tr)| ty == type_name && tr == trait_name)
}

pub fn validate_function_bounds(
    func: &Function,
    type_args: &[TypeAnnotation],
    trait_impls: &[(String, String)],
    span: Span,
) -> Option<NyraError> {
    if func.type_param_bounds.is_empty() {
        return None;
    }
    for (i, param) in func.type_params.iter().enumerate() {
        let Some(required) = func.type_param_bounds.get(param) else {
            continue;
        };
        if required.is_empty() {
            continue;
        }
        let Some(arg) = type_args.get(i) else {
            continue;
        };
        let Some(concrete) = concrete_type_name(arg) else {
            continue;
        };
        for bound in required {
            if !trait_impl_exists(trait_impls, &concrete, bound) {
                return Some(
                    NyraError::new(
                        ErrorKind::Type,
                        span.clone(),
                        format!(
                            "type `{concrete}` does not implement trait `{bound}` (required by type parameter `{param}` on `{}`)",
                            func.name
                        ),
                    )
                    .note(format!("add `impl {bound} for {concrete}`")),
                );
            }
        }
    }
    None
}

fn collect_generic_calls_from_expr(
    expr: &Expression,
    originals: &std::collections::HashMap<String, Function>,
    trait_impls: &[(String, String)],
    out: &mut Vec<NyraError>,
) {
    match expr {
        Expression::Call(c) if !c.type_args.is_empty() => {
            if let Some(func) = originals.get(&c.callee) {
                if let Some(err) =
                    validate_function_bounds(func, &c.type_args, trait_impls, expr_span(expr))
                {
                    out.push(err);
                }
            }
            for a in &c.args {
                collect_generic_calls_from_expr(a, originals, trait_impls, out);
            }
        }
        Expression::Binary(b) => {
            collect_generic_calls_from_expr(&b.left, originals, trait_impls, out);
            collect_generic_calls_from_expr(&b.right, originals, trait_impls, out);
        }
        Expression::Unary(u) => {
            collect_generic_calls_from_expr(&u.operand, originals, trait_impls, out);
        }
        Expression::Grouped(g) => collect_generic_calls_from_expr(g, originals, trait_impls, out),
        Expression::If(i) => {
            collect_generic_calls_from_expr(&i.condition, originals, trait_impls, out);
            for_each_expr_in_block(&i.then_block, &mut |e| {
                collect_generic_calls_from_expr(e, originals, trait_impls, out);
            });
            for_each_expr_in_block(&i.else_block, &mut |e| {
                collect_generic_calls_from_expr(e, originals, trait_impls, out);
            });
        }
        Expression::Match(m) => {
            collect_generic_calls_from_expr(&m.scrutinee, originals, trait_impls, out);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    collect_generic_calls_from_expr(g, originals, trait_impls, out);
                }
                for_each_expr_in_block(&arm.body, &mut |e| collect_generic_calls_from_expr(e, originals, trait_impls, out));
            }
        }
        Expression::Await(e) => collect_generic_calls_from_expr(e, originals, trait_impls, out),
        Expression::MethodCall(mc) => {
            collect_generic_calls_from_expr(&mc.object, originals, trait_impls, out);
            for a in &mc.args {
                collect_generic_calls_from_expr(a, originals, trait_impls, out);
            }
        }
        Expression::FieldAccess(f) => {
            collect_generic_calls_from_expr(&f.object, originals, trait_impls, out);
        }
        Expression::Index(ix) => {
            collect_generic_calls_from_expr(&ix.object, originals, trait_impls, out);
            collect_generic_calls_from_expr(&ix.index, originals, trait_impls, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_generic_calls_from_expr(e, originals, trait_impls, out);
            }
        }
        Expression::StructLiteral(s) => {
            for (_, v) in &s.fields {
                collect_generic_calls_from_expr(v, originals, trait_impls, out);
            }
            for sp in &s.spreads {
                collect_generic_calls_from_expr(sp, originals, trait_impls, out);
            }
        }
        Expression::Call(c) => {
            for a in &c.args {
                collect_generic_calls_from_expr(a, originals, trait_impls, out);
            }
        }
        _ => {}
    }
}

fn collect_from_stmt(
    stmt: &Statement,
    originals: &std::collections::HashMap<String, Function>,
    trait_impls: &[(String, String)],
    out: &mut Vec<NyraError>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            collect_generic_calls_from_expr(&l.value, originals, trait_impls, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_generic_calls_from_expr(v, originals, trait_impls, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            collect_generic_calls_from_expr(e, originals, trait_impls, out);
        }
        Statement::Print(p) => {
            for a in &p.args {
                collect_generic_calls_from_expr(a, originals, trait_impls, out);
            }
        }
        Statement::If(i) => {
            collect_generic_calls_from_expr(&i.condition, originals, trait_impls, out);
            for s in &i.then_block.statements {
                collect_from_stmt(s, originals, trait_impls, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_from_stmt(s, originals, trait_impls, out);
                }
            }
        }
        Statement::While(w) => {
            collect_generic_calls_from_expr(&w.condition, originals, trait_impls, out);
            for s in &w.body.statements {
                collect_from_stmt(s, originals, trait_impls, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_generic_calls_from_expr(e, originals, trait_impls, out));
            for s in &f.body.statements {
                collect_from_stmt(s, originals, trait_impls, out);
            }
        }
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for s in &b.statements {
                collect_from_stmt(s, originals, trait_impls, out);
            }
        }
        _ => {}
    }
}

pub fn validate_trait_bounds(program: &Program) -> Vec<NyraError> {
    let originals: std::collections::HashMap<String, Function> = program
        .functions
        .iter()
        .filter(|f| !f.type_params.is_empty())
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    if originals.is_empty() {
        return vec![];
    }
    let trait_impls: Vec<(String, String)> = program
        .trait_impls
        .iter()
        .map(|ti| (ti.type_name.clone(), ti.trait_name.clone()))
        .collect();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for f in &program.functions {
        for stmt in &f.body.statements {
            collect_from_stmt(stmt, &originals, &trait_impls, &mut errors);
        }
    }
    for inst in &program.export_instances {
        if let Some(func) = originals.get(&inst.fn_name) {
            if let Some(err) = validate_function_bounds(
                func,
                &inst.type_args,
                &trait_impls,
                Span::default(),
            ) {
                let key = err.message.clone();
                if seen.insert(key) {
                    errors.push(err);
                }
            }
        }
    }
    errors
}
