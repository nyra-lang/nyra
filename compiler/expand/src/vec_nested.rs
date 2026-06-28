//! Synthesize `Vec_Vec_{T}_new/push/get/len/free` for nested `Vec<Vec<T>>` when `T`
//! is a Copy scalar backed by `Vec_{T}_*` runtime (`i32` MVP).

use std::collections::HashSet;

use ast::*;
use errors::Span;

fn nested_scalar_name(vec_name: &str) -> Option<&str> {
    vec_name.strip_prefix("Vec__Vec__")
}

fn is_supported_scalar(inner: &str) -> bool {
    matches!(inner, "i32")
}

fn helper_base(inner: &str) -> String {
    format!("Vec_Vec_{inner}")
}

fn inner_struct_name(inner: &str) -> String {
    format!("Vec__{inner}")
}

fn inner_free_fn(inner: &str) -> &'static str {
    match inner {
        "i32" => "Vec_i32_free",
        _ => "Vec_i32_free",
    }
}

fn make_fn(name: &str, params: Vec<Param>, ret: Option<TypeAnnotation>, body: Block) -> Function {
    Function {
        name: name.into(),
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: Span::default(),
        type_params: vec![],
        type_param_bounds: std::collections::HashMap::new(),
        lifetime_params: vec![],
        params,
        return_type: ret,
        body,
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
        doc: None,
    }
}

fn var(name: &str, span: &Span) -> Expression {
    Expression::Variable {
        name: name.into(),
        span: span.clone(),
    }
}

fn call(callee: &str, args: Vec<Expression>, span: &Span) -> Expression {
    Expression::Call(CallExpr {
        callee: callee.into(),
        type_args: vec![],
        args,
        span: span.clone(),
    })
}

fn field(obj: &str, field_name: &str, span: &Span) -> Expression {
    Expression::FieldAccess(Box::new(FieldAccessExpr {
        object: var(obj, span),
        field: field_name.into(),
        optional: false,
        span: span.clone(),
    }))
}

fn int_lit(n: i64, _span: &Span) -> Expression {
    Expression::Literal(Literal::Int(n))
}

