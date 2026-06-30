use ast::*;
use errors::{ErrorKind, NyraError, Span};
use types::Type;

use crate::TypeChecker;
use crate::TypeEnv;

fn is_string_like(ty: &Type) -> bool {
    if ty == &Type::String || ty == &Type::Unknown {
        return true;
    }
    matches!(
        ty,
        Type::Ref {
            inner,
            mutable: false,
            ..
        } if **inner == Type::String
    )
}

pub fn string_method_borrows_receiver(method: &str) -> bool {
    matches!(
        method,
        "clone" | "length" | "len" | "split" | "trim" | "contains" | "starts_with"
            | "ends_with" | "replace" | "replacen" | "to_upper" | "to_lower"
    )
}

impl TypeChecker {
    fn check_string_arg(
        &mut self,
        mc: &MethodCallExpr,
        idx: usize,
        env: &mut TypeEnv,
        sp: &Span,
    ) {
        let ty = self.check_expr(&mc.args[idx], env);
        if !is_string_like(&ty) {
            self.errors.push(NyraError::new(
                ErrorKind::Type,
                sp.clone(),
                format!("'.{}' argument must be string", mc.method),
            ));
        }
    }

    pub(super) fn check_string_method(
        &mut self,
        mc: &MethodCallExpr,
        obj_ty: &Type,
        env: &mut TypeEnv,
        sp: &Span,
    ) -> Option<Type> {
        if !matches!(obj_ty, Type::String) {
            return None;
        }

        let method = mc.method.as_str();
        let ret = match method {
            "split" => {
                if mc.args.len() != 1 {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!("'.split' expects 1 argument, got {}", mc.args.len()),
                    ));
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::VecStr
            }
            "trim" | "to_upper" | "to_lower" => {
                if !mc.args.is_empty() {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!("'.{method}' expects no arguments"),
                    ));
                }
                Type::String
            }
            "contains" | "starts_with" | "ends_with" => {
                if mc.args.len() != 1 {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!("'.{method}' expects 1 argument, got {}", mc.args.len()),
                    ));
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::Integer(ast::IntKind::I32)
            }
            "replace" => {
                if mc.args.len() != 2 {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!("'.replace' expects 2 arguments, got {}", mc.args.len()),
                    ));
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                    self.check_string_arg(mc, 1, env, sp);
                }
                Type::String
            }
            "replacen" => {
                if mc.args.len() != 3 {
                    self.errors.push(NyraError::new(
                        ErrorKind::Type,
                        sp.clone(),
                        format!("'.replacen' expects 3 arguments, got {}", mc.args.len()),
                    ));
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                    self.check_string_arg(mc, 1, env, sp);
                    let count_ty = self.check_expr(&mc.args[2], env);
                    if count_ty != Type::Integer(ast::IntKind::I32) && count_ty != Type::Unknown {
                        self.errors.push(NyraError::new(
                            ErrorKind::Type,
                            sp.clone(),
                            "'.replacen' count argument must be i32",
                        ));
                    }
                }
                Type::String
            }
            _ => return None,
        };
        Some(ret)
    }
}
