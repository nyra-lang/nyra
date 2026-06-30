use ast::*;

fn desugar_expr(expr: &Expression, counter: &mut usize) -> Expression {
    match expr {
        Expression::Binary(b) if b.op == BinaryOp::NullishCoalesce => {
            let left = desugar_expr(&b.left, counter);
            let right = desugar_expr(&b.right, counter);
            let bind = format!("__nc_{counter}");
            *counter += 1;
            let span = b.span.clone();
            Expression::Match(Box::new(MatchExpr {
                scrutinee: Box::new(left.clone()),
                arms: vec![
                    MatchArm {
                        pattern: MatchPattern::Qualified("Option".into(), "None".into()),
                        guard: None,
                        body: block_from_expr(right),
                    },
                    MatchArm {
                        pattern: MatchPattern::QualifiedBind(
                            "Option".into(),
                            "Some".into(),
                            MatchPayloadPattern::Bind(bind.clone()),
                        ),
                        guard: None,
                        body: block_from_expr(Expression::Variable {
                            name: bind,
                            span: span.clone(),
                        }),
                    },
                ],
                span,
            }))
        }
        Expression::FieldAccess(f) if f.optional => {
            let object = desugar_expr(&f.object, counter);
            let bind = format!("__opt_{counter}");
            *counter += 1;
            let inner_field = Expression::FieldAccess(Box::new(FieldAccessExpr {
                object: Expression::Variable {
                    name: bind.clone(),
                    span: f.span.clone(),
                },
                field: f.field.clone(),
                optional: false,
                span: f.span.clone(),
            }));
            let span = f.span.clone();
            Expression::Match(Box::new(MatchExpr {
                scrutinee: Box::new(object),
                arms: vec![
                    MatchArm {
                        pattern: MatchPattern::Qualified("Option".into(), "None".into()),
                        guard: None,
                        body: block_from_expr(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: Some("Option".into()),
                            variant: "None".into(),
                            args: vec![],
                            span: span.clone(),
                        })),
                    },
                    MatchArm {
                        pattern: MatchPattern::QualifiedBind(
                            "Option".into(),
                            "Some".into(),
                            MatchPayloadPattern::Bind(bind.clone()),
                        ),
                        guard: None,
                        body: block_from_expr(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: Some("Option".into()),
                            variant: "Some".into(),
                            args: vec![inner_field],
                            span: span.clone(),
                        })),
                    },
                ],
                span,
            }))
        }
        Expression::MethodCall(m) if m.optional => {
            let object = desugar_expr(&m.object, counter);
            let bind = format!("__opt_{counter}");
            *counter += 1;
            let inner_call = Expression::MethodCall(Box::new(MethodCallExpr {
                object: Expression::Variable {
                    name: bind.clone(),
                    span: m.span.clone(),
                },
                method: m.method.clone(),
                span: m.span.clone(),
                args: m
                    .args
                    .iter()
                    .map(|a| desugar_expr(a, counter))
                    .collect(),
                optional: false,
            }));
            let span = m.span.clone();
            Expression::Match(Box::new(MatchExpr {
                scrutinee: Box::new(object),
                arms: vec![
                    MatchArm {
                        pattern: MatchPattern::Qualified("Option".into(), "None".into()),
                        guard: None,
                        body: block_from_expr(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: Some("Option".into()),
                            variant: "None".into(),
                            args: vec![],
                            span: span.clone(),
                        })),
                    },
                    MatchArm {
                        pattern: MatchPattern::QualifiedBind(
                            "Option".into(),
                            "Some".into(),
                            MatchPayloadPattern::Bind(bind.clone()),
                        ),
                        guard: None,
                        body: block_from_expr(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: Some("Option".into()),
                            variant: "Some".into(),
                            args: vec![inner_call],
                            span: span.clone(),
                        })),
                    },
                ],
                span,
            }))
        }
        Expression::Binary(b) => Expression::Binary(Box::new(BinaryExpr {
            left: desugar_expr(&b.left, counter),
            op: b.op,
            right: desugar_expr(&b.right, counter),
            span: b.span.clone(),
        })),
        Expression::Unary(u) => Expression::Unary(Box::new(UnaryExpr {
            op: u.op,
            operand: desugar_expr(&u.operand, counter),
            span: u.span.clone(),
        })),
        Expression::Call(c) => Expression::Call(CallExpr {
            callee: c.callee.clone(),
            type_args: c.type_args.clone(),
            args: c
                .args
                .iter()
                .map(|a| desugar_expr(a, counter))
                .collect(),
            span: c.span.clone(),
        }),
        Expression::MethodCall(m) => Expression::MethodCall(Box::new(MethodCallExpr {
            object: desugar_expr(&m.object, counter),
            method: m.method.clone(),
            span: m.span.clone(),
            args: m
                .args
                .iter()
                .map(|a| desugar_expr(a, counter))
                .collect(),
            optional: m.optional,
        })),
        Expression::FieldAccess(f) => Expression::FieldAccess(Box::new(FieldAccessExpr {
            object: desugar_expr(&f.object, counter),
            field: f.field.clone(),
            optional: f.optional,
            span: f.span.clone(),
        })),
        Expression::StructLiteral(s) => Expression::StructLiteral(StructLiteralExpr {
            name: s.name.clone(),
            spreads: s
                .spreads
                .iter()
                .map(|b| desugar_expr(b, counter))
                .collect(),
            fields: s
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), desugar_expr(v, counter)))
                .collect(),
            span: s.span.clone(),
        }),
        Expression::EnumVariant(e) => Expression::EnumVariant(EnumVariantExpr {
            enum_name: e.enum_name.clone(),
            variant: e.variant.clone(),
            args: e
                .args
                .iter()
                .map(|a| desugar_expr(a, counter))
                .collect(),
            span: e.span.clone(),
        }),
        Expression::Match(m) => Expression::Match(Box::new(MatchExpr {
            scrutinee: Box::new(desugar_expr(&m.scrutinee, counter)),
            arms: m
                .arms
                .iter()
                .map(|a| MatchArm {
                    pattern: a.pattern.clone(),
                    guard: a.guard.as_ref().map(|g| desugar_expr(g, counter)),
                    body: desugar_block(&a.body, counter),
                })
                .collect(),
            span: m.span.clone(),
        })),
        Expression::If(i) => Expression::If(Box::new(IfExpr {
            condition: desugar_expr(&i.condition, counter),
            then_block: desugar_block(&i.then_block, counter),
            else_block: desugar_block(&i.else_block, counter),
            span: i.span.clone(),
        })),
        Expression::Index(ix) => Expression::Index(Box::new(IndexExpr {
            object: desugar_expr(&ix.object, counter),
            index: desugar_expr(&ix.index, counter),
            span: ix.span.clone(),
        })),
        Expression::ArrayLiteral(al) => Expression::ArrayLiteral(ArrayLiteralExpr {
            spreads: al
                .spreads
                .iter()
                .map(|e| desugar_expr(e, counter))
                .collect(),
            elems: al
                .elems
                .iter()
                .map(|e| desugar_expr(e, counter))
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
            element: Box::new(desugar_expr(element, counter)),
            count: *count,
            count_from: count_from.clone(),
            count_expr: count_expr
                .as_ref()
                .map(|e| Box::new(desugar_expr(e, counter))),
            span: span.clone(),
        },
        Expression::TupleLiteral(elems) => Expression::TupleLiteral(
            elems
                .iter()
                .map(|e| desugar_expr(e, counter))
                .collect(),
        ),
        Expression::Grouped(inner) => Expression::Grouped(Box::new(desugar_expr(inner, counter))),
        Expression::Await(inner) => Expression::Await(Box::new(desugar_expr(inner, counter))),
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteralExpr {
            parts: t
                .parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Static(s) => TemplatePart::Static(s.clone()),
                    TemplatePart::Interpolation(e) => {
                        TemplatePart::Interpolation(Box::new(desugar_expr(e, counter)))
                    }
                })
                .collect(),
            span: t.span.clone(),
        }),
        Expression::Cast(c) => Expression::Cast(Box::new(CastExpr {
            expr: desugar_expr(&c.expr, counter),
            target_type: c.target_type.clone(),
            span: c.span.clone(),
        })),
        Expression::ArrowFn(a) => Expression::ArrowFn(Box::new(ArrowFnExpr {
            params: a.params.clone(),
            body: match &a.body {
                ArrowBody::Expr(e) => ArrowBody::Expr(desugar_expr(e, counter)),
                ArrowBody::Block(b) => ArrowBody::Block(desugar_block(b, counter)),
            },
            span: a.span.clone(),
        })),
        other => other.clone(),
    }
}

