//! Desugar `expr?` (Result/Option propagation) into `let` + early `return` on failure.

use std::collections::HashMap;

use ast::*;
use ast::expr_span;
use ast::IntKind;
use errors::Span;

pub fn desugar_try(program: &mut Program) {
    let fn_returns = build_fn_returns(program);
    let mut counter = 0usize;
    for f in &mut program.functions {
        f.body.statements = desugar_try_stmts(
            &f.body.statements,
            &fn_returns,
            f.return_type.as_ref(),
            &mut counter,
        );
    }
    for imp in &mut program.impls {
        for method in &mut imp.methods {
            method.body.statements = desugar_try_stmts(
                &method.body.statements,
                &fn_returns,
                method.return_type.as_ref(),
                &mut counter,
            );
        }
    }
    for ti in &mut program.trait_impls {
        for method in &mut ti.methods {
            method.body.statements = desugar_try_stmts(
                &method.body.statements,
                &fn_returns,
                method.return_type.as_ref(),
                &mut counter,
            );
        }
    }
}

struct HoistResult {
    prelude: Vec<Statement>,
    expr: Expression,
}

fn build_fn_returns(program: &Program) -> HashMap<String, TypeAnnotation> {
    let mut out = HashMap::new();
    for f in &program.functions {
        if let Some(ret) = &f.return_type {
            out.insert(f.name.clone(), ret.clone());
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TryKind {
    Result,
    Option,
}

/// How to use the success value after `?` desugar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrySuccessMode {
    /// `let x = f()?` — unwrap `Ok`/`Some` payload.
    Unwrap,
    /// `return f()?` — keep the full enum on the success path.
    KeepEnum,
}

fn try_success_expr(
    tmp_expr: &Expression,
    enum_name: &str,
    kind: TryKind,
    span: &Span,
    mode: TrySuccessMode,
) -> Expression {
    match mode {
        TrySuccessMode::KeepEnum => tmp_expr.clone(),
        TrySuccessMode::Unwrap => unwrap_ok_expr(tmp_expr, enum_name, kind, span),
    }
}

fn try_kind_for_enum(enum_name: &str) -> TryKind {
    if enum_name.starts_with("Option") {
        TryKind::Option
    } else {
        TryKind::Result
    }
}

fn enum_name_from_ann(ann: &TypeAnnotation) -> Option<String> {
    match ann {
        TypeAnnotation::Enum(n) | TypeAnnotation::Struct(n) => Some(n.clone()),
        TypeAnnotation::Applied { base, args } => {
            let suffix: String = args
                .iter()
                .map(mangle_type_for_enum)
                .collect::<Vec<_>>()
                .join("_");
            Some(format!("{base}__{suffix}"))
        }
        _ => None,
    }
}

fn mangle_type_for_enum(t: &TypeAnnotation) -> String {
    match t {
        TypeAnnotation::Integer(k) => k.name().into(),
        TypeAnnotation::F32 => "f32".into(),
        TypeAnnotation::F64 => "f64".into(),
        TypeAnnotation::Char => "char".into(),
        TypeAnnotation::Bool => "bool".into(),
        TypeAnnotation::String => "string".into(),
        TypeAnnotation::Enum(n) => format!("E_{n}"),
        TypeAnnotation::Struct(n) => format!("S_{n}"),
        TypeAnnotation::Applied { base, args } => {
            let suffix: String = args.iter().map(mangle_type_for_enum).collect::<Vec<_>>().join("_");
            format!("{base}__{suffix}")
        }
        _ => "unknown".into(),
    }
}

fn ann_from_mangled_token(tok: &str) -> Option<TypeAnnotation> {
    IntKind::parse_name(tok).map(TypeAnnotation::Integer).or_else(|| {
        match tok {
            "f32" => Some(TypeAnnotation::F32),
            "f64" => Some(TypeAnnotation::F64),
            "char" => Some(TypeAnnotation::Char),
            "bool" => Some(TypeAnnotation::Bool),
            "string" => Some(TypeAnnotation::String),
            other if other.starts_with("S_") => {
                Some(TypeAnnotation::Struct(other.trim_start_matches("S_").to_string()))
            }
            other if other.starts_with("E_") => {
                Some(TypeAnnotation::Enum(other.trim_start_matches("E_").to_string()))
            }
            other if !other.is_empty() => Some(TypeAnnotation::Enum(other.to_string())),
            _ => None,
        }
    })
}

fn ann_from_mangled_parts(parts: &[&str], start: usize) -> Option<(TypeAnnotation, usize)> {
    let tok = *parts.get(start)?;
    match tok {
        "S" => {
            let name = *parts.get(start + 1)?;
            Some((TypeAnnotation::Struct(name.to_string()), start + 2))
        }
        "E" => {
            let name = *parts.get(start + 1)?;
            Some((TypeAnnotation::Enum(name.to_string()), start + 2))
        }
        _ => ann_from_mangled_token(tok).map(|ann| (ann, start + 1)),
    }
}

fn result_payload_anns_from_mangled_suffix(
    suffix: &str,
) -> (Option<TypeAnnotation>, Option<TypeAnnotation>) {
    let parts: Vec<&str> = suffix.split('_').filter(|p| !p.is_empty()).collect();
    let Some((ok, next)) = ann_from_mangled_parts(&parts, 0) else {
        return (None, None);
    };
    let Some((err, _)) = ann_from_mangled_parts(&parts, next) else {
        return (Some(ok), None);
    };
    (Some(ok), Some(err))
}

fn ok_payload_ann_from_enum_name(enum_name: &str) -> Option<TypeAnnotation> {
    if let Some(suffix) = enum_name.strip_prefix("Result__") {
        let (ok, _) = result_payload_anns_from_mangled_suffix(suffix);
        return ok;
    }
    if let Some(suffix) = enum_name.strip_prefix("Option__") {
        let parts: Vec<&str> = suffix.split('_').filter(|p| !p.is_empty()).collect();
        return ann_from_mangled_parts(&parts, 0).map(|(ann, _)| ann);
    }
    None
}

fn zero_expr_for_ann(ann: Option<TypeAnnotation>) -> Expression {
    match ann {
        Some(TypeAnnotation::String) => Expression::Literal(Literal::String(String::new())),
        Some(TypeAnnotation::Bool) => Expression::Literal(Literal::Bool(false)),
        Some(TypeAnnotation::F32) => Expression::Literal(Literal::Float(0.0, ast::FloatKind::F32)),
        Some(TypeAnnotation::F64) => Expression::Literal(Literal::Float(0.0, ast::FloatKind::F64)),
        Some(TypeAnnotation::Char) => Expression::Literal(Literal::Char(0)),
        Some(TypeAnnotation::Integer(k)) => Expression::Literal(Literal::IntKind(0, k)),
        _ => Expression::Literal(Literal::Int(0)),
    }
}

fn success_payload_expr(bind: &str, ok_ann: Option<&TypeAnnotation>, span: &Span) -> Expression {
    let var = Expression::Variable {
        name: bind.to_string(),
        span: span.clone(),
    };
    if matches!(ok_ann, Some(TypeAnnotation::String)) {
        return Expression::MethodCall(Box::new(MethodCallExpr {
            object: var,
            method: "clone".into(),
            args: vec![],
            optional: false,
            span: span.clone(),
        }));
    }
    var
}

fn ok_err_payload_anns(
    ret: Option<&TypeAnnotation>,
) -> (Option<TypeAnnotation>, Option<TypeAnnotation>) {
    match ret {
        Some(TypeAnnotation::Applied { base, args })
            if base == "Result" && args.len() >= 2 =>
        {
            (Some(args[0].clone()), Some(args[1].clone()))
        }
        Some(TypeAnnotation::Enum(name)) if name.starts_with("Result__") => {
            if let Some(suffix) = name.strip_prefix("Result__") {
                return result_payload_anns_from_mangled_suffix(suffix);
            }
            (None, None)
        }
        _ => (None, None),
    }
}

fn payload_ann_for_variant(
    ret: Option<&TypeAnnotation>,
    variant: &str,
) -> Option<TypeAnnotation> {
    let (ok, err) = ok_err_payload_anns(ret);
    match variant {
        "Ok" | "Some" => ok,
        "Err" | "None" => err,
        _ => None,
    }
}

fn contains_try(expr: &Expression) -> bool {
    match expr {
        Expression::Unary(u) if u.op == UnaryOp::Try => true,
        Expression::Unary(u) => contains_try(&u.operand),
        Expression::Binary(b) => contains_try(&b.left) || contains_try(&b.right),
        Expression::Call(c) => c.args.iter().any(contains_try),
        Expression::MethodCall(m) => {
            contains_try(&m.object) || m.args.iter().any(contains_try)
        }
        Expression::FieldAccess(f) => contains_try(&f.object),
        Expression::Index(ix) => contains_try(&ix.object) || contains_try(&ix.index),
        Expression::Match(m) => {
            contains_try(&m.scrutinee)
                || m.arms.iter().any(|a| a.body.statements.iter().any(stmt_contains_try))
        }
        Expression::If(i) => {
            contains_try(&i.condition)
                || i.then_block.statements.iter().any(stmt_contains_try)
                || i.else_block.statements.iter().any(stmt_contains_try)
        }
        Expression::Grouped(g) => contains_try(g),
        Expression::ArrayLiteral(al) => al.all_exprs().any(contains_try),
        Expression::ArrayRepeat { element, .. } => contains_try(element),
        Expression::TupleLiteral(elems) => elems.iter().any(contains_try),
        Expression::StructLiteral(s) => {
            s.fields.iter().any(|(_, e)| contains_try(e))
                || s.spreads.iter().any(contains_try)
        }
        Expression::TemplateLiteral(t) => t.parts.iter().any(|p| {
            matches!(p, TemplatePart::Interpolation(e) if contains_try(e))
        }),
        Expression::Cast(c) => contains_try(&c.expr),
        Expression::ArrowFn(a) => match &a.body {
            ArrowBody::Expr(e) => contains_try(e),
            ArrowBody::Block(b) => b.statements.iter().any(|s| stmt_contains_try(s)),
        },
        _ => false,
    }
}

fn stmt_contains_try(stmt: &Statement) -> bool {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => contains_try(&l.value),
        Statement::Assign(a) => contains_try(&a.target) || contains_try(&a.value),
        Statement::Return(r) => r.value.as_ref().is_some_and(contains_try),
        Statement::Expression(e) | Statement::Defer(e) => contains_try(e),
        Statement::Print(p) => {
            p.args.iter().any(contains_try) || p.color.as_ref().is_some_and(contains_try)
        }
        Statement::If(i) => {
            contains_try(&i.condition)
                || i.then_block.statements.iter().any(stmt_contains_try)
                || i
                    .else_block
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_contains_try))
        }
        _ => false,
    }
}

