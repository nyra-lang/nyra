//! Synthesize `Vec_{Struct}_new/push/get/len/free` for relocatable structs
//! (Copy scalars, `string`, `StrVec`, nested relocatable structs) via parallel columns.

use std::collections::HashMap;

use ast::*;
use errors::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
enum RelocLeaf {
    String,
    Scalar(TypeAnnotation),
}

#[derive(Clone, Debug)]
struct RelocColumn {
    col_name: String,
    path: Vec<String>,
    leaf: RelocLeaf,
}

fn is_copy_scalar(ty: &TypeAnnotation) -> bool {
    match ty {
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
        | TypeAnnotation::Bool
        | TypeAnnotation::Ptr
        | TypeAnnotation::Enum(_) => true,
        _ => false,
    }
}

fn is_string_like_field(ty: &TypeAnnotation) -> bool {
    match ty {
        TypeAnnotation::String | TypeAnnotation::VecStr => true,
        TypeAnnotation::Struct(name) if name == "StrVec" => true,
        TypeAnnotation::Applied { base, args }
            if base == "Vec" && args.len() == 1 && matches!(&args[0], TypeAnnotation::String) =>
        {
            true
        }
        _ => false,
    }
}

fn struct_is_all_copy(fields: &[StructField]) -> bool {
    fields.iter().all(|f| is_copy_scalar(&f.ty))
}

fn struct_is_relocatable(name: &str, structs: &HashMap<String, StructDef>) -> bool {
    let Some(sdef) = structs.get(name) else {
        return false;
    };
    sdef.fields
        .iter()
        .all(|f| is_reloc_field(&f.ty, structs))
}

fn is_reloc_field(ty: &TypeAnnotation, structs: &HashMap<String, StructDef>) -> bool {
    if is_copy_scalar(ty) || is_string_like_field(ty) {
        return true;
    }
    if let TypeAnnotation::Struct(name) = ty {
        return struct_is_relocatable(name, structs);
    }
    false
}

fn column_name(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        format!("{field}_vec")
    } else {
        format!("{prefix}_{field}_vec")
    }
}

