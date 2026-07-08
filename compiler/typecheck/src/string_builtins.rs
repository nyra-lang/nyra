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
    // All `String_*` stdlib helpers borrow the receiver (`&string`).
    if method.starts_with("String_") {
        return true;
    }
    matches!(
        method,
        "clone" | "length" | "len" | "split" | "trim" | "contains" | "starts_with"
            | "ends_with" | "replace" | "replacen" | "to_upper" | "to_lower" | "strip_suffix"
            // Case-conversion string builtins — all take `&string`.
            | "to_snake_case" | "to_lowercase" | "to_titlecase" | "to_capitalize"
            | "to_camel_case" | "to_kebab_case" | "to_pascal_case"
            | "to_screaming_snake_case" | "to_train_case" | "to_dot_case"
            // JS-style string aliases mapping to `String_*` (`&string` | "strip_prefix" | "index" | "is_empty" | "last_index" | "repeat" | "trim_end" | "trim_start" | "splitn" | "count" | "fields" | "pad_end" | "pad_start" | "split_once") helpers.
            | "toUpperCase" | "toLowerCase" | "includes" | "stripSuffix")
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
            // [builtin-dev:strip_suffix:string]
            "strip_suffix" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".strip_suffix"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::String
            }
            // [/builtin-dev:strip_suffix:string]
                        // [builtin-dev:to_snake_case:string]
            "to_snake_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_snake_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_snake_case:string]
            
            
            // [builtin-dev:to_lowercase:string]
            "to_lowercase" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_lowercase"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_lowercase:string]
            
            // [builtin-dev:to_titlecase:string]
            "to_titlecase" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_titlecase"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_titlecase:string]
            
            // [builtin-dev:to_capitalize:string]
            "to_capitalize" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_capitalize"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_capitalize:string]
            
            // [builtin-dev:to_camel_case:string]
            "to_camel_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_camel_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_camel_case:string]
            
            // [builtin-dev:to_kebab_case:string]
            "to_kebab_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_kebab_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_kebab_case:string]
            
            // [builtin-dev:to_pascal_case:string]
            "to_pascal_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_pascal_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_pascal_case:string]
            
            // [builtin-dev:to_screaming_snake_case:string]
            "to_screaming_snake_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_screaming_snake_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_screaming_snake_case:string]
            
            // [builtin-dev:to_train_case:string]
            "to_train_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_train_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_train_case:string]
            
            // [builtin-dev:to_dot_case:string]
            "to_dot_case" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".to_dot_case"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:to_dot_case:string]
            
            
            // [builtin-dev:strip_prefix:string]
            "strip_prefix" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".strip_prefix"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::String
            }
            // [/builtin-dev:strip_prefix:string]
            
            // [builtin-dev:index:string]
            "index" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".index"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::Integer(ast::IntKind::I32)
            }
            // [/builtin-dev:index:string]
            
            // [builtin-dev:is_empty:string]
            "is_empty" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".is_empty"), 0, mc.args.len(), sp.clone());
                }
                Type::Integer(ast::IntKind::I32)
            }
            // [/builtin-dev:is_empty:string]
            
            // [builtin-dev:last_index:string]
            "last_index" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".last_index"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::Integer(ast::IntKind::I32)
            }
            // [/builtin-dev:last_index:string]
            
            // [builtin-dev:repeat:string]
            "repeat" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".repeat"), 1, mc.args.len(), sp.clone());
                } else {
                    let _arg0 = self.check_expr(&mc.args[0], env);
                    if _arg0 != Type::Integer(ast::IntKind::I32) && _arg0 != Type::Unknown {
                        diagnostics::wrong_arity(self, &format!(".repeat arg 0"), 0, 0, sp.clone());
                    }
                }
                Type::String
            }
            // [/builtin-dev:repeat:string]
            
            // [builtin-dev:trim_end:string]
            "trim_end" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".trim_end"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:trim_end:string]
            
            // [builtin-dev:trim_start:string]
            "trim_start" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".trim_start"), 0, mc.args.len(), sp.clone());
                }
                Type::String
            }
            // [/builtin-dev:trim_start:string]
            
            // [builtin-dev:splitn:string]
            "splitn" => {
                if mc.args.len() != 2 {
                    diagnostics::wrong_arity(self, &format!(".splitn"), 2, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                    let _arg1 = self.check_expr(&mc.args[1], env);
                    if _arg1 != Type::Integer(ast::IntKind::I32) && _arg1 != Type::Unknown {
                        diagnostics::wrong_arity(self, &format!(".splitn arg 1"), 0, 0, sp.clone());
                    }
                }
                Type::VecStr
            }
            // [/builtin-dev:splitn:string]
            
            // [builtin-dev:count:string]
            "count" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".count"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::Integer(ast::IntKind::I32)
            }
            // [/builtin-dev:count:string]
            
            // [builtin-dev:fields:string]
            "fields" => {
                if !mc.args.is_empty() {
                    diagnostics::wrong_arity(self, &format!(".fields"), 0, mc.args.len(), sp.clone());
                }
                Type::VecStr
            }
            // [/builtin-dev:fields:string]
            
            // [builtin-dev:pad_end:string]
            "pad_end" => {
                if mc.args.len() != 2 {
                    diagnostics::wrong_arity(self, &format!(".pad_end"), 2, mc.args.len(), sp.clone());
                } else {
                    let _arg0 = self.check_expr(&mc.args[0], env);
                    if _arg0 != Type::Integer(ast::IntKind::I32) && _arg0 != Type::Unknown {
                        diagnostics::wrong_arity(self, &format!(".pad_end arg 0"), 0, 0, sp.clone());
                    }
                    self.check_string_arg(mc, 1, env, sp);
                }
                Type::String
            }
            // [/builtin-dev:pad_end:string]
            
            // [builtin-dev:pad_start:string]
            "pad_start" => {
                if mc.args.len() != 2 {
                    diagnostics::wrong_arity(self, &format!(".pad_start"), 2, mc.args.len(), sp.clone());
                } else {
                    let _arg0 = self.check_expr(&mc.args[0], env);
                    if _arg0 != Type::Integer(ast::IntKind::I32) && _arg0 != Type::Unknown {
                        diagnostics::wrong_arity(self, &format!(".pad_start arg 0"), 0, 0, sp.clone());
                    }
                    self.check_string_arg(mc, 1, env, sp);
                }
                Type::String
            }
            // [/builtin-dev:pad_start:string]
            
            // [builtin-dev:split_once:string]
            "split_once" => {
                if mc.args.len() != 1 {
                    diagnostics::wrong_arity(self, &format!(".split_once"), 1, mc.args.len(), sp.clone());
                } else {
                    self.check_string_arg(mc, 0, env, sp);
                }
                Type::String
            }
            // [/builtin-dev:split_once:string]
            
_ => return None,
        };
        Some(ret)
    }
}
