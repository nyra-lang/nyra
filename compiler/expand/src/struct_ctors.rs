//! Desugar `User("Ahmed")` / `User()` into struct literals.

use std::collections::HashMap;

use ast::*;
use errors::Span;

fn default_for_type(ty: &TypeAnnotation) -> Expression {
    match ty {
        TypeAnnotation::Integer(ast::IntKind::I32) | TypeAnnotation::Integer(ast::IntKind::I64) | TypeAnnotation::Integer(ast::IntKind::U32) => {
            Expression::Literal(Literal::Int(0))
        }
        TypeAnnotation::F32 => Expression::Literal(Literal::Float(0.0, FloatKind::F32)),
        TypeAnnotation::F64 => Expression::Literal(Literal::Float(0.0, FloatKind::F64)),
        TypeAnnotation::Char => Expression::Literal(Literal::Char(0)),
        TypeAnnotation::Bool => Expression::Literal(Literal::Bool(false)),
        TypeAnnotation::String => Expression::Literal(Literal::String(String::new())),
        _ => Expression::Literal(Literal::Int(0)),
    }
}

fn desugar_call_to_struct(
    callee: &str,
    args: Vec<Expression>,
    span: Span,
    structs: &HashMap<String, StructDef>,
    functions: &HashMap<String, ()>,
) -> Option<Expression> {
    if functions.contains_key(callee) {
        return None;
    }
    let sdef = structs.get(callee)?;
    let mut fields = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let fname = sdef.fields.get(i)?.name.clone();
        fields.push((fname, arg.clone()));
    }
    for f in sdef.fields.iter().skip(args.len()) {
        fields.push((f.name.clone(), default_for_type(&f.ty)));
    }
    Some(Expression::StructLiteral(StructLiteralExpr {
        name: callee.to_string(),
        spreads: vec![],
        fields,
        span,
    }))
}

fn rewrite_expr(
    expr: &mut Expression,
    structs: &HashMap<String, StructDef>,
    functions: &HashMap<String, ()>,
) {
    match expr {
        Expression::Call(c) if c.type_args.is_empty() && structs.contains_key(&c.callee) => {
            if let Some(sl) =
                desugar_call_to_struct(&c.callee, c.args.clone(), c.span.clone(), structs, functions)
            {
                *expr = sl;
                rewrite_expr(expr, structs, functions);
                return;
            }
        }
        Expression::Call(c) => {
            for a in &mut c.args {
                rewrite_expr(a, structs, functions);
            }
        }
        Expression::Binary(b) => {
            rewrite_expr(&mut b.left, structs, functions);
            rewrite_expr(&mut b.right, structs, functions);
        }
        Expression::Unary(u) => rewrite_expr(&mut u.operand, structs, functions),
        Expression::Grouped(g) => rewrite_expr(g, structs, functions),
        Expression::If(i) => {
            rewrite_expr(&mut i.condition, structs, functions);
            for_each_expr_in_block_mut(&mut i.then_block, &mut |e| rewrite_expr(e, structs, functions));
            for_each_expr_in_block_mut(&mut i.else_block, &mut |e| rewrite_expr(e, structs, functions));
        }
        Expression::Match(m) => {
            rewrite_expr(&mut m.scrutinee, structs, functions);
            for arm in &mut m.arms {
                if let Some(g) = &mut arm.guard {
                    rewrite_expr(g, structs, functions);
                }
                for_each_expr_in_block_mut(&mut arm.body, &mut |e| rewrite_expr(e, structs, functions));
            }
        }
        Expression::Await(e) => rewrite_expr(e, structs, functions),
        Expression::MethodCall(mc) => {
            rewrite_expr(&mut mc.object, structs, functions);
            for a in &mut mc.args {
                rewrite_expr(a, structs, functions);
            }
        }
        Expression::FieldAccess(f) => rewrite_expr(&mut f.object, structs, functions),
        Expression::Index(ix) => {
            rewrite_expr(&mut ix.object, structs, functions);
            rewrite_expr(&mut ix.index, structs, functions);
        }
        Expression::ArrayLiteral(al) => {
            for e in al.all_exprs_mut() {
                rewrite_expr(e, structs, functions);
            }
        }
        Expression::ArrayRepeat { element, .. } => rewrite_expr(element, structs, functions),
        Expression::TupleLiteral(elems) => {
            for e in elems {
                rewrite_expr(e, structs, functions);
            }
        }
        Expression::StructLiteral(s) => {
            for spread in &mut s.spreads {
                rewrite_expr(spread, structs, functions);
            }
            for (_, e) in &mut s.fields {
                rewrite_expr(e, structs, functions);
            }
        }
        Expression::TemplateLiteral(t) => {
            for part in &mut t.parts {
                if let TemplatePart::Interpolation(e) = part {
                    rewrite_expr(e, structs, functions);
                }
            }
        }
        Expression::Cast(c) => rewrite_expr(&mut c.expr, structs, functions),
        _ => {}
    }
}

fn rewrite_stmt(
    stmt: &mut Statement,
    structs: &HashMap<String, StructDef>,
    functions: &HashMap<String, ()>,
) {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => rewrite_expr(&mut l.value, structs, functions),
        Statement::Assign(a) => {
            rewrite_expr(&mut a.target, structs, functions);
            rewrite_expr(&mut a.value, structs, functions);
        }
        Statement::Return(r) => {
            if let Some(v) = &mut r.value {
                rewrite_expr(v, structs, functions);
            }
        }
        Statement::If(i) => {
            rewrite_expr(&mut i.condition, structs, functions);
            for s in &mut i.then_block.statements {
                rewrite_stmt(s, structs, functions);
            }
            if let Some(e) = &mut i.else_block {
                for s in &mut e.statements {
                    rewrite_stmt(s, structs, functions);
                }
            }
        }
        Statement::While(w) => {
            rewrite_expr(&mut w.condition, structs, functions);
            for s in &mut w.body.statements {
                rewrite_stmt(s, structs, functions);
            }
        }
        Statement::For(f) => {
            f.map_exprs_mut(|e| rewrite_expr(e, structs, functions));
            for s in &mut f.body.statements {
                rewrite_stmt(s, structs, functions);
            }
        }
        Statement::Expression(e) | Statement::Defer(e) => rewrite_expr(e, structs, functions),
        Statement::Print(p) => {
            for a in &mut p.args {
                rewrite_expr(a, structs, functions);
            }
            if let Some(c) = &mut p.color {
                rewrite_expr(c, structs, functions);
            }
        }
        Statement::Spawn(s) => {
            for stmt in &mut s.body.statements {
                rewrite_stmt(stmt, structs, functions);
            }
        }
        Statement::Benchmark(b) => {
            for s in &mut b.statements {
                rewrite_stmt(s, structs, functions);
            }
        }
        Statement::Unsafe(b) => {
            for s in &mut b.statements {
                rewrite_stmt(s, structs, functions);
            }
        }
        _ => {}
    }
}

pub fn desugar_struct_constructors(program: &mut Program) {
    let structs: HashMap<String, StructDef> = program
        .structs
        .iter()
        .filter(|s| s.type_params.is_empty())
        .map(|s| (s.name.clone(), s.clone()))
        .collect();
    let functions: HashMap<String, ()> = program
        .functions
        .iter()
        .map(|f| (f.name.clone(), ()))
        .collect();
    for f in &mut program.functions {
        for stmt in &mut f.body.statements {
            rewrite_stmt(stmt, &structs, &functions);
        }
    }
}
