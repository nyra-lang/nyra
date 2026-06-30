use std::collections::HashMap;

use ast::*;
use ownership::arrow_has_captures;

fn expr_has_arrow(expr: &Expression) -> bool {
    match expr {
        Expression::ArrowFn(_) => true,
        Expression::Binary(b) => expr_has_arrow(&b.left) || expr_has_arrow(&b.right),
        Expression::Unary(u) => expr_has_arrow(&u.operand),
        Expression::Call(c) => c.args.iter().any(expr_has_arrow),
        Expression::MethodCall(m) => {
            expr_has_arrow(&m.object) || m.args.iter().any(expr_has_arrow)
        }
        Expression::FieldAccess(f) => expr_has_arrow(&f.object),
        Expression::StructLiteral(s) => {
            s.spreads.iter().any(|b| expr_has_arrow(b))
                || s.fields.iter().any(|(_, v)| expr_has_arrow(v))
        }
        Expression::EnumVariant(e) => e.args.iter().any(expr_has_arrow),
        Expression::Match(m) => {
            expr_has_arrow(&m.scrutinee)
                || m.arms.iter().any(|a| {
                    a.guard.as_ref().is_some_and(expr_has_arrow) || block_has_arrow(&a.body)
                })
        }
        Expression::If(i) => {
            expr_has_arrow(&i.condition)
                || block_has_arrow(&i.then_block)
                || block_has_arrow(&i.else_block)
        }
        Expression::Index(ix) => expr_has_arrow(&ix.object) || expr_has_arrow(&ix.index),
        Expression::ArrayLiteral(al) => al.all_exprs().any(expr_has_arrow),
        Expression::ArrayRepeat { element, .. } => expr_has_arrow(element),
        Expression::TupleLiteral(elems) => elems.iter().any(expr_has_arrow),
        Expression::Grouped(inner) => expr_has_arrow(inner),
        Expression::Await(inner) => expr_has_arrow(inner),
        Expression::TemplateLiteral(t) => t.parts.iter().any(|p| {
            matches!(p, TemplatePart::Interpolation(e) if expr_has_arrow(e))
        }),
        Expression::Cast(c) => expr_has_arrow(&c.expr),
        _ => false,
    }
}

