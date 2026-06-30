//! Desugar typed `await` on `Future_*` structs to runtime await calls.

use ast::*;
use errors::Span;
use typecheck::TypeChecker;
use types::Type;

fn expr_call(callee: &str, args: Vec<Expression>, span: Span) -> Expression {
    Expression::Call(CallExpr {
        callee: callee.into(),
        type_args: vec![],
        args,
        span,
    })
}

fn expr_field(object: Expression, field: &str, span: Span) -> Expression {
    Expression::FieldAccess(Box::new(FieldAccessExpr {
        object,
        field: field.into(),
        optional: false,
        span: span.clone(),
    }))
}

fn future_await_callee(struct_name: &str) -> Option<&'static str> {
    match struct_name {
        "Future_bool" => Some("async_await_bool"),
        "Future_string" => Some("async_await_ptr"),
        _ => None,
    }
}

fn rewrite_await_expr(expr: &mut Expression, checker: &TypeChecker) {
    match expr {
        Expression::Await(inner) => {
            let inner_span = expr_span(inner);
            if let Some(Type::Struct(name)) = checker.expression_type_hint(inner) {
                if let Some(callee) = future_await_callee(&name) {
                    let handle = expr_field((**inner).clone(), "handle", inner_span.clone());
                    *expr = expr_call(callee, vec![handle], inner_span);
                    return;
                }
                if name == "Future_i32" {
                    let handle = expr_field((**inner).clone(), "handle", inner_span.clone());
                    *expr = Expression::Await(Box::new(handle));
                    return;
                }
            }
        }
        Expression::Binary(b) => {
            rewrite_await_expr(&mut b.left, checker);
            rewrite_await_expr(&mut b.right, checker);
        }
        Expression::Unary(u) => rewrite_await_expr(&mut u.operand, checker),
        Expression::Grouped(inner) => rewrite_await_expr(inner, checker),
        Expression::Call(c) => {
            for arg in &mut c.args {
                rewrite_await_expr(arg, checker);
            }
        }
        Expression::If(i) => {
            rewrite_await_expr(&mut i.condition, checker);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| rewrite_await_expr(e, checker));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| rewrite_await_expr(e, checker));
        }
        Expression::FieldAccess(f) => rewrite_await_expr(&mut f.object, checker),
        Expression::Index(ix) => {
            rewrite_await_expr(&mut ix.object, checker);
            rewrite_await_expr(&mut ix.index, checker);
        }
        Expression::StructLiteral(sl) => {
            for (_, v) in &mut sl.fields {
                rewrite_await_expr(v, checker);
            }
            for s in &mut sl.spreads {
                rewrite_await_expr(s, checker);
            }
        }
        Expression::ArrayLiteral(al) => {
            for e in &mut al.elems {
                rewrite_await_expr(e, checker);
            }
            for s in &mut al.spreads {
                rewrite_await_expr(s, checker);
            }
        }
        Expression::MethodCall(mc) => {
            rewrite_await_expr(&mut mc.object, checker);
            for a in &mut mc.args {
                rewrite_await_expr(a, checker);
            }
        }
        Expression::Cast(c) => rewrite_await_expr(&mut c.expr, checker),
        Expression::Match(m) => {
            rewrite_await_expr(&mut m.scrutinee, checker);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    rewrite_await_expr(g, checker);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| rewrite_await_expr(e, checker));
            }
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rewrite_await_expr(e, checker);
            }
        }
        _ => {}
    }
}

fn rewrite_block(block: &mut Block, checker: &TypeChecker) {
    for stmt in &mut block.statements {
        rewrite_stmt(stmt, checker);
    }
}

fn rewrite_stmt(stmt: &mut Statement, checker: &TypeChecker) {
    match stmt {
        Statement::Let(l) => rewrite_await_expr(&mut l.value, checker),
        Statement::Expression(e) => rewrite_await_expr(e, checker),
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                rewrite_await_expr(v, checker);
            }
        }
        Statement::If(i) => {
            rewrite_await_expr(&mut i.condition, checker);
            rewrite_block(&mut i.then_block, checker);
            if let Some(eb) = &mut i.else_block {
                rewrite_block(eb, checker);
            }
        }
        Statement::While(w) => {
            rewrite_await_expr(&mut w.condition, checker);
            rewrite_block(&mut w.body, checker);
        }
        Statement::For(f) => {
            match &mut f.kind {
                ForKind::Range { start, end } => {
                    rewrite_await_expr(start, checker);
                    rewrite_await_expr(end, checker);
                }
                ForKind::Iterable { iterable } => {
                    rewrite_await_expr(iterable, checker);
                }
            }
            rewrite_block(&mut f.body, checker);
        }
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
            rewrite_block(b, checker);
        }
        _ => {}
    }
}

pub fn desugar_future_await(program: &mut Program, checker: &TypeChecker) {
    for f in &mut program.functions {
        rewrite_block(&mut f.body, checker);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            rewrite_block(&mut m.body, checker);
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            rewrite_block(&mut m.body, checker);
        }
    }
}
