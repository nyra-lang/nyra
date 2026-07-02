//! Desugar `for x in arr` to indexed `for __i in 0..N` inside async functions (post-typecheck).

use std::collections::HashMap;

use ast::*;
use errors::Span;
use typecheck::TypeChecker;
use types::Type;

fn expr_int(n: i64, span: Span) -> Expression {
    Expression::Literal(Literal::Int(n))
}

fn expr_var(name: &str, span: Span) -> Expression {
    Expression::Variable {
        name: name.into(),
        span,
    }
}

fn array_len_from_type(ty: &Type) -> Option<usize> {
    match ty {
        Type::Array { len: Some(n), .. } => Some(*n),
        _ => None,
    }
}

fn array_literal_len(expr: &Expression) -> Option<usize> {
    match expr {
        Expression::ArrayLiteral(al) if al.spreads.is_empty() => Some(al.elems.len()),
        _ => None,
    }
}

fn infer_iterable_len(
    expr: &Expression,
    env: &HashMap<String, Type>,
    params: &[Param],
    checker: &TypeChecker,
) -> Option<usize> {
    if let Some(n) = array_literal_len(expr) {
        return Some(n);
    }
    if let Expression::Variable { name, .. } = expr {
        if let Some(ty) = env.get(name) {
            if let Some(n) = array_len_from_type(ty) {
                return Some(n);
            }
        }
        if let Some(p) = params.iter().find(|p| &p.name == name) {
            if let Some(n) = array_len_from_type(&checker.type_from_ann(&p.ty)) {
                return Some(n);
            }
        }
    }
    checker
        .expression_type_hint(expr)
        .as_ref()
        .and_then(array_len_from_type)
}

fn desugar_one_for(
    f: &ForStmt,
    env: &HashMap<String, Type>,
    params: &[Param],
    checker: &TypeChecker,
) -> Option<Vec<Statement>> {
    if f.parallel.is_some() || f.progress.is_some() {
        return None;
    }
    let ForKind::Iterable { iterable } = &f.kind else {
        return None;
    };
    let len = infer_iterable_len(iterable, env, params, checker)?;
    let span = ast::expr_span(iterable);
    let idx = format!("__nyra_for_{}", f.var);
    let iter_tmp = format!("__nyra_iter_{}", f.var);
    let elem = Expression::Index(Box::new(IndexExpr {
        object: Expression::Variable {
            name: iter_tmp.clone(),
            span: span.clone(),
        },
        index: expr_var(&idx, span.clone()),
        span: span.clone(),
    }));
    let mut body = f.body.clone();
    body.statements.insert(
        0,
        Statement::Let(LetStmt {
            name: f.var.clone(),
            mutable: true,
            destructure: vec![],
            span: span.clone(),
            ty: None,
            value: elem,
        }),
    );
    let range_for = ForStmt {
        var: idx,
        parallel: None,
        progress: None,
        kind: ForKind::Range {
            start: expr_int(0, span.clone()),
            end: expr_int(len as i64, span.clone()),
        },
        body,
    };
    Some(vec![
        Statement::Let(LetStmt {
            name: iter_tmp,
            mutable: false,
            destructure: vec![],
            span: span.clone(),
            ty: None,
            value: iterable.clone(),
        }),
        Statement::For(range_for),
    ])
}

fn bind_let(env: &mut HashMap<String, Type>, l: &LetStmt, params: &[Param], checker: &TypeChecker) {
    let ty = l
        .ty
        .as_ref()
        .map(|a| checker.type_from_ann(a))
        .or_else(|| infer_assign_type(&l.value, env, params, checker))
        .unwrap_or(Type::Unknown);
    if ty != Type::Unknown {
        env.insert(l.name.clone(), ty);
    }
}

fn infer_assign_type(
    value: &Expression,
    env: &HashMap<String, Type>,
    params: &[Param],
    checker: &TypeChecker,
) -> Option<Type> {
    match value {
        Expression::Variable { name, .. } => env
            .get(name)
            .cloned()
            .or_else(|| {
                params
                    .iter()
                    .find(|p| &p.name == name)
                    .map(|p| checker.type_from_ann(&p.ty))
            }),
        _ => checker.expression_type_hint(value),
    }
}

fn desugar_block(block: &mut Block, env: &mut HashMap<String, Type>, params: &[Param], checker: &TypeChecker) {
    let mut i = 0usize;
    while i < block.statements.len() {
        if let Statement::For(f) = block.statements[i].clone() {
            if let Some(replacement) = desugar_one_for(&f, env, params, checker) {
                block.statements.splice(i..i + 1, replacement);
                if let Statement::Let(l) = &block.statements[i] {
                    bind_let(env, &l.clone(), params, checker);
                }
                i += 1;
                continue;
            }
        }
        desugar_stmt(&mut block.statements[i], env, params, checker);
        i += 1;
    }
}

fn desugar_stmt(stmt: &mut Statement, env: &mut HashMap<String, Type>, params: &[Param], checker: &TypeChecker) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            bind_let(env, l, params, checker);
        }
        Statement::For(f) => {
            desugar_block(&mut f.body, env, params, checker);
        }
        Statement::If(i) => {
            desugar_block(&mut i.then_block, env, params, checker);
            if let Some(eb) = &mut i.else_block {
                desugar_block(eb, env, params, checker);
            }
        }
        Statement::While(w) => desugar_block(&mut w.body, env, params, checker),
        Statement::Spawn(s) => desugar_block(&mut s.body, env, params, checker),
        Statement::Unsafe(b) | Statement::Benchmark(b) => desugar_block(b, env, params, checker),
        _ => {}
    }
}

pub fn desugar_async_for_in_loops(program: &mut Program, checker: &TypeChecker) {
    let mut desugar_one = |params: &[Param], name: &str, body: &mut Block| {
        let mut env: HashMap<String, Type> = HashMap::new();
        if let Some(sig) = checker.env.functions.get(name) {
            for (p, ty) in params.iter().zip(sig.params.iter()) {
                env.insert(p.name.clone(), ty.clone());
            }
        } else {
            for p in params {
                env.insert(p.name.clone(), checker.type_from_ann(&p.ty));
            }
        }
        desugar_block(body, &mut env, params, checker);
    };
    for f in &mut program.functions {
        if f.is_async {
            desugar_one(&f.params, &f.name, &mut f.body);
        }
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            if m.is_async {
                desugar_one(&m.params, &m.name, &mut m.body);
            }
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            if m.is_async {
                desugar_one(&m.params, &m.name, &mut m.body);
            }
        }
    }
}