fn desugar_expr(expr: &Expression, program: &mut Program, counter: &mut usize) -> Expression {
    match expr {
        Expression::ArrowFn(arrow) if !arrow_has_captures(arrow) => {
            let name = format!("__arrow_{counter}");
            *counter += 1;
            let body = match &arrow.body {
                ArrowBody::Expr(e) => Block {
                    statements: vec![Statement::Return(ReturnStmt {
                        value: Some(desugar_expr(e, program, counter)),
                    })],
                },
                ArrowBody::Block(b) => desugar_block(b, program, counter),
            };
            program.functions.push(Function {
                name: name.clone(),
                doc: None,
                is_test: false,
                ignore_test: false,
                should_fail_test: false,
                is_async: false,
                exported: false,
        public: false,
                span: arrow.span.clone(),
                type_params: vec![],
                type_param_bounds: HashMap::new(),
                lifetime_params: vec![],
                params: arrow.params.clone(),
                return_type: None,
                body,
                inline: false,
                hot: false,
                cold: false,
                comptime: false,
            });
            Expression::Variable {
                name,
                span: arrow.span.clone(),
            }
        }
        Expression::Binary(b) => Expression::Binary(Box::new(BinaryExpr {
            left: desugar_expr(&b.left, program, counter),
            op: b.op,
            right: desugar_expr(&b.right, program, counter),
            span: b.span.clone(),
        })),
        Expression::Unary(u) => Expression::Unary(Box::new(UnaryExpr {
            op: u.op,
            operand: desugar_expr(&u.operand, program, counter),
            span: u.span.clone(),
        })),
        Expression::Call(c) => Expression::Call(CallExpr {
            callee: c.callee.clone(),
            type_args: c.type_args.clone(),
            args: c
                .args
                .iter()
                .map(|a| desugar_expr(a, program, counter))
                .collect(),
            span: c.span.clone(),
        }),
        Expression::MethodCall(m) => Expression::MethodCall(Box::new(MethodCallExpr {
            object: desugar_expr(&m.object, program, counter),
            method: m.method.clone(),
            span: m.span.clone(),
            args: m
                .args
                .iter()
                .map(|a| desugar_expr(a, program, counter))
                .collect(),
            optional: m.optional,
        })),
        Expression::FieldAccess(f) => Expression::FieldAccess(Box::new(FieldAccessExpr {
            object: desugar_expr(&f.object, program, counter),
            field: f.field.clone(),
            optional: f.optional,
            span: f.span.clone(),
        })),
        Expression::StructLiteral(s) => Expression::StructLiteral(StructLiteralExpr {
            name: s.name.clone(),
            spreads: s
                .spreads
                .iter()
                .map(|b| desugar_expr(b, program, counter))
                .collect(),
            fields: s
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), desugar_expr(v, program, counter)))
                .collect(),
            span: s.span.clone(),
        }),
        Expression::EnumVariant(e) => Expression::EnumVariant(EnumVariantExpr {
            enum_name: e.enum_name.clone(),
            variant: e.variant.clone(),
            args: e
                .args
                .iter()
                .map(|a| desugar_expr(a, program, counter))
                .collect(),
            span: e.span.clone(),
        }),
        Expression::Match(m) => Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(desugar_expr(&m.scrutinee, program, counter)),
            arms: m
                .arms
                .iter()
                .map(|a| MatchArm {
                    pattern: a.pattern.clone(),
                    guard: a.guard.as_ref().map(|g| desugar_expr(g, program, counter)),
                    body: desugar_block(&a.body, program, counter),
                })
                .collect(),
            span: m.span.clone(),
        })),
        Expression::If(i) => Expression::If(Box::new(IfExpr {
            condition: desugar_expr(&i.condition, program, counter),
            then_block: desugar_block(&i.then_block, program, counter),
            else_block: desugar_block(&i.else_block, program, counter),
            span: i.span.clone(),
        })),
        Expression::Index(ix) => Expression::Index(Box::new(IndexExpr {
            object: desugar_expr(&ix.object, program, counter),
            index: desugar_expr(&ix.index, program, counter),
            span: ix.span.clone(),
        })),
        Expression::ArrayLiteral(al) => Expression::ArrayLiteral(ArrayLiteralExpr {
            spreads: al
                .spreads
                .iter()
                .map(|e| desugar_expr(e, program, counter))
                .collect(),
            elems: al
                .elems
                .iter()
                .map(|e| desugar_expr(e, program, counter))
                .collect(),
            span: al.span.clone(),
        }),
        Expression::ArrayRepeat {
            element,
            count,
            count_from,
            count_expr,
            span,
        } => Expression::ArrayRepeat {
            element: Box::new(desugar_expr(element, program, counter)),
            count: *count,
            count_from: count_from.clone(),
            count_expr: count_expr
                .as_ref()
                .map(|e| Box::new(desugar_expr(e, program, counter))),
            span: span.clone(),
        },
        Expression::TupleLiteral(elems) => Expression::TupleLiteral(
            elems
                .iter()
                .map(|e| desugar_expr(e, program, counter))
                .collect(),
        ),
        Expression::Grouped(inner) => {
            Expression::Grouped(Box::new(desugar_expr(inner, program, counter)))
        }
        Expression::Await(inner) => Expression::Await(Box::new(desugar_expr(inner, program, counter))),
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteralExpr {
            parts: t
                .parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Static(s) => TemplatePart::Static(s.clone()),
                    TemplatePart::Interpolation(e) => {
                        TemplatePart::Interpolation(Box::new(desugar_expr(e, program, counter)))
                    }
                })
                .collect(),
            span: t.span.clone(),
        }),
        Expression::Cast(c) => Expression::Cast(Box::new(CastExpr {
            expr: desugar_expr(&c.expr, program, counter),
            target_type: c.target_type.clone(),
            span: c.span.clone(),
        })),
        other => other.clone(),
    }
}