fn flatten_reloc_columns(
    prefix: &str,
    path_prefix: &[String],
    fields: &[StructField],
    structs: &HashMap<String, StructDef>,
    out: &mut Vec<RelocColumn>,
) -> bool {
    for f in fields {
        let mut path = path_prefix.to_vec();
        path.push(f.name.clone());
        if is_string_like_field(&f.ty) {
            out.push(RelocColumn {
                col_name: column_name(prefix, &f.name),
                path,
                leaf: RelocLeaf::String,
            });
        } else if is_copy_scalar(&f.ty) {
            out.push(RelocColumn {
                col_name: column_name(prefix, &f.name),
                path,
                leaf: RelocLeaf::Scalar(f.ty.clone()),
            });
        } else if let TypeAnnotation::Struct(name) = &f.ty {
            let Some(nested) = structs.get(name) else {
                return false;
            };
            if struct_is_all_copy(&nested.fields) {
                return false;
            }
            let nested_prefix = if prefix.is_empty() {
                f.name.clone()
            } else {
                format!("{prefix}_{}", f.name)
            };
            if !flatten_reloc_columns(&nested_prefix, &path, &nested.fields, structs, out) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn reloc_struct_name(vec_name: &str) -> Option<String> {
    let rest = vec_name.strip_prefix("Vec__S_")?;
    Some(rest.to_string())
}

fn helper_base(vec_name: &str) -> String {
    format!("Vec_{}", reloc_struct_name(vec_name).unwrap_or_default())
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

fn field_expr_on(object: Expression, field_name: &str, span: &Span) -> Expression {
    Expression::FieldAccess(Box::new(FieldAccessExpr {
        object,
        field: field_name.into(),
        optional: false,
        span: span.clone(),
    }))
}

fn item_path_expr(path: &[String], span: &Span) -> Expression {
    let mut expr = var("item", span);
    for part in path {
        expr = field_expr_on(expr, part, span);
    }
    expr
}

fn field_on_v(col: &str, span: &Span) -> Expression {
    field_expr_on(var("v", span), col, span)
}

fn backing_new_expr(leaf: &RelocLeaf, span: &Span) -> Expression {
    match leaf {
        RelocLeaf::String => call("Vec_str_new", vec![], span),
        RelocLeaf::Scalar(_) => call("Vec_i32_new", vec![], span),
    }
}

fn backing_push_expr(col: &RelocColumn, span: &Span) -> Expression {
    let mut item = item_path_expr(&col.path, span);
    if let RelocLeaf::Scalar(ty) = &col.leaf {
        item = scalar_to_storage_expr(item, ty, span);
    }
    match &col.leaf {
        RelocLeaf::String => call(
            "Vec_str_push",
            vec![field_on_v(&col.col_name, span), item],
            span,
        ),
        RelocLeaf::Scalar(_) => call(
            "Vec_i32_push",
            vec![field_on_v(&col.col_name, span), item],
            span,
        ),
    }
}

fn scalar_to_storage_expr(value: Expression, ty: &TypeAnnotation, span: &Span) -> Expression {
    if matches!(ty, TypeAnnotation::Bool) {
        Expression::If(Box::new(IfExpr {
            condition: value,
            then_block: block_from_expr(Expression::Literal(Literal::Int(1))),
            else_block: block_from_expr(Expression::Literal(Literal::Int(0))),
            span: span.clone(),
        }))
    } else {
        value
    }
}

fn storage_to_scalar_expr(value: Expression, ty: &TypeAnnotation, span: &Span) -> Expression {
    if matches!(ty, TypeAnnotation::Bool) {
        Expression::Binary(Box::new(BinaryExpr {
            left: value,
            op: BinaryOp::Ne,
            right: Expression::Literal(Literal::Int(0)),
            span: span.clone(),
        }))
    } else {
        value
    }
}

fn backing_get_expr(col: &RelocColumn, span: &Span) -> Expression {
    let raw = match &col.leaf {
        RelocLeaf::String => call(
            "Vec_str_get",
            vec![field_on_v(&col.col_name, span), var("index", span)],
            span,
        ),
        RelocLeaf::Scalar(ty) => storage_to_scalar_expr(
            call(
                "Vec_i32_get",
                vec![field_on_v(&col.col_name, span), var("index", span)],
                span,
            ),
            ty,
            span,
        ),
    };
    raw
}

fn backing_len_expr(col: &RelocColumn, span: &Span) -> Expression {
    match col.leaf {
        RelocLeaf::String => call("Vec_str_len", vec![field_on_v(&col.col_name, span)], span),
        RelocLeaf::Scalar(_) => call("Vec_i32_len", vec![field_on_v(&col.col_name, span)], span),
    }
}

fn backing_free_stmt(col: &RelocColumn, span: &Span) -> Statement {
    let callee = match col.leaf {
        RelocLeaf::String => "Vec_str_free",
        RelocLeaf::Scalar(_) => "Vec_i32_free",
    };
    Statement::Expression(call(callee, vec![field_on_v(&col.col_name, span)], span))
}

fn build_struct_literal(
    name: &str,
    fields: &[(String, Expression)],
    span: &Span,
) -> Expression {
    Expression::StructLiteral(StructLiteralExpr {
        name: name.into(),
        spreads: vec![],
        fields: fields.to_vec(),
        span: span.clone(),
    })
}

fn rebuild_struct_from_columns(
    name: &str,
    struct_fields: &[StructField],
    structs: &HashMap<String, StructDef>,
    columns: &[RelocColumn],
    col_idx: &mut usize,
    span: &Span,
) -> Option<Expression> {
    let mut out_fields = Vec::new();
    for f in struct_fields {
        if is_string_like_field(&f.ty) || is_copy_scalar(&f.ty) {
            let col = columns.get(*col_idx)?;
            *col_idx += 1;
            out_fields.push((f.name.clone(), backing_get_expr(col, span)));
        } else if let TypeAnnotation::Struct(nested_name) = &f.ty {
            let nested = structs.get(nested_name)?;
            let nested_expr =
                rebuild_struct_from_columns(nested_name, &nested.fields, structs, columns, col_idx, span)?;
            out_fields.push((f.name.clone(), nested_expr));
        } else {
            return None;
        }
    }
    Some(build_struct_literal(name, &out_fields, span))
}

fn synthesize_vec_reloc_api(
    vec_name: &str,
    elem: &StructDef,
    columns: &[RelocColumn],
    structs: &HashMap<String, StructDef>,
) -> Vec<Function> {
    let span = Span::default();
    let elem_name = elem.name.clone();
    let vec_ty = TypeAnnotation::Struct(vec_name.into());
    let elem_ty = TypeAnnotation::Struct(elem_name.clone());
    let base = helper_base(vec_name);

    let new_fn = make_fn(
        &format!("{base}_new"),
        vec![],
        Some(vec_ty.clone()),
        Block {
            statements: vec![Statement::Return(ReturnStmt {
                value: Some(build_struct_literal(
                    vec_name,
                    &columns
                        .iter()
                        .map(|c| (c.col_name.clone(), backing_new_expr(&c.leaf, &span)))
                        .collect::<Vec<_>>(),
                    &span,
                )),
            })],
        },
    );

    let mut push_stmts: Vec<Statement> = columns
        .iter()
        .map(|c| Statement::Expression(backing_push_expr(c, &span)))
        .collect();
    push_stmts.push(Statement::Return(ReturnStmt {
        value: Some(var("v", &span)),
    }));

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
            statements: push_stmts,
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
            statements: vec![Statement::Return(ReturnStmt {
                value: Some({
                    let mut idx = 0usize;
                    rebuild_struct_from_columns(
                        &elem_name,
                        &elem.fields,
                        structs,
                        columns,
                        &mut idx,
                        &span,
                    )
                    .unwrap_or_else(|| build_struct_literal(&elem_name, &[], &span))
                }),
            })],
        },
    );

    let first_col = columns.first().expect("reloc vec has columns");
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
                value: Some(backing_len_expr(first_col, &span)),
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
            statements: columns
                .iter()
                .map(|c| backing_free_stmt(c, &span))
                .collect(),
        },
    );

    vec![new_fn, push_fn, get_fn, len_fn, free_fn]
}

