//! Compile-time `size_of` / `align_of` builtins.

use ast::*;
use types::{align_of_ann, is_layout_intrinsic_fn, size_of_ann};

use crate::{TypeChecker, TypeEnv};
use crate::diagnostics;
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
            diagnostics::layout_intrinsic_no_args(self, callee, sp.clone());
        }
        let ann = type_args?.first().cloned().or_else(|| {
            diagnostics::layout_intrinsic_requires_type_arg(self, callee, sp.clone());
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
