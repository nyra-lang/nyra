//! Math intrinsics — direct LLVM intrinsic lowering (no function call overhead).
use ast::*;
use types::{MathIntrinsic, resolve_math_intrinsic};

use super::{Codegen, Env, ExprValue};

impl Codegen {
    pub(super) fn compile_math_intrinsic_call(
        &mut self,
        call: &CallExpr,
        env: &Env,
    ) -> Option<ExprValue> {
        let kind = if call.callee == "abs" && call.args.len() == 1 {
            let ty = self.infer_expr_llvm_ty(&call.args[0], env);
            if ty == "double" {
                MathIntrinsic::AbsF64
            } else {
                MathIntrinsic::AbsI32
            }
        } else {
            resolve_math_intrinsic(&call.callee)?
        };

        match kind {
            MathIntrinsic::AbsI32 => {
                if call.args.len() != 1 {
                    return None;
                }
                let arg = self.compile_expr(&call.args[0], env);
                self.ensure_intrinsic_decl("llvm.abs.i32", "declare i32 @llvm.abs.i32(i32, i1)");
                let reg = self.fresh("abs");
                self.emit(&format!(
                    "  %{reg} = call i32 @llvm.abs.i32(i32 {}, i1 true)",
                    arg.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "i32".into(),
                })
            }
            MathIntrinsic::AbsF64 => {
                if call.args.len() != 1 {
                    return None;
                }
                let arg = self.compile_expr(&call.args[0], env);
                self.ensure_intrinsic_decl("llvm.fabs.f64", "declare double @llvm.fabs.f64(double)");
                let reg = self.fresh("fabs");
                self.emit(&format!(
                    "  %{reg} = call double @llvm.fabs.f64(double {})",
                    arg.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::MinI32 | MathIntrinsic::MaxI32 => {
                if call.args.len() != 2 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let b = self.compile_expr(&call.args[1], env);
                let intrinsic = if kind == MathIntrinsic::MinI32 {
                    "llvm.smin.i32"
                } else {
                    "llvm.smax.i32"
                };
                self.ensure_intrinsic_decl(intrinsic, &format!("declare i32 @{intrinsic}(i32, i32)"));
                let reg = self.fresh("mm");
                self.emit(&format!(
                    "  %{reg} = call i32 @{intrinsic}(i32 {}, i32 {})",
                    a.reg, b.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "i32".into(),
                })
            }
            MathIntrinsic::MinF64 | MathIntrinsic::MaxF64 => {
                if call.args.len() != 2 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let b = self.compile_expr(&call.args[1], env);
                let intrinsic = if kind == MathIntrinsic::MinF64 {
                    "llvm.minnum.f64"
                } else {
                    "llvm.maxnum.f64"
                };
                self.ensure_intrinsic_decl(
                    intrinsic,
                    &format!("declare double @{intrinsic}(double, double)"),
                );
                let reg = self.fresh("mm");
                self.emit(&format!(
                    "  %{reg} = call double @{intrinsic}(double {}, double {})",
                    a.reg, b.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::SinF64 => {
                if call.args.len() != 1 {
                    return None;
                }
                let arg = self.compile_expr(&call.args[0], env);
                self.record_runtime("sin_f64");
                let reg = self.fresh("sin");
                self.emit(&format!(
                    "  %{reg} = call double @sin_f64(double {})",
                    arg.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::CosF64 => {
                if call.args.len() != 1 {
                    return None;
                }
                let arg = self.compile_expr(&call.args[0], env);
                self.record_runtime("cos_f64");
                let reg = self.fresh("cos");
                self.emit(&format!(
                    "  %{reg} = call double @cos_f64(double {})",
                    arg.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::TanF64 => {
                if call.args.len() != 1 {
                    return None;
                }
                let arg = self.compile_expr(&call.args[0], env);
                self.record_runtime("tan_f64");
                let reg = self.fresh("tan");
                self.emit(&format!(
                    "  %{reg} = call double @tan_f64(double {})",
                    arg.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::Atan2F64 => {
                if call.args.len() != 2 {
                    return None;
                }
                let y = self.compile_expr(&call.args[0], env);
                let x = self.compile_expr(&call.args[1], env);
                self.record_runtime("atan2_f64");
                let reg = self.fresh("atan2");
                self.emit(&format!(
                    "  %{reg} = call double @atan2_f64(double {}, double {})",
                    y.reg, x.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: "double".into(),
                })
            }
            MathIntrinsic::ClampI32 => {
                if call.args.len() != 3 {
                    return None;
                }
                let x = self.compile_expr(&call.args[0], env);
                let lo = self.compile_expr(&call.args[1], env);
                let hi = self.compile_expr(&call.args[2], env);
                self.ensure_intrinsic_decl("llvm.smax.i32", "declare i32 @llvm.smax.i32(i32, i32)");
                self.ensure_intrinsic_decl("llvm.smin.i32", "declare i32 @llvm.smin.i32(i32, i32)");
                let t1 = self.fresh("clamp");
                let t2 = self.fresh("clamp");
                self.emit(&format!(
                    "  %{t1} = call i32 @llvm.smax.i32(i32 {}, i32 {})",
                    x.reg, lo.reg
                ));
                self.emit(&format!(
                    "  %{t2} = call i32 @llvm.smin.i32(i32 %{t1}, i32 {})",
                    hi.reg
                ));
                Some(ExprValue {
                    reg: format!("%{t2}"),
                    ty: "i32".into(),
                })
            }
        }
    }

    pub(super) fn compile_layout_intrinsic_call(
        &mut self,
        call: &CallExpr,
        _env: &Env,
    ) -> Option<ExprValue> {
        let value = match call.callee.as_str() {
            "size_of" => {
                let ann = call.type_args.first()?;
                types::size_of_ann(ann, &self.struct_layout_infos, &self.union_layout_infos)
            }
            "align_of" => {
                let ann = call.type_args.first()?;
                types::align_of_ann(ann, &self.struct_layout_infos, &self.union_layout_infos)
            }
            _ => return None,
        };
        Some(ExprValue {
            reg: value.to_string(),
            ty: "i32".into(),
        })
    }

    fn ensure_intrinsic_decl(&mut self, name: &str, decl: &str) {
        if self.intrinsic_decls.insert(name.to_string()) {
            self.intrinsic_decl_lines.push(decl.to_string());
        }
    }

    pub(super) fn compile_random_builtin_call(
        &mut self,
        call: &CallExpr,
        env: &Env,
    ) -> Option<ExprValue> {
        match call.callee.as_str() {
            "random" => self.compile_random_int_call(call, env),
            "random_f64" => self.compile_random_f64_call(call, env),
            _ => None,
        }
    }

    fn compile_random_int_call(&mut self, call: &CallExpr, env: &Env) -> Option<ExprValue> {
        let kind = if let Some(TypeAnnotation::Integer(k)) = call.type_args.first() {
            *k
        } else if call.args.len() == 2 {
            let k0 = self.infer_random_int_kind(&call.args[0], env);
            let k1 = self.infer_random_int_kind(&call.args[1], env);
            IntKind::unify(k0, k1)
        } else {
            IntKind::I32
        };

        let (llvm_ty, full_sym, full_decl, range_sym, range_decl) =
            Self::random_int_runtime(kind);
        let reg = self.fresh("random");

        if call.args.is_empty() {
            self.ensure_runtime_fn_decl(full_sym, full_decl);
            self.emit(&format!("  %{reg} = call {llvm_ty} @{full_sym}()"));
        } else if call.args.len() == 2 {
            let lo_raw = self.compile_expr(&call.args[0], env);
            let hi_raw = self.compile_expr(&call.args[1], env);
            let lo = self.coerce_expr_to_llvm_type(lo_raw, llvm_ty);
            let hi = self.coerce_expr_to_llvm_type(hi_raw, llvm_ty);
            self.ensure_runtime_fn_decl(range_sym, range_decl);
            self.emit(&format!(
                "  %{reg} = call {llvm_ty} @{range_sym}({llvm_ty} {}, {llvm_ty} {})",
                lo.reg, hi.reg
            ));
        } else {
            return None;
        }

        let result_ty = types::int_llvm(kind);
        let result = ExprValue {
            reg: format!("%{reg}"),
            ty: result_ty.into(),
        };
        if result_ty != llvm_ty {
            Some(self.coerce_expr_to_llvm_type(result, result_ty))
        } else {
            Some(result)
        }
    }

    fn compile_random_f64_call(&mut self, call: &CallExpr, env: &Env) -> Option<ExprValue> {
        let reg = self.fresh("random_f64");
        if call.args.is_empty() {
            self.ensure_runtime_fn_decl("rand_f64", "declare double @rand_f64()");
            self.emit(&format!("  %{reg} = call double @rand_f64()"));
        } else if call.args.len() == 2 {
            let lo_raw = self.compile_expr(&call.args[0], env);
            let hi_raw = self.compile_expr(&call.args[1], env);
            let lo = self.coerce_expr_to_llvm_type(lo_raw, "double");
            let hi = self.coerce_expr_to_llvm_type(hi_raw, "double");
            self.ensure_runtime_fn_decl(
                "rand_f64_range",
                "declare double @rand_f64_range(double, double)",
            );
            self.emit(&format!(
                "  %{reg} = call double @rand_f64_range(double {}, double {})",
                lo.reg, hi.reg
            ));
        } else {
            return None;
        }
        Some(ExprValue {
            reg: format!("%{reg}"),
            ty: "double".into(),
        })
    }

    pub(super) fn infer_random_int_kind(&self, expr: &Expression, env: &Env) -> IntKind {
        match expr {
            Expression::Literal(Literal::IntKind(_, k)) => *k,
            Expression::Literal(Literal::Int(_)) => IntKind::I32,
            Expression::Variable { name, .. } => self
                .local_int_kinds
                .get(name)
                .copied()
                .unwrap_or_else(|| {
                    env.get(name)
                        .map(|b| Self::binding_ty(b))
                        .map(|ty| Self::int_kind_from_llvm_ty_default_signed(ty))
                        .unwrap_or(IntKind::I32)
                }),
            Expression::Grouped(inner) => self.infer_random_int_kind(inner, env),
            _ => IntKind::I32,
        }
    }

    fn int_kind_from_llvm_ty_default_signed(ty: &str) -> IntKind {
        match ty {
            "i8" => IntKind::I8,
            "i16" => IntKind::I16,
            "i32" => IntKind::I32,
            "i64" => IntKind::I64,
            "i128" => IntKind::I128,
            _ => IntKind::I32,
        }
    }

    fn random_int_runtime(
        kind: IntKind,
    ) -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) {
        match kind {
            IntKind::U8 | IntKind::U16 | IntKind::U32 => (
                "i32",
                "rand_u32",
                "declare i32 @rand_u32()",
                "rand_range_u32",
                "declare i32 @rand_range_u32(i32, i32)",
            ),
            IntKind::U64 | IntKind::USize | IntKind::U128 => (
                "i64",
                "rand_u64",
                "declare i64 @rand_u64()",
                "rand_range_u64",
                "declare i64 @rand_range_u64(i64, i64)",
            ),
            IntKind::I64 | IntKind::ISize | IntKind::I128 => (
                "i64",
                "rand_i64",
                "declare i64 @rand_i64()",
                "rand_range_i64",
                "declare i64 @rand_range_i64(i64, i64)",
            ),
            _ => (
                "i32",
                "rand_i32",
                "declare i32 @rand_i32()",
                "rand_range",
                "declare i32 @rand_range(i32, i32)",
            ),
        }
    }
}
