//! `bytes` type method and operator builtins.

use types::Type;

use crate::TypeChecker;
use crate::diagnostics;

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
            diagnostics::bytes_index_requires_bytes(checker, sp.clone());
        }
        Type::Integer(ast::IntKind::I32)
    }
}
