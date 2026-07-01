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
}
