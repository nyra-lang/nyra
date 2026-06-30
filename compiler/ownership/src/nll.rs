use std::collections::{HashMap, HashSet};

use ast::*;

/// Last statement index (0-based) where a binding is used in a block.
pub fn compute_last_uses(block: &Block) -> HashMap<String, usize> {
    let mut last = HashMap::new();
    for (idx, stmt) in block.statements.iter().enumerate() {
        record_stmt_uses(stmt, idx, &mut last);
    }
    last
}

fn record_stmt_uses(stmt: &Statement, idx: usize, last: &mut HashMap<String, usize>) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            record_expr_uses(&l.value, idx, last);
        }
        Statement::Assign(a) => {
            record_expr_uses(&a.target, idx, last);
            record_expr_uses(&a.value, idx, last);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                record_expr_uses(v, idx, last);
            }
        }
        Statement::If(i) => {
            record_expr_uses(&i.condition, idx, last);
            for inner in &i.then_block.statements {
                record_stmt_uses(inner, idx, last);
            }
            if let Some(e) = &i.else_block {
                for inner in &e.statements {
                    record_stmt_uses(inner, idx, last);
                }
            }
        }
        Statement::While(w) => {
            record_expr_uses(&w.condition, idx, last);
            for inner in &w.body.statements {
                record_stmt_uses(inner, idx, last);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| record_expr_uses(e, idx, last));
            record_binding_use(&f.var, idx, last);
            for inner in &f.body.statements {
                record_stmt_uses(inner, idx, last);
            }
        }
        Statement::Print(p) => {
            for arg in &p.args {
                record_expr_uses(arg, idx, last);
            }
            if let Some(c) = &p.color {
                record_expr_uses(c, idx, last);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            record_expr_uses(e, idx, last);
        }
        Statement::Unsafe(body) => {
            for (i, s) in body.statements.iter().enumerate() {
                record_stmt_uses(s, idx + i, last);
            }
        }
        Statement::Benchmark(body) => {
            for (i, s) in body.statements.iter().enumerate() {
                record_stmt_uses(s, idx + i, last);
            }
        }
        Statement::Spawn(body) => {
            for inner in &body.statements {
                record_stmt_uses(inner, idx, last);
            }
        }
        Statement::Asm { .. } => {}
        Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn record_binding_use(name: &str, idx: usize, last: &mut HashMap<String, usize>) {
    last.insert(name.to_string(), idx);
}

fn record_expr_uses(expr: &Expression, idx: usize, last: &mut HashMap<String, usize>) {
    match expr {
        Expression::Variable { name, .. } => record_binding_use(name, idx, last),
        Expression::Unary(u) => {
            if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
                return;
            }
            record_expr_uses(&u.operand, idx, last);
        }
        Expression::Binary(b) => {
            record_expr_uses(&b.left, idx, last);
            record_expr_uses(&b.right, idx, last);
        }
        Expression::Call(c) => {
            for a in &c.args {
                record_expr_uses(a, idx, last);
            }
        }
        Expression::MethodCall(mc) => {
            record_expr_uses(&mc.object, idx, last);
            for a in &mc.args {
                record_expr_uses(a, idx, last);
            }
        }
        Expression::If(i) => {
            record_expr_uses(&i.condition, idx, last);
            for_each_expr_in_block(&i.then_block, &mut |e| record_expr_uses(e, idx, last));
            for_each_expr_in_block(&i.else_block, &mut |e| record_expr_uses(e, idx, last));
        }
        Expression::Match(m) => {
            record_expr_uses(&m.scrutinee, idx, last);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    record_expr_uses(g, idx, last);
                }
                for_each_expr_in_block(&arm.body, &mut |e| record_expr_uses(e, idx, last));
            }
        }
        Expression::Await(inner) => record_expr_uses(inner, idx, last),
        Expression::FieldAccess(f) => record_expr_uses(&f.object, idx, last),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                record_expr_uses(spread, idx, last);
            }
            for (_, e) in &s.fields {
                record_expr_uses(e, idx, last);
            }
        }
        Expression::Grouped(g) => record_expr_uses(g, idx, last),
        Expression::Index(ix) => {
            record_expr_uses(&ix.object, idx, last);
            record_expr_uses(&ix.index, idx, last);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                record_expr_uses(e, idx, last);
            }
        }
        Expression::ArrayRepeat { element, .. } => record_expr_uses(element, idx, last),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                record_expr_uses(e, idx, last);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    record_expr_uses(e, idx, last);
                }
            }
        }
        Expression::Cast(c) => record_expr_uses(&c.expr, idx, last),
        Expression::ArrowFn(_) => {}
        Expression::ComptimeBlock { .. } => {}
        Expression::Literal(_) | Expression::EnumVariant(_) | Expression::Invalid => {}
    }
}

