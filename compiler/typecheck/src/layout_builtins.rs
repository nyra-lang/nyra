//! Compile-time `size_of` / `align_of` builtins.

use ast::*;
use errors::{ErrorKind, NyraError};
use types::{align_of_ann, is_layout_intrinsic_fn, size_of_ann};

use crate::{TypeChecker, TypeEnv};
use types::Type;

impl TypeChecker {
    pub fn check_layout_intrinsic(
        &mut self,
        callee: &str,
        args: &[Expression],
        type_args: Option<&[TypeAnnotation]>,
        sp: &errors::Span,
        env: &mut TypeEnv,
    ) -> Option<Type> {
        if !is_layout_intrinsic_fn(callee) {
            return None;
        }
        if !args.is_empty() {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                sp.clone(),
                format!("`{callee}` takes no runtime arguments (use type parameter)"),
            ));
        }
        let ann = type_args?.first().cloned().or_else(|| {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                sp.clone(),
                format!("`{callee}` requires a type argument (e.g. `{callee}<i32>()`)"),
            ));
            None
        })?;
        let _ = env;
        let value = if callee == "size_of" {
            size_of_ann(&ann, &self.structs, &self.unions)
        } else {
            align_of_ann(&ann, &self.structs, &self.unions)
        };
        let _ = value;
        Some(Type::Integer(ast::IntKind::I32))
    }
}
