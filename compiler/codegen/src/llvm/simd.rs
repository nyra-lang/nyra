//! Portable SIMD intrinsics — LLVM vector operations.
use ast::*;
use types::{resolve_simd_intrinsic, SimdIntrinsic};

use super::{Codegen, Env, ExprValue};

impl Codegen {
    pub(super) fn compile_simd_intrinsic_call(
        &mut self,
        call: &CallExpr,
        env: &Env,
    ) -> Option<ExprValue> {
        let kind = resolve_simd_intrinsic(&call.callee)?;

        match kind {
            SimdIntrinsic::AddI32x4 | SimdIntrinsic::AddF32x4 | SimdIntrinsic::AddF64x2 => {
                if call.args.len() != 2 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let b = self.compile_expr(&call.args[1], env);
                let reg = self.fresh("simd_add");
                self.emit(&format!(
                    "  %{reg} = add {} {}, {}",
                    a.ty, a.reg, b.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: a.ty,
                })
            }
            SimdIntrinsic::MulI32x4 | SimdIntrinsic::MulF32x4 | SimdIntrinsic::MulF64x2 => {
                if call.args.len() != 2 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let b = self.compile_expr(&call.args[1], env);
                let reg = self.fresh("simd_mul");
                self.emit(&format!(
                    "  %{reg} = mul {} {}, {}",
                    a.ty, a.reg, b.reg
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: a.ty,
                })
            }
            SimdIntrinsic::SplatI32x4 => {
                if call.args.len() != 1 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let reg = self.fresh("splat");
                self.emit(&format!(
                    "  %{reg} = insertelement <4 x i32> poison, i32 {}, i32 0",
                    a.reg
                ));
                let r2 = self.fresh("splat");
                self.emit(&format!(
                    "  %{r2} = shufflevector <4 x i32> %{reg}, <4 x i32> poison, <4 x i32> zeroinitializer"
                ));
                Some(ExprValue {
                    reg: format!("%{r2}"),
                    ty: "<4 x i32>".into(),
                })
            }
            SimdIntrinsic::SplatF32x4 => {
                if call.args.len() != 1 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let reg = self.fresh("splat");
                self.emit(&format!(
                    "  %{reg} = insertelement <4 x float> poison, float {}, i32 0",
                    a.reg
                ));
                let r2 = self.fresh("splat");
                self.emit(&format!(
                    "  %{r2} = shufflevector <4 x float> %{reg}, <4 x float> poison, <4 x i32> zeroinitializer"
                ));
                Some(ExprValue {
                    reg: format!("%{r2}"),
                    ty: "<4 x float>".into(),
                })
            }
            SimdIntrinsic::SplatF64x2 => {
                if call.args.len() != 1 {
                    return None;
                }
                let a = self.compile_expr(&call.args[0], env);
                let reg = self.fresh("splat");
                self.emit(&format!(
                    "  %{reg} = insertelement <2 x double> poison, double {}, i32 0",
                    a.reg
                ));
                let r2 = self.fresh("splat");
                self.emit(&format!(
                    "  %{r2} = shufflevector <2 x double> %{reg}, <2 x double> poison, <2 x i32> zeroinitializer"
                ));
                Some(ExprValue {
                    reg: format!("%{r2}"),
                    ty: "<2 x double>".into(),
                })
            }
            SimdIntrinsic::LoadI32x4 | SimdIntrinsic::LoadF32x4 | SimdIntrinsic::LoadF64x2 => {
                if call.args.len() != 1 {
                    return None;
                }
                let ptr = self.compile_expr(&call.args[0], env);
                let (vec_ty, elem_ty) = match kind {
                    SimdIntrinsic::LoadI32x4 => ("<4 x i32>", "i32"),
                    SimdIntrinsic::LoadF32x4 => ("<4 x float>", "float"),
                    _ => ("<2 x double>", "double"),
                };
                let bc = self.fresh("bc");
                self.emit(&format!("  %{bc} = bitcast ptr {} to {elem_ty}*", ptr.reg));
                let reg = self.fresh("load");
                self.emit(&format!(
                    "  %{reg} = load {vec_ty}, {vec_ty}* %{bc}"
                ));
                Some(ExprValue {
                    reg: format!("%{reg}"),
                    ty: vec_ty.into(),
                })
            }
            SimdIntrinsic::StoreI32x4 | SimdIntrinsic::StoreF32x4 | SimdIntrinsic::StoreF64x2 => {
                if call.args.len() != 2 {
                    return None;
                }
                let ptr = self.compile_expr(&call.args[0], env);
                let val = self.compile_expr(&call.args[1], env);
                let vec_ty = match kind {
                    SimdIntrinsic::StoreI32x4 => "<4 x i32>",
                    SimdIntrinsic::StoreF32x4 => "<4 x float>",
                    _ => "<2 x double>",
                };
                let bc = self.fresh("bc");
                self.emit(&format!("  %{bc} = bitcast ptr {} to {vec_ty}*", ptr.reg));
                self.emit(&format!(
                    "  store {vec_ty} {}, {vec_ty}* %{bc}",
                    val.reg
                ));
                Some(ExprValue {
                    reg: "0".into(),
                    ty: "void".into(),
                })
            }
        }
    }
}
