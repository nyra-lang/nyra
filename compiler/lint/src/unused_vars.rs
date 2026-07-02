use std::collections::HashMap;

use ast::{for_each_expr_in_block, Block, Expression, Function, Program, Statement};
use errors::{ErrorKind, NyraError, Span};

#[derive(Clone)]
struct Binding {
    name: String,
    span: Span,
    used: bool,
}

pub fn check_unused_variables(program: &Program) -> Vec<NyraError> {
    let mut warnings = Vec::new();
    for f in &program.functions {
        if f.is_test {
            continue;
        }
        check_function(f, &mut warnings);
    }
    warnings
}

fn check_function(func: &Function, out: &mut Vec<NyraError>) {
    let mut scopes: Vec<HashMap<String, Binding>> = vec![HashMap::new()];

    for p in &func.params {
        if is_ignored_param(&p.name) {
            continue;
        }
        scopes[0].insert(
            p.name.clone(),
            Binding {
                name: p.name.clone(),
                span: func.span.clone(),
                used: false,
            },
        );
    }

    check_block(&func.body, &mut scopes, out);

    for binding in scopes[0].values() {
        if !binding.used {
            out.push(unused_var_diag(binding));
        }
    }
}

fn check_block(block: &Block, scopes: &mut Vec<HashMap<String, Binding>>, out: &mut Vec<NyraError>) {
    scopes.push(HashMap::new());
    let depth = scopes.len() - 1;
    for stmt in &block.statements {
        check_stmt(stmt, scopes, out);
    }
    let local = scopes.pop().expect("scope stack");
    for binding in local.values() {
        if !binding.used {
            // Shadowed names are handled by inner scope only.
            if scopes.get(depth.saturating_sub(1)).is_some_and(|outer| outer.contains_key(&binding.name)) {
                continue;
            }
            out.push(unused_var_diag(binding));
        }
    }
}

