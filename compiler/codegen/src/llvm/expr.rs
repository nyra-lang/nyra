#![allow(unused_imports)]
//! Expression lowering (calls, operators, literals dispatch).
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
    escape_string, host_target_triple, is_array_ty, is_string_builtin_method, llvm_arith_rhs, llvm_binop_operand,
    llvm_cmp_operand, llvm_float_const, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    /// LLVM result type for `expr` without emitting instructions (for match result alloca).
    pub(super) fn infer_expr_llvm_ty(&self, expr: &Expression, env: &Env) -> String {
        match expr {
            Expression::Literal(Literal::Int(_)) => "i32".into(),
            Expression::Literal(Literal::IntKind(_, k)) => types::int_llvm(*k).into(),
            Expression::Literal(Literal::Float(_, k)) => types::float_llvm(*k).into(),
            Expression::Literal(Literal::Char(_)) => "i32".into(),
            Expression::Literal(Literal::Bool(_)) => "i1".into(),
            Expression::Literal(Literal::String(_)) => "ptr".into(),
            Expression::Call(c) => {
                let ret = self
                    .call_returns
                    .get(&c.callee)
                    .cloned()
                    .unwrap_or_else(|| "i32".into());
                if ret.starts_with('%') {
                    ret
                } else {
                    ret
                }
            }
            Expression::StructLiteral(sl) => format!("%{}", sl.name),
            Expression::EnumVariant(ev) => {
                if let Some(en) = &ev.enum_name {
                    if self.enum_has_payload.get(en).copied().unwrap_or(false) {
                        return struct_ptr_type(en);
                    }
                }
                "i32".into()
            }
            Expression::Variable { name, .. } => env
                .get(name)
                .map(|b| match b {
                    Binding::Reg { ty, .. }
                    | Binding::Param { ty, .. }
                    | Binding::Stack { ty, .. } => ty.clone(),
                    _ => "i32".into(),
                })
                .unwrap_or_else(|| "i32".into()),
            Expression::Grouped(inner) => self.infer_expr_llvm_ty(inner, env),
            Expression::If(i) => self.infer_block_expr_llvm_ty(&i.then_block, env),
            Expression::Match(m) => m
                .arms
                .first()
                .map(|a| self.infer_block_expr_llvm_ty(&a.body, env))
                .unwrap_or_else(|| "i32".into()),
            Expression::Binary(_) => "i32".into(),
            Expression::Unary(u) => match u.op {
                UnaryOp::Ref | UnaryOp::RefMut => {
                    let inner = self.infer_expr_llvm_ty(&u.operand, env);
                    llvm_ptr(&inner)
                }
                _ => self.infer_expr_llvm_ty(&u.operand, env),
            },
            _ => "i32".into(),
        }
    }

    pub(super) fn compile_expr(
        &mut self,
        expr: &Expression,
        env: &Env,
    ) -> ExprValue {
        match expr {
            Expression::Literal(Literal::Int(n)) => ExprValue {
                reg: n.to_string(),
                ty: "i32".into(),
            },
            Expression::Literal(Literal::IntKind(n, k)) => ExprValue {
                reg: n.to_string(),
                ty: types::int_llvm(*k).into(),
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
                let reg = self.fresh("str");
                self.emit(&format!(
                    "  %{reg} = getelementptr inbounds i8, ptr @.str.{idx}, i64 0"
                ));
                ExprValue {
                    reg: format!("%{reg}"),
                    ty: "ptr".into(),
                }
            }
            Expression::Variable { name, .. } => {
                if let Some(binding) = env.get(name) {
                    self.binding_to_expr(binding)
                } else if let Some(v) = self.module_consts.get(name) {
                    v.clone()
                } else if self.functions.contains_key(name) {
                    ExprValue {
                        reg: format!("@{name}"),
                        ty: "ptr".into(),
                    }
                } else {
                    ExprValue {
                        reg: "0".into(),
                        ty: "i32".into(),
                    }
                }
            }
            Expression::Binary(bin) => self.compile_binary(bin, env),
            Expression::Unary(u) => {
                let reg = self.fresh("unary");
                match u.op {
                    UnaryOp::Ref | UnaryOp::RefMut => {
                        if let Expression::Variable { name, .. } = &u.operand {
                            if let Some(binding) = env.get(name) {
                                let ty = Self::binding_ty(binding);
                                let ptr_ty = llvm_ptr(ty);
                                // `let mut` scalars are SSA registers; C out-params need a stack slot.
                                if self.mut_ssa_locals.contains(name) {
                                    if let Binding::Reg { reg, .. } = binding {
                                        let llvm_ty = llvm_storage_ty(ty);
                                        let slot = self.fresh("refmut");
                                        self.emit(&format!(
                                            "  %{slot} = alloca {llvm_ty}, align 8"
                                        ));
                                        let reg_ref = if reg.starts_with('%') {
                                            reg.clone()
                                        } else if reg.chars().all(|c| {
                                            c.is_ascii_digit() || c == '-' || c == '.'
                                        }) {
                                            reg.clone()
                                        } else {
                                            format!("%{reg}")
                                        };
                                        self.emit(&format!(
                                            "  store {llvm_ty} {reg_ref}, {} %{slot}",
                                            llvm_ptr(llvm_ty)
                                        ));
                                        return ExprValue {
                                            reg: format!("%{slot}"),
                                            ty: ptr_ty,
                                        };
                                    }
                                }
                                let ptr_reg = match binding {
                                    Binding::Stack { slot, ty } if ty == "ptr" && u.op == UnaryOp::Ref => {
                                        let loaded = self.fresh("ref");
                                        self.emit(&format!(
                                            "  %{loaded} = load ptr, ptr %{slot}"
                                        ));
                                        format!("%{loaded}")
                                    }
                                    Binding::Stack { slot, .. } => format!("%{slot}"),
                                    Binding::Reg { reg, .. } => {
                                        if reg.starts_with('%') {
                                            reg.clone()
                                        } else {
                                            format!("%{reg}")
                                        }
                                    }
                                    Binding::Param { index, .. } => format!("%{index}"),
                                    Binding::Closure(_) => "0".to_string(),
                                    Binding::PromotedStruct {
                                        struct_name,
                                        fields,
                                        ..
                                    } => {
                                        let mat =
                                            self.materialize_promoted_struct(struct_name, fields);
                                        format!("%{}", mat.reg.trim_start_matches('%'))
                                    }
                                    Binding::LocalChannel { slot } => format!("%{slot}"),
                                };
                                return ExprValue {
                                    reg: ptr_reg,
                                    ty: ptr_ty,
                                };
                            }
                        }
                        let inner = self.compile_expr(&u.operand, env);
                        ExprValue {
                            reg: inner.reg,
                            ty: llvm_ptr(&inner.ty),
                        }
                    }
                    UnaryOp::Deref => {
                        let inner = self.compile_expr(&u.operand, env);
                        let loaded = self.fresh("load");
                        let (elem_ty, ptr_ty) = if inner.ty == "ptr" {
                            ("i32".to_string(), "ptr".to_string())
                        } else {
                            (
                                inner.ty.trim_start_matches('%').to_string(),
                                llvm_ptr(&inner.ty),
                            )
                        };
                        let ptr_op = if inner.ty == "ptr" {
                            let p = self.materialize_ptr_reg(&inner.reg);
                            if p.starts_with('%') {
                                format!("ptr {p}")
                            } else {
                                format!("ptr %{p}")
                            }
                        } else if inner.reg.starts_with('%') {
                            format!("{ptr_ty} {}", inner.reg)
                        } else {
                            format!("{ptr_ty} {}", inner.reg)
                        };
                        self.emit(&format!(
                            "  %{loaded} = load {elem_ty}, {ptr_op}"
                        ));
                        ExprValue {
                            reg: format!("%{loaded}"),
                            ty: elem_ty,
                        }
                    }
                    UnaryOp::Neg => {
                        let inner = self.compile_expr(&u.operand, env);
                        self.emit(&format!(
                            "  %{reg} = sub {} 0, {}",
                            inner.ty, inner.reg
                        ));
                        ExprValue {
                            reg: format!("%{reg}"),
                            ty: inner.ty,
                        }
                    }
                    UnaryOp::Not => {
                        let inner = self.compile_expr(&u.operand, env);
                        self.emit(&format!("  %{reg} = xor i1 {}, true", inner.reg));
                        ExprValue {
                            reg: format!("%{reg}"),
                            ty: "i1".into(),
                        }
                    }
                    UnaryOp::Move | UnaryOp::Clone | UnaryOp::Try => self.compile_expr(&u.operand, env),
                }
            }
            Expression::Call(call) => {
                if call.callee == "flush" {
                    self.emit_runtime_call("stdout_flush", "  call void @stdout_flush()");
                    return ExprValue {
                        reg: "0".into(),
                        ty: "void".into(),
                    };
                }
                if call.callee == "input" {
                    let prompt = if call.args.is_empty() {
                        "null".to_string()
                    } else {
                        let v = self.compile_expr(&call.args[0], env);
                        self.materialize_ptr_reg(&v.reg)
                    };
                    let reg = self.fresh("input");
                    self.emit_runtime_call(
                        "stdin_read_line",
                        &format!("  %{reg} = call ptr @stdin_read_line(ptr {prompt})"),
                    );
                    return ExprValue {
                        reg: format!("%{reg}"),
                        ty: "ptr".into(),
                    };
                }
                if call.callee == "date" {
                    self.inject_builtin_date_struct();
                    let alloca = self.fresh("alloca");
                    self.emit(&format!("  %{alloca} = alloca %Date"));
                    self.emit_runtime_call(
                        "date_now",
                        &format!("  call void @date_now(ptr %{alloca})"),
                    );
                    return ExprValue {
                        reg: alloca,
                        ty: struct_ptr_type("%Date"),
                    };
                }
                if call.callee == "write" {
                    self.compile_buffered_io_args(&call.args, env, false);
                    return ExprValue {
                        reg: "0".into(),
                        ty: "void".into(),
                    };
                }
                if call.callee == "println" {
                    self.compile_buffered_io_args(&call.args, env, true);
                    return ExprValue {
                        reg: "0".into(),
                        ty: "void".into(),
                    };
                }
                if let Some(v) = self.compile_math_intrinsic_call(call, env) {
                    return v;
                }
                if let Some(Binding::Closure(meta)) = env.get(&call.callee).cloned() {
                    return self.compile_closure_call(&meta, &call.args, env);
                }
                if let Some(mut sig) = self.current_fn_ptrs.get(&call.callee).cloned() {
                    sig.reg = self.fn_ptr_call_target(&call.callee, env);
                    if let (Some(slot), Some(env_alloca)) =
                        (sig.invoke_slot.clone(), sig.env_alloca.clone())
                    {
                        self.emit(&format!(
                            "  store ptr %{env_alloca}, ptr @{slot}"
                        ));
                    }
                    let mut arg_regs = Vec::new();
                    let mut arg_tys = Vec::new();
                    for a in &call.args {
                        let v = self.compile_expr(a, env);
                        if is_struct_pointer_type(&v.ty) {
                            arg_regs.push(format!("%{}", v.reg.trim_start_matches('%')));
                            arg_tys.push(v.ty.clone());
                        } else if v.ty.starts_with('%') {
                            arg_regs.push(format!("%{}", v.reg.trim_start_matches('%')));
                            arg_tys.push(struct_ptr_type(&v.ty));
                        } else if v.ty == "ptr" {
                            arg_regs.push(self.materialize_ptr_reg(&v.reg));
                            arg_tys.push("ptr".into());
                        } else {
                            arg_regs.push(v.reg.clone());
                            arg_tys.push(v.ty.clone());
                        }
                    }
                    let args = arg_regs
                        .iter()
                        .zip(arg_tys.iter())
                        .map(|(r, t)| format!("{t} {r}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ret_ty = sig.ret_ty.clone();
                    if ret_ty == "void" {
                        self.emit(&format!("  call void {}({args})", sig.reg));
                        if let Some(slot) = sig.invoke_slot {
                            self.emit(&format!("  store ptr null, ptr @{slot}"));
                        }
                        return ExprValue { reg: "0".into(), ty: "void".into() };
                    }
                    let reg = self.fresh("call");
                    self.emit(&format!(
                        "  %{reg} = call {ret_ty} {}({args})",
                        sig.reg
                    ));
                    if let Some(slot) = sig.invoke_slot {
                        self.emit(&format!("  store ptr null, ptr @{slot}"));
                    }
                    if ret_ty.starts_with('%') {
                        return self.materialize_struct_call_ret(&ret_ty, &ret_ty, &format!("%{reg}"));
                    }
                    return ExprValue { reg: format!("%{reg}"), ty: ret_ty };
                }
                let mut arg_regs = Vec::new();
                let mut arg_tys = Vec::new();
                let mut closure_slots_to_clear = Vec::new();
                let param_tys = self.fn_param_llvm_types(&call.callee);
                for (arg_idx, a) in call.args.iter().enumerate() {
                    if let Expression::Variable { name, .. } = a {
                        if let Some(Binding::Closure(meta)) = env.get(name) {
                            self.store_closure_invoke_slot(meta);
                            if matches!(meta.env_kind, EnvKind::Stack { .. }) {
                                closure_slots_to_clear.push(meta.invoke_slot.clone());
                            }
                            arg_regs.push(format!("@{}", meta.wrap_symbol));
                            arg_tys.push("ptr".into());
                            continue;
                        }
                        if let Some(func) = self.functions.get(&call.callee) {
                            if let Some(param) = func.params.get(arg_idx) {
                                if param.mutable {
                                    if let TypeAnnotation::Struct(_) = &param.ty {
                                        if let Some(Binding::Stack { slot, ty }) = env.get(name) {
                                            let ptr_ty = if ty.ends_with('*') {
                                                ty.clone()
                                            } else {
                                                struct_ptr_type(ty)
                                            };
                                            arg_regs.push(format!("%{slot}"));
                                            arg_tys.push(ptr_ty);
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if matches!(a, Expression::ArrowFn(_)) {
                        self.closure_force_heap = false;
                    }
                    let v = self.compile_expr(a, env);
                    let v = if let Some(types) = &param_tys {
                        if let Some(expected) = types.get(arg_idx) {
                            self.coerce_expr_to_llvm_type(v, expected)
                        } else {
                            v
                        }
                    } else {
                        v
                    };
                    self.closure_force_heap = false;
                    if let Some(meta) = self.pending_closure_meta.take() {
                        self.store_closure_invoke_slot(&meta);
                        if matches!(meta.env_kind, EnvKind::Stack { .. }) {
                            closure_slots_to_clear.push(meta.invoke_slot.clone());
                        }
                        arg_regs.push(format!("@{}", meta.wrap_symbol));
                        arg_tys.push("ptr".into());
                    } else if is_struct_pointer_type(&v.ty) {
                        let is_extern_c = self.is_extern_c_call(&call.callee);
                        self.push_call_arg(&v, is_extern_c, &mut arg_regs, &mut arg_tys);
                    } else if v.ty.starts_with('%') {
                        let is_extern_c = self.is_extern_c_call(&call.callee);
                        self.push_call_arg(&v, is_extern_c, &mut arg_regs, &mut arg_tys);
                    } else if is_array_ty(&v.ty) {
                        let (reg, ty) = self.materialize_array_call_arg(&v);
                        arg_regs.push(reg);
                        arg_tys.push(ty);
                    } else if v.ty == "ptr" {
                        arg_regs.push(self.materialize_ptr_reg(&v.reg));
                        arg_tys.push("ptr".into());
                    } else {
                        arg_regs.push(v.reg.clone());
                        arg_tys.push(v.ty.clone());
                    }
                }
                let llvm_callee = if self.functions.contains_key(&call.callee) {
                    call.callee.clone()
                } else {
                    self.runtime_callee(&call.callee)
                };
                let ret_ty = self
                    .call_returns
                    .get(&call.callee)
                    .cloned()
                    .unwrap_or_else(|| "i32".to_string());
                let sret_struct = if self.is_extern_c_call(&call.callee) {
                    self.extern_call_needs_sret(&ret_ty)
                } else {
                    None
                };
                let mut sret_alloca = None;
                if let Some(struct_name) = sret_struct {
                    let alloca = self.fresh("sret");
                    let logical_ty = format!("%{struct_name}");
                    self.emit(&format!("  %{alloca} = alloca {logical_ty}"));
                    arg_regs.insert(0, format!("%{alloca}"));
                    arg_tys.insert(
                        0,
                        format!("%{struct_name}* sret(%{struct_name})"),
                    );
                    sret_alloca = Some((alloca, logical_ty));
                }
                let args = arg_regs
                    .iter()
                    .zip(arg_tys.iter())
                    .map(|(r, t)| format!("{t} {r}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if (call.callee == "channel_send" || call.callee == "channel_send")
                    && call.args.len() >= 2
                {
                    if let Some(slot) = self.resolve_local_channel_slot(&call.args[0], env) {
                        let val = self.compile_expr(&call.args[1], env);
                        self.emit_local_channel_send(&slot, &val);
                        for slot in &closure_slots_to_clear {
                            self.emit(&format!("  store ptr null, ptr @{slot}"));
                        }
                        return ExprValue {
                            reg: "0".into(),
                            ty: "void".into(),
                        };
                    }
                }
                if (call.callee == "channel_recv" || call.callee == "channel_recv")
                    && !call.args.is_empty()
                {
                    if let Some(slot) = self.resolve_local_channel_slot(&call.args[0], env) {
                        let reg = self.emit_local_channel_recv(&slot);
                        for slot in &closure_slots_to_clear {
                            self.emit(&format!("  store ptr null, ptr @{slot}"));
                        }
                        return ExprValue {
                            reg: format!("%{reg}"),
                            ty: "i32".into(),
                        };
                    }
                }
                if call.callee == "channel_free" && !call.args.is_empty() {
                    if self
                        .resolve_local_channel_slot(&call.args[0], env)
                        .is_some()
                    {
                        for slot in &closure_slots_to_clear {
                            self.emit(&format!("  store ptr null, ptr @{slot}"));
                        }
                        return ExprValue {
                            reg: "0".into(),
                            ty: "void".into(),
                        };
                    }
                }
                if ret_ty == "void" || sret_alloca.is_some() {
                    if self.is_runtime_symbol(&llvm_callee) {
                        self.emit_runtime_call(
                            &llvm_callee,
                            &format!("  call void @{llvm_callee}({args})"),
                        );
                    } else {
                        self.emit(&format!("  call void @{llvm_callee}({args})"));
                    }
                    for slot in &closure_slots_to_clear {
                        self.emit(&format!("  store ptr null, ptr @{slot}"));
                    }
                    if let Some((alloca, logical_ty)) = sret_alloca {
                        return ExprValue {
                            reg: alloca,
                            ty: struct_ptr_type(&logical_ty),
                        };
                    }
                    return ExprValue {
                        reg: "0".into(),
                        ty: "void".into(),
                    };
                }
                let reg = self.fresh("call");
                let llvm_ret_ty = self.llvm_extern_call_ret_ty(&call.callee, &ret_ty);
                if self.is_runtime_symbol(&llvm_callee) {
                    self.emit_runtime_call(
                        &llvm_callee,
                        &format!("  %{reg} = call {llvm_ret_ty} @{llvm_callee}({args})"),
                    );
                } else {
                    self.emit(&format!(
                        "  %{reg} = call {llvm_ret_ty} @{llvm_callee}({args})"
                    ));
                }
                if ret_ty.starts_with('%') {
                    let val = if llvm_ret_ty != ret_ty {
                        let alloca = self.fresh("alloca");
                        self.emit(&format!("  %{alloca} = alloca {ret_ty}"));
                        self.store_coerced_extern_struct_ret(
                            &ret_ty,
                            &llvm_ret_ty,
                            &format!("%{reg}"),
                            &alloca,
                        );
                        ExprValue {
                            reg: alloca,
                            ty: struct_ptr_type(&ret_ty),
                        }
                    } else {
                        self.materialize_struct_call_ret(&ret_ty, &llvm_ret_ty, &format!("%{reg}"))
                    };
                    for slot in &closure_slots_to_clear {
                        self.emit(&format!("  store ptr null, ptr @{slot}"));
                    }
                    val
                } else if is_array_ty(&ret_ty) {
                    let slot = self.fresh("arr.ret");
                    self.emit(&format!("  %{slot} = alloca {ret_ty}"));
                    self.emit(&format!(
                        "  store {ret_ty} %{reg}, {ret_ty}* %{slot}"
                    ));
                    for slot_name in &closure_slots_to_clear {
                        self.emit(&format!("  store ptr null, ptr @{slot_name}"));
                    }
                    ExprValue {
                        reg: slot,
                        ty: ret_ty,
                    }
                } else {
                    for slot in &closure_slots_to_clear {
                        self.emit(&format!("  store ptr null, ptr @{slot}"));
                    }
                    ExprValue {
                        reg: format!("%{reg}"),
                        ty: ret_ty,
                    }
                }
            }
            Expression::FieldAccess(fa) => self.compile_field_access(fa, env),
            Expression::StructLiteral(sl) => self.compile_struct_literal(sl, env, false),
            Expression::Match(m) => self.compile_match(m, env),
            Expression::If(i) => self.compile_if_expr(i, env),
            Expression::Index(ix) => self.compile_index(ix, env),
            Expression::ArrayLiteral(al) => self.compile_array_literal(al, env),
            Expression::ArrayRepeat { element, count, .. } => {
                self.compile_array_repeat(element, *count, env)
            }
            Expression::TupleLiteral(elems) => self.compile_tuple_literal(elems, env),
            Expression::EnumVariant(ev) => self.compile_enum_variant(ev, env),
            Expression::MethodCall(mc) if mc.method == "send" && mc.args.len() == 1 => {
                if let Some(slot) = self.resolve_local_channel_slot(&mc.object, env) {
                    let val = self.compile_expr(&mc.args[0], env);
                    self.emit_local_channel_send(&slot, &val);
                    return self.compile_expr(&mc.object, env);
                }
                let callee = self.method_callee_name(&mc.object, &mc.method, env);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                self.compile_expr(
                    &Expression::Call(CallExpr {
                        callee,
                        type_args: vec![],
                        args,
                        span: mc.span.clone(),
                    }),
                    env,
                )
            }
            Expression::MethodCall(mc) if mc.method == "recv" && mc.args.is_empty() => {
                if let Some(slot) = self.resolve_local_channel_slot(&mc.object, env) {
                    let reg = self.emit_local_channel_recv(&slot);
                    return ExprValue {
                        reg: format!("%{reg}"),
                        ty: "i32".into(),
                    };
                }
                let callee = self.method_callee_name(&mc.object, &mc.method, env);
                self.compile_expr(
                    &Expression::Call(CallExpr {
                        callee,
                        type_args: vec![],
                        args: vec![mc.object.clone()],
                        span: mc.span.clone(),
                    }),
                    env,
                )
            }
            Expression::MethodCall(mc)
                if is_string_builtin_method(&mc.method)
                    && self.expr_receiver_struct_name(&mc.object, env).is_none() =>
            {
                self.compile_string_method(mc, env)
            }
            Expression::MethodCall(mc) if mc.method == "length" || mc.method == "len" => {
                let obj = self.compile_expr(&mc.object, env);
                if let Some(n) = array_len_from_ty(&obj.ty) {
                    return ExprValue {
                        reg: n.to_string(),
                        ty: "i32".into(),
                    };
                }
                if obj.ty == "ptr" {
                    let reg = self.fresh("strlen");
                    let str_reg = if obj.reg.starts_with('%') {
                        obj.reg.clone()
                    } else {
                        format!("%{}", obj.reg)
                    };
                    self.emit_runtime_call(
                        "strlen",
                        &format!("  %{reg} = call i32 @strlen(ptr {str_reg})"),
                    );
                    return ExprValue {
                        reg: format!("%{reg}"),
                        ty: "i32".into(),
                    };
                }
                if obj.ty == "vec_str" {
                    let reg = self.fresh("vec_strlen");
                    let vec_reg = llvm_ptr_reg(&obj.reg);
                    self.emit_runtime_call(
                        "vec_str_len",
                        &format!("  %{reg} = call i32 @vec_str_len(ptr {vec_reg})"),
                    );
                    return ExprValue {
                        reg: format!("%{reg}"),
                        ty: "i32".into(),
                    };
                }
                let callee = self.method_callee_name(&mc.object, &mc.method, env);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                self.compile_expr(
                    &Expression::Call(CallExpr {
                        callee,
                        type_args: vec![],
                        args,
                        span: mc.span.clone(),
                    }),
                    env,
                )
            }
            Expression::MethodCall(mc) if mc.method == "clone" => {
                let obj = self.compile_expr(&mc.object, env);
                if obj.ty == "ptr" || obj.ty.contains("i8") {
                    let reg = self.fresh("str_clone");
                    self.emit_runtime_call(
                        "str_clone",
                        &format!("  %{reg} = call ptr @str_clone(ptr {})", llvm_ptr_reg(&obj.reg)),
                    );
                    return ExprValue {
                        reg: format!("%{reg}"),
                        ty: "ptr".into(),
                    };
                }
                let callee = self.method_callee_name(&mc.object, &mc.method, env);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                self.compile_expr(
                    &Expression::Call(CallExpr {
                        callee,
                        type_args: vec![],
                        args,
                        span: mc.span.clone(),
                    }),
                    env,
                )
            }
            Expression::MethodCall(mc) if mc.method == "sort" => {
                self.compile_array_sort(mc, env)
            }
            Expression::MethodCall(mc) if mc.method == "sort_by" => {
                self.compile_array_sort_by(mc, env)
            }
            Expression::MethodCall(mc) => {
                let callee = self.method_callee_name(&mc.object, &mc.method, env);
                let mut args = vec![mc.object.clone()];
                args.extend(mc.args.clone());
                self.compile_expr(
                    &Expression::Call(CallExpr {
                        callee,
                        type_args: vec![],
                        args,
                        span: mc.span.clone(),
                    }),
                    env,
                )
            }
            Expression::Grouped(inner) => self.compile_expr(inner, env),
            Expression::Await(inner) => {
                let inner_v = self.compile_expr(inner, env);
                let future_name = struct_name_from_llvm_ty(&inner_v.ty)
                    .filter(|n| n.starts_with("Future_"));
                let handle_reg = if future_name.is_some() {
                    let h = self.fresh("future_h");
                    self.emit(&format!(
                        "  %{h} = extractvalue {} {}, 0",
                        inner_v.ty, inner_v.reg
                    ));
                    format!("%{h}")
                } else {
                    inner_v.reg.clone()
                };
                match future_name.as_deref() {
                    Some("Future_bool") => {
                        let raw = self.fresh("await_raw");
                        self.emit_runtime_call(
                            "async_await_bool",
                            &format!(
                                "  %{raw} = call i32 @async_await_bool(i32 {handle_reg})"
                            ),
                        );
                        let reg = self.fresh("await");
                        self.emit(&format!("  %{reg} = icmp ne i32 %{raw}, 0"));
                        ExprValue {
                            reg: format!("%{reg}"),
                            ty: "i1".into(),
                        }
                    }
                    Some("Future_string") => {
                        let reg = self.fresh("await");
                        self.emit_runtime_call(
                            "async_await_ptr",
                            &format!(
                                "  %{reg} = call ptr @async_await_ptr(i32 {handle_reg})"
                            ),
                        );
                        ExprValue {
                            reg: format!("%{reg}"),
                            ty: "ptr".into(),
                        }
                    }
                    _ => {
                        let reg = self.fresh("await");
                        self.emit_runtime_call(
                            "async_await",
                            &format!("  %{reg} = call i32 @async_await(i32 {handle_reg})"),
                        );
                        ExprValue {
                            reg: format!("%{reg}"),
                            ty: "i32".into(),
                        }
                    }
                }
            }
            Expression::TemplateLiteral(t) => self.compile_template_literal(t, env),
            Expression::Cast(c) => self.compile_cast(c, env),
            Expression::ArrowFn(arrow) => {
                if let Some(ptr) = self.compiling_drop {
                    let arrow = arrow.clone();
                    let drop_state = unsafe { &mut *ptr };
                    let force_heap = self.closure_force_heap;
                    return self.compile_arrow_fn(&arrow, env, drop_state, force_heap);
                }
                ExprValue {
                    reg: "0".into(),
                    ty: "ptr".into(),
                }
            }
            Expression::Invalid => ExprValue {
                reg: "0".into(),
                ty: "i32".into(),
            },
            Expression::ComptimeBlock { .. } => ExprValue {
                reg: "0".into(),
                ty: "i32".into(),
            },
        }
    }
}

