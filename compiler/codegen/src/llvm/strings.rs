#![allow(unused_imports)]
//! String builtins, template literals, and printf helpers.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use ast::*;
use ownership::{
    arrow_has_captures, arrow_to_block, callee_returns_owned, collect_arrow_captures,
    collect_captures, DropPlan, EscapePlan, EscapeState,
};

use crate::ansi_color::color_spec_to_ansi;
use crate::runtime_map::RuntimeProfile;

use super::{
    Binding, ClosureMeta, Codegen, DropState, Env, EnvKind, ExprValue, FnPtrSig, LoopPhiContext,
    NestedFnCodegenScope, LOCAL_CHANNEL_CAP, LOCAL_CHANNEL_TYPE,
};
use super::util::{
    array_elem_from_ty, array_len_from_ty, assign_target_name, collect_assigned_in_block,
    escape_string, host_target_triple, is_array_ty, is_string_builtin_method, llvm_arith_rhs,
    llvm_binop_operand, llvm_cmp_operand, llvm_float_const, llvm_ptr, llvm_ptr_reg, llvm_storage_ty,
    llvm_string_len, llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann,
    llvm_value_operand, resolve_struct_field_name, struct_name_from_llvm_ty, struct_ptr_type,
    struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn emit_i32_to_string(&mut self, reg: &str, ty: &str) -> ExprValue {
        let src = if ty == "i1" {
            let ext = self.fresh("zext");
            self.emit(&format!("  %{ext} = zext i1 {reg} to i32"));
            format!("%{ext}")
        } else {
            reg.to_string()
        };
        let out = self.fresh("tostr");
        self.emit_runtime_call(
            "i32_to_string",
            &format!("  %{out} = call ptr @i32_to_string(i32 {src})"),
        );
        ExprValue {
            reg: format!("%{out}"),
            ty: "ptr".into(),
        }
    }

    pub(super) fn emit_i64_to_string(&mut self, reg: &str, ty: &str) -> ExprValue {
        let src = if ty == "i64" {
            reg.to_string()
        } else {
            let ext = self.fresh("sext");
            self.emit(&format!("  %{ext} = sext {ty} {reg} to i64"));
            format!("%{ext}")
        };
        let out = self.fresh("tostr");
        self.emit_runtime_call(
            "i64_to_string",
            &format!("  %{out} = call ptr @i64_to_string(i64 {src})"),
        );
        ExprValue {
            reg: format!("%{out}"),
            ty: "ptr".into(),
        }
    }

    pub(super) fn heap_clone_string(&mut self, val: ExprValue) -> ExprValue {
        let ptr = self.materialize_ptr_reg(&val.reg);
        let reg = self.fresh("str_clone");
        self.emit_runtime_call(
            "str_clone",
            &format!("  %{reg} = call ptr @str_clone(ptr {ptr})"),
        );
        ExprValue {
            reg: format!("%{reg}"),
            ty: "ptr".into(),
        }
    }

    pub(super) fn expr_string_is_heap_owned(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Call(c) => callee_returns_owned(&c.callee),
            Expression::MethodCall(mc) => matches!(
                mc.method.as_str(),
                "trim" | "to_upper" | "to_lower" | "replace" | "replacen"
            ),
            Expression::TemplateLiteral(_) => true,
            Expression::Variable { name, .. } => {
                !self.current_func.is_empty()
                    && self.drop_plan.is_owned_in(&self.current_func, name)
            }
            _ => false,
        }
    }

    pub(super) fn rvalue_produces_heap_string(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Call(c) => callee_returns_owned(&c.callee),
            Expression::MethodCall(mc) => matches!(
                mc.method.as_str(),
                "trim" | "to_upper" | "to_lower" | "replace" | "replacen"
            ),
            Expression::TemplateLiteral(_) => true,
            _ => false,
        }
    }

    pub(super) fn emit_strcat(&mut self, left: &ExprValue, right: &ExprValue) -> ExprValue {
        let a = self.materialize_ptr_reg(&left.reg);
        let b = self.materialize_ptr_reg(&right.reg);
        let reg = self.fresh("strcat");
        self.emit_runtime_call(
            "strcat",
            &format!("  %{reg} = call ptr @strcat(ptr {a}, ptr {b})"),
        );
        ExprValue {
            reg: format!("%{reg}"),
            ty: "ptr".into(),
        }
    }

    pub(super) fn emit_free(&mut self, reg: &str) {
        let ptr = self.materialize_ptr_reg(reg);
        self.emit_runtime_call("free", &format!("  call void @free(ptr {ptr})"));
    }

    pub(super) fn compile_template_literal(
        &mut self,
        t: &TemplateLiteralExpr,
        env: &Env,
    ) -> ExprValue {
        let mut acc: Option<(ExprValue, bool)> = None;
        for part in &t.parts {
            let piece = match part {
                TemplatePart::Static(text) => self.compile_string_piece(
                    &Expression::Literal(Literal::String(text.clone())),
                    env,
                ),
                TemplatePart::Interpolation(expr) => self.compile_string_piece(expr, env),
            };
            acc = Some(match acc {
                None => piece,
                Some((prev, prev_owned)) => {
                    let merged = self.emit_strcat(&prev, &piece.0);
                    if prev_owned {
                        self.emit_free(&prev.reg);
                    }
                    if piece.1 {
                        self.emit_free(&piece.0.reg);
                    }
                    (merged, true)
                }
            });
        }
        acc.map(|(v, _)| v).unwrap_or_else(|| {
            let idx = self.intern_string("");
            let reg = self.fresh("str");
            self.emit(&format!(
                "  %{reg} = getelementptr inbounds i8, ptr @.str.{idx}, i64 0"
            ));
            ExprValue {
                reg: format!("%{reg}"),
                ty: "ptr".into(),
            }
        })
    }

    pub(super) fn compile_buffered_io(
        &mut self,
        expr: &Expression,
        env: &Env,
        newline: bool,
    ) {
        let val = self.compile_expr(expr, env);
        if is_array_ty(&val.ty) {
            let formatted = self.emit_array_debug_string(&val);
            let ptr = self.materialize_ptr_reg(&formatted.reg);
            let callee = if newline {
                "stdout_writeln_str"
            } else {
                "stdout_write_str"
            };
            self.emit_runtime_call(callee, &format!("  call void @{callee}(ptr {ptr})"));
            return;
        }
        let (write_fn, writeln_fn) = if val.ty == "ptr" || val.ty == "i8*" {
            ("stdout_write_str", "stdout_writeln_str")
        } else {
            ("stdout_write_i32", "stdout_writeln_i32")
        };
        let callee = if newline { writeln_fn } else { write_fn };
        if val.ty == "ptr" || val.ty == "i8*" {
            let ptr = self.materialize_ptr_reg(&val.reg);
            self.emit_runtime_call(
                callee,
                &format!("  call void @{callee}(ptr {ptr})"),
            );
        } else if val.ty == "i1" {
            let ext = self.fresh("zext");
            self.emit(&format!("  %{ext} = zext i1 {} to i32", val.reg));
            self.emit_runtime_call(
                callee,
                &format!("  call void @{callee}(i32 %{ext})"),
            );
        } else {
            self.emit_runtime_call(
                callee,
                &format!("  call void @{callee}(i32 {})", val.reg),
            );
        }
    }

    /// Compile a top-level `const` initializer without emitting function-body IR.
    pub(super) fn compile_module_const_value(&mut self, expr: &Expression) -> ExprValue {
        match expr {
            Expression::Literal(Literal::Int(n)) => ExprValue {
                reg: n.to_string(),
                ty: "i32".into(),
            },
            Expression::Literal(Literal::Float(n, k)) => {
                let llvm_ty = types::float_llvm(*k).into();
                let reg = llvm_float_const(*n, *k);
                ExprValue { reg, ty: llvm_ty }
            }
            Expression::Literal(Literal::Char(cp)) => ExprValue {
                reg: cp.to_string(),
                ty: "char".into(),
            },
            Expression::Literal(Literal::Bool(b)) => ExprValue {
                reg: if *b { "1" } else { "0" }.into(),
                ty: "i1".into(),
            },
            Expression::Literal(Literal::String(s)) => {
                let idx = self.intern_string(s);
                ExprValue {
                    reg: format!("@.str.{idx}"),
                    ty: "ptr".into(),
                }
            }
            Expression::Variable { name, .. } => self
                .module_consts
                .get(name)
                .cloned()
                .unwrap_or(ExprValue {
                    reg: "0".into(),
                    ty: "i32".into(),
                }),
            _ => self.compile_expr(expr, &HashMap::new()),
        }
    }

    /// Turn a global `@.str.N` or SSA `ptr` into a `ptr` suitable for calls/loads.
    pub(super) fn materialize_ptr_reg(&mut self, reg: &str) -> String {
        if reg.starts_with('@') {
            let name = reg.trim_start_matches('@');
            if self.functions.contains_key(name) {
                return reg.to_string();
            }
            let tmp = self.fresh("str");
            self.emit(&format!(
                "  %{tmp} = getelementptr inbounds i8, ptr {reg}, i64 0"
            ));
            format!("%{tmp}")
        } else {
            reg.to_string()
        }
    }

    pub(super) fn compile_string_method(
        &mut self,
        mc: &MethodCallExpr,
        env: &Env,
    ) -> ExprValue {
        let obj = self.compile_expr(&mc.object, env);
        let str_reg = llvm_ptr_reg(&obj.reg);
        let method = mc.method.as_str();
        match method {
            "split" => {
                let sep = self.compile_expr(&mc.args[0], env);
                let sep_reg = llvm_ptr_reg(&sep.reg);
                let reg = self.fresh("split");
                self.emit_runtime_call(
                    "str_split",
                    &format!(
                        "  %{reg} = call ptr @str_split(ptr {str_reg}, ptr {sep_reg})"
                    ),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "vec_str".into(),
                }
            }
            "trim" => {
                let reg = self.fresh("trim");
                self.emit_runtime_call(
                    "str_trim",
                    &format!("  %{reg} = call ptr @str_trim(ptr {str_reg})"),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            "to_upper" => {
                let reg = self.fresh("upper");
                self.emit_runtime_call(
                    "str_to_upper",
                    &format!("  %{reg} = call ptr @str_to_upper(ptr {str_reg})"),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            "to_lower" => {
                let reg = self.fresh("lower");
                self.emit_runtime_call(
                    "str_to_lower",
                    &format!("  %{reg} = call ptr @str_to_lower(ptr {str_reg})"),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            "contains" | "starts_with" | "ends_with" => {
                let arg = self.compile_expr(&mc.args[0], env);
                let arg_reg = llvm_ptr_reg(&arg.reg);
                let sym = match method {
                    "contains" => "str_contains",
                    "starts_with" => "str_starts_with",
                    _ => "str_ends_with",
                };
                let reg = self.fresh(method);
                self.emit_runtime_call(
                    sym,
                    &format!("  %{reg} = call i32 @{sym}(ptr {str_reg}, ptr {arg_reg})"),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "i32".into(),
                }
            }
            "replace" => {
                let from = self.compile_expr(&mc.args[0], env);
                let to = self.compile_expr(&mc.args[1], env);
                let from_reg = llvm_ptr_reg(&from.reg);
                let to_reg = llvm_ptr_reg(&to.reg);
                let reg = self.fresh("replace");
                self.emit_runtime_call(
                    "str_replace",
                    &format!(
                        "  %{reg} = call ptr @str_replace(ptr {str_reg}, ptr {from_reg}, ptr {to_reg})"
                    ),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            "replacen" => {
                let from = self.compile_expr(&mc.args[0], env);
                let to = self.compile_expr(&mc.args[1], env);
                let count = self.compile_expr(&mc.args[2], env);
                let from_reg = llvm_ptr_reg(&from.reg);
                let to_reg = llvm_ptr_reg(&to.reg);
                let count_reg = llvm_value_operand(&count.reg);
                let reg = self.fresh("replacen");
                self.emit_runtime_call(
                    "str_replacen",
                    &format!(
                        "  %{reg} = call ptr @str_replacen(ptr {str_reg}, ptr {from_reg}, ptr {to_reg}, i32 {count_reg})"
                    ),
                );
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            _ => ExprValue {
                reg: "0".into(),
                ty: "i32".into(),
            },
        }
    }

    /// Struct type name for the receiver of a method call (supports chaining and field access).
    pub(super) fn expr_receiver_struct_name(
        &self,
        expr: &Expression,
        env: &Env,
    ) -> Option<String> {
        match expr {
            Expression::Variable { name, .. } => env
                .get(name)
                .and_then(|b| struct_name_from_llvm_ty(Self::binding_ty(b))),
            Expression::Call(c) => self.call_returns.get(&c.callee).and_then(|ret| {
                if ret.starts_with('%') {
                    struct_name_from_llvm_ty(ret)
                } else {
                    None
                }
            }),
            Expression::MethodCall(mc) => self.expr_receiver_struct_name(&mc.object, env),
            Expression::FieldAccess(fa) => {
                let struct_name = self.expr_receiver_struct_name(&fa.object, env)?;
                let fields = self.struct_fields.get(&struct_name)?;
                let (_, field_ann) = fields.iter().find(|(n, _)| n == &fa.field)?;
                match field_ann {
                    TypeAnnotation::Struct(n) => Some(n.clone()),
                    _ => None,
                }
            }
            Expression::StructLiteral(sl) => Some(sl.name.clone()),
            Expression::Grouped(inner) => self.expr_receiver_struct_name(inner, env),
            _ => None,
        }
    }

    pub(super) fn method_callee_name(
        &self,
        object: &Expression,
        method: &str,
        env: &Env,
    ) -> String {
        if let Expression::Variable { name, .. } = object {
            if let Some(binding) = env.get(name) {
                let ty = Self::binding_ty(binding);
                if ty.starts_with("%Dyn_") {
                    let trait_name = ty
                        .trim_start_matches("%Dyn_")
                        .trim_end_matches('*');
                    return format!("__dyn_{trait_name}_{method}");
                }
            }
        }
        if let Some(struct_name) = self.expr_receiver_struct_name(object, env) {
            if method == "drop" {
                if let Some(callee) = self.drop_plan.custom_drop_fns.get(&struct_name) {
                    return callee.clone();
                }
            }
            if let Some(callee) = self
                .trait_method_callees
                .get(&(struct_name.clone(), method.to_string()))
            {
                return callee.clone();
            }
            return format!("{struct_name}_{method}");
        }
        method.to_string()
    }

    pub(super) fn expr_is_fn_pointer_operand(&self, expr: &Expression, env: &Env) -> bool {
        match expr {
            Expression::Variable { name, .. } => self.current_fn_ptrs.contains_key(name),
            Expression::Grouped(inner) => self.expr_is_fn_pointer_operand(inner, env),
            Expression::Cast(c) => self.expr_is_fn_pointer_operand(&c.expr, env),
            _ => false,
        }
    }

    pub(super) fn should_compare_ptr_as_string(
        &self,
        left: &Expression,
        right: &Expression,
        left_val: &ExprValue,
        right_val: &ExprValue,
        env: &Env,
    ) -> bool {
        left_val.ty == "ptr"
            && right_val.ty == "ptr"
            && !self.expr_is_fn_pointer_operand(left, env)
            && !self.expr_is_fn_pointer_operand(right, env)
    }

    pub(super) fn expr_is_string_operand(&self, expr: &Expression, env: &Env) -> bool {
        match expr {
            Expression::Literal(Literal::String(_)) | Expression::TemplateLiteral(_) => true,
            Expression::Binary(b) if b.op == BinaryOp::Add => {
                self.expr_is_string_operand(&b.left, env)
                    || self.expr_is_string_operand(&b.right, env)
            }
            Expression::Variable { name, .. } => env.get(name).is_some_and(|b| {
                Self::binding_ty(b) == "ptr" && !self.current_fn_ptrs.contains_key(name)
            }),
            Expression::Grouped(inner) => self.expr_is_string_operand(inner, env),
            Expression::Cast(c) => self.expr_is_string_operand(&c.expr, env),
            Expression::Call(c) => {
                if c.callee == "str_cat" || c.callee == "strcat" {
                    return true;
                }
                self.call_returns
                    .get(&c.callee)
                    .is_some_and(|ty| ty == "ptr" || ty == "string")
            }
            _ => false,
        }
    }

    pub(super) fn compile_string_ord(
        &mut self,
        left: &ExprValue,
        right: &ExprValue,
        op: BinaryOp,
    ) -> ExprValue {
        let lp = self.materialize_ptr_reg(&left.reg);
        let rp = self.materialize_ptr_reg(&right.reg);
        let cmp = self.fresh("strcmp");
        self.emit_runtime_call(
            "str_cmp",
            &format!("  %{cmp} = call i32 @str_cmp(ptr {lp}, ptr {rp})"),
        );
        let reg = self.fresh("strord");
        let pred = match op {
            BinaryOp::Lt => "slt",
            BinaryOp::Gt => "sgt",
            BinaryOp::Le => "sle",
            BinaryOp::Ge => "sge",
            _ => "eq",
        };
        self.emit(&format!("  %{reg} = icmp {pred} i32 %{cmp}, 0"));
        ExprValue {
            reg: format!("%{reg}"),
            ty: "i1".into(),
        }
    }

    pub(super) fn compile_string_eq(
        &mut self,
        left: &ExprValue,
        right: &ExprValue,
        want_eq: bool,
    ) -> ExprValue {
        let lp = self.materialize_ptr_reg(&left.reg);
        let rp = self.materialize_ptr_reg(&right.reg);
        let cmp = self.fresh("strcmp");
        self.emit_runtime_call(
            "str_cmp",
            &format!("  %{cmp} = call i32 @str_cmp(ptr {lp}, ptr {rp})"),
        );
        let reg = self.fresh("streq");
        let pred = if want_eq { "eq" } else { "ne" };
        self.emit(&format!("  %{reg} = icmp {pred} i32 %{cmp}, 0"));
        ExprValue {
            reg: format!("%{reg}"),
            ty: "i1".into(),
        }
    }
}