fn synthesize_vec_nested_api(vec_name: &str, inner: &str) -> Vec<Function> {
    let span = Span::default();
    let vec_ty = TypeAnnotation::Struct(vec_name.into());
    let inner_ty = TypeAnnotation::Struct(inner_struct_name(inner));
    let base = helper_base(inner);
    let inner_free = inner_free_fn(inner);

    let new_fn = make_fn(
        &format!("{base}_new"),
        vec![],
        Some(vec_ty.clone()),
        Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(Expression::StructLiteral(StructLiteralExpr {
                    name: vec_name.into(),
                    spreads: vec![],
                    fields: vec![(
                        "handle".into(),
                        call("vec_bytes_new", vec![int_lit(8, &span)], &span),
                    )],
                    span: span.clone(),
                })),
            })],
        },
    );

    let push_fn = make_fn(
        &format!("{base}_push"),
        vec![
            Param {
                name: "v".into(),
                mutable: false,
                ty: vec_ty.clone(),
                destructure: vec![],
                no_escape: false,
            },
            Param {
                name: "item".into(),
                mutable: false,
                ty: inner_ty.clone(),
                destructure: vec![],
                no_escape: false,
            },
        ],
        Some(vec_ty.clone()),
        Block {
            statements: vec![
                Statement::Expression(call(
                    "vec_bytes_push_ptr",
                    vec![field("v", "handle", &span), field("item", "handle", &span)],
                    &span,
                )),
                Statement::Return(ReturnStmt {
                    value: Some(var("v", &span)),
                }),
            ],
        },
    );

    let get_fn = make_fn(
        &format!("{base}_get"),
        vec![
            Param {
                name: "v".into(),
                mutable: false,
                ty: vec_ty.clone(),
                destructure: vec![],
                no_escape: false,
            },
            Param {
                name: "index".into(),
                mutable: false,
                ty: TypeAnnotation::Integer(ast::IntKind::I32),
                destructure: vec![],
                no_escape: false,
            },
        ],
        Some(TypeAnnotation::Ptr),
        Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(call(
                    "vec_bytes_get_ptr",
                    vec![field("v", "handle", &span), var("index", &span)],
                    &span,
                )),
            })],
        },
    );

    let len_fn = make_fn(
        &format!("{base}_len"),
        vec![Param {
            name: "v".into(),
            mutable: false,
            ty: vec_ty.clone(),
            destructure: vec![],
            no_escape: false,
        }],
        Some(TypeAnnotation::Integer(ast::IntKind::I32)),
        Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(call("vec_bytes_len", vec![field("v", "handle", &span)], &span)),
            })],
        },
    );

    let free_fn = make_fn(
        &format!("{base}_free"),
        vec![Param {
            name: "v".into(),
            mutable: false,
            ty: vec_ty.clone(),
            destructure: vec![],
            no_escape: false,
        }],
        Some(TypeAnnotation::Void),
        Block {
            statements: vec![
                Statement::Let(LetStmt {
                    name: "n".into(),
                    mutable: false,
                    destructure: vec![],
                    span: span.clone(),
                    ty: Some(TypeAnnotation::Integer(ast::IntKind::I32)),
                    value: call("vec_bytes_len", vec![field("v", "handle", &span)], &span),
                }),
                Statement::Let(LetStmt {
                    name: "i".into(),
                    mutable: true,
                    destructure: vec![],
                    span: span.clone(),
                    ty: Some(TypeAnnotation::Integer(ast::IntKind::I32)),
                    value: int_lit(0, &span),
                }),
                Statement::While(WhileStmt {
                    condition: Expression::Binary(Box::new(BinaryExpr {
                        left: var("i", &span),
                        op: BinaryOp::Lt,
                        right: var("n", &span),
                        span: span.clone(),
                    })),
                    body: Block {
                        statements: vec![
                            Statement::Let(LetStmt {
                                name: "row".into(),
                                mutable: false,
                                destructure: vec![],
                                span: span.clone(),
                                ty: Some(TypeAnnotation::Ptr),
                                value: call(
                                    "vec_bytes_get_ptr",
                                    vec![field("v", "handle", &span), var("i", &span)],
                                    &span,
                                ),
                            }),
                            Statement::Expression(call(inner_free, vec![var("row", &span)], &span)),
                            Statement::Assign(AssignStmt {
                                target: var("i", &span),
                                value: Expression::Binary(Box::new(BinaryExpr {
                                    left: var("i", &span),
                                    op: BinaryOp::Add,
                                    right: int_lit(1, &span),
                                    span: span.clone(),
                                })),
                                span: span.clone(),
                            }),
                        ],
                    },
                }),
                Statement::Expression(call(
                    "vec_bytes_free",
                    vec![field("v", "handle", &span)],
                    &span,
                )),
            ],
        },
    );

    let push_handle_fn = make_fn(
        &format!("{base}_push_handle"),
        vec![
            Param {
                name: "v".into(),
                mutable: false,
                ty: vec_ty.clone(),
                destructure: vec![],
                no_escape: false,
            },
            Param {
                name: "handle".into(),
                mutable: false,
                ty: TypeAnnotation::Ptr,
                destructure: vec![],
                no_escape: false,
            },
        ],
        Some(vec_ty),
        Block {
            statements: vec![
                Statement::Expression(call(
                    "vec_bytes_push_ptr",
                    vec![field("v", "handle", &span), var("handle", &span)],
                    &span,
                )),
                Statement::Return(ReturnStmt {
                    value: Some(var("v", &span)),
                }),
            ],
        },
    );

    vec![new_fn, push_fn, get_fn, len_fn, free_fn, push_handle_fn]
}

pub fn synthesize_vec_nested_helpers(program: &mut Program) {
    let existing: HashSet<String> = program.functions.iter().map(|f| f.name.clone()).collect();

    let candidates: Vec<String> = program
        .structs
        .iter()
        .filter_map(|s| nested_scalar_name(&s.name).map(|inner| (s.name.clone(), inner)))
        .filter(|(_, inner)| is_supported_scalar(inner))
        .map(|(name, _)| name)
        .collect();

    for vec_name in candidates {
        let Some(inner) = nested_scalar_name(&vec_name) else {
            continue;
        };
        let base = helper_base(inner);
        if existing.contains(&format!("{base}_new")) {
            continue;
        }
        for f in synthesize_vec_nested_api(&vec_name, inner) {
            program.functions.push(f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_vec_vec_i32_helpers() {
        let mut program = Program {
            structs: vec![StructDef {
                name: "Vec__Vec__i32".into(),
                doc: None,
                type_params: vec![],
                attrs: StructAttrs::default(),
                fields: vec![StructField {
                    name: "handle".into(),
                    ty: TypeAnnotation::Ptr,
                }],
                public: true,
            }],
            ..Program::default()
        };
        synthesize_vec_nested_helpers(&mut program);
        let names: Vec<String> = program.functions.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"Vec_Vec_i32_new".to_string()));
        assert!(names.contains(&"Vec_Vec_i32_push".to_string()));
        assert!(names.contains(&"Vec_Vec_i32_free".to_string()));
    }
}
