//! Synthesize `{Struct}_json_encode` / `{Struct}_json_decode` for concrete structs.

use std::collections::{HashMap, HashSet};

use ast::*;
use errors::Span;

use super::serde_traits;

fn vec_collection_elem(ty: &TypeAnnotation) -> Option<&TypeAnnotation> {
    match ty {
        TypeAnnotation::Applied { base, args } if base == "Vec" && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}

fn vec_i32_ptr_field(field_ty: &TypeAnnotation) -> bool {
    match field_ty {
        TypeAnnotation::Applied { base, args } if base == "Vec" && args.len() == 1 => {
            matches!(args[0], TypeAnnotation::Integer(_))
        }
        TypeAnnotation::Struct(name) if name.starts_with("Vec__") => {
            !name.contains("string")
        }
        TypeAnnotation::Ptr => true,
        _ => false,
    }
}

fn vec_str_field(field_ty: &TypeAnnotation) -> bool {
    match field_ty {
        TypeAnnotation::Applied { base, args } if base == "Vec" && args.len() == 1 => {
            matches!(args[0], TypeAnnotation::String)
        }
        TypeAnnotation::Struct(name) if name == "StrVec" => true,
        _ => false,
    }
}

fn str_vec_handle_expr(self_field: Expression, span: &Span) -> Expression {
    Expression::FieldAccess(Box::new(FieldAccessExpr {
        object: self_field,
        field: "handle".into(),
        optional: false,
        span: span.clone(),
    }))
}

fn opaque_ptr_field(field_ty: &TypeAnnotation) -> bool {
    matches!(field_ty, TypeAnnotation::RawPtr { .. })
}

fn encode_value_expr(field_ty: &TypeAnnotation, self_field: Expression, serde_structs: &HashSet<String>) -> Expression {
    let span = Span::default();
    if opaque_ptr_field(field_ty) {
        let ptr_arg = Expression::Cast(Box::new(CastExpr {
            expr: self_field,
            target_type: TypeAnnotation::Ptr,
            span: span.clone(),
        }));
        return Expression::Call(CallExpr {
            callee: "json_encode_ptr_token".into(),
            type_args: vec![],
            args: vec![ptr_arg],
            span: span.clone(),
        });
    }
    if vec_i32_ptr_field(field_ty) {
        return Expression::Call(CallExpr {
            callee: "json_encode_i32_array".into(),
            type_args: vec![],
            args: vec![self_field],
            span: span.clone(),
        });
    }
    if vec_str_field(field_ty) {
        return Expression::Call(CallExpr {
            callee: "json_encode_str_array".into(),
            type_args: vec![],
            args: vec![str_vec_handle_expr(self_field, &span)],
            span: span.clone(),
        });
    }
    if let Some(elem) = vec_collection_elem(field_ty) {
        if matches!(elem, TypeAnnotation::Integer(_)) {
            return Expression::Call(CallExpr {
                callee: "json_encode_i32_array".into(),
                type_args: vec![],
                args: vec![self_field],
                span: span.clone(),
            });
        }
        if matches!(elem, TypeAnnotation::String) {
            return Expression::Call(CallExpr {
                callee: "json_encode_str_array".into(),
                type_args: vec![],
                args: vec![str_vec_handle_expr(self_field, &span)],
                span: span.clone(),
            });
        }
    }
    match field_ty {
        TypeAnnotation::Integer(_) => Expression::Call(CallExpr {
            callee: "i32_to_string".into(),
            type_args: vec![],
            args: vec![self_field],
            span: span.clone(),
        }),
        TypeAnnotation::Bool => Expression::If(Box::new(IfExpr {
            condition: self_field.clone(),
            then_expr: Expression::Literal(Literal::String("true".into())),
            else_expr: Expression::Literal(Literal::String("false".into())),
            span: span.clone(),
        })),
        TypeAnnotation::Struct(name) if serde_structs.contains(name) => Expression::Call(CallExpr {
            callee: format!("{name}_json_encode"),
            type_args: vec![],
            args: vec![self_field],
            span: span.clone(),
        }),
        _ => self_field,
    }
}

fn encode_field_value(
    field_name: &str,
    field_ty: &TypeAnnotation,
    self_field: Expression,
    serde_structs: &HashSet<String>,
    span: &Span,
) -> (Vec<Statement>, Expression) {
    if let TypeAnnotation::Array { elem, len: Some(n) } = field_ty {
        if matches!(elem.as_ref(), TypeAnnotation::Integer(_)) {
            let tmp = format!("__nyra_arr_{field_name}");
            let mut stmts = vec![Statement::Let(LetStmt {
                name: tmp.clone(),
                mutable: false,
                destructure: vec![],
                span: span.clone(),
                ty: Some(TypeAnnotation::Ptr),
                value: Expression::Call(CallExpr {
                    callee: "Vec_i32_new".into(),
                    type_args: vec![],
                    args: vec![],
                    span: span.clone(),
                }),
            })];
            for i in 0..*n {
                stmts.push(Statement::Expression(Expression::Call(CallExpr {
                    callee: "Vec_i32_push".into(),
                    type_args: vec![],
                    args: vec![
                        Expression::Variable {
                            name: tmp.clone(),
                            span: span.clone(),
                        },
                        Expression::Index(Box::new(IndexExpr {
                            object: self_field.clone(),
                            index: Expression::Literal(Literal::Int(i as i64)),
                            span: span.clone(),
                        })),
                    ],
                    span: span.clone(),
                })));
            }
            return (
                stmts,
                Expression::Call(CallExpr {
                    callee: "json_encode_i32_array".into(),
                    type_args: vec![],
                    args: vec![Expression::Variable {
                        name: tmp,
                        span: span.clone(),
                    }],
                    span: span.clone(),
                }),
            );
        }
    }
    (
        vec![],
        encode_value_expr(field_ty, self_field, serde_structs),
    )
}

fn decode_field_value(
    field_ty: &TypeAnnotation,
    field_name: &str,
    serde_structs: &HashSet<String>,
) -> (Vec<Statement>, Expression) {
    let span = Span::default();
    if let TypeAnnotation::Array { elem, len: Some(n) } = field_ty {
        if matches!(elem.as_ref(), TypeAnnotation::Integer(_)) {
            let tmp = format!("__nyra_dec_{field_name}");
            let decoded = decode_value_expr(
                &TypeAnnotation::Applied {
                    base: "Vec".into(),
                    args: vec![TypeAnnotation::Integer(IntKind::I32)],
                },
                "json",
                field_name,
                serde_structs,
            );
            let mut elems = Vec::new();
            for i in 0..*n {
                elems.push(Expression::Call(CallExpr {
                    callee: "Vec_i32_get".into(),
                    type_args: vec![],
                    args: vec![
                        Expression::Variable {
                            name: tmp.clone(),
                            span: span.clone(),
                        },
                        Expression::Literal(Literal::Int(i as i64)),
                    ],
                    span: span.clone(),
                }));
            }
            let stmts = vec![Statement::Let(LetStmt {
                name: tmp,
                mutable: false,
                destructure: vec![],
                span: span.clone(),
                ty: Some(TypeAnnotation::Ptr),
                value: decoded,
            })];
            return (stmts, Expression::ArrayLiteral(ArrayLiteralExpr::from_elems(elems)));
        }
    }
    (
        vec![],
        decode_value_expr(field_ty, "json", field_name, serde_structs),
    )
}

fn decode_value_expr(
    field_ty: &TypeAnnotation,
    json: &str,
    field_name: &str,
    serde_structs: &HashSet<String>,
) -> Expression {
    let span = Span::default();
    let json_var = Expression::Variable {
        name: json.into(),
        span: span.clone(),
    };
    let key = Expression::Literal(Literal::String(field_name.into()));
    if opaque_ptr_field(field_ty) {
        let decoded = Expression::Call(CallExpr {
            callee: "json_decode_ptr_token".into(),
            type_args: vec![],
            args: vec![json_var.clone(), key.clone()],
            span: span.clone(),
        });
        return Expression::Cast(Box::new(CastExpr {
            expr: decoded,
            target_type: field_ty.clone(),
            span,
        }));
    }
    if vec_i32_ptr_field(field_ty) {
        let arr = Expression::Call(CallExpr {
            callee: "decode_array".into(),
            type_args: vec![],
            args: vec![json_var.clone(), key.clone()],
            span: span.clone(),
        });
        return Expression::Call(CallExpr {
            callee: "json_decode_i32_array".into(),
            type_args: vec![],
            args: vec![arr],
            span,
        });
    }
    if vec_str_field(field_ty) {
        let arr = Expression::Call(CallExpr {
            callee: "decode_array".into(),
            type_args: vec![],
            args: vec![json_var.clone(), key.clone()],
            span: span.clone(),
        });
        let handle = Expression::Call(CallExpr {
            callee: "json_decode_str_array".into(),
            type_args: vec![],
            args: vec![arr],
            span: span.clone(),
        });
        return Expression::StructLiteral(StructLiteralExpr {
            name: "StrVec".into(),
            spreads: vec![],
            fields: vec![("handle".into(), handle)],
            span,
        });
    }
    if let Some(elem) = vec_collection_elem(field_ty) {
        if matches!(elem, TypeAnnotation::Integer(_)) {
            let arr = Expression::Call(CallExpr {
                callee: "decode_array".into(),
                type_args: vec![],
                args: vec![json_var.clone(), key.clone()],
                span: span.clone(),
            });
            return Expression::Call(CallExpr {
                callee: "json_decode_i32_array".into(),
                type_args: vec![],
                args: vec![arr],
                span,
            });
        }
        if matches!(elem, TypeAnnotation::String) {
            let arr = Expression::Call(CallExpr {
                callee: "decode_array".into(),
                type_args: vec![],
                args: vec![json_var.clone(), key.clone()],
                span: span.clone(),
            });
            let handle = Expression::Call(CallExpr {
                callee: "json_decode_str_array".into(),
                type_args: vec![],
                args: vec![arr],
                span: span.clone(),
            });
            return Expression::StructLiteral(StructLiteralExpr {
                name: "StrVec".into(),
                spreads: vec![],
                fields: vec![("handle".into(), handle)],
                span,
            });
        }
    }
    match field_ty {
        TypeAnnotation::Integer(_) => Expression::Call(CallExpr {
            callee: "decode_i32".into(),
            type_args: vec![],
            args: vec![json_var.clone(), key.clone()],
            span: span.clone(),
        }),
        TypeAnnotation::Bool => {
            let decoded = Expression::Call(CallExpr {
                callee: "decode_bool".into(),
                type_args: vec![],
                args: vec![json_var.clone(), key.clone()],
                span: span.clone(),
            });
            Expression::If(Box::new(IfExpr {
                condition: Expression::Binary(Box::new(BinaryExpr {
                    left: decoded,
                    op: BinaryOp::Ne,
                    right: Expression::Literal(Literal::Int(0)),
                    span: span.clone(),
                })),
                then_expr: Expression::Literal(Literal::Bool(true)),
                else_expr: Expression::Literal(Literal::Bool(false)),
                span,
            }))
        }
        TypeAnnotation::Struct(name) if serde_structs.contains(name) => {
            let inner = Expression::Call(CallExpr {
                callee: "decode_object".into(),
                type_args: vec![],
                args: vec![json_var, key],
                span: span.clone(),
            });
            Expression::Call(CallExpr {
                callee: format!("{name}_json_decode"),
                type_args: vec![],
                args: vec![inner],
                span,
            })
        }
        _ => Expression::Call(CallExpr {
            callee: "decode_string".into(),
            type_args: vec![],
            args: vec![json_var, key],
            span,
        }),
    }
}

fn synthesize_json_encode(sdef: &StructDef, serde_structs: &HashSet<String>) -> Function {
    let span = Span::default();
    let struct_name = &sdef.name;
    let fn_name = format!("{struct_name}_json_encode");
    let self_var = Expression::Variable {
        name: "self".into(),
        span: span.clone(),
    };

    let mut stmts = vec![
        Statement::Let(LetStmt {
            name: "keys".into(),
            mutable: false,
            destructure: vec![],
            span: span.clone(),
            ty: Some(TypeAnnotation::Ptr),
            value: Expression::Call(CallExpr {
                callee: "Vec_str_new".into(),
                type_args: vec![],
                args: vec![],
                span: span.clone(),
            }),
        }),
        Statement::Let(LetStmt {
            name: "values".into(),
            mutable: false,
            destructure: vec![],
            span: span.clone(),
            ty: Some(TypeAnnotation::Ptr),
            value: Expression::Call(CallExpr {
                callee: "Vec_str_new".into(),
                type_args: vec![],
                args: vec![],
                span: span.clone(),
            }),
        }),
    ];

    for field in &sdef.fields {
        let field_expr = Expression::FieldAccess(Box::new(FieldAccessExpr {
            object: self_var.clone(),
            field: field.name.clone(),
            optional: false,
            span: span.clone(),
        }));
        let (pre, encoded) = encode_field_value(&field.name, &field.ty, field_expr, serde_structs, &span);
        stmts.extend(pre);
        stmts.push(Statement::Expression(Expression::Call(CallExpr {
            callee: "Vec_str_push".into(),
            type_args: vec![],
            args: vec![
                Expression::Variable {
                    name: "keys".into(),
                    span: span.clone(),
                },
                Expression::Literal(Literal::String(field.name.clone())),
            ],
            span: span.clone(),
        })));
        stmts.push(Statement::Expression(Expression::Call(CallExpr {
            callee: "Vec_str_push".into(),
            type_args: vec![],
            args: vec![
                Expression::Variable {
                    name: "values".into(),
                    span: span.clone(),
                },
                encoded,
            ],
            span: span.clone(),
        })));
    }

    stmts.push(Statement::Let(LetStmt {
        name: "out".into(),
        mutable: false,
        destructure: vec![],
        span: span.clone(),
        ty: None,
        value: Expression::Call(CallExpr {
            callee: "json_encode_object".into(),
            type_args: vec![],
            args: vec![
                Expression::Variable {
                    name: "keys".into(),
                    span: span.clone(),
                },
                Expression::Variable {
                    name: "values".into(),
                    span: span.clone(),
                },
            ],
            span: span.clone(),
        }),
    }));
    stmts.push(Statement::Expression(Expression::Call(CallExpr {
        callee: "Vec_str_free".into(),
        type_args: vec![],
        args: vec![Expression::Variable {
            name: "keys".into(),
            span: span.clone(),
        }],
        span: span.clone(),
    })));
    stmts.push(Statement::Expression(Expression::Call(CallExpr {
        callee: "Vec_str_free".into(),
        type_args: vec![],
        args: vec![Expression::Variable {
            name: "values".into(),
            span: span.clone(),
        }],
        span: span.clone(),
    })));
    stmts.push(Statement::Return(ReturnStmt {
        value: Some(Expression::Variable {
            name: "out".into(),
            span: span.clone(),
        }),
    }));

    Function {
        name: fn_name,
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "self".into(),
            ty: TypeAnnotation::Struct(struct_name.clone()),
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::String),
        body: wrap_unsafe_if_rawptr(sdef, stmts),
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn synthesize_json_decode(sdef: &StructDef, serde_structs: &HashSet<String>) -> Function {
    let span = Span::default();
    let struct_name = &sdef.name;
    let fn_name = format!("{struct_name}_json_decode");
    let mut pre_stmts = Vec::new();
    let mut fields: Vec<(String, Expression)> = Vec::new();
    for f in &sdef.fields {
        let (pre, val) = decode_field_value(&f.ty, &f.name, serde_structs);
        pre_stmts.extend(pre);
        fields.push((f.name.clone(), val));
    }

    let struct_lit = Expression::StructLiteral(StructLiteralExpr {
        name: struct_name.clone(),
        fields,
        spreads: vec![],
        span: span.clone(),
    });

    let mut body_stmts = pre_stmts;
    body_stmts.push(Statement::Return(ReturnStmt {
        value: Some(struct_lit),
    }));

    Function {
        name: fn_name,
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "json".into(),
            ty: TypeAnnotation::String,
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Struct(struct_name.clone())),
        body: wrap_unsafe_if_rawptr(sdef, body_stmts),
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn field_type_supported(field_ty: &TypeAnnotation, serde_structs: &HashSet<String>) -> bool {
    if opaque_ptr_field(field_ty) || vec_i32_ptr_field(field_ty) || vec_str_field(field_ty) {
        return true;
    }
    if let Some(elem) = vec_collection_elem(field_ty) {
        return matches!(elem, TypeAnnotation::Integer(_) | TypeAnnotation::String);
    }
    match field_ty {
        TypeAnnotation::Integer(_) | TypeAnnotation::Bool | TypeAnnotation::String => true,
        TypeAnnotation::Struct(name) => serde_structs.contains(name),
        TypeAnnotation::Array {
            elem,
            len: Some(_),
        } => field_type_supported(elem, serde_structs),
        _ => false,
    }
}

fn struct_supports_bin_serde(
    sdef: &StructDef,
    serde_structs: &HashSet<String>,
    bin_structs: &HashSet<String>,
) -> bool {
    if !struct_supports_auto_serde(sdef, serde_structs) {
        return false;
    }
    sdef.fields.iter().all(|f| match &f.ty {
        TypeAnnotation::Integer(_) | TypeAnnotation::Bool | TypeAnnotation::String => true,
        TypeAnnotation::Struct(name) => bin_structs.contains(name),
        _ => false,
    })
}

fn collect_bin_structs(program: &Program, serde_structs: &HashSet<String>) -> HashSet<String> {
    let mut eligible: HashSet<String> = HashSet::new();
    loop {
        let before = eligible.len();
        for s in &program.structs {
            if !serde_structs.contains(&s.name) {
                continue;
            }
            if struct_supports_bin_serde(s, serde_structs, &eligible) {
                eligible.insert(s.name.clone());
            }
        }
        if eligible.len() == before {
            break;
        }
    }
    eligible
}

fn struct_supports_auto_serde(sdef: &StructDef, serde_structs: &HashSet<String>) -> bool {
    if sdef.attrs.repr_c {
        return false;
    }
    if !sdef.type_params.is_empty() {
        return false;
    }
    if sdef.name == "StrVec"
        || sdef.name.starts_with("Dyn_")
        || sdef.name.starts_with("__Anon")
        || sdef.name.starts_with("Vec__")
    {
        return false;
    }
    sdef.fields
        .iter()
        .all(|f| field_type_supported(&f.ty, serde_structs))
}

fn struct_has_rawptr_field(sdef: &StructDef) -> bool {
    sdef.fields.iter().any(|f| opaque_ptr_field(&f.ty))
}

fn wrap_unsafe_if_rawptr(sdef: &StructDef, stmts: Vec<Statement>) -> Block {
    let block = Block { statements: stmts };
    if struct_has_rawptr_field(sdef) {
        Block {
            statements: vec![Statement::Unsafe(block)],
        }
    } else {
        block
    }
}

fn synthesize_bin_encode(sdef: &StructDef, serde_structs: &HashSet<String>, bin_structs: &HashSet<String>) -> Function {
    let span = Span::default();
    let struct_name = &sdef.name;
    let fn_name = format!("{struct_name}_bin_encode");
    let self_var = Expression::Variable {
        name: "self".into(),
        span: span.clone(),
    };

    let mut stmts = vec![Statement::Let(LetStmt {
        name: "buf".into(),
        mutable: false,
        destructure: vec![],
        span: span.clone(),
        ty: Some(TypeAnnotation::Ptr),
        value: Expression::Call(CallExpr {
            callee: "bin_buf_new".into(),
            type_args: vec![],
            args: vec![],
            span: span.clone(),
        }),
    })];

    for field in &sdef.fields {
        let field_expr = Expression::FieldAccess(Box::new(FieldAccessExpr {
            object: self_var.clone(),
            field: field.name.clone(),
            optional: false,
            span: span.clone(),
        }));
        let (pre, write_expr) =
            bin_write_field_expr(&field.ty, field_expr, serde_structs, bin_structs, &span);
        stmts.extend(pre);
        stmts.push(Statement::Expression(write_expr));
    }

    stmts.push(Statement::Return(ReturnStmt {
        value: Some(Expression::Call(CallExpr {
            callee: "bin_buf_finish".into(),
            type_args: vec![],
            args: vec![Expression::Variable {
                name: "buf".into(),
                span: span.clone(),
            }],
            span: span.clone(),
        })),
    }));

    Function {
        name: fn_name,
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "self".into(),
            ty: TypeAnnotation::Struct(struct_name.clone()),
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Ptr),
        body: wrap_unsafe_if_rawptr(sdef, stmts),
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn bin_write_field_expr(
    field_ty: &TypeAnnotation,
    self_field: Expression,
    serde_structs: &HashSet<String>,
    bin_structs: &HashSet<String>,
    span: &Span,
) -> (Vec<Statement>, Expression) {
    let buf = Expression::Variable {
        name: "buf".into(),
        span: span.clone(),
    };
    if matches!(field_ty, TypeAnnotation::Integer(_)) {
        return (
            vec![],
            Expression::Call(CallExpr {
                callee: "bin_buf_write_i32".into(),
                type_args: vec![],
                args: vec![buf, self_field],
                span: span.clone(),
            }),
        );
    }
    if matches!(field_ty, TypeAnnotation::Bool) {
        let as_i32 = Expression::If(Box::new(IfExpr {
            condition: self_field,
            then_expr: Expression::Literal(Literal::Int(1)),
            else_expr: Expression::Literal(Literal::Int(0)),
            span: span.clone(),
        }));
        return (
            vec![],
            Expression::Call(CallExpr {
                callee: "bin_buf_write_bool".into(),
                type_args: vec![],
                args: vec![buf, as_i32],
                span: span.clone(),
            }),
        );
    }
    if matches!(field_ty, TypeAnnotation::String) {
        return (
            vec![],
            Expression::Call(CallExpr {
                callee: "bin_buf_write_string".into(),
                type_args: vec![],
                args: vec![buf, self_field],
                span: span.clone(),
            }),
        );
    }
    if let TypeAnnotation::Struct(name) = field_ty {
        if bin_structs.contains(name) {
            let tmp = format!("__nyra_bin_{name}");
            let nested = Expression::Call(CallExpr {
                callee: format!("{name}_bin_encode"),
                type_args: vec![],
                args: vec![self_field],
                span: span.clone(),
            });
            let stmt = Statement::Let(LetStmt {
                name: tmp.clone(),
                mutable: false,
                destructure: vec![],
                span: span.clone(),
                ty: Some(TypeAnnotation::Ptr),
                value: nested,
            });
            return (
                vec![stmt],
                Expression::Call(CallExpr {
                    callee: "bin_buf_append_blob".into(),
                    type_args: vec![],
                    args: vec![
                        buf,
                        Expression::Variable {
                            name: tmp,
                            span: span.clone(),
                        },
                    ],
                    span: span.clone(),
                }),
            );
        }
    }
    // Fallback: embed JSON text for unsupported field shapes.
    let json_tmp = format!("__nyra_bin_json_{}", span.start.line);
    let encoded = encode_value_expr(field_ty, self_field, serde_structs);
    (
        vec![Statement::Let(LetStmt {
            name: json_tmp.clone(),
            mutable: false,
            destructure: vec![],
            span: span.clone(),
            ty: None,
            value: encoded,
        })],
        Expression::Call(CallExpr {
            callee: "bin_buf_write_string".into(),
            type_args: vec![],
            args: vec![
                buf,
                Expression::Variable {
                    name: json_tmp,
                    span: span.clone(),
                },
            ],
            span: span.clone(),
        }),
    )
}

fn synthesize_bin_decode(sdef: &StructDef, serde_structs: &HashSet<String>, bin_structs: &HashSet<String>) -> Function {
    let span = Span::default();
    let struct_name = &sdef.name;
    let fn_name = format!("{struct_name}_bin_decode");

    let mut stmts: Vec<Statement> = Vec::new();
    let mut fields: Vec<(String, Expression)> = Vec::new();
    let mut off_expr = Expression::Literal(Literal::Int(4));

    for field in &sdef.fields {
        let (pre, val, width_expr) = bin_decode_field_value(
            &field.ty,
            &field.name,
            &off_expr,
            serde_structs,
            bin_structs,
            &span,
        );
        let next_off = format!("__nyra_bin_off_{}", field.name);
        stmts.extend(pre);
        fields.push((field.name.clone(), val));
        stmts.push(Statement::Let(LetStmt {
            name: next_off.clone(),
            mutable: false,
            destructure: vec![],
            span: span.clone(),
            ty: Some(TypeAnnotation::Integer(IntKind::I32)),
            value: Expression::Binary(Box::new(BinaryExpr {
                left: off_expr,
                op: BinaryOp::Add,
                right: width_expr,
                span: span.clone(),
            })),
        }));
        off_expr = Expression::Variable {
            name: next_off,
            span: span.clone(),
        };
    }

    stmts.push(Statement::Return(ReturnStmt {
        value: Some(Expression::StructLiteral(StructLiteralExpr {
            name: struct_name.clone(),
            fields,
            spreads: vec![],
            span: span.clone(),
        })),
    }));

    Function {
        name: fn_name,
        doc: None,
        is_test: false,
        ignore_test: false,
        should_fail_test: false,
        is_async: false,
        exported: false,
        public: false,
        span: span.clone(),
        type_params: vec![],
        type_param_bounds: HashMap::new(),
        lifetime_params: vec![],
        params: vec![Param {
            name: "data".into(),
            ty: TypeAnnotation::Ptr,
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Struct(struct_name.clone())),
        body: Block {
            statements: stmts,
        },
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

fn bin_decode_field_value(
    field_ty: &TypeAnnotation,
    field_name: &str,
    off: &Expression,
    serde_structs: &HashSet<String>,
    bin_structs: &HashSet<String>,
    span: &Span,
) -> (Vec<Statement>, Expression, Expression) {
    let data = Expression::Variable {
        name: "data".into(),
        span: span.clone(),
    };
    if matches!(field_ty, TypeAnnotation::Integer(_)) {
        return (
            vec![],
            Expression::Call(CallExpr {
                callee: "bin_decode_i32_at".into(),
                type_args: vec![],
                args: vec![data.clone(), off.clone()],
                span: span.clone(),
            }),
            Expression::Call(CallExpr {
                callee: "bin_field_width_i32".into(),
                type_args: vec![],
                args: vec![],
                span: span.clone(),
            }),
        );
    }
    if matches!(field_ty, TypeAnnotation::Bool) {
        let raw = Expression::Call(CallExpr {
            callee: "bin_decode_bool_at".into(),
            type_args: vec![],
            args: vec![data.clone(), off.clone()],
            span: span.clone(),
        });
        return (
            vec![],
            Expression::Binary(Box::new(BinaryExpr {
                left: raw,
                op: BinaryOp::Ne,
                right: Expression::Literal(Literal::Int(0)),
                span: span.clone(),
            })),
            Expression::Call(CallExpr {
                callee: "bin_field_width_bool".into(),
                type_args: vec![],
                args: vec![],
                span: span.clone(),
            }),
        );
    }
    if matches!(field_ty, TypeAnnotation::String) {
        return (
            vec![],
            Expression::Call(CallExpr {
                callee: "bin_decode_string_at".into(),
                type_args: vec![],
                args: vec![data.clone(), off.clone()],
                span: span.clone(),
            }),
            Expression::Call(CallExpr {
                callee: "bin_field_width_string_at".into(),
                type_args: vec![],
                args: vec![data, off.clone()],
                span: span.clone(),
            }),
        );
    }
    if let TypeAnnotation::Struct(name) = field_ty {
        if bin_structs.contains(name) {
            let blob = format!("__nyra_bin_blob_{field_name}");
            let blob_stmt = Statement::Let(LetStmt {
                name: blob.clone(),
                mutable: false,
                destructure: vec![],
                span: span.clone(),
                ty: Some(TypeAnnotation::Ptr),
                value: Expression::Call(CallExpr {
                    callee: "bin_decode_blob_at".into(),
                    type_args: vec![],
                    args: vec![data.clone(), off.clone()],
                    span: span.clone(),
                }),
            });
            let width = Expression::Call(CallExpr {
                callee: "bin_field_width_blob_at".into(),
                type_args: vec![],
                args: vec![data.clone(), off.clone()],
                span: span.clone(),
            });
            return (
                vec![blob_stmt],
                Expression::Call(CallExpr {
                    callee: format!("{name}_bin_decode"),
                    type_args: vec![],
                    args: vec![Expression::Variable {
                        name: blob,
                        span: span.clone(),
                    }],
                    span: span.clone(),
                }),
                width,
            );
        }
    }
    let json_tmp = format!("__nyra_bin_json_{field_name}");
    let json_stmt = Statement::Let(LetStmt {
        name: json_tmp.clone(),
        mutable: false,
        destructure: vec![],
        span: span.clone(),
        ty: None,
        value: Expression::Call(CallExpr {
            callee: "bin_decode_string_at".into(),
            type_args: vec![],
            args: vec![data.clone(), off.clone()],
            span: span.clone(),
        }),
    });
    let decoded = decode_value_expr(field_ty, &json_tmp, field_name, serde_structs);
    let width = Expression::Call(CallExpr {
        callee: "bin_field_width_string_at".into(),
        type_args: vec![],
        args: vec![data.clone(), off.clone()],
        span: span.clone(),
    });
    (vec![json_stmt], decoded, width)
}

fn collect_serde_structs(program: &Program) -> HashSet<String> {
    let mut eligible: HashSet<String> = HashSet::new();
    loop {
        let before = eligible.len();
        for s in &program.structs {
            if struct_supports_auto_serde(s, &eligible) {
                eligible.insert(s.name.clone());
            }
        }
        if eligible.len() == before {
            break;
        }
    }
    eligible
}

pub fn synthesize_struct_json_helpers(program: &mut Program) {
    let existing: HashSet<String> = program.functions.iter().map(|f| f.name.clone()).collect();
    let serde_structs = collect_serde_structs(program);
    let bin_structs = collect_bin_structs(program, &serde_structs);
    let mut structs: Vec<StructDef> = program
        .structs
        .iter()
        .filter(|s| serde_structs.contains(&s.name))
        .cloned()
        .collect();
    structs.sort_by(|a, b| a.name.cmp(&b.name));
    let mut serde_impl_structs: HashSet<String> = HashSet::new();
    for sdef in &structs {
        if !struct_supports_auto_serde(sdef, &serde_structs) {
            continue;
        }
        let enc = format!("{}_json_encode", sdef.name);
        let dec = format!("{}_json_decode", sdef.name);
        if !existing.contains(&enc) {
            program.functions.push(synthesize_json_encode(sdef, &serde_structs));
        }
        if !existing.contains(&dec) {
            program.functions.push(synthesize_json_decode(sdef, &serde_structs));
        }
        serde_impl_structs.insert(sdef.name.clone());
        if bin_structs.contains(&sdef.name) {
            let bin_enc = format!("{}_bin_encode", sdef.name);
            let bin_dec = format!("{}_bin_decode", sdef.name);
            if !existing.contains(&bin_enc) {
                program.functions.push(synthesize_bin_encode(sdef, &serde_structs, &bin_structs));
            }
            if !existing.contains(&bin_dec) {
                program.functions.push(synthesize_bin_decode(sdef, &serde_structs, &bin_structs));
            }
        }
    }
    serde_traits::synthesize_serde_trait_impls(program, &serde_impl_structs, &bin_structs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser;

    #[test]
    fn serde_skips_structs_with_unsupported_field_types() {
        let src = r#"
struct Supported {
    n: i32
}

struct Unsupported {
    ratio: f64
    label: string
}

fn main() {
    return
}
"#;
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        synthesize_struct_json_helpers(&mut program);
        assert!(
            program
                .functions
                .iter()
                .any(|f| f.name == "Supported_json_encode"),
            "expected JSON helper for supported struct"
        );
        assert!(
            !program
                .functions
                .iter()
                .any(|f| f.name == "Unsupported_json_encode"),
            "must not synthesize JSON for f64 fields"
        );
        assert!(
            program.trait_impls.iter().any(|t| {
                t.trait_name == "Serialize" && t.type_name == "Supported"
            }),
            "expected Serialize impl for supported struct"
        );
        assert!(
            !program
                .trait_impls
                .iter()
                .any(|t| t.type_name == "Unsupported"),
            "must not synthesize Serialize/Deserialize for unsupported struct fields"
        );
    }

    #[test]
    fn bin_serde_skips_structs_with_non_bin_nested_fields() {
        let src = r#"
struct Handle {
    h: ptr
}

struct Parent {
    child: Handle
}

fn main() {
    return
}
"#;
        let (tokens, _) = lexer::Lexer::new(src, "t.ny").tokenize();
        let (mut program, _) = parser::Parser::new(tokens).parse();
        synthesize_struct_json_helpers(&mut program);
        assert!(
            program
                .functions
                .iter()
                .any(|f| f.name == "Parent_json_encode"),
            "expected JSON helper for parent struct"
        );
        assert!(
            !program
                .functions
                .iter()
                .any(|f| f.name == "Parent_bin_encode"),
            "must not synthesize bin encode when nested struct lacks bin support"
        );
        assert!(
            !program
                .functions
                .iter()
                .any(|f| f.name == "Handle_bin_encode"),
            "handle-only struct must not get bin encode"
        );
    }
}