/// Collect free variables referenced in a block (outer captures for spawn).
pub fn collect_captures(block: &Block, declared: &std::collections::HashSet<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for stmt in &block.statements {
        collect_stmt_captures(stmt, declared, &mut seen, &mut out);
    }
    out
}

fn collect_stmt_captures(
    stmt: &Statement,
    declared: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let mut local = declared.clone();
            if l.destructure.is_empty() {
                local.insert(l.name.clone());
            } else {
                for n in &l.destructure {
                    local.insert(n.clone());
                }
            }
            collect_expr_captures(&l.value, &local, seen, out);
        }
        Statement::Assign(a) => {
            collect_expr_captures(&a.target, declared, seen, out);
            collect_expr_captures(&a.value, declared, seen, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_expr_captures(v, declared, seen, out);
            }
        }
        Statement::If(i) => {
            collect_expr_captures(&i.condition, declared, seen, out);
            for s in &i.then_block.statements {
                collect_stmt_captures(s, declared, seen, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_stmt_captures(s, declared, seen, out);
                }
            }
        }
        Statement::While(w) => {
            collect_expr_captures(&w.condition, declared, seen, out);
            for s in &w.body.statements {
                collect_stmt_captures(s, declared, seen, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_expr_captures(e, declared, seen, out));
            let mut local = declared.clone();
            local.insert(f.var.clone());
            for s in &f.body.statements {
                collect_stmt_captures(s, &local, seen, out);
            }
        }
        Statement::Print(p) => {
            for arg in &p.args {
                collect_expr_captures(arg, declared, seen, out);
            }
            if let Some(c) = &p.color {
                collect_expr_captures(c, declared, seen, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            collect_expr_captures(e, declared, seen, out);
        }
        Statement::Unsafe(body) => {
            for s in &body.statements {
                collect_stmt_captures(s, declared, seen, out);
            }
        }
        Statement::Benchmark(body) => {
            for s in &body.statements {
                collect_stmt_captures(s, declared, seen, out);
            }
        }
        Statement::Spawn(body) => {
            for s in &body.statements {
                collect_stmt_captures(s, declared, seen, out);
            }
        }
        Statement::Asm { .. } => {}
        Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn collect_expr_captures(
    expr: &Expression,
    declared: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    match expr {
        Expression::Variable { name, .. } => {
            if declared.contains(name) && seen.insert(name.clone()) {
                out.push(name.clone());
            }
        }
        Expression::Unary(u) => {
            if !matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
                collect_expr_captures(&u.operand, declared, seen, out);
            }
        }
        Expression::Binary(b) => {
            collect_expr_captures(&b.left, declared, seen, out);
            collect_expr_captures(&b.right, declared, seen, out);
        }
        Expression::Call(c) => {
            for a in &c.args {
                collect_expr_captures(a, declared, seen, out);
            }
        }
        Expression::MethodCall(mc) => {
            collect_expr_captures(&mc.object, declared, seen, out);
            for a in &mc.args {
                collect_expr_captures(a, declared, seen, out);
            }
        }
        Expression::If(i) => {
            collect_expr_captures(&i.condition, declared, seen, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_expr_captures(e, declared, seen, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_expr_captures(e, declared, seen, out));
        }
        Expression::Match(m) => {
            collect_expr_captures(&m.scrutinee, declared, seen, out);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    collect_expr_captures(g, declared, seen, out);
                }
                for_each_expr_in_block(&arm.body, &mut |e| collect_expr_captures(e, declared, seen, out));
            }
        }
        Expression::Await(inner) => collect_expr_captures(inner, declared, seen, out),
        Expression::FieldAccess(f) => collect_expr_captures(&f.object, declared, seen, out),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_expr_captures(spread, declared, seen, out);
            }
            for (_, e) in &s.fields {
                collect_expr_captures(e, declared, seen, out);
            }
        }
        Expression::Grouped(g) => collect_expr_captures(g, declared, seen, out),
        Expression::Index(ix) => {
            collect_expr_captures(&ix.object, declared, seen, out);
            collect_expr_captures(&ix.index, declared, seen, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_expr_captures(e, declared, seen, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            collect_expr_captures(element, declared, seen, out);
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_expr_captures(e, declared, seen, out);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_expr_captures(e, declared, seen, out);
                }
            }
        }
        Expression::Cast(c) => collect_expr_captures(&c.expr, declared, seen, out),
        Expression::ArrowFn(a) => {
            let mut local = declared.clone();
            for p in &a.params {
                local.insert(p.name.clone());
            }
            let block = arrow_to_block(a);
            for stmt in &block.statements {
                collect_stmt_captures(stmt, &local, seen, out);
            }
        }
        Expression::ComptimeBlock { body, .. } => {
            for stmt in &body.statements {
                collect_stmt_captures(stmt, declared, seen, out);
            }
        }
        Expression::Literal(_) | Expression::EnumVariant(_) | Expression::Invalid => {}
    }
}