fn check_stmt(stmt: &Statement, scopes: &mut Vec<HashMap<String, Binding>>, out: &mut Vec<NyraError>) {
    match stmt {
        Statement::Let(ls) | Statement::Const(ls) => {
            mark_expr_uses(&ls.value, scopes);
            if !is_ignored_name(&ls.name) {
                scopes
                    .last_mut()
                    .unwrap()
                    .insert(
                        ls.name.clone(),
                        Binding {
                            name: ls.name.clone(),
                            span: ls.span.clone(),
                            used: false,
                        },
                    );
            }
            for name in &ls.destructure {
                if !is_ignored_name(name) {
                    scopes.last_mut().unwrap().insert(
                        name.clone(),
                        Binding {
                            name: name.clone(),
                            span: ls.span.clone(),
                            used: false,
                        },
                    );
                }
            }
        }
        Statement::Assign(a) => {
            mark_expr_uses(&a.target, scopes);
            mark_expr_uses(&a.value, scopes);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                mark_expr_uses(v, scopes);
            }
        }
        Statement::If(i) => {
            mark_expr_uses(&i.condition, scopes);
            check_block(&i.then_block, scopes, out);
            if let Some(el) = &i.else_block {
                check_block(el, scopes, out);
            }
        }
        Statement::While(w) => {
            mark_expr_uses(&w.condition, scopes);
            check_block(&w.body, scopes, out);
        }
        Statement::For(f) => {
            match &f.kind {
                ast::ForKind::Range { start, end } => {
                    mark_expr_uses(start, scopes);
                    mark_expr_uses(end, scopes);
                }
                ast::ForKind::Iterable { iterable } => mark_expr_uses(iterable, scopes),
            }
            scopes.push(HashMap::new());
            if !is_ignored_name(&f.var) {
                scopes.last_mut().unwrap().insert(
                    f.var.clone(),
                    Binding {
                        name: f.var.clone(),
                        span: Span::default(),
                        used: false,
                    },
                );
            }
            for stmt in &f.body.statements {
                check_stmt(stmt, scopes, out);
            }
            let loop_scope = scopes.pop().expect("loop scope");
            if let Some(binding) = loop_scope.get(&f.var) {
                if !binding.used && !is_ignored_name(&f.var) {
                    out.push(unused_var_diag(binding));
                }
            }
        }
        Statement::Expression(e) => mark_expr_uses(e, scopes),
        Statement::Print(p) => {
            for e in &p.args {
                mark_expr_uses(e, scopes);
            }
            if let Some(c) = &p.color {
                mark_expr_uses(c, scopes);
            }
        }
        Statement::Defer(e) => mark_expr_uses(e, scopes),
        Statement::Spawn(s) => check_block(&s.body, scopes, out),
        Statement::Unsafe(b) | Statement::Benchmark(b) => check_block(b, scopes, out),
        Statement::Asm { .. } | Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn mark_expr_uses(expr: &Expression, scopes: &mut Vec<HashMap<String, Binding>>) {
    match expr {
        Expression::Variable { name, .. } => mark_used(name, scopes),
        Expression::Binary(b) => {
            mark_expr_uses(&b.left, scopes);
            mark_expr_uses(&b.right, scopes);
        }
        Expression::Unary(u) => mark_expr_uses(&u.operand, scopes),
        Expression::Call(c) => {
            mark_used(&c.callee, scopes);
            for a in &c.args {
                mark_expr_uses(a, scopes);
            }
        }
        Expression::MethodCall(m) => {
            mark_expr_uses(&m.object, scopes);
            for a in &m.args {
                mark_expr_uses(a, scopes);
            }
        }
        Expression::FieldAccess(f) => mark_expr_uses(&f.object, scopes),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                mark_expr_uses(spread, scopes);
            }
            for (_, v) in &s.fields {
                mark_expr_uses(v, scopes);
            }
        }
        Expression::EnumVariant(v) => {
            for a in &v.args {
                mark_expr_uses(a, scopes);
            }
        }
        Expression::Match(m) => {
            mark_expr_uses(&m.scrutinee, scopes);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    mark_expr_uses(g, scopes);
                }
                for_each_expr_in_block(&arm.body, &mut |e| mark_expr_uses(e, scopes));
            }
        }
        Expression::If(i) => {
            mark_expr_uses(&i.condition, scopes);
            for_each_expr_in_block(&i.then_block, &mut |e| mark_expr_uses(e, scopes));
            for_each_expr_in_block(&i.else_block, &mut |e| mark_expr_uses(e, scopes));
        }
        Expression::Index(i) => {
            mark_expr_uses(&i.object, scopes);
            mark_expr_uses(&i.index, scopes);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                mark_expr_uses(e, scopes);
            }
        }
        Expression::TupleLiteral(items) => {
            for e in items {
                mark_expr_uses(e, scopes);
            }
        }
        Expression::ArrayRepeat { element, .. } => mark_expr_uses(element, scopes),
        Expression::Grouped(e) | Expression::Await(e) => mark_expr_uses(e, scopes),
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let ast::TemplatePart::Interpolation(e) = part {
                    mark_expr_uses(e, scopes);
                }
            }
        }
        Expression::Cast(c) => mark_expr_uses(&c.expr, scopes),
        Expression::ArrowFn(a) => match &a.body {
            ast::ArrowBody::Expr(e) => mark_expr_uses(e, scopes),
            ast::ArrowBody::Block(b) => check_block(b, scopes, &mut vec![]),
        },
        Expression::ComptimeBlock { body, .. } => check_block(body, scopes, &mut vec![]),
        Expression::Spawn { body, .. } => check_block(body, scopes, &mut vec![]),
        Expression::ParallelSearch(ps) => {
            ps.for_each_expr(|e| mark_expr_uses(e, scopes));
            check_block(&ps.body, scopes, &mut vec![]);
        }
        Expression::Literal(_) | Expression::Invalid => {}
    }
}

fn mark_used(name: &str, scopes: &mut Vec<HashMap<String, Binding>>) {
    for scope in scopes.iter_mut().rev() {
        if let Some(binding) = scope.get_mut(name) {
            binding.used = true;
            return;
        }
    }
}

fn is_ignored_name(name: &str) -> bool {
    name.is_empty() || name.starts_with('_')
}

fn is_ignored_param(name: &str) -> bool {
    is_ignored_name(name) || name == "self"
}

fn unused_var_diag(binding: &Binding) -> NyraError {
    NyraError::coded_warning(
        "W003",
        ErrorKind::Lint,
        binding.span.clone(),
        format!("unused variable `{}`", binding.name),
    )
    .label("declared here but never read")
    .help(format!(
        "prefix with `_` if intentional: `let _{} = ...`",
        binding.name
    ))
    .help(format!(
        "or remove the binding if `{}` is not needed",
        binding.name
    ))
}
