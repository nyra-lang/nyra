//! Synthesize `Vec_{Struct}_new/push/get/len/free` for monomorph `Vec__S_*` POD vectors.

use std::collections::HashMap;

use ast::*;
use errors::Span;

fn is_copy_field(ty: &TypeAnnotation) -> bool {
    match ty {
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
        | TypeAnnotation::Bool
        | TypeAnnotation::Ptr => true,
        TypeAnnotation::Enum(_) => true,
        TypeAnnotation::Struct(name) if name.starts_with("Vec__") => false,
        _ => false,
    }
}

fn struct_elem_size(fields: &[StructField]) -> Option<i32> {
    if fields.iter().any(|f| matches!(f.ty, TypeAnnotation::String)) {
        return None;
    }
    let mut size = 0i32;
    for f in fields {
        if !is_copy_field(&f.ty) {
            return None;
        }
        let (align, sz) = field_size_align(&f.ty);
        size = (size + align - 1) / align * align;
        size += sz;
    }
    Some((size + 7) / 8 * 8)
}

fn field_size_align(ty: &TypeAnnotation) -> (i32, i32) {
    match ty {
        TypeAnnotation::Integer(ast::IntKind::I64)
        | TypeAnnotation::Integer(ast::IntKind::U64)
        | TypeAnnotation::F64
        | TypeAnnotation::Ptr => (8, 8),
        TypeAnnotation::F32 => (4, 4),
        _ => (4, 4),
    }
}

fn pod_struct_name(vec_name: &str) -> Option<String> {
    let rest = vec_name.strip_prefix("Vec__S_")?;
    Some(rest.to_string())
}

fn helper_base(vec_name: &str) -> String {
    format!("Vec_{}", pod_struct_name(vec_name).unwrap_or_default())
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
        type_param_bounds: HashMap::new(),
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

fn field(obj: &str, field: &str, span: &Span) -> Expression {
    Expression::FieldAccess(Box::new(FieldAccessExpr {
        object: var(obj, span),
        field: field.into(),
        optional: false,
        span: span.clone(),
    }))
}

fn int_lit(n: i64, _span: &Span) -> Expression {
    Expression::Literal(Literal::Int(n))
}

fn default_for_type(ty: &TypeAnnotation, span: &Span) -> Expression {
    match ty {
        TypeAnnotation::Integer(_) => int_lit(0, span),
        TypeAnnotation::F32 | TypeAnnotation::F64 => {
            Expression::Literal(Literal::Float(0.0, FloatKind::F64))
        }
        TypeAnnotation::Char => Expression::Literal(Literal::Char(0)),
        TypeAnnotation::Bool => Expression::Literal(Literal::Bool(false)),
        TypeAnnotation::Enum(_) => int_lit(0, span),
        _ => int_lit(0, span),
    }
}

fn synthesize_vec_pod_api(
    vec_name: &str,
    elem: &StructDef,
    elem_size: i32,
) -> Vec<Function> {
    let span = Span::default();
    let elem_name = elem.name.clone();
    let vec_ty = TypeAnnotation::Struct(vec_name.into());
    let elem_ty = TypeAnnotation::Struct(elem_name.clone());
    let base = helper_base(vec_name);

    let default_elem = Expression::StructLiteral(StructLiteralExpr {
        name: elem_name.clone(),
        spreads: vec![],
        fields: elem
            .fields
            .iter()
            .map(|f| (f.name.clone(), default_for_type(&f.ty, &span)))
            .collect(),
        span: span.clone(),
    });

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
                        call("vec_bytes_new", vec![int_lit(elem_size as i64, &span)], &span),
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
                ty: elem_ty.clone(),
                destructure: vec![],
                no_escape: false,
            },
        ],
        Some(vec_ty.clone()),
        Block {
            statements: vec![
                Statement::Expression(call(
                    "vec_bytes_push",
                    vec![field("v", "handle", &span), var("item", &span)],
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
        Some(elem_ty.clone()),
        Block {
            statements: vec![
                Statement::Let(LetStmt {
                    name: "out".into(),
                    mutable: false,
                    destructure: vec![],
                    span: span.clone(),
                    ty: Some(elem_ty.clone()),
                    value: default_elem.clone(),
                }),
                Statement::Expression(call(
                    "vec_bytes_get",
                    vec![field("v", "handle", &span), var("index", &span), var("out", &span)],
                    &span,
                )),
                Statement::Return(ReturnStmt {
                    value: Some(var("out", &span)),
                }),
            ],
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
            ty: vec_ty,
            destructure: vec![],
            no_escape: false,
        }],
        Some(TypeAnnotation::Void),
        Block {
            statements: vec![Statement::Expression(call(
                "vec_bytes_free",
                vec![field("v", "handle", &span)],
                &span,
            ))],
        },
    );

    vec![new_fn, push_fn, get_fn, len_fn, free_fn]
}

fn has_string_field(fields: &[StructField]) -> bool {
    fields.iter().any(|f| matches!(f.ty, TypeAnnotation::String))
}

fn is_handle_vec_struct(sdef: &StructDef) -> bool {
    sdef.fields.len() == 1 && sdef.fields[0].name == "handle"
}

pub fn synthesize_vec_pod_helpers(program: &mut Program) {
    let structs: HashMap<String, StructDef> = program
        .structs
        .iter()
        .map(|s| (s.name.clone(), s.clone()))
        .collect();
    let existing: std::collections::HashSet<String> = program
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect();

    for sdef in structs.values() {
        if !sdef.name.starts_with("Vec__S_") {
            continue;
        }
        let Some(elem_name) = pod_struct_name(&sdef.name) else {
            continue;
        };
        let Some(elem) = structs.get(&elem_name) else {
            continue;
        };
        if has_string_field(&elem.fields) {
            continue;
        }
        if !is_handle_vec_struct(sdef) {
            continue;
        }
        let Some(elem_size) = struct_elem_size(&elem.fields) else {
            continue;
        };
        let base = helper_base(&sdef.name);
        if existing.contains(&format!("{base}_new")) {
            continue;
        }
        for f in synthesize_vec_pod_api(&sdef.name, elem, elem_size) {
            program.functions.push(f);
        }
    }
}
