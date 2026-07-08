use ast::*;
use errors::Span;
use types::integer::{int_kind_of, unify_integer_types};
use types::{is_integer, Type};

use super::{TypeChecker, TypeEnv};
use super::diagnostics;

impl TypeChecker {
    pub(super) fn check_random_builtin_call(
        &mut self,
        call: &CallExpr,
        env: &mut TypeEnv,
        sp: Span,
    ) -> Option<Type> {
        match call.callee.as_str() {
            "random" => self.check_random_int_call(call, env, sp),
            "random_f64" => self.check_random_f64_call(call, env, sp),
            _ => None,
        }
    }

    fn check_random_int_call(
        &mut self,
        call: &CallExpr,
        env: &mut TypeEnv,
        sp: Span,
    ) -> Option<Type> {
        if call.args.len() != 0 && call.args.len() != 2 {
            diagnostics::builtin_wrong_arity_range(self, "random", 0, 2, call.args.len(), sp);
            return Some(Type::Unknown);
        }

        if let Some(TypeAnnotation::Integer(k)) = call.type_args.first() {
            if call.args.len() == 2 {
                for arg in &call.args {
                    let arg_ty = self.check_expr(arg, env);
                    if !is_integer(&arg_ty) && arg_ty != Type::Unknown {
                        diagnostics::builtin_arg_type(
                            self,
                            "random",
                            format!(
                                "expected integer matching `<{}>`, found {}",
                                k.name(),
                                diagnostics::type_pretty(&arg_ty),
                            ),
                            sp.clone(),
                        );
                    }
                }
            }
            return Some(Type::Integer(*k));
        }

        if call.args.is_empty() {
            return Some(Type::Integer(IntKind::I32));
        }

        let lo_ty = self.check_expr(&call.args[0], env);
        let hi_ty = self.check_expr(&call.args[1], env);
        for arg_ty in [&lo_ty, &hi_ty] {
            if !is_integer(arg_ty) && *arg_ty != Type::Unknown {
                diagnostics::builtin_arg_type(
                    self,
                    "random",
                    format!(
                        "expected integer bounds, found {}",
                        diagnostics::type_pretty(arg_ty),
                    ),
                    sp.clone(),
                );
            }
        }

        let unified = unify_integer_types(lo_ty, hi_ty);
        if int_kind_of(&unified).is_some() {
            Some(unified)
        } else {
            Some(Type::Integer(IntKind::I32))
        }
    }

    fn check_random_f64_call(
        &mut self,
        call: &CallExpr,
        env: &mut TypeEnv,
        sp: Span,
    ) -> Option<Type> {
        if call.args.len() != 0 && call.args.len() != 2 {
            diagnostics::builtin_wrong_arity_range(self, "random_f64", 0, 2, call.args.len(), sp);
            return Some(Type::Unknown);
        }

        if call.args.len() == 2 {
            for arg in &call.args {
                let arg_ty = self.check_expr(arg, env);
                let ok = arg_ty == Type::F32
                    || arg_ty == Type::F64
                    || is_integer(&arg_ty)
                    || arg_ty == Type::Unknown;
                if !ok {
                    diagnostics::builtin_arg_type(
                        self,
                        "random_f64",
                        format!(
                            "expected `f64` (or numeric), found {}",
                            diagnostics::type_pretty(&arg_ty),
                        ),
                        sp.clone(),
                    );
                }
            }
        }

        Some(Type::F64)
    }
}
