#![allow(unused_imports)]
//! Match, if-expr, enum variants, and loop-carried SSA phis.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use ast::*;
use ownership::{
    arrow_has_captures, arrow_to_block, callee_returns_owned, collect_arrow_captures,
    collect_captures, DropPlan, EscapePlan, EscapeState,
};

use types::enum_pattern_matches;
use crate::runtime_map::RuntimeProfile;

use super::{
    Binding, ClosureMeta, Codegen, DropState, Env, EnvKind, ExprValue, FnPtrSig, LoopPhiContext,
    NestedFnCodegenScope, LOCAL_CHANNEL_CAP, LOCAL_CHANNEL_TYPE,
};
use super::util::{
    array_elem_from_ty, array_len_from_ty, assign_target_name, collect_assigned_in_block,
    escape_string, host_target_triple, is_string_builtin_method, llvm_arith_rhs, llvm_binop_operand,
    llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn loop_carried_ssa_in_body(&self, body: &Block, env: &Env) -> Vec<String> {
        let mut names: Vec<String> = collect_assigned_in_block(body)
            .into_iter()
            .filter(|n| self.mut_ssa_locals.contains(n) && env.contains_key(n))
            .collect();
        names.sort();
        names
    }

    pub(super) fn sync_loop_latch_regs(&mut self, env: &Env) {
        let Some(ctx) = self.loop_stack.last().cloned() else {
            return;
        };
        for name in &ctx.carried {
            let Some(latch) = ctx.latch_regs.get(name) else {
                continue;
            };
            let Some(binding) = env.get(name) else {
                continue;
            };
            let ty = Self::binding_ty(binding);
            let Binding::Reg { reg, .. } = binding else {
                continue;
            };
            if reg == latch {
                continue;
            }
            let src = Self::reg_operand_from_binding(binding);
            self.emit(&format!("  %{latch} = add {ty} 0, {src}"));
        }
    }

    /// Append a loop header phi incoming edge from the current block.
    pub(super) fn append_loop_phi_backedge(&mut self, env: &Env) {
        let Some(ctx) = self.loop_stack.last().cloned() else {
            return;
        };
        let backedge_label = self.current_block.clone();
        for name in &ctx.carried {
            let Some((phi_reg, ty)) = ctx.header_phi_regs.get(name) else {
                continue;
            };
            let Some(binding) = env.get(name) else {
                continue;
            };
            let src = Self::reg_operand_from_binding(binding);
            let incoming = self.fresh("loop.in");
            self.emit(&format!("  %{incoming} = add {ty} 0, {src}"));
            let edge = format!(", [%{incoming}, %{backedge_label}]");
            for line in self.ir_body_mut() {
                if line.contains(&format!("%{phi_reg} = phi")) {
                    *line = format!("{}{}", line.trim_end(), edge);
                }
            }
        }
    }

    /// Patch header phis to take loop-carried values from the block that backs to the header
    /// (after the body — often a merge block, not the `while.body` / `for.body` label).
    pub(super) fn finalize_loop_phi_backedges(&mut self, env: &Env) {
        let Some(ctx) = self.loop_stack.last().cloned() else {
            return;
        };
        let backedge_label = self.current_block.clone();
        for name in &ctx.carried {
            let Some((phi_reg, _ty)) = ctx.header_phi_regs.get(name) else {
                continue;
            };
            let Some(latch) = ctx.latch_regs.get(name) else {
                continue;
            };
            let Some(binding) = env.get(name) else {
                continue;
            };
            let src = Self::reg_operand_from_binding(binding);
            let needle = format!("[%{latch}, %{}]", ctx.latch_label);
            let replacement = format!("[{src}, %{backedge_label}]");
            for line in self.ir_body_mut() {
                if line.contains(&format!("%{phi_reg} = phi")) && line.contains(&needle) {
                    *line = line.replace(&needle, &replacement);
                }
            }
        }
    }

    pub(super) fn emit_loop_break(&mut self, env: &Env) {
        let Some(ctx) = self.loop_stack.last_mut() else {
            return;
        };
        let break_from = self.current_block.clone();
        let carried = ctx.carried.clone();
        let mut snap = HashMap::new();
        for name in &carried {
            if let Some(b) = env.get(name) {
                snap.insert(
                    name.clone(),
                    (
                        Self::reg_operand_from_binding(b),
                        Self::binding_ty(b).to_string(),
                    ),
                );
            }
        }
        let end_label = ctx.end_label.clone();
        ctx.break_edges.push((break_from, snap));
        self.emit(&format!("  br label %{end_label}"));
        let after = self.fresh_label("after.break");
        self.emit_block_label(&after);
        self.emit("  unreachable");
        self.current_block = after;
    }

    pub(super) fn emit_loop_continue(&mut self, env: &Env) {
        let cond_label = self
            .loop_stack
            .last()
            .map(|ctx| ctx.cond_label.clone());
        let Some(cond_label) = cond_label else {
            return;
        };
        self.sync_loop_latch_regs(env);
        self.append_loop_phi_backedge(env);
        self.emit(&format!("  br label %{cond_label}"));
        let after = self.fresh_label("after.continue");
        self.emit_block_label(&after);
        self.emit("  unreachable");
        self.current_block = after;
    }

    pub(super) fn restore_env_after_loop(
        &self,
        env: &mut Env,
        pre_loop: &Env,
        carried: &[String],
    ) {
        let carried_set: HashSet<String> = carried.iter().cloned().collect();
        let mut merged = pre_loop.clone();
        for name in &carried_set {
            if let Some(b) = env.get(name) {
                merged.insert(name.clone(), b.clone());
            }
        }
        *env = merged;
    }

    pub(super) fn emit_loop_exit_phis(&mut self, env: &mut Env, ctx: &LoopPhiContext) {
        for name in &ctx.carried {
            let Some((phi_reg, ty)) = ctx.header_phi_regs.get(name) else {
                continue;
            };
            let mut parts = vec![format!("[%{phi_reg}, %{}]", ctx.exit_pred)];
            for (break_lbl, snap) in &ctx.break_edges {
                if let Some((op, _)) = snap.get(name) {
                    parts.push(format!("[{op}, %{break_lbl}]"));
                }
            }
            let exit = self.fresh("loop.exit");
            self.emit(&format!(
                "  %{exit} = phi {ty} {}",
                parts.join(", ")
            ));
            env.insert(
                name.clone(),
                Binding::Reg {
                    reg: exit,
                    ty: ty.clone(),
                },
            );
        }
    }

    pub(super) fn snapshot_ssa_regs(&self, env: &Env, names: &[String]) -> HashMap<String, (String, String)> {
        names
            .iter()
            .filter_map(|n| {
                env.get(n).map(|b| {
                    (
                        n.clone(),
                        (
                            Self::reg_operand_from_binding(b),
                            Self::binding_ty(b).to_string(),
                        ),
                    )
                })
            })
            .collect()
    }

    pub(super) fn apply_ssa_snapshot(&mut self, env: &mut Env, snap: &HashMap<String, (String, String)>) {
        for (name, (op, ty)) in snap {
            let reg = if op.starts_with('%') {
                op.trim_start_matches('%').to_string()
            } else {
                op.clone()
            };
            env.insert(
                name.clone(),
                Binding::Reg {
                    reg,
                    ty: ty.clone(),
                },
            );
        }
    }

    pub(super) fn emit_if_merge_phis(
        &mut self,
        env: &mut Env,
        merge_vars: &[String],
        pre_if: &HashMap<String, (String, String)>,
        then_end: &HashMap<String, (String, String)>,
        else_end: &HashMap<String, (String, String)>,
        then_pred: &str,
        else_pred: &str,
    ) {
        for name in merge_vars {
            let ty = pre_if
                .get(name)
                .or_else(|| then_end.get(name))
                .or_else(|| else_end.get(name))
                .map(|(_, ty)| ty.clone())
                .unwrap_or_else(|| "i32".into());
            let default_op = if ty == "ptr" { "null" } else { "0" };
            let then_op = then_end
                .get(name)
                .map(|(op, _)| op.clone())
                .or_else(|| pre_if.get(name).map(|(op, _)| op.clone()))
                .unwrap_or_else(|| default_op.into());
            let else_op = else_end
                .get(name)
                .map(|(op, _)| op.clone())
                .or_else(|| pre_if.get(name).map(|(op, _)| op.clone()))
                .unwrap_or_else(|| default_op.into());
            let phi_reg = self.fresh("if.phi");
            self.emit(&format!(
                "  %{phi_reg} = phi {ty} [{then_op}, %{then_pred}], [{else_op}, %{else_pred}]"
            ));
            env.insert(
                name.clone(),
                Binding::Reg {
                    reg: phi_reg,
                    ty,
                },
            );
        }
    }

    pub(super) fn variant_tag(&self, enum_name: &str, variant: &str) -> i64 {
        self.enum_variants
            .get(enum_name)
            .and_then(|vars| vars.iter().position(|v| v == variant))
            .unwrap_or(0) as i64
    }

    pub(super) fn compile_enum_variant(
        &mut self,
        ev: &EnumVariantExpr,
        env: &Env,
    ) -> ExprValue {
        let en = match &ev.enum_name {
            Some(n) => n.clone(),
            None => {
                return ExprValue {
                    reg: "0".into(),
                    ty: "i32".into(),
                };
            }
        };
        let tag = self.variant_tag(&en, &ev.variant);
        if self.enum_has_payload.get(&en).copied().unwrap_or(false) {
            let alloca = self.fresh("enum");
            self.emit(&format!("  %{alloca} = alloca %{en}"));
            let tag_gep = self.fresh("gep");
            self.emit(&format!(
                "  %{tag_gep} = getelementptr inbounds %{en}, %{en}* %{alloca}, i32 0, i32 0"
            ));
            self.emit(&format!(
                "  store i32 {}, i32* %{tag_gep}",
                tag
            ));
            if !ev.args.is_empty() {
                let arg_expr = &ev.args[0];
                let mut payload = self.compile_expr(arg_expr, env);
                if matches!(arg_expr, Expression::Literal(Literal::String(_)))
                    && payload.ty == "ptr"
                {
                    payload = self.heap_clone_string(payload);
                }
                let payload_ty = self
                    .enum_payload_llvm
                    .get(&en)
                    .cloned()
                    .unwrap_or_else(|| payload.ty.clone());
                let variant_payload_ty = self
                    .enum_variant_payload_llvm
                    .get(&en)
                    .and_then(|m| m.get(&ev.variant))
                    .cloned()
                    .unwrap_or_else(|| payload.ty.clone());
                let pay_gep = self.fresh("gep");
                self.emit(&format!(
                    "  %{pay_gep} = getelementptr inbounds %{en}, %{en}* %{alloca}, i32 0, i32 1"
                ));
                let store_val = if payload.ty.ends_with('*') && variant_payload_ty.starts_with('%') {
                    let loaded = self.fresh("load");
                    self.emit(&format!(
                        "  %{loaded} = load {variant_payload_ty}, {variant_payload_ty}* {}",
                        self.reg_op(&payload)
                    ));
                    format!("%{loaded}")
                } else {
                    self.reg_op(&payload)
                };
                if variant_payload_ty != payload_ty {
                    let bc = self.fresh("bc");
                    self.emit(&format!("  %{bc} = bitcast ptr %{pay_gep} to ptr"));
                    self.emit(&format!(
                        "  store {variant_payload_ty} {store_val}, ptr %{bc}"
                    ));
                } else {
                    self.emit(&format!(
                        "  store {payload_ty} {store_val}, {} %{pay_gep}",
                        super::util::llvm_ptr(&payload_ty)
                    ));
                }
            }
            return ExprValue {
                reg: alloca,
                ty: format!("%{en}*"),
            };
        }
        ExprValue {
            reg: tag.to_string(),
            ty: "i32".into(),
        }
    }

    pub(super) fn compile_if_expr(
        &mut self,
        i: &IfExpr,
        env: &Env,
    ) -> ExprValue {
        let cond = self.compile_expr(&i.condition, env);
        let mut then_drop = DropState::default();
        let then_v = self.compile_block_as_expr(&i.then_block, env, &mut then_drop);
        let mut else_drop = DropState::default();
        let else_v = self.compile_block_as_expr(&i.else_block, env, &mut else_drop);
        let merge = self.fresh_label("if.expr");
        let then_l = self.fresh_label("if.then");
        let else_l = self.fresh_label("if.else");
        let result = self.fresh("alloca");
        self.emit(&format!("  %{result} = alloca {}", then_v.ty));
        self.emit(&format!(
            "  br i1 {}, label %{then_l}, label %{else_l}",
            cond.reg
        ));
        self.emit_block_label(&then_l);
        self.emit(&format!(
            "  store {} {}, {} %{}",
            then_v.ty, then_v.reg, llvm_ptr(&then_v.ty), result
        ));
        self.emit(&format!("  br label %{merge}"));
        self.emit_block_label(&else_l);
        self.emit(&format!(
            "  store {} {}, {} %{}",
            else_v.ty, else_v.reg, llvm_ptr(&else_v.ty), result
        ));
        self.emit(&format!("  br label %{merge}"));
        self.emit_block_label(&merge);
        let loaded = self.fresh("load");
        self.emit(&format!(
            "  %{loaded} = load {}, {} %{}",
            then_v.ty,
            llvm_ptr(&then_v.ty),
            result
        ));
        ExprValue {
            reg: format!("%{loaded}"),
            ty: then_v.ty,
        }
    }

    pub(super) fn compile_match(
        &mut self,
        m: &MatchExpr,
        env: &Env,
    ) -> ExprValue {
        let scrutinee = self.compile_expr(&m.scrutinee, env);
        let result_ty = m
            .arms
            .iter()
            .map(|a| self.infer_block_expr_llvm_ty(&a.body, env))
            .find(|ty| ty != "void")
            .unwrap_or_else(|| "i32".into());
        let result_ty = struct_value_type(&result_ty);
        let result_alloca = self.fresh("alloca");
        self.emit(&format!("  %{result_alloca} = alloca {result_ty}"));
        let end_l = self.fresh_label("match.end");
        let mut chain_label = self.fresh_label("match.chain");

        self.emit(&format!("  br label %{chain_label}"));
        for (i, arm) in m.arms.iter().enumerate() {
            let is_last = i + 1 == m.arms.len();
            self.emit(&format!("{chain_label}:"));
            let body_l = self.fresh_label("match.body");
            let next_l = self.fresh_label("match.next");
            let fail_l = if is_last {
                end_l.clone()
            } else {
                next_l.clone()
            };
            let enum_name = self.match_scrutinee_enum(&m.scrutinee, &scrutinee, env);
            let resolve_enum = |pattern: &str| -> String {
                enum_name
                    .as_ref()
                    .filter(|scrutinee| enum_pattern_matches(pattern, scrutinee))
                    .cloned()
                    .unwrap_or_else(|| pattern.to_string())
            };
            match &arm.pattern {
                MatchPattern::Wildcard => {
                    self.emit(&format!("  br label %{body_l}"));
                }
                MatchPattern::Literal(lit) => {
                    let idx = self.intern_string(lit);
                    let lit_reg = self.fresh("pat");
                    self.emit(&format!(
                        "  %{lit_reg} = getelementptr inbounds i8, ptr @.str.{idx}, i64 0"
                    ));
                    let lit_val = ExprValue {
                        reg: format!("%{lit_reg}"),
                        ty: "ptr".into(),
                    };
                    let eq = self.compile_string_eq(&scrutinee, &lit_val, true);
                    self.emit(&format!(
                        "  br i1 {}, label %{body_l}, label %{fail_l}",
                        eq.reg
                    ));
                }
                MatchPattern::Variant(v) => {
                    if enum_name.is_some() {
                        let expected = enum_name
                            .as_ref()
                            .map(|e| self.variant_tag(e, v))
                            .unwrap_or(0);
                        let tag_reg = self.load_enum_tag(&scrutinee, enum_name.as_ref().unwrap());
                        let cmp = self.fresh("cmp");
                        self.emit(&format!(
                            "  %{cmp} = icmp eq i32 {tag_reg}, {expected}"
                        ));
                        self.emit(&format!(
                            "  br i1 %{cmp}, label %{body_l}, label %{fail_l}"
                        ));
                    } else {
                        self.emit(&format!("  br label %{body_l}"));
                    }
                }
                MatchPattern::Qualified(en, v) => {
                    let llvm_en = resolve_enum(en);
                    let expected = self.variant_tag(&llvm_en, v);
                    let tag_reg = self.load_enum_tag(&scrutinee, &llvm_en);
                    let cmp = self.fresh("cmp");
                    self.emit(&format!(
                        "  %{cmp} = icmp eq i32 {tag_reg}, {expected}"
                    ));
                    self.emit(&format!(
                        "  br i1 %{cmp}, label %{body_l}, label %{fail_l}"
                    ));
                }
                MatchPattern::QualifiedBind(en, v, _bind) => {
                    let llvm_en = resolve_enum(en);
                    let expected = self.variant_tag(&llvm_en, v);
                    let tag_reg = self.load_enum_tag(&scrutinee, &llvm_en);
                    let cmp = self.fresh("cmp");
                    self.emit(&format!(
                        "  %{cmp} = icmp eq i32 {tag_reg}, {expected}"
                    ));
                    self.emit(&format!(
                        "  br i1 %{cmp}, label %{body_l}, label %{fail_l}"
                    ));
                }
                MatchPattern::Or(_) => {
                    panic!("match or-patterns must be desugared before codegen");
                }
                MatchPattern::Struct(_, _) | MatchPattern::Tuple(_) => {
                    self.emit(&format!("  br label %{body_l}"));
                }
            }
            self.emit(&format!("{body_l}:"));
            let mut arm_env = env.clone();
            let payload_ok_l = self.fresh_label("match.payload.ok");
            if let MatchPattern::QualifiedBind(en, v, payload) = &arm.pattern {
                let llvm_en = enum_name
                    .as_ref()
                    .filter(|scrutinee| enum_pattern_matches(en, scrutinee))
                    .cloned()
                    .unwrap_or_else(|| en.clone());
                self.compile_payload_pattern_guard(
                    &scrutinee,
                    &llvm_en,
                    v,
                    payload,
                    &payload_ok_l,
                    &fail_l,
                    &mut arm_env,
                );
            } else {
                self.emit(&format!("  br label %{payload_ok_l}"));
            }
            self.emit(&format!("{payload_ok_l}:"));
            if let MatchPattern::Struct(struct_name, fields) = &arm.pattern {
                self.compile_struct_pattern_bindings(&scrutinee, struct_name, fields, &mut arm_env);
            }
            if let MatchPattern::Tuple(binds) = &arm.pattern {
                self.compile_tuple_pattern_bindings(&scrutinee, binds, &mut arm_env);
            }
            if let MatchPattern::Variant(name) = &arm.pattern {
                if enum_name.is_none() {
                    arm_env.insert(
                        name.clone(),
                        Binding::Reg {
                            reg: scrutinee.reg.trim_start_matches('%').to_string(),
                            ty: scrutinee.ty.clone(),
                        },
                    );
                }
            }
            if let Some(guard) = &arm.guard {
                let g = self.compile_expr(guard, &arm_env);
                let guard_fail = if is_last {
                    end_l.clone()
                } else {
                    next_l.clone()
                };
                let guard_ok = self.fresh_label("match.guard.ok");
                self.emit(&format!(
                    "  br i1 {}, label %{guard_ok}, label %{guard_fail}",
                    g.reg
                ));
                self.emit(&format!("{guard_ok}:"));
            }
            let mut arm_drop = DropState::default();
            let val = self.compile_block_as_expr(&arm.body, &arm_env, &mut arm_drop);
            self.emit_struct_store(&val, &result_alloca, &result_ty);
            self.emit(&format!("  br label %{end_l}"));
            if !is_last {
                chain_label = next_l;
            }
        }
        self.emit(&format!("{end_l}:"));
        let loaded = self.fresh("load");
        self.emit(&format!(
            "  %{loaded} = load {result_ty}, {} %{}",
            llvm_ptr(&result_ty),
            result_alloca
        ));
        ExprValue {
            reg: format!("%{loaded}"),
            ty: result_ty,
        }
    }

    fn resolve_pattern_enum_name(&self, val: &ExprValue, pattern_enum: &str) -> String {
        if !pattern_enum.is_empty() {
            return pattern_enum.to_string();
        }
        struct_name_from_llvm_ty(&val.ty).unwrap_or_default()
    }

    fn load_variant_payload_value(
        &mut self,
        scrutinee: &ExprValue,
        enum_name: &str,
        variant: Option<&str>,
    ) -> ExprValue {
        if !self.enum_has_payload.get(enum_name).copied().unwrap_or(false) {
            return scrutinee.clone();
        }
        let slot_ty = self
            .enum_payload_llvm
            .get(enum_name)
            .cloned()
            .unwrap_or_else(|| "i32".into());
        let payload_ty = variant
            .and_then(|v| {
                self.enum_variant_payload_llvm
                    .get(enum_name)
                    .and_then(|m| m.get(v))
                    .cloned()
            })
            .unwrap_or_else(|| slot_ty.clone());
        let ptr = self.enum_scrutinee_ptr(scrutinee);
        let pay_gep = self.fresh("gep");
        self.emit(&format!(
            "  %{pay_gep} = getelementptr inbounds %{enum_name}, %{enum_name}* {ptr}, i32 0, i32 1"
        ));
        if payload_ty != slot_ty {
            let bc = self.fresh("bc");
            self.emit(&format!("  %{bc} = bitcast ptr %{pay_gep} to ptr"));
            let out = self.fresh("load");
            self.emit(&format!(
                "  %{out} = load {payload_ty}, ptr %{bc}"
            ));
            return ExprValue {
                reg: format!("%{out}"),
                ty: payload_ty,
            };
        }
        let loaded = self.fresh("load");
        self.emit(&format!(
            "  %{loaded} = load {slot_ty}, {} %{pay_gep}",
            super::util::llvm_ptr(&slot_ty)
        ));
        ExprValue {
            reg: format!("%{loaded}"),
            ty: payload_ty,
        }
    }

    fn emit_enum_tag_branch(
        &mut self,
        val: &ExprValue,
        enum_name: &str,
        variant: &str,
        ok_l: &str,
        fail_l: &str,
    ) {
        let expected = self.variant_tag(enum_name, variant);
        let tag_reg = self.load_enum_tag(val, enum_name);
        let cmp = self.fresh("cmp");
        self.emit(&format!("  %{cmp} = icmp eq i32 {tag_reg}, {expected}"));
        self.emit(&format!(
            "  br i1 %{cmp}, label %{ok_l}, label %{fail_l}"
        ));
    }

    fn compile_payload_pattern_guard(
        &mut self,
        scrutinee: &ExprValue,
        enum_name: &str,
        _variant: &str,
        payload: &MatchPayloadPattern,
        ok_l: &str,
        fail_l: &str,
        arm_env: &mut Env,
    ) {
        match payload {
            MatchPayloadPattern::Bind(name) => {
                let val = self.load_variant_payload_value(scrutinee, enum_name, Some(_variant));
                arm_env.insert(
                    name.clone(),
                    Binding::Reg {
                        reg: val.reg.trim_start_matches('%').to_string(),
                        ty: val.ty,
                    },
                );
                self.emit(&format!("  br label %{ok_l}"));
            }
            MatchPayloadPattern::Wildcard => {
                self.emit(&format!("  br label %{ok_l}"));
            }
            MatchPayloadPattern::Nested(pat) => {
                let val = self.load_variant_payload_value(scrutinee, enum_name, Some(_variant));
                self.compile_pattern_on_enum_value(pat, &val, ok_l, fail_l, arm_env);
            }
        }
    }

    fn compile_payload_bindings_on_enum(
        &mut self,
        val: &ExprValue,
        enum_name: &str,
        _variant: &str,
        payload: &MatchPayloadPattern,
        ok_l: &str,
        fail_l: &str,
        arm_env: &mut Env,
    ) {
        match payload {
            MatchPayloadPattern::Bind(name) => {
                let inner = self.load_variant_payload_value(val, enum_name, Some(_variant));
                arm_env.insert(
                    name.clone(),
                    Binding::Reg {
                        reg: inner.reg.trim_start_matches('%').to_string(),
                        ty: inner.ty,
                    },
                );
                self.emit(&format!("  br label %{ok_l}"));
            }
            MatchPayloadPattern::Wildcard => {
                self.emit(&format!("  br label %{ok_l}"));
            }
            MatchPayloadPattern::Nested(pat) => {
                let inner = self.load_variant_payload_value(val, enum_name, Some(_variant));
                self.compile_pattern_on_enum_value(pat, &inner, ok_l, fail_l, arm_env);
            }
        }
    }

    fn compile_pattern_on_enum_value(
        &mut self,
        pat: &MatchPattern,
        val: &ExprValue,
        ok_l: &str,
        fail_l: &str,
        arm_env: &mut Env,
    ) {
        match pat {
            MatchPattern::Qualified(en, v) => {
                let enum_name = self.resolve_pattern_enum_name(val, en);
                let tag_ok = self.fresh_label("match.nested.ok");
                self.emit_enum_tag_branch(val, &enum_name, v, &tag_ok, fail_l);
                self.emit(&format!("{tag_ok}:"));
                self.emit(&format!("  br label %{ok_l}"));
            }
            MatchPattern::QualifiedBind(en, v, inner_payload) => {
                let enum_name = self.resolve_pattern_enum_name(val, en);
                let tag_ok = self.fresh_label("match.nested.ok");
                self.emit_enum_tag_branch(val, &enum_name, v, &tag_ok, fail_l);
                self.emit(&format!("{tag_ok}:"));
                self.compile_payload_bindings_on_enum(
                    val,
                    &enum_name,
                    v,
                    inner_payload,
                    ok_l,
                    fail_l,
                    arm_env,
                );
            }
            _ => {
                self.emit(&format!("  br label %{fail_l}"));
            }
        }
    }

    fn compile_struct_pattern_bindings(
        &mut self,
        scrutinee: &ExprValue,
        struct_name: &str,
        fields: &[StructMatchField],
        arm_env: &mut Env,
    ) {
        let llvm_struct = format!("%{struct_name}");
        let base_ptr = if scrutinee.ty.ends_with('*') {
            self.reg_op(scrutinee)
        } else {
            let tmp = self.fresh("alloca");
            self.emit(&format!("  %{tmp} = alloca {llvm_struct}"));
            self.emit(&format!(
                "  store {llvm_struct} {}, {llvm_struct}* %{tmp}",
                self.reg_op(scrutinee)
            ));
            format!("%{tmp}")
        };
        for field_pat in fields {
            let bind = field_pat
                .bind
                .as_deref()
                .unwrap_or(field_pat.field.as_str());
            if bind == "_" {
                continue;
            }
            let idx = self
                .field_index(struct_name, &field_pat.field)
                .unwrap_or(0);
            let field_ty = self
                .struct_fields
                .get(struct_name)
                .and_then(|fs| fs.get(idx))
                .map(|(_, ty)| self.llvm_type_of(ty))
                .unwrap_or_else(|| "i32".into());
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {idx}"
            ));
            let loaded = self.fresh("load");
            self.emit(&format!(
                "  %{loaded} = load {field_ty}, {field_ty}* %{gep}"
            ));
            arm_env.insert(
                bind.to_string(),
                Binding::Reg {
                    reg: loaded,
                    ty: field_ty,
                },
            );
        }
    }

    fn compile_tuple_pattern_bindings(
        &mut self,
        scrutinee: &ExprValue,
        binds: &[MatchPayloadPattern],
        arm_env: &mut Env,
    ) {
        let struct_name = scrutinee
            .ty
            .trim_start_matches('%')
            .trim_end_matches('*')
            .to_string();
        let field_anns = self
            .tuple_fields
            .get(&struct_name)
            .cloned()
            .unwrap_or_default();
        let llvm_struct = format!("%{struct_name}");
        let base_ptr = if scrutinee.ty.ends_with('*') {
            self.reg_op(scrutinee)
        } else {
            let tmp = self.fresh("alloca");
            self.emit(&format!("  %{tmp} = alloca {llvm_struct}"));
            self.emit(&format!(
                "  store {llvm_struct} {}, {llvm_struct}* %{tmp}",
                self.reg_op(scrutinee)
            ));
            format!("%{tmp}")
        };
        for (idx, bind_pat) in binds.iter().enumerate() {
            let MatchPayloadPattern::Bind(name) = bind_pat else {
                continue;
            };
            let field_ty = field_anns
                .get(idx)
                .map(|ann| self.llvm_type_of(ann))
                .unwrap_or_else(|| "i32".into());
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {idx}"
            ));
            let loaded = self.fresh("load");
            self.emit(&format!(
                "  %{loaded} = load {field_ty}, {field_ty}* %{gep}"
            ));
            arm_env.insert(
                name.clone(),
                Binding::Reg {
                    reg: loaded,
                    ty: field_ty,
                },
            );
        }
    }

    fn match_scrutinee_enum(
        &self,
        expr: &Expression,
        val: &ExprValue,
        env: &Env,
    ) -> Option<String> {
        if let Expression::Variable { name, .. } = expr {
            if let Some(en) = self.enum_locals.get(name) {
                return Some(en.clone());
            }
            if let Some(binding) = env.get(name) {
                let ty = match binding {
                    Binding::Param { ty, .. } | Binding::Reg { ty, .. } | Binding::Stack { ty, .. } => {
                        ty.as_str()
                    }
                    _ => return None,
                };
                if let Some(n) = struct_name_from_llvm_ty(ty) {
                    if self.enum_names.contains(&n) {
                        return Some(n);
                    }
                }
            }
        }
        if let Some(n) = struct_name_from_llvm_ty(&val.ty) {
            if self.enum_names.contains(&n) {
                return Some(n);
            }
        }
        None
    }
}