fn desugar_block(block: &Block, counter: &mut usize) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|s| desugar_stmt(s, counter))
            .collect(),
    }
}

fn desugar_stmt(stmt: &Statement, counter: &mut usize) -> Statement {
    match stmt {
        Statement::Let(l) | Statement::Const(l) => {
            let mut s = stmt.clone();
            if let Statement::Let(ref mut x) | Statement::Const(ref mut x) = s {
                x.value = desugar_expr(&l.value, counter);
            }
            s
        }
        Statement::Assign(a) => Statement::Assign(AssignStmt {
            target: desugar_expr(&a.target, counter),
            value: desugar_expr(&a.value, counter),
            span: a.span.clone(),
        }),
        Statement::Return(r) => Statement::Return(ReturnStmt {
            value: r.value.as_ref().map(|v| desugar_expr(v, counter)),
        }),
        Statement::Expression(e) => Statement::Expression(desugar_expr(e, counter)),
        Statement::Print(p) => Statement::Print(
            p.clone().map_expressions(|a| desugar_expr(&a, counter)),
        ),
        Statement::Defer(e) => Statement::Defer(desugar_expr(e, counter)),
        Statement::If(i) => Statement::If(IfStmt {
            condition: desugar_expr(&i.condition, counter),
            then_block: desugar_block(&i.then_block, counter),
            else_block: i
                .else_block
                .as_ref()
                .map(|b| desugar_block(b, counter)),
        }),
        Statement::While(w) => Statement::While(WhileStmt {
            condition: desugar_expr(&w.condition, counter),
            body: desugar_block(&w.body, counter),
        }),
        Statement::For(f) => Statement::For(ForStmt {
            var: f.var.clone(),
            parallel: f.parallel.clone(),
            progress: f.progress.clone(),
            kind: match &f.kind {
                ForKind::Range { start, end } => ForKind::Range {
                    start: desugar_expr(start, counter),
                    end: desugar_expr(end, counter),
                },
                ForKind::Iterable { iterable } => ForKind::Iterable {
                    iterable: desugar_expr(iterable, counter),
                },
            },
            body: desugar_block(&f.body, counter),
        }),
        other => other.clone(),
    }
}

