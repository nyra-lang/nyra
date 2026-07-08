//! Desugar `clone x` → `x.clone()` before auto-borrow coercion.

use ast::*;

fn desugar_expr(expr: &mut Expression) {
    if let Expression::Unary(u) = expr {
        if u.op == UnaryOp::Clone {
            let operand = u.operand.clone();
            if binding_name(&operand).is_some()
                || matches!(operand, Expression::FieldAccess(_))
            {
                *expr = Expression::MethodCall(Box::new(MethodCallExpr {
                    object: operand,
                    method: "clone".into(),
                    args: vec![],
                    optional: false,
                    span: u.span.clone(),
                }));
                desugar_expr(expr);
                return;
            }
        }
    }

    match expr {
        Expression::Binary(b) => {
            desugar_expr(&mut b.left);
            desugar_expr(&mut b.right);
        }
        Expression::Unary(u) => desugar_expr(&mut u.operand),
        Expression::Grouped(g) => desugar_expr(g),
        Expression::Call(c) => {
            for a in &mut c.args {
                desugar_expr(a);
            }
        }
        Expression::MethodCall(mc) => {
            desugar_expr(&mut mc.object);
            for a in &mut mc.args {
                desugar_expr(a);
            }
        }
        Expression::If(i) => {
            desugar_expr(&mut i.condition);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| desugar_expr(e));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| desugar_expr(e));
        }
        Expression::Match(m) => {
            desugar_expr(&mut m.scrutinee);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    desugar_expr(g);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| desugar_expr(e));
            }
        }
        Expression::Await(e) => desugar_expr(e),
        Expression::FieldAccess(f) => desugar_expr(&mut f.object),
        Expression::Index(ix) => {
            desugar_expr(&mut ix.object);
            desugar_expr(&mut ix.index);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                desugar_expr(e);
            }
        }
        Expression::ArrayRepeat { element, .. } => desugar_expr(element),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                desugar_expr(e);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                desugar_expr(spread);
            }
            for (_, e) in &mut s.fields {
                desugar_expr(e);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    desugar_expr(e);
                }
            }
        }
        Expression::Cast(c) => desugar_expr(&mut c.expr),
        _ => {}
    }
}

fn desugar_stmt(stmt: &mut Statement) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => desugar_expr(&mut l.value),
        Statement::Assign(a) => {
            desugar_expr(&mut a.target);
            desugar_expr(&mut a.value);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                desugar_expr(v);
            }
        }
        Statement::If(i) => {
            desugar_expr(&mut i.condition);
            for s in &mut i.then_block.statements {
                desugar_stmt(s);
            }
            if let Some(e) = &mut i.else_block {
                for s in &mut e.statements {
                    desugar_stmt(s);
                }
            }
        }
        Statement::While(w) => {
            desugar_expr(&mut w.condition);
            for s in &mut w.body.statements {
                desugar_stmt(s);
            }
        }
        Statement::For(f) => {
            f.map_exprs_mut(desugar_expr);
            for s in &mut f.body.statements {
                desugar_stmt(s);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => desugar_expr(e),
        Statement::Print(p) => {
            for a in &mut p.args {
                desugar_expr(a);
            }
            if let Some(c) = &mut p.color {
                desugar_expr(c);
            }
        }
        Statement::Spawn(s) => {
            for stmt in &mut s.body.statements {
                desugar_stmt(stmt);
            }
        }
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            for stmt in &mut b.statements {
                desugar_stmt(stmt);
            }
        }
        _ => {}
    }
}

fn desugar_block(block: &mut Block) {
    for stmt in &mut block.statements {
        desugar_stmt(stmt);
    }
}

pub fn desugar_clone_prefix(program: &mut Program) {
    for f in &mut program.functions {
        desugar_block(&mut f.body);
    }
    for imp in &mut program.impls {
        for m in &mut imp.methods {
            desugar_block(&mut m.body);
        }
    }
    for ti in &mut program.trait_impls {
        for m in &mut ti.methods {
            desugar_block(&mut m.body);
        }
    }
}

fn binding_name(expr: &Expression) -> Option<&str> {
    ast::binding_name(expr)
}
