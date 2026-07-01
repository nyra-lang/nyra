//! `bytes` type method and operator builtins.

use ast::*;
use errors::{ErrorKind, NyraError};

use crate::TypeChecker;
use types::Type;

impl TypeChecker {
    pub fn bytes_method_return_type(method: &str) -> Option<Type> {
        match method {
            "len" | "length" => Some(Type::Integer(ast::IntKind::I64)),
            "to_string" => Some(Type::String),
            _ => None,
        }
    }

    pub fn check_bytes_index(
        checker: &mut TypeChecker,
        obj_ty: &Type,
        sp: &errors::Span,
    ) -> Type {
        if obj_ty != &Type::Bytes && *obj_ty != Type::Unknown {
            checker.errors.push(NyraError::new(
                ErrorKind::Type,
                sp.clone(),
                "indexing requires `bytes` value",
            ));
        }
        Type::Integer(ast::IntKind::I32)
    }
}