/// Convert an arrow body to a block (expr body → `return expr`).
pub fn arrow_to_block(arrow: &ArrowFnExpr) -> Block {
    match &arrow.body {
        ArrowBody::Expr(e) => Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(e.clone()),
            })],
        },
        ArrowBody::Block(b) => b.clone(),
    }
}

/// Outer variables captured by an arrow (excluding parameter names).
pub fn collect_arrow_captures(arrow: &ArrowFnExpr, outer: &HashSet<String>) -> Vec<String> {
    let block = arrow_to_block(arrow);
    let mut declared = outer.clone();
    for p in &arrow.params {
        declared.insert(p.name.clone());
    }
    let param_names: HashSet<String> = arrow.params.iter().map(|p| p.name.clone()).collect();
    collect_captures(&block, &declared)
        .into_iter()
        .filter(|n| !param_names.contains(n))
        .collect()
}

/// True when the arrow body references a variable that is neither a parameter nor locally declared.
pub fn arrow_has_captures(arrow: &ArrowFnExpr) -> bool {
    let mut bound: HashSet<String> = arrow.params.iter().map(|p| p.name.clone()).collect();
    let block = arrow_to_block(arrow);
    !collect_free_vars_block(&block, &mut bound).is_empty()
}

fn collect_free_vars_block(block: &Block, bound: &mut HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for stmt in &block.statements {
        collect_free_vars_stmt(stmt, bound, &mut seen, &mut out);
    }
    out
}

