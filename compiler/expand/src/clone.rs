//! Synthesize `Clone` trait impls for structs with cloneable fields.

use std::collections::HashMap;

use ast::*;
use errors::Span;

fn field_is_cloneable(ty: &TypeAnnotation) -> bool {
    match ty {
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
        | TypeAnnotation::Bool
        | TypeAnnotation::String => true,
        TypeAnnotation::Struct(_) => true,
        TypeAnnotation::Enum(_) => true,
        TypeAnnotation::Array { elem, .. } => field_is_cloneable(elem),
        TypeAnnotation::Tuple(elems) => elems.iter().all(field_is_cloneable),
        _ => false,
    }
}

fn struct_is_cloneable(s: &StructDef) -> bool {
    s.type_params.is_empty() && s.fields.iter().all(|f| field_is_cloneable(&f.ty))
}

fn synthesize_clone_method(type_name: &str, sdef: &StructDef) -> Function {
    let span = Span::default();
    let self_ty = TypeAnnotation::Struct(type_name.to_string());
    let mut stmts = Vec::new();
    let mut field_inits = Vec::new();

    for field in &sdef.fields {
        let access = Expression::FieldAccess(Box::new(FieldAccessExpr {
            object: Expression::Variable {
                name: "self".into(),
                span: span.clone(),
            },
            field: field.name.clone(),
            optional: false,
            span: span.clone(),
        }));
        let value = if field.ty == TypeAnnotation::String {
            Expression::MethodCall(Box::new(MethodCallExpr {
                object: access,
                method: "clone".into(),
                args: vec![],
                optional: false,
                span: span.clone(),
            }))
        } else {
            access
        };
        field_inits.push((field.name.clone(), value));
    }

    stmts.push(Statement::Return(ReturnStmt {
        value: Some(Expression::StructLiteral(StructLiteralExpr {
            name: type_name.to_string(),
            spreads: vec![],
            fields: field_inits,
            span: span.clone(),
        })),
    }));

    Function {
        name: format!("{type_name}_clone"),
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
            ty: self_ty,
            destructure: vec![],
            no_escape: false,
            mutable: false,
        }],
        return_type: Some(TypeAnnotation::Struct(type_name.to_string())),
        body: Block { statements: stmts },
        inline: false,
        hot: false,
        cold: false,
        comptime: false,
    }
}

pub fn synthesize_clone_impls(program: &mut Program) {
    for s in &program.structs {
        if !struct_is_cloneable(s) {
            continue;
        }
        if program.trait_impls.iter().any(|ti| {
            ti.trait_name == "Clone" && ti.type_name == s.name
        }) {
            continue;
        }
        let method = synthesize_clone_method(&s.name, s);
        program.trait_impls.push(TraitImpl {
            type_name: s.name.clone(),
            trait_name: "Clone".into(),
            methods: vec![method],
        });
    }
}
