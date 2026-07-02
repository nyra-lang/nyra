//! Compiler math intrinsics (`abs_i32`, `min_i32`, …) — always available, no stdlib required.

use ast::*;
use errors::Span;
use types::{is_integer, resolve_math_intrinsic, MathIntrinsic, Type};

use super::{TypeChecker, TypeEnv};
use super::diagnostics;

impl TypeChecker {
    pub(super) fn check_math_intrinsic_call(
        &mut self,
        call: &CallExpr,
        env: &mut TypeEnv,
        sp: Span,
    ) -> Option<Type> {
        let kind = if call.callee == "abs" {
            if call.args.len() != 1 {
                diagnostics::wrong_arity(self, "abs", 1, call.args.len(), sp);
                return Some(Type::Unknown);
            }
            let arg_ty = self.check_expr(&call.args[0], env);
            if arg_ty == Type::F64 {
                return Some(Type::F64);
            }
            if is_integer(&arg_ty) || arg_ty == Type::Unknown {
                return Some(Type::Integer(IntKind::I32));
            }
            diagnostics::builtin_arg_type(
                self,
                "abs",
                format!("expected `i32` or `f64`, found {}", diagnostics::type_pretty(&arg_ty)),
                sp,
            );
            return Some(Type::Unknown);
        } else {
            resolve_math_intrinsic(&call.callee)?
        };

        let (expected, ret) = match kind {
            MathIntrinsic::AbsI32 => (1, Type::Integer(IntKind::I32)),
            MathIntrinsic::AbsF64 => (1, Type::F64),
            MathIntrinsic::MinI32 | MathIntrinsic::MaxI32 => (2, Type::Integer(IntKind::I32)),
            MathIntrinsic::MinF64 | MathIntrinsic::MaxF64 => (2, Type::F64),
            MathIntrinsic::ClampI32 => (3, Type::Integer(IntKind::I32)),
            MathIntrinsic::SinF64 | MathIntrinsic::CosF64 | MathIntrinsic::TanF64 => {
                (1, Type::F64)
            }
            MathIntrinsic::Atan2F64 => (2, Type::F64),
        };

        if call.args.len() != expected {
            diagnostics::wrong_arity(self, &call.callee, expected, call.args.len(), sp);
            return Some(Type::Unknown);
        }

        for arg in &call.args {
            let arg_ty = self.check_expr(arg, env);
            let ok = match kind {
                MathIntrinsic::AbsI32
                | MathIntrinsic::MinI32
                | MathIntrinsic::MaxI32
                | MathIntrinsic::ClampI32 => is_integer(&arg_ty) || arg_ty == Type::Unknown,
                MathIntrinsic::AbsF64
                | MathIntrinsic::MinF64
                | MathIntrinsic::MaxF64
                | MathIntrinsic::SinF64
                | MathIntrinsic::CosF64
                | MathIntrinsic::TanF64
                | MathIntrinsic::Atan2F64 => {
                    arg_ty == Type::F32 || arg_ty == Type::F64 || arg_ty == Type::Unknown
                }
            };
            if !ok {
                diagnostics::builtin_arg_type(
                    self,
                    &call.callee,
                    format!("found {}", diagnostics::type_pretty(&arg_ty)),
                    sp.clone(),
                );
            }
        }
        Some(ret)
    }
}
