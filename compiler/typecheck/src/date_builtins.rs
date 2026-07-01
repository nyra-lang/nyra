use std::collections::HashMap;

use ast::TypeAnnotation;
use types::{StructInfo, Type};

use crate::TypeChecker;
use crate::{FunctionSignature, TypeEnv};

pub const DATE_STRUCT: &str = "Date";

pub fn date_field_alias(field: &str) -> &'static str {
    match field {
        "minutes" => "minute",
        "seconds" => "second",
        "weekday" => "week",
        _ => "",
    }
}

pub fn resolve_date_field_name(field: &str) -> &str {
    let alias = date_field_alias(field);
    if alias.is_empty() { field } else { alias }
}

pub fn date_struct_fields() -> Vec<(&'static str, TypeAnnotation)> {
    vec![
        ("year", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("month", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("day", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("hour", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("minute", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("second", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("week", TypeAnnotation::Integer(ast::IntKind::I32)),
        ("millisecond", TypeAnnotation::Integer(ast::IntKind::I32)),
    ]
}

impl TypeChecker {
    pub fn register_date_builtin(&mut self) {
        if self.structs.contains_key(DATE_STRUCT) {
            return;
        }
        let mut fields = HashMap::new();
        let mut field_anns = HashMap::new();
        let mut field_order = Vec::new();
        for (name, ann) in date_struct_fields() {
            fields.insert(name.to_string(), Type::from(ann.clone()));
            field_anns.insert(name.to_string(), ann.clone());
            field_order.push(name.to_string());
        }
        self.structs.insert(
            DATE_STRUCT.to_string(),
            StructInfo {
                fields,
                field_anns,
                field_order,
                repr_c: true,
                align: None,
                packed: false,
            },
        );
    }

    pub fn register_date_env(env: &mut TypeEnv) {
        env.functions.insert(
            "date".into(),
            FunctionSignature {
                params: vec![],
                return_type: Type::Struct(DATE_STRUCT.to_string()),
            },
        );
    }

    pub fn resolve_date_field<'a>(struct_name: &str, field: &'a str) -> &'a str {
        if struct_name == DATE_STRUCT {
            resolve_date_field_name(field)
        } else {
            field
        }
    }
}
