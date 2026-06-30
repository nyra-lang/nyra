//! Auto-clone string arguments for builtins that borrow but use move-on-call ABI.

use ast::*;

const CLONE_ALL_ARGS_BUILTINS: &[&str] = &["strcat"];

/// First argument is borrowed at runtime but Nyra move-on-call would consume the binding.
const CLONE_FIRST_ARG_BUILTINS: &[&str] = &[
    "substring",
    "strstr_pos",
    "str_split",
    "String_split",
];

fn clone_expr(expr: &Expression) -> Expression {
    match expr {
        Expression::Variable { name, span } => Expression::MethodCall(Box::new(MethodCallExpr {
            object: Expression::Variable {
                name: name.clone(),
                span: span.clone(),
            },
            method: "clone".into(),
            args: vec![],
            optional: false,
            span: span.clone(),
        })),
        other => other.clone(),
    }
}

fn maybe_clone_call_args(callee: &str, args: &mut [Expression]) {
    if CLONE_ALL_ARGS_BUILTINS.contains(&callee) {
        for a in args.iter_mut() {
            if matches!(a, Expression::Variable { .. }) {
                *a = clone_expr(a);
            }
        }
        return;
    }
    if CLONE_FIRST_ARG_BUILTINS.contains(&callee) {
        if let Some(first) = args.first_mut() {
            if matches!(first, Expression::Variable { .. }) {
                *first = clone_expr(first);
            }
        }
    }
}

fn desugar_call(expr: &mut Expression) {
    match expr {
        Expression::Call(c) => {
            for a in &mut c.args {
                desugar_call(a);
            }
            maybe_clone_call_args(&c.callee, &mut c.args);
        }
        Expression::Binary(b) => {
            desugar_call(&mut b.left);
            desugar_call(&mut b.right);
        }
        Expression::Unary(u) => desugar_call(&mut u.operand),
        Expression::Grouped(g) => desugar_call(g),
        Expression::MethodCall(mc) => {
            desugar_call(&mut mc.object);
            for a in &mut mc.args {
                desugar_call(a);
            }
            if mc.method == "split" && matches!(&mc.object, Expression::Variable { .. }) {
                mc.object = clone_expr(&mc.object);
            }
        }
        Expression::If(i) => {
            desugar_call(&mut i.condition);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| desugar_call(e));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| desugar_call(e));
        }
        Expression::Match(m) => {
            desugar_call(&mut m.scrutinee);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    desugar_call(g);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| desugar_call(e));
            }
        }
        Expression::Await(e) => desugar_call(e),
        Expression::FieldAccess(f) => desugar_call(&mut f.object),
        Expression::Index(ix) => {
            desugar_call(&mut ix.object);
            desugar_call(&mut ix.index);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                desugar_call(e);
            }
        }
        Expression::ArrayRepeat { element, .. } => desugar_call(element),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                desugar_call(e);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                desugar_call(spread);
            }
            for (_, e) in &mut s.fields {
                desugar_call(e);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    desugar_call(e);
                }
            }
        }
        _ => {}
    }
}

fn desugar_stmt(stmt: &mut Statement) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => desugar_call(&mut l.value),
        Statement::Assign(a) => {
            desugar_call(&mut a.target);
            desugar_call(&mut a.value);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                desugar_call(v);
            }
        }
        Statement::Expression(e) => desugar_call(e),
        Statement::Print(p) => {
            *p = p.clone().map_expressions(|e| {
                let mut x = e;
                desugar_call(&mut x);
                x
            });
        }
        Statement::Defer(e) => desugar_call(e),
        Statement::For(f) => {
            match &mut f.kind {
                ForKind::Range { start, end } => {
                    desugar_call(start);
                    desugar_call(end);
                }
                ForKind::Iterable { iterable } => desugar_call(iterable),
            }
            for s in &mut f.body.statements {
                desugar_stmt(s);
            }
        }
        Statement::While(w) => {
            desugar_call(&mut w.condition);
            for s in &mut w.body.statements {
                desugar_stmt(s);
            }
        }
        _ => {}
    }
}

pub fn desugar_string_borrow_builtins(program: &mut Program) {
    for f in &mut program.functions {
        for stmt in &mut f.body.statements {
            desugar_stmt(stmt);
        }
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            for stmt in &mut m.body.statements {
                desugar_stmt(stmt);
            }
        }
    }
}