pub fn synthesize_vec_reloc_helpers(program: &mut Program) {
    let elem_by_vec: HashMap<String, String> = program
        .structs
        .iter()
        .filter_map(|s| reloc_struct_name(&s.name).map(|elem| (s.name.clone(), elem)))
        .collect();
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

    for (vec_name, elem_name) in &elem_by_vec {
        let Some(elem) = structs.get(elem_name) else {
            continue;
        };
        if struct_is_all_copy(&elem.fields) {
            continue;
        }
        if !elem
            .fields
            .iter()
            .all(|f| is_reloc_field(&f.ty, &structs))
        {
            continue;
        }
        let mut columns = Vec::new();
        if !flatten_reloc_columns("", &[], &elem.fields, &structs, &mut columns) || columns.is_empty()
        {
            continue;
        }
        let base = helper_base(vec_name);
        if existing.contains(&format!("{base}_new")) {
            continue;
        }
        if let Some(sdef) = program.structs.iter_mut().find(|s| s.name == *vec_name) {
            sdef.fields = columns
                .iter()
                .map(|c| StructField {
                    name: c.col_name.clone(),
                    ty: TypeAnnotation::Ptr,
                })
                .collect();
        }
        for f in synthesize_vec_reloc_api(vec_name, elem, &columns, &structs) {
            program.functions.push(f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_row_program() -> Program {
        Program {
            structs: vec![
                StructDef {
                    name: "LabelRow".into(),
                    doc: None,
                    type_params: vec![],
                    attrs: StructAttrs::default(),
                    fields: vec![
                        StructField {
                            name: "label".into(),
                            ty: TypeAnnotation::String,
                        },
                        StructField {
                            name: "count".into(),
                            ty: TypeAnnotation::Integer(ast::IntKind::I32),
                        },
                    ],
                    public: true,
                },
                StructDef {
                    name: "Vec__S_LabelRow".into(),
                    doc: None,
                    type_params: vec![],
                    attrs: StructAttrs::default(),
                    fields: vec![StructField {
                        name: "handle".into(),
                        ty: TypeAnnotation::Ptr,
                    }],
                    public: true,
                },
            ],
            ..Program::default()
        }
    }

    #[test]
    fn synthesizes_vec_label_row_helpers() {
        let mut program = label_row_program();
        synthesize_vec_reloc_helpers(&mut program);
        let names: Vec<String> = program.functions.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"Vec_LabelRow_new".to_string()));
        assert!(names.contains(&"Vec_LabelRow_push".to_string()));
        assert!(names.contains(&"Vec_LabelRow_free".to_string()));
        let vec_def = program.structs.iter().find(|s| s.name == "Vec__S_LabelRow").unwrap();
        assert_eq!(vec_def.fields.len(), 2);
        assert_eq!(vec_def.fields[0].name, "label_vec");
    }

    #[test]
    fn flattens_nested_reloc_struct_columns() {
        let mut structs = HashMap::new();
        structs.insert(
            "Inner".into(),
            StructDef {
                name: "Inner".into(),
                doc: None,
                type_params: vec![],
                attrs: StructAttrs::default(),
                fields: vec![
                    StructField {
                        name: "tag".into(),
                        ty: TypeAnnotation::String,
                    },
                    StructField {
                        name: "n".into(),
                        ty: TypeAnnotation::Integer(ast::IntKind::I32),
                    },
                ],
                public: true,
            },
        );
        let outer = StructDef {
            name: "Outer".into(),
            doc: None,
            type_params: vec![],
            attrs: StructAttrs::default(),
            fields: vec![
                StructField {
                    name: "inner".into(),
                    ty: TypeAnnotation::Struct("Inner".into()),
                },
                StructField {
                    name: "score".into(),
                    ty: TypeAnnotation::Integer(ast::IntKind::I32),
                },
            ],
            public: true,
        };
        let mut cols = Vec::new();
        assert!(flatten_reloc_columns("", &[], &outer.fields, &structs, &mut cols));
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].col_name, "inner_tag_vec");
        assert_eq!(cols[2].col_name, "score_vec");
    }
}