pub fn infer_nullish_option_types(program: &mut Program) {
    for f in &mut program.functions {
        infer_nullish_option_types_block(&mut f.body.statements);
    }
    for imp in &mut program.impls {
        for method in &mut imp.methods {
            infer_nullish_option_types_block(&mut method.body.statements);
        }
    }
    for ti in &mut program.trait_impls {
        for method in &mut ti.methods {
            infer_nullish_option_types_block(&mut method.body.statements);
        }
    }
}

fn infer_nullish_rhs_type(expr: &Expression) -> Option<TypeAnnotation> {
    match expr {
        Expression::Literal(Literal::Int(_)) => Some(TypeAnnotation::Integer(ast::IntKind::I32)),
        Expression::Literal(Literal::Float(_, _)) => Some(TypeAnnotation::F64),
        Expression::Literal(Literal::Char(_)) => Some(TypeAnnotation::Char),
        Expression::Literal(Literal::Bool(_)) => Some(TypeAnnotation::Bool),
        Expression::Literal(Literal::String(_)) => Some(TypeAnnotation::String),
        _ => None,
    }
}

fn is_untyped_option_none(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::EnumVariant(ev)
            if ev.enum_name.as_deref() == Some("Option")
                && ev.variant == "None"
                && ev.args.is_empty()
    )
}

fn nullish_rhs_type_for_var(expr: &Expression, name: &str) -> Option<TypeAnnotation> {
    match expr {
        Expression::Binary(b) if b.op == BinaryOp::NullishCoalesce => {
            if matches!(&b.left, Expression::Variable { name: n, .. } if n == name) {
                infer_nullish_rhs_type(&b.right)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn infer_nullish_option_types_block(stmts: &mut [Statement]) {
    for stmt in stmts.iter_mut() {
        match stmt {
            Statement::If(i) => {
                infer_nullish_option_types_block(&mut i.then_block.statements);
                if let Some(e) = &mut i.else_block {
                    infer_nullish_option_types_block(&mut e.statements);
                }
            }
            Statement::While(w) => infer_nullish_option_types_block(&mut w.body.statements),
            Statement::For(f) => infer_nullish_option_types_block(&mut f.body.statements),
            Statement::Spawn(b) | Statement::Unsafe(b) | Statement::Benchmark(b) => {
                infer_nullish_option_types_block(&mut b.statements);
            }
            _ => {}
        }
    }

    let mut patches: Vec<(usize, TypeAnnotation)> = Vec::new();
    for i in 0..stmts.len() {
        if let Statement::Let(l) | Statement::Const(l) = &stmts[i] {
            if l.ty.is_none() && is_untyped_option_none(&l.value) {
                let var_name = l.name.clone();
                for rest in &stmts[i + 1..] {
                    if let Statement::Let(l2) | Statement::Const(l2) = rest {
                        if let Some(payload) = nullish_rhs_type_for_var(&l2.value, &var_name) {
                            patches.push((i, payload));
                            break;
                        }
                    }
                }
            }
        }
    }
    for (i, payload) in patches {
        if let Statement::Let(l) | Statement::Const(l) = &mut stmts[i] {
            l.ty = Some(TypeAnnotation::Applied {
                base: "Option".into(),
                args: vec![payload],
            });
        }
    }
}

pub fn desugar_nullish(program: &mut Program) {
    let mut counter = 0usize;
    for f in &mut program.functions {
        f.body = desugar_block(&f.body, &mut counter);
    }
    for imp in &mut program.impls {
        for method in &mut imp.methods {
            method.body = desugar_block(&method.body, &mut counter);
        }
    }
    for ti in &mut program.trait_impls {
        for method in &mut ti.methods {
            method.body = desugar_block(&method.body, &mut counter);
        }
    }
}