fn collect_free_vars_stmt(
    stmt: &Statement,
    bound: &mut HashSet<String>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            collect_free_vars_expr(&l.value, bound, seen, out);
            if l.destructure.is_empty() {
                bound.insert(l.name.clone());
            } else {
                for n in &l.destructure {
                    bound.insert(n.clone());
                }
            }
        }
        Statement::Assign(a) => {
            collect_free_vars_expr(&a.target, bound, seen, out);
            collect_free_vars_expr(&a.value, bound, seen, out);
        }
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                collect_free_vars_expr(v, bound, seen, out);
            }
        }
        Statement::If(i) => {
            collect_free_vars_expr(&i.condition, bound, seen, out);
            for s in &i.then_block.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
            if let Some(e) = &i.else_block {
                for s in &e.statements {
                    collect_free_vars_stmt(s, bound, seen, out);
                }
            }
        }
        Statement::While(w) => {
            collect_free_vars_expr(&w.condition, bound, seen, out);
            for s in &w.body.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Statement::For(f) => {
            f.for_each_expr(|e| collect_free_vars_expr(e, bound, seen, out));
            bound.insert(f.var.clone());
            for s in &f.body.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Statement::Print(p) => {
            for arg in &p.args {
                collect_free_vars_expr(arg, bound, seen, out);
            }
            if let Some(c) = &p.color {
                collect_free_vars_expr(c, bound, seen, out);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => {
            collect_free_vars_expr(e, bound, seen, out);
        }
        Statement::Unsafe(body) => {
            for s in &body.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Statement::Benchmark(body) => {
            for s in &body.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Statement::Spawn(body) => {
            for s in &body.statements {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Statement::Asm { .. } => {}
        Statement::Import(_) | Statement::Break { .. } | Statement::Continue { .. } => {}
    }
}

fn collect_free_vars_expr(
    expr: &Expression,
    bound: &HashSet<String>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match expr {
        Expression::Variable { name, .. } => {
            if !bound.contains(name) && seen.insert(name.clone()) {
                out.push(name.clone());
            }
        }
        Expression::Unary(u) => {
            if !matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
                collect_free_vars_expr(&u.operand, bound, seen, out);
            }
        }
        Expression::Binary(b) => {
            collect_free_vars_expr(&b.left, bound, seen, out);
            collect_free_vars_expr(&b.right, bound, seen, out);
        }
        Expression::Call(c) => {
            for a in &c.args {
                collect_free_vars_expr(a, bound, seen, out);
            }
        }
        Expression::MethodCall(mc) => {
            collect_free_vars_expr(&mc.object, bound, seen, out);
            for a in &mc.args {
                collect_free_vars_expr(a, bound, seen, out);
            }
        }
        Expression::If(i) => {
            collect_free_vars_expr(&i.condition, bound, seen, out);
            for_each_expr_in_block(&i.then_block, &mut |e| collect_free_vars_expr(e, bound, seen, out));
            for_each_expr_in_block(&i.else_block, &mut |e| collect_free_vars_expr(e, bound, seen, out));
        }
        Expression::Match(m) => {
            collect_free_vars_expr(&m.scrutinee, bound, seen, out);
            for arm in &m.arms {
                if let Some(g) = &arm.guard {
                    collect_free_vars_expr(g, bound, seen, out);
                }
                for_each_expr_in_block(&arm.body, &mut |e| collect_free_vars_expr(e, bound, seen, out));
            }
        }
        Expression::Await(inner) => collect_free_vars_expr(inner, bound, seen, out),
        Expression::FieldAccess(f) => collect_free_vars_expr(&f.object, bound, seen, out),
        Expression::StructLiteral(s) => {
            for spread in &s.spreads {
                collect_free_vars_expr(spread, bound, seen, out);
            }
            for (_, e) in &s.fields {
                collect_free_vars_expr(e, bound, seen, out);
            }
        }
        Expression::Grouped(g) => collect_free_vars_expr(g, bound, seen, out),
        Expression::Index(ix) => {
            collect_free_vars_expr(&ix.object, bound, seen, out);
            collect_free_vars_expr(&ix.index, bound, seen, out);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs() {
                collect_free_vars_expr(e, bound, seen, out);
            }
        }
        Expression::ArrayRepeat { element, .. } => {
            collect_free_vars_expr(element, bound, seen, out);
        }
        Expression::TupleLiteral(elems) => {
            for e in elems {
                collect_free_vars_expr(e, bound, seen, out);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    collect_free_vars_expr(e, bound, seen, out);
                }
            }
        }
        Expression::Cast(c) => collect_free_vars_expr(&c.expr, bound, seen, out),
        Expression::ArrowFn(a) => {
            let mut inner = bound.clone();
            for p in &a.params {
                inner.insert(p.name.clone());
            }
            let block = arrow_to_block(a);
            for free in collect_free_vars_block(&block, &mut inner) {
                if seen.insert(free.clone()) {
                    out.push(free);
                }
            }
        }
        Expression::ComptimeBlock { body, .. } => {
            for free in collect_free_vars_block(body, &mut bound.clone()) {
                if seen.insert(free.clone()) {
                    out.push(free);
                }
            }
        }
        Expression::Literal(_) | Expression::EnumVariant(_) | Expression::Invalid => {}
    }
}