fn desugar_block(block: &Block, program: &mut Program, counter: &mut usize) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|s| desugar_stmt(s, program, counter))
            .collect(),
    }
}

fn desugar_stmt(stmt: &Statement, program: &mut Program, counter: &mut usize) -> Statement {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let mut s = stmt.clone();
            if let Statement::Let(ref mut x) | Statement::Const(ref mut x) = s {
                x.value = desugar_expr(&l.value, program, counter);
            }
            s
        }
        Statement::Assign(a) => Statement::Assign(AssignStmt {
            target: desugar_expr(&a.target, program, counter),
            value: desugar_expr(&a.value, program, counter),
            span: a.span.clone(),
        }),
        Statement::Return(r) => Statement::Return(ReturnStmt {
            value: r.value.as_ref().map(|v| desugar_expr(v, program, counter)),
        }),
        Statement::Expression(e) => Statement::Expression(desugar_expr(e, program, counter)),
        Statement::Print(p) => Statement::Print(
            p.clone().map_expressions(|a| desugar_expr(&a, program, counter)),
        ),
        Statement::Defer(e) => Statement::Defer(desugar_expr(e, program, counter)),
        Statement::If(i) => Statement::If(IfStmt {
            condition: desugar_expr(&i.condition, program, counter),
            then_block: desugar_block(&i.then_block, program, counter),
            else_block: i
                .else_block
                .as_ref()
                .map(|b| desugar_block(b, program, counter)),
        }),
        Statement::While(w) => Statement::While(WhileStmt {
            condition: desugar_expr(&w.condition, program, counter),
            body: desugar_block(&w.body, program, counter),
        }),
        Statement::For(f) => Statement::For(ForStmt {
            var: f.var.clone(),
            parallel: f.parallel.clone(),
            progress: f.progress.clone(),
            kind: match &f.kind {
                ForKind::Range { start, end } => ForKind::Range {
                    start: desugar_expr(start, program, counter),
                    end: desugar_expr(end, program, counter),
                },
                ForKind::Iterable { iterable } => ForKind::Iterable {
                    iterable: desugar_expr(iterable, program, counter),
                },
            },
            body: desugar_block(&f.body, program, counter),
        }),
        other => other.clone(),
    }
}

pub fn desugar_arrows(program: &mut Program) {
    let mut counter = 0usize;
    let needs: Vec<usize> = program
        .functions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.body.statements.iter().any(|s| stmt_has_arrow(s)))
        .map(|(i, _)| i)
        .collect();
    for idx in needs {
        let body = program.functions[idx].body.clone();
        program.functions[idx].body = desugar_block(&body, program, &mut counter);
    }
}

fn stmt_has_arrow(stmt: &Statement) -> bool {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => expr_has_arrow(&l.value),
        Statement::Assign(a) => expr_has_arrow(&a.target) || expr_has_arrow(&a.value),
        Statement::Return(r) => r.value.as_ref().is_some_and(expr_has_arrow),
        Statement::Expression(e) => expr_has_arrow(e),
        Statement::Print(p) => {
            p.args.iter().any(expr_has_arrow) || p.color.as_ref().is_some_and(expr_has_arrow)
        }
        Statement::Defer(e) => expr_has_arrow(e),
        Statement::If(i) => {
            expr_has_arrow(&i.condition)
                || block_has_arrow(&i.then_block)
                || i.else_block.as_ref().is_some_and(block_has_arrow)
        }
        Statement::While(w) => expr_has_arrow(&w.condition) || block_has_arrow(&w.body),
        Statement::For(f) => {
            let mut has = block_has_arrow(&f.body);
            f.for_each_expr(|e| {
                if expr_has_arrow(e) {
                    has = true;
                }
            });
            has
        }
        _ => false,
    }
}

fn block_has_arrow(block: &Block) -> bool {
    block.statements.iter().any(stmt_has_arrow)
}