fn infer_try_enum(expr: &Expression, fn_returns: &HashMap<String, TypeAnnotation>) -> Option<String> {
    match expr {
        Expression::EnumVariant(ev) => ev.enum_name.clone(),
        Expression::Call(c) => fn_returns.get(&c.callee).and_then(enum_name_from_ann),
        Expression::Variable { name, .. } if name.starts_with("__try_") => None,
        _ => None,
    }
}

fn desugar_try_stmts(
    stmts: &[Statement],
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    let mut out = Vec::new();
    for stmt in stmts {
        out.extend(desugar_try_stmt(stmt, fn_returns, current_fn_return, counter));
    }
    out
}

fn desugar_try_stmt(
    stmt: &Statement,
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    match stmt {
        Statement::Let(l) if is_try(&l.value) => desugar_try_binding(
            &l.name,
            l.mutable,
            l.ty.clone(),
            &l.value,
            &l.span,
            false,
            fn_returns,
            current_fn_return,
            counter,
            None,
        ),
        Statement::Const(c) if is_try(&c.value) => desugar_try_binding(
            &c.name,
            false,
            c.ty.clone(),
            &c.value,
            &c.span,
            true,
            fn_returns,
            current_fn_return,
            counter,
            None,
        ),
        Statement::Return(r) => {
            if let Some(v) = &r.value {
                if let Expression::Match(m) = v {
                    if m.arms.iter().any(|a| a.body.statements.iter().any(stmt_contains_try))
                        || contains_try(&m.scrutinee)
                    {
                        return desugar_return_match_with_try(
                            m,
                            fn_returns,
                            current_fn_return,
                            counter,
                        );
                    }
                }
                if contains_try(v) {
                    let h = hoist_try_expr(v, fn_returns, counter, TrySuccessMode::KeepEnum);
                    let mut out = h.prelude;
                    out.push(Statement::Return(ReturnStmt {
                        value: Some(h.expr),
                    }));
                    return out;
                }
            }
            vec![stmt.clone()]
        }
        Statement::Expression(e) if is_try(e) => desugar_try_expr_stmt(e, fn_returns, counter),
        Statement::Expression(e) if contains_try(e) => {
            let h = hoist_try_expr(e, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = h.prelude;
            out.push(Statement::Expression(h.expr));
            out
        }
        _ => expand_stmt_without_top_level_try(stmt, fn_returns, current_fn_return, counter),
    }
}

fn expand_stmt_without_top_level_try(
    stmt: &Statement,
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    match stmt {
        Statement::Let(l) => {
            if let Expression::Match(m) = &l.value {
                if m.arms.iter().any(|a| a.body.statements.iter().any(stmt_contains_try))
                    || contains_try(&m.scrutinee) {
                    return desugar_let_match_with_try(l, fn_returns, current_fn_return, counter);
                }
            }
            let h = hoist_try_expr(&l.value, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = h.prelude;
            out.push(Statement::Let(LetStmt {
                value: h.expr,
                ..l.clone()
            }));
            out
        }
        Statement::Const(c) => {
            let h = hoist_try_expr(&c.value, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = h.prelude;
            out.push(Statement::Const(LetStmt {
                value: h.expr,
                ..c.clone()
            }));
            out
        }
        Statement::Assign(a) => {
            let th = hoist_try_expr(&a.target, fn_returns, counter, TrySuccessMode::Unwrap);
            let vh = hoist_try_expr(&a.value, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = th.prelude;
            out.extend(vh.prelude);
            out.push(Statement::Assign(AssignStmt {
                target: th.expr,
                value: vh.expr,
                span: a.span.clone(),
            }));
            out
        }
        Statement::Print(p) => {
            let mut out = Vec::new();
            let mut args = Vec::new();
            for a in &p.args {
                let h = hoist_try_expr(a, fn_returns, counter, TrySuccessMode::Unwrap);
                out.extend(h.prelude);
                args.push(h.expr);
            }
            let color = if let Some(c) = &p.color {
                let h = hoist_try_expr(c, fn_returns, counter, TrySuccessMode::Unwrap);
                out.extend(h.prelude);
                Some(h.expr)
            } else {
                None
            };
            out.push(Statement::Print(PrintStmt { args, color }));
            out
        }
        Statement::If(i) => {
            let ch = hoist_try_expr(&i.condition, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = ch.prelude;
            out.push(Statement::If(IfStmt {
                condition: ch.expr,
                then_block: Block {
                    statements: desugar_try_stmts(
                        &i.then_block.statements,
                        fn_returns,
                        current_fn_return,
                        counter,
                    ),
                },
                else_block: i.else_block.as_ref().map(|b| Block {
                    statements: desugar_try_stmts(
                        &b.statements,
                        fn_returns,
                        current_fn_return,
                        counter,
                    ),
                }),
            }));
            out
        }
        Statement::While(w) => {
            let ch = hoist_try_expr(&w.condition, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = ch.prelude;
            out.push(Statement::While(WhileStmt {
                condition: ch.expr,
                body: Block {
                    statements: desugar_try_stmts(
                        &w.body.statements,
                        fn_returns,
                        current_fn_return,
                        counter,
                    ),
                },
            }));
            out
        }
        Statement::For(f) => {
            let mut out = Vec::new();
            let kind = match &f.kind {
                ForKind::Range { start, end } => {
                    let sh = hoist_try_expr(start, fn_returns, counter, TrySuccessMode::Unwrap);
                    let eh = hoist_try_expr(end, fn_returns, counter, TrySuccessMode::Unwrap);
                    out.extend(sh.prelude);
                    out.extend(eh.prelude);
                    ForKind::Range {
                        start: sh.expr,
                        end: eh.expr,
                    }
                }
                ForKind::Iterable { iterable } => {
                    let h = hoist_try_expr(iterable, fn_returns, counter, TrySuccessMode::Unwrap);
                    out.extend(h.prelude);
                    ForKind::Iterable { iterable: h.expr }
                }
            };
            out.push(Statement::For(ForStmt {
                var: f.var.clone(),
                parallel: f.parallel.clone(),
                progress: f.progress.clone(),
                kind,
                body: Block {
                    statements: desugar_try_stmts(
                        &f.body.statements,
                        fn_returns,
                        current_fn_return,
                        counter,
                    ),
                },
            }));
            out
        }
        Statement::Defer(e) => {
            let h = hoist_try_expr(e, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut out = h.prelude;
            out.push(Statement::Defer(h.expr));
            out
        }
        Statement::Spawn(sp) => vec![Statement::Spawn(SpawnStmt {
            kind: sp.kind,
            body: Block {
                statements: desugar_try_stmts(
                    &sp.body.statements,
                    fn_returns,
                    current_fn_return,
                    counter,
                ),
            },
        })],
        Statement::Unsafe(b) | Statement::Benchmark(b) => {
            let mut s = stmt.clone();
            if let Statement::Unsafe(ref mut blk) | Statement::Benchmark(ref mut blk) = s {
                blk.statements = desugar_try_stmts(
                    &b.statements,
                    fn_returns,
                    current_fn_return,
                    counter,
                );
            }
            vec![s]
        }
        other => vec![other.clone()],
    }
}

fn hoist_try_in_block(
    block: &Block,
    fn_returns: &HashMap<String, TypeAnnotation>,
    counter: &mut usize,
    success_mode: TrySuccessMode,
) -> (Vec<Statement>, Block) {
    let mut prelude = Vec::new();
    let mut out = Vec::new();
    for stmt in &block.statements {
        match stmt {
            Statement::Expression(e) if contains_try(e) => {
                let h = hoist_try_expr(e, fn_returns, counter, success_mode);
                prelude.extend(h.prelude);
                out.push(Statement::Expression(h.expr));
            }
            Statement::Let(l) if contains_try(&l.value) => {
                let h = hoist_try_expr(&l.value, fn_returns, counter, success_mode);
                prelude.extend(h.prelude);
                let mut nl = l.clone();
                nl.value = h.expr;
                out.push(Statement::Let(nl));
            }
            Statement::Const(c) if contains_try(&c.value) => {
                let h = hoist_try_expr(&c.value, fn_returns, counter, success_mode);
                prelude.extend(h.prelude);
                let mut nc = c.clone();
                nc.value = h.expr;
                out.push(Statement::Const(nc));
            }
            Statement::Return(r) => {
                if let Some(v) = &r.value {
                    if contains_try(v) {
                        let h = hoist_try_expr(v, fn_returns, counter, success_mode);
                        prelude.extend(h.prelude);
                        out.push(Statement::Return(ReturnStmt {
                            value: Some(h.expr),
                        }));
                        continue;
                    }
                }
                out.push(stmt.clone());
            }
            other => out.push(other.clone()),
        }
    }
    (prelude, Block { statements: out })
}

fn hoist_try_expr(
    expr: &Expression,
    fn_returns: &HashMap<String, TypeAnnotation>,
    counter: &mut usize,
    success_mode: TrySuccessMode,
) -> HoistResult {
    if let Expression::Unary(u) = expr {
        if u.op == UnaryOp::Try {
        let inner = hoist_try_expr(&u.operand, fn_returns, counter, success_mode);
        let enum_name = infer_try_enum(&inner.expr, fn_returns).unwrap_or_else(|| "Result".into());
        let kind = try_kind_for_enum(&enum_name);
        let span = u.span.clone();
        let tmp = format!("__try_{counter}");
        *counter += 1;
        let tmp_expr = Expression::Variable {
            name: tmp.clone(),
            span: span.clone(),
        };
        let mut prelude = inner.prelude;
        prelude.extend(try_prelude_stmts(
            &tmp,
            inner.expr,
            &tmp_expr,
            &enum_name,
            kind,
            &span,
        ));
        return HoistResult {
            prelude,
            expr: try_success_expr(&tmp_expr, &enum_name, kind, &span, success_mode),
        };
        }
    }

    match expr {
        Expression::Binary(b) => {
            let lh = hoist_try_expr(&b.left, fn_returns, counter, TrySuccessMode::Unwrap);
            let rh = hoist_try_expr(&b.right, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut prelude = lh.prelude;
            prelude.extend(rh.prelude);
            HoistResult {
                prelude,
                expr: Expression::Binary(Box::new(BinaryExpr {
                    left: lh.expr,
                    op: b.op,
                    right: rh.expr,
                    span: b.span.clone(),
                })),
            }
        }
        Expression::Unary(u) => {
            let inner = hoist_try_expr(&u.operand, fn_returns, counter, TrySuccessMode::Unwrap);
            HoistResult {
                prelude: inner.prelude,
                expr: Expression::Unary(Box::new(UnaryExpr {
                    op: u.op,
                    operand: inner.expr,
                    span: u.span.clone(),
                })),
            }
        }
        Expression::Call(c) => {
            let mut prelude = Vec::new();
            let args: Vec<_> = c
                .args
                .iter()
                .map(|a| {
                    let h = hoist_try_expr(a, fn_returns, counter, TrySuccessMode::Unwrap);
                    prelude.extend(h.prelude);
                    h.expr
                })
                .collect();
            HoistResult {
                prelude,
                expr: Expression::Call(CallExpr {
                    args,
                    ..c.clone()
                }),
            }
        }
        Expression::MethodCall(m) => {
            let oh = hoist_try_expr(&m.object, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut prelude = oh.prelude;
            let args: Vec<_> = m
                .args
                .iter()
                .map(|a| {
                    let h = hoist_try_expr(a, fn_returns, counter, TrySuccessMode::Unwrap);
                    prelude.extend(h.prelude);
                    h.expr
                })
                .collect();
            HoistResult {
                prelude,
                expr: Expression::MethodCall(Box::new(MethodCallExpr {
                    object: oh.expr,
                    method: m.method.clone(),
                    span: m.span.clone(),
                    args,
                    optional: m.optional,
                })),
            }
        }
        Expression::FieldAccess(f) => {
            let inner = hoist_try_expr(&f.object, fn_returns, counter, TrySuccessMode::Unwrap);
            HoistResult {
                prelude: inner.prelude,
                expr: Expression::FieldAccess(Box::new(FieldAccessExpr {
                    object: inner.expr,
                    field: f.field.clone(),
                    optional: f.optional,
                    span: f.span.clone(),
                })),
            }
        }
        Expression::Index(ix) => {
            let oh = hoist_try_expr(&ix.object, fn_returns, counter, TrySuccessMode::Unwrap);
            let ih = hoist_try_expr(&ix.index, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut prelude = oh.prelude;
            prelude.extend(ih.prelude);
            HoistResult {
                prelude,
                expr: Expression::Index(Box::new(IndexExpr {
                    object: oh.expr,
                    index: ih.expr,
                    span: ix.span.clone(),
                })),
            }
        }
        Expression::If(i) => {
            let ch = hoist_try_expr(&i.condition, fn_returns, counter, TrySuccessMode::Unwrap);
            let (tp, then_block) =
                hoist_try_in_block(&i.then_block, fn_returns, counter, TrySuccessMode::Unwrap);
            let (ep, else_block) =
                hoist_try_in_block(&i.else_block, fn_returns, counter, TrySuccessMode::Unwrap);
            let mut prelude = ch.prelude;
            prelude.extend(tp);
            prelude.extend(ep);
            HoistResult {
                prelude,
                expr: Expression::If(Box::new(IfExpr {
                    condition: ch.expr,
                    then_block,
                    else_block,
                    span: i.span.clone(),
                })),
            }
        }
        Expression::Grouped(g) => {
            let inner = hoist_try_expr(g, fn_returns, counter, TrySuccessMode::Unwrap);
            HoistResult {
                prelude: inner.prelude,
                expr: Expression::Grouped(Box::new(inner.expr)),
            }
        }
        other => HoistResult {
            prelude: Vec::new(),
            expr: other.clone(),
        },
    }
}

fn enum_inst_from_match_patterns(m: &MatchExpr) -> String {
    for arm in &m.arms {
        match &arm.pattern {
            MatchPattern::QualifiedBind(en, _, _) | MatchPattern::Qualified(en, _) => {
                return en.clone();
            }
            _ => {}
        }
    }
    "Result".into()
}

fn fn_returns_compatible_enum(
    ret: Option<&TypeAnnotation>,
    enum_inst: &str,
) -> bool {
    ret.and_then(enum_name_from_ann)
        .is_some_and(|n| {
            n == enum_inst
                || enum_inst.starts_with(&format!("{n}__"))
                || enum_inst.starts_with(&format!("{n}_"))
                || n.starts_with(&format!("{enum_inst}__"))
                || n.starts_with(&format!("{enum_inst}_"))
        })
}

fn is_failure_variant(enum_inst: &str, variant: &str) -> bool {
    if enum_inst.starts_with("Option") {
        variant == "None"
    } else {
        variant == "Err"
    }
}

fn success_variant_name(enum_inst: &str) -> &'static str {
    if enum_inst.starts_with("Option") {
        "Some"
    } else {
        "Ok"
    }
}

fn failure_variant_name(enum_inst: &str) -> &'static str {
    if enum_inst.starts_with("Option") {
        "None"
    } else {
        "Err"
    }
}

fn arm_expr_is_full_enum(expr: &Expression, enum_inst: &str) -> bool {
    match expr {
        Expression::Call { .. } => true,
        Expression::EnumVariant(ev) => ev
            .enum_name
            .as_deref()
            .is_some_and(|n| n == enum_inst || n == "Result" || n == "Option"),
        Expression::Match(_) => true,
        _ => false,
    }
}

fn strip_top_level_try(expr: &Expression) -> Expression {
    if is_try(expr) {
        unwrap_try(expr)
    } else {
        expr.clone()
    }
}

fn wrap_arm_as_enum(
    enum_inst: &str,
    variant: &str,
    payload: Expression,
    span: &Span,
) -> Expression {
    Expression::EnumVariant(EnumVariantExpr {
        enum_name: Some(enum_inst.to_string()),
        variant: variant.to_string(),
        args: vec![payload],
        span: span.clone(),
    })
}

fn arm_body_for_lifted_match(
    pattern: &MatchPattern,
    body: &Block,
    enum_inst: &str,
    span: &Span,
) -> Block {
    let trailing = block_trailing_expression(body).unwrap_or(Expression::Invalid);
    let expr = transform_arm_body_expr(pattern, &trailing, enum_inst, span);
    let mut stmts = body.statements.clone();
    if let Some(last) = stmts.last_mut() {
        match last {
            Statement::Expression(e) => *e = expr,
            Statement::Return(r) => r.value = Some(expr),
            _ => stmts.push(Statement::Expression(expr)),
        }
    } else {
        stmts.push(Statement::Expression(expr));
    }
    Block { statements: stmts }
}

fn transform_arm_body_expr(
    pattern: &MatchPattern,
    body: &Expression,
    enum_inst: &str,
    span: &Span,
) -> Expression {
    let inner = strip_top_level_try(body);
    let (enum_name, variant) = match pattern {
        MatchPattern::Qualified(en, v) => (en.clone(), v.clone()),
        MatchPattern::QualifiedBind(en, v, _) => (en.clone(), v.clone()),
        _ => {
            return wrap_arm_as_enum(
                enum_inst,
                success_variant_name(enum_inst),
                inner,
                span,
            );
        }
    };
    let resolved = if enum_name == "Result" || enum_name == "Option" {
        enum_inst.to_string()
    } else {
        enum_name
    };
    if is_failure_variant(&resolved, &variant) {
        if variant == "None" {
            return Expression::EnumVariant(EnumVariantExpr {
                enum_name: Some(enum_inst.to_string()),
                variant: "None".into(),
                args: vec![],
                span: span.clone(),
            });
        }
        if arm_expr_is_full_enum(&inner, enum_inst) {
            inner
        } else {
            wrap_arm_as_enum(
                enum_inst,
                failure_variant_name(enum_inst),
                inner,
                span,
            )
        }
    } else if arm_expr_is_full_enum(&inner, enum_inst) {
        inner
    } else {
        wrap_arm_as_enum(
            enum_inst,
            success_variant_name(enum_inst),
            inner,
            span,
        )
    }
}

fn desugar_let_match_with_try(
    l: &LetStmt,
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    let Expression::Match(m) = &l.value else {
        return vec![Statement::Let(l.clone())];
    };
    let enum_inst = current_fn_return
        .and_then(enum_name_from_ann)
        .unwrap_or_else(|| enum_inst_from_match_patterns(m));
    let sh = hoist_try_expr(&m.scrutinee, fn_returns, counter, TrySuccessMode::Unwrap);
    let mut out = sh.prelude;
    let scr = format!("__match_{counter}");
    *counter += 1;
    let span = m.span.clone();
    out.push(Statement::Let(LetStmt {
        mutable: false,
        name: scr.clone(),
        destructure: vec![],
        ty: Some(TypeAnnotation::Enum(enum_inst.clone())),
        value: sh.expr,
        span: span.clone(),
    }));
    let scr_expr = Expression::Variable {
        name: scr.clone(),
        span: span.clone(),
    };

    let lifted = format!("__lifted_{counter}");
    *counter += 1;
    let lifted_arms: Vec<MatchArm> = m
        .arms
        .iter()
        .map(|arm| MatchArm {
            pattern: resolve_match_pattern(&arm.pattern, &enum_inst),
            guard: arm.guard.clone(),
            body: arm_body_for_lifted_match(&arm.pattern, &arm.body, &enum_inst, &span),
        })
        .collect();
    out.push(Statement::Let(LetStmt {
        mutable: false,
        name: lifted.clone(),
        destructure: vec![],
        ty: Some(TypeAnnotation::Enum(enum_inst.clone())),
        value: Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(scr_expr),
            arms: lifted_arms,
            span: span.clone(),
        })),
        span: span.clone(),
    }));

    let lifted_expr = Expression::Variable {
        name: lifted.clone(),
        span: span.clone(),
    };
    if fn_returns_compatible_enum(current_fn_return, &enum_inst) {
        out.extend(desugar_try_binding(
            &l.name,
            l.mutable,
            l.ty.clone(),
            &Expression::Unary(Box::new(UnaryExpr {
                op: UnaryOp::Try,
                operand: lifted_expr,
                span: l.span.clone(),
            })),
            &l.span,
            false,
            fn_returns,
            current_fn_return,
            counter,
            Some(&enum_inst),
        ));
    } else {
        let ok_v = success_variant_name(&enum_inst);
        let fail_v = failure_variant_name(&enum_inst);
        let ok_bind = format!("__ok_{counter}");
        *counter += 1;
        let err_bind = format!("__err_{counter}");
        *counter += 1;
        out.push(Statement::Let(LetStmt {
            mutable: l.mutable,
            name: l.name.clone(),
            destructure: l.destructure.clone(),
            ty: l.ty.clone(),
            value: Expression::Match(Box::new(MatchExpr {
                scrutinee: Box::new(lifted_expr),
                arms: vec![
                    MatchArm {
                        pattern: qualified_pattern(&enum_inst, &enum_inst, ok_v, Some(&ok_bind)),
                        guard: None,
                        body: block_from_expr(Expression::Variable {
                            name: ok_bind,
                            span: l.span.clone(),
                        }),
                    },
                    MatchArm {
                        pattern: qualified_pattern(&enum_inst, &enum_inst, fail_v, Some(&err_bind)),
                        guard: None,
                        body: block_from_expr(Expression::Variable {
                            name: err_bind,
                            span: l.span.clone(),
                        }),
                    },
                ],
                span: l.span.clone(),
            })),
            span: l.span.clone(),
        }));
    }
    out
}

fn desugar_return_match_with_try(
    m: &MatchExpr,
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    let enum_inst = current_fn_return
        .and_then(enum_name_from_ann)
        .unwrap_or_else(|| enum_inst_from_match_patterns(m));
    let sh = hoist_try_expr(&m.scrutinee, fn_returns, counter, TrySuccessMode::Unwrap);
    let mut out = sh.prelude;
    let scr = format!("__match_{counter}");
    *counter += 1;
    let span = m.span.clone();
    out.push(Statement::Let(LetStmt {
        mutable: false,
        name: scr.clone(),
        destructure: vec![],
        ty: Some(TypeAnnotation::Enum(enum_inst.clone())),
        value: sh.expr,
        span: span.clone(),
    }));
    let scr_expr = Expression::Variable {
        name: scr.clone(),
        span: span.clone(),
    };

    let mut else_stmts: Vec<Statement> = vec![Statement::Return(ReturnStmt { value: None })];
    for arm in m.arms.iter().rev() {
        let mut then_stmts = Vec::new();
        bind_pattern_vars(
            &mut then_stmts,
            &scr_expr,
            &arm.pattern,
            &enum_inst,
            current_fn_return,
            &span,
        );
        let (prelude, hoisted) =
            hoist_try_in_block(&arm.body, fn_returns, counter, TrySuccessMode::KeepEnum);
        let mut ret_expr = block_trailing_expression(&hoisted).unwrap_or(Expression::Invalid);
        resolve_generic_enum_variants(&mut ret_expr, &enum_inst);
        then_stmts.extend(prelude);
        then_stmts.push(Statement::Return(ReturnStmt {
            value: Some(ret_expr),
        }));
        else_stmts = vec![Statement::If(IfStmt {
            condition: pattern_matches_expr(&scr_expr, &arm.pattern, &enum_inst, &span),
            then_block: Block {
                statements: then_stmts,
            },
            else_block: Some(Block {
                statements: else_stmts,
            }),
        })];
    }
    out.extend(else_stmts);
    out
}

fn resolve_generic_enum_variants(expr: &mut Expression, inst: &str) {
    if let Expression::EnumVariant(ev) = expr {
        if matches!(ev.enum_name.as_deref(), Some("Result") | Some("Option")) {
            ev.enum_name = Some(inst.to_string());
        }
    }
    match expr {
        Expression::Call(c) => {
            for a in &mut c.args {
                resolve_generic_enum_variants(a, inst);
            }
        }
        Expression::Binary(b) => {
            resolve_generic_enum_variants(&mut b.left, inst);
            resolve_generic_enum_variants(&mut b.right, inst);
        }
        _ => {}
    }
}

fn pattern_matches_expr(
    scrutinee: &Expression,
    pattern: &MatchPattern,
    enum_inst: &str,
    span: &Span,
) -> Expression {
    let resolved = resolve_match_pattern(pattern, enum_inst);
    Expression::Binary(Box::new(BinaryExpr {
        left: Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(scrutinee.clone()),
            arms: vec![
                MatchArm {
                    pattern: resolved,
                    guard: None,
                    body: block_from_expr(Expression::Literal(Literal::Int(1))),
                },
                MatchArm {
                    pattern: MatchPattern::Wildcard,
                    guard: None,
                    body: block_from_expr(Expression::Literal(Literal::Int(0))),
                },
            ],
            span: span.clone(),
        })),
        op: BinaryOp::Eq,
        right: Expression::Literal(Literal::Int(1)),
        span: span.clone(),
    }))
}

fn resolve_match_pattern(pattern: &MatchPattern, enum_inst: &str) -> MatchPattern {
    match pattern {
        MatchPattern::Qualified(en, v) if en == "Result" || en == "Option" => {
            MatchPattern::Qualified(enum_inst.to_string(), v.clone())
        }
        MatchPattern::QualifiedBind(en, v, b) if en == "Result" || en == "Option" => {
            MatchPattern::QualifiedBind(enum_inst.to_string(), v.clone(), b.clone())
        }
        other => other.clone(),
    }
}

fn bind_pattern_vars(
    stmts: &mut Vec<Statement>,
    scrutinee: &Expression,
    pattern: &MatchPattern,
    enum_inst: &str,
    current_fn_return: Option<&TypeAnnotation>,
    span: &Span,
) {
    let (enum_name, variant, bind) = match pattern {
        MatchPattern::QualifiedBind(en, v, MatchPayloadPattern::Bind(b)) => {
            (en.clone(), v.clone(), b.clone())
        }
        MatchPattern::Qualified(en, v) => (en.clone(), v.clone(), "_".into()),
        _ => return,
    };
    if bind == "_" {
        return;
    }
    let resolved = if enum_name == "Result" || enum_name == "Option" {
        enum_inst.to_string()
    } else {
        enum_name
    };
    stmts.push(Statement::Let(LetStmt {
        mutable: false,
        name: bind.clone(),
        destructure: vec![],
        ty: payload_ann_for_variant(current_fn_return, &variant),
        value: unwrap_variant_payload(scrutinee, &resolved, &variant, &bind, span),
        span: span.clone(),
    }));
}

fn unwrap_variant_payload(
    scrutinee: &Expression,
    enum_name: &str,
    variant: &str,
    bind: &str,
    span: &Span,
) -> Expression {
    Expression::Match(Box::new(MatchExpr {
        scrutinee: Box::new(scrutinee.clone()),
        arms: vec![
            MatchArm {
                pattern: qualified_pattern(enum_name, enum_name, variant, Some(bind)),
                guard: None,
                body: block_from_expr(Expression::Variable {
                    name: bind.to_string(),
                    span: span.clone(),
                }),
            },
            MatchArm {
                pattern: MatchPattern::Wildcard,
                guard: None,
                body: block_from_expr(Expression::Literal(Literal::Int(0))),
            },
        ],
        span: span.clone(),
    }))
}

fn try_prelude_stmts(
    tmp: &str,
    value: Expression,
    tmp_expr: &Expression,
    enum_name: &str,
    kind: TryKind,
    span: &Span,
) -> Vec<Statement> {
    vec![
        Statement::Let(LetStmt {
            mutable: false,
            name: tmp.to_string(),
            destructure: vec![],
            ty: Some(TypeAnnotation::Enum(enum_name.to_string())),
            value,
            span: span.clone(),
        }),
        Statement::Let(LetStmt {
            mutable: false,
            name: format!("{tmp}_fail"),
            destructure: vec![],
            ty: Some(TypeAnnotation::Integer(IntKind::I32)),
            value: is_failure_match(tmp_expr, enum_name, kind, span),
            span: span.clone(),
        }),
        Statement::If(IfStmt {
            condition: Expression::Binary(Box::new(BinaryExpr {
                left: Expression::Variable {
                    name: format!("{tmp}_fail"),
                    span: span.clone(),
                },
                op: BinaryOp::Eq,
                right: Expression::Literal(Literal::Int(1)),
                span: span.clone(),
            })),
            then_block: Block {
                statements: vec![Statement::Return(ReturnStmt {
                    value: Some(tmp_expr.clone()),
                })],
            },
            else_block: None,
        }),
    ]
}

fn desugar_try_binding(
    name: &str,
    mutable: bool,
    ty: Option<TypeAnnotation>,
    value: &Expression,
    span: &Span,
    is_const: bool,
    fn_returns: &HashMap<String, TypeAnnotation>,
    current_fn_return: Option<&TypeAnnotation>,
    counter: &mut usize,
    enum_hint: Option<&str>,
) -> Vec<Statement> {
    let inner = unwrap_try(value);
    let enum_name = enum_hint
        .map(str::to_string)
        .or_else(|| infer_try_enum(&inner, fn_returns))
        .unwrap_or_else(|| "Result".into());
    let kind = try_kind_for_enum(&enum_name);
    let tmp = format!("__try_{counter}");
    *counter += 1;
    let tmp_expr = Expression::Variable {
        name: tmp.clone(),
        span: span.clone(),
    };
    let bind_ty = ty.or_else(|| payload_ann_for_variant(current_fn_return, "Ok"));
    let bind_stmt = if is_const {
        Statement::Const(LetStmt {
            mutable: false,
            name: name.to_string(),
            destructure: vec![],
            ty: bind_ty,
            value: unwrap_ok_expr(&tmp_expr, &enum_name, kind, span),
            span: span.clone(),
        })
    } else {
        Statement::Let(LetStmt {
            mutable,
            name: name.to_string(),
            destructure: vec![],
            ty: bind_ty,
            value: unwrap_ok_expr(&tmp_expr, &enum_name, kind, span),
            span: span.clone(),
        })
    };
    let mut out = try_prelude_stmts(&tmp, inner, &tmp_expr, &enum_name, kind, span);
    out.push(bind_stmt);
    out
}

fn desugar_try_expr_stmt(
    expr: &Expression,
    fn_returns: &HashMap<String, TypeAnnotation>,
    counter: &mut usize,
) -> Vec<Statement> {
    let inner = unwrap_try(expr);
    let enum_name = infer_try_enum(&inner, fn_returns).unwrap_or_else(|| "Result".into());
    let kind = try_kind_for_enum(&enum_name);
    let tmp = format!("__try_{counter}");
    *counter += 1;
    let tmp_expr = Expression::Variable {
        name: tmp.clone(),
        span: expr_span(expr),
    };
    try_prelude_stmts(&tmp, inner, &tmp_expr, &enum_name, kind, &expr_span(expr))
}

fn is_try(expr: &Expression) -> bool {
    matches!(expr, Expression::Unary(u) if u.op == UnaryOp::Try)
}

fn unwrap_try(expr: &Expression) -> Expression {
    match expr {
        Expression::Unary(u) if u.op == UnaryOp::Try => u.operand.clone(),
        other => other.clone(),
    }
}

fn failure_variant(kind: TryKind) -> (&'static str, &'static str) {
    match kind {
        TryKind::Result => ("Result", "Err"),
        TryKind::Option => ("Option", "None"),
    }
}

fn success_variant(kind: TryKind) -> (&'static str, &'static str) {
    match kind {
        TryKind::Result => ("Result", "Ok"),
        TryKind::Option => ("Option", "Some"),
    }
}

fn qualified_pattern(
    enum_name: &str,
    base: &str,
    variant: &str,
    bind: Option<&str>,
) -> MatchPattern {
    let resolved = if enum_name == base
        || enum_name.starts_with(&format!("{base}__"))
        || enum_name.starts_with(&format!("{base}_"))
    {
        enum_name
    } else {
        base
    };
    if let Some(b) = bind {
        MatchPattern::QualifiedBind(
            resolved.to_string(),
            variant.to_string(),
            MatchPayloadPattern::Bind(b.to_string()),
        )
    } else {
        MatchPattern::Qualified(resolved.to_string(), variant.to_string())
    }
}

fn is_failure_match(value: &Expression, enum_name: &str, kind: TryKind, span: &Span) -> Expression {
    let (base, fail_v) = failure_variant(kind);
    let (ok_base, ok_v) = success_variant(kind);
    Expression::Match(Box::new(MatchExpr {
        scrutinee: Box::new(value.clone()),
        arms: vec![
            MatchArm {
                pattern: qualified_pattern(enum_name, base, fail_v, Some("_")),
                guard: None,
                body: block_from_expr(Expression::Literal(Literal::Int(1))),
            },
            MatchArm {
                pattern: qualified_pattern(enum_name, ok_base, ok_v, Some("_")),
                guard: None,
                body: block_from_expr(Expression::Literal(Literal::Int(0))),
            },
        ],
        span: span.clone(),
    }))
}

fn unwrap_ok_expr(value: &Expression, enum_name: &str, kind: TryKind, span: &Span) -> Expression {
    let (base, ok_v) = success_variant(kind);
    let (fail_base, fail_v) = failure_variant(kind);
    let bind = "__ok".to_string();
    let ok_ann = ok_payload_ann_from_enum_name(enum_name);
    let fallback = zero_expr_for_ann(ok_ann.clone());
    let success = success_payload_expr(&bind, ok_ann.as_ref(), span);
    Expression::Match(Box::new(MatchExpr {
        scrutinee: Box::new(value.clone()),
        arms: vec![
            MatchArm {
                pattern: qualified_pattern(enum_name, base, ok_v, Some(&bind)),
                guard: None,
                body: block_from_expr(success),
            },
            MatchArm {
                pattern: qualified_pattern(enum_name, fail_base, fail_v, Some("_")),
                guard: None,
                body: block_from_expr(fallback),
            },
        ],
        span: span.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_expand_try(src: &str) -> Program {
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        crate::expand_program(&mut program);
        monomorph::monomorphize_program(&mut program);
        desugar_try(&mut program);
        program
    }

    #[test]
    fn desugars_result_question_on_let_binding() {
        let src = r#"enum Result_i32_i32 { Ok(i32), Err(i32) }
fn step(n: i32) -> Result_i32_i32 { return Result_i32_i32.Ok(n) }
fn main() -> Result_i32_i32 {
    let a = step(1)?
    return Result_i32_i32.Ok(a)
}"#;
        let program = parse_and_expand_try(src);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        assert!(
            main.body
                .statements
                .iter()
                .any(|s| matches!(s, Statement::If(_))),
            "expected early-return if after ? desugar"
        );
        assert!(
            !format!("{main:?}").contains("UnaryOp::Try"),
            "Try operator should be desugared away"
        );
    }

    #[test]
    fn desugars_try_in_call_argument() {
        let src = r#"enum Result_i32_i32 { Ok(i32), Err(i32) }
fn step(n: i32) -> Result_i32_i32 { return Result_i32_i32.Ok(n) }
fn main() -> Result_i32_i32 {
    print(step(1)?)
    return Result_i32_i32.Ok(0)
}"#;
        let program = parse_and_expand_try(src);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        assert!(
            !format!("{main:?}").contains("UnaryOp::Try"),
            "nested ? in print arg should desugar"
        );
    }

    #[test]
    fn desugars_return_match_arm_with_try() {
        let src = r#"fn step(n: i32) -> Result<i32, i32> { return Result.Ok(n) }
fn main() -> Result<i32, i32> {
    let res = step(1)
    return match res {
        Result.Ok(x) => step(x)?
        Result.Err(e) => Result.Err(e)
    }
}"#;
        let program = parse_and_expand_try(src);
        let main = program.functions.iter().find(|f| f.name == "main").unwrap();
        let body = format!("{main:?}");
        assert!(
            !body.contains("UnaryOp::Try"),
            "try should be desugared: {body}"
        );
        assert!(
            !body.contains("Expression::Match"),
            "return match should become if-chain: {body}"
        );
    }

    #[test]
    fn desugars_let_match_with_try_in_result_fn() {
        let src = r#"enum Result_i32_i32 { Ok(i32), Err(i32) }
fn ok_step(n: i32) -> Result_i32_i32 { return Result_i32_i32.Ok(n) }
fn pipeline() -> Result_i32_i32 {
    let n = match Result_i32_i32.Ok(1) {
        Result_i32_i32.Ok(v) => ok_step(v)?
        Result_i32_i32.Err(e) => e
    }
    return Result_i32_i32.Ok(n)
}"#;
        let program = parse_and_expand_try(src);
        let pipeline = program.functions.iter().find(|f| f.name == "pipeline").unwrap();
        let body = format!("{pipeline:?}");
        assert!(!body.contains("UnaryOp::Try"), "try in let-match should desugar: {body}");
        assert!(
            body.contains("__lifted_"),
            "expected lifted Result match: {body}"
        );
    }

    #[test]
    fn desugars_generic_result_err_in_if() {
        let src = r#"fn fail_if_zero(n: i32) -> Result<i32, i32> {
    if n == 0 {
        return Result.Err(1)
    }
    return Result.Ok(n)
}"#;
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        monomorph::monomorphize_program(&mut program);
        let ev = program.functions[0].body.statements[0].clone();
        if let Statement::If(i) = ev {
            let ret = &i.then_block.statements[0];
            if let Statement::Return(r) = ret {
                if let Some(Expression::EnumVariant(ev)) = &r.value {
                    assert!(
                        ev.enum_name.as_deref() == Some("Result__i32_i32"),
                        "expected full Result inst, got {:?}",
                        ev.enum_name
                    );
                }
            }
        }
    }
}
