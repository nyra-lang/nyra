use crate::*;
use errors::Span;

pub fn variable_name(expr: &Expression) -> Option<&str> {
    binding_name(expr)
}

/// Binding name for a variable, including `move x` / `clone x` wrappers.
pub fn binding_name(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Variable { name, .. } => Some(name.as_str()),
        Expression::Unary(u) if matches!(u.op, UnaryOp::Move | UnaryOp::Clone) => {
            binding_name(&u.operand)
        }
        _ => None,
    }
}

pub fn is_explicit_move(expr: &Expression) -> bool {
    matches!(expr, Expression::Unary(u) if u.op == UnaryOp::Move)
}

pub fn expr_span(expr: &Expression) -> Span {
    match expr {
        Expression::Variable { span, .. } => span.clone(),
        Expression::Binary(b) => b.span.clone(),
        Expression::Unary(u) => u.span.clone(),
        Expression::Call(c) => c.span.clone(),
        Expression::FieldAccess(f) => f.span.clone(),
        Expression::StructLiteral(s) => s.span.clone(),
        Expression::MethodCall(m) => m.span.clone(),
        Expression::Match(m) => m.span.clone(),
        Expression::If(i) => i.span.clone(),
        Expression::Index(ix) => ix.span.clone(),
        Expression::EnumVariant(e) => e.span.clone(),
        Expression::Grouped(inner) => expr_span(inner),
        Expression::Await(inner) => expr_span(inner),
        Expression::TemplateLiteral(t) => t.span.clone(),
        Expression::Cast(c) => c.span.clone(),
        Expression::ArrowFn(a) => a.span.clone(),
        Expression::ComptimeBlock { span, .. } => span.clone(),
        Expression::ArrayLiteral(al) => al
            .all_exprs()
            .next()
            .map(expr_span)
            .unwrap_or_else(|| al.span.clone()),
        Expression::ArrayRepeat { span, .. } => span.clone(),
        Expression::TupleLiteral(elems) => elems
            .first()
            .map(expr_span)
            .unwrap_or_default(),
        Expression::Literal(_) | Expression::Invalid => Span::default(),
    }
}

pub fn stmt_span(stmt: &Statement) -> Span {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => l.span.clone(),
        Statement::Assign(a) => a.span.clone(),
        Statement::Return(r) => r
            .value
            .as_ref()
            .map(expr_span)
            .unwrap_or_default(),
        Statement::If(i) => expr_span(&i.condition),
        Statement::While(w) => expr_span(&w.condition),
        Statement::Break { span } | Statement::Continue { span } => span.clone(),
        Statement::For(f) => match &f.kind {
            ForKind::Range { start, .. } => expr_span(start),
            ForKind::Iterable { iterable } => expr_span(iterable),
        },
        Statement::Print(p) => p
            .args
            .first()
            .map(expr_span)
            .or_else(|| p.color.as_ref().map(expr_span))
            .unwrap_or_default(),
        Statement::Expression(e) | Statement::Defer(e) => expr_span(e),
        Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => b
            .statements
            .first()
            .map(stmt_span)
            .unwrap_or_default(),
        Statement::Asm { span, .. } => span.clone(),
        Statement::Import(_) => Span::default(),
    }
}
