//! Desugar `match` or-patterns (`A | B => body`) into separate arms.

use ast::*;

fn flatten_pattern(pattern: &MatchPattern) -> Vec<MatchPattern> {
    match pattern {
        MatchPattern::Or(ps) => ps.iter().flat_map(flatten_pattern).collect(),
        other => vec![other.clone()],
    }
}

fn desugar_match_expr(m: &mut MatchExpr) {
    let mut new_arms = Vec::new();
    for arm in m.arms.drain(..) {
        let patterns = flatten_pattern(&arm.pattern);
        for pattern in patterns {
            new_arms.push(MatchArm {
                pattern,
                guard: arm.guard.clone(),
                body: arm.body.clone(),
            });
        }
    }
    m.arms = new_arms;
}

fn desugar_expr(expr: &mut Expression) {
    match expr {
        Expression::Binary(b) => {
            desugar_expr(&mut b.left);
            desugar_expr(&mut b.right);
        }
        Expression::Unary(u) => desugar_expr(&mut u.operand),
        Expression::Grouped(inner) => desugar_expr(inner),
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
        Expression::FieldAccess(f) => desugar_expr(&mut f.object),
        Expression::Index(ix) => {
            desugar_expr(&mut ix.object);
            desugar_expr(&mut ix.index);
        }
        Expression::ArrayLiteral(a) => {
            for e in &mut a.elems {
                desugar_expr(e);
            }
            for s in &mut a.spreads {
                desugar_expr(s);
            }
        }
        Expression::StructLiteral(sl) => {
            for s in &mut sl.spreads {
                desugar_expr(s);
            }
            for (_, v) in &mut sl.fields {
                desugar_expr(v);
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
            desugar_match_expr(m);
        }
        Expression::Await(inner) => desugar_expr(inner),
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    desugar_expr(e);
                }
            }
        }
        Expression::Cast(c) => desugar_expr(&mut c.expr),
        Expression::ArrowFn(a) => match &mut a.body {
            ArrowBody::Expr(e) => desugar_expr(e),
            ArrowBody::Block(b) => desugar_block(b),
        },
        _ => {}
    }
}

fn desugar_block(block: &mut Block) {
    for stmt in &mut block.statements {
        desugar_stmt(stmt);
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
        Statement::Expression(e) => desugar_expr(e),
        Statement::Print(p) => {
            *p = p.clone().map_expressions(|mut e| {
                desugar_expr(&mut e);
                e
            });
        }
        Statement::Defer(e) => desugar_expr(e),
        Statement::If(i) => {
            desugar_expr(&mut i.condition);
            desugar_block(&mut i.then_block);
            if let Some(eb) = &mut i.else_block {
                desugar_block(eb);
            }
        }
        Statement::While(w) => {
            desugar_expr(&mut w.condition);
            desugar_block(&mut w.body);
        }
        Statement::For(f) => {
            match &mut f.kind {
                ForKind::Range { start, end } => {
                    desugar_expr(start);
                    desugar_expr(end);
                }
                ForKind::Iterable { iterable } => desugar_expr(iterable),
            }
            desugar_block(&mut f.body);
        }
        Statement::Break { .. } | Statement::Continue { .. } => {}
        _ => {}
    }
}

pub fn desugar_match_or_patterns(program: &mut Program) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast::{Expression, Literal};
    use errors::Span;

    #[test]
    fn flattens_or_pattern_into_arms() {
        let mut m = MatchExpr {
            scrutinee: Box::new(Expression::Variable {
                name: "c".into(),
                span: Span::default(),
            }),
            arms: vec![MatchArm {
                pattern: MatchPattern::Or(vec![
                    MatchPattern::Qualified("Color".into(), "Red".into()),
                    MatchPattern::Qualified("Color".into(), "Blue".into()),
                ]),
                guard: None,
                body: block_from_expr(Expression::Literal(Literal::Int(1))),
            }],
            span: Span::default(),
        };
        desugar_match_expr(&mut m);
        assert_eq!(m.arms.len(), 2);
        assert!(matches!(
            &m.arms[0].pattern,
            MatchPattern::Qualified(en, v) if en == "Color" && v == "Red"
        ));
        assert!(matches!(
            &m.arms[1].pattern,
            MatchPattern::Qualified(en, v) if en == "Color" && v == "Blue"
        ));
    }
}
