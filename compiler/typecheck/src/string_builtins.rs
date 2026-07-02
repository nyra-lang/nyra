use ast::*;
use errors::Span;
use types::Type;

use crate::TypeChecker;
use crate::TypeEnv;
use crate::diagnostics;

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
            diagnostics::string_method_arg_must_be_string(self, &mc.method, sp.clone());
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
                    diagnostics::wrong_arity(self, ".split", 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::VecStr
            }
            "trim" | "to_upper" | "to_lower" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".{method}"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            "contains" | "starts_with" | "ends_with" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".{method}"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::Integer(ast::IntKind::I32)
            }
            "replace" => {
                if mc.args.len() != 2 {
                    diagnostics::wrong_arity(self, ".replace", 2, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                    self.check_string_arg(mc, 1, env, sp);
                }
                Type::String
            }
            "replacen" => {
                if mc.args.len() != 3 {
                    diagnostics::wrong_arity(self, ".replacen", 3, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                    self.check_string_arg(mc, 1, env, sp);
                    let count_ty = self.check_expr(&mc.args[2], env);
                    if count_ty != Type::Integer(ast::IntKind::I32) && count_ty != Type::Unknown {
                        diagnostics::string_replacen_count_must_be_i32(self, sp.clone());
                    }
                }
                Type::String
            }
            _ => return None,
        };
        Some(ret)
    }
}
