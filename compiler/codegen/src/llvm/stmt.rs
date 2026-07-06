#![allow(unused_imports)]
//! Statement lowering (`let`, control flow, loops, etc.).
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
    llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn compile_statement(
        &mut self,
        stmt: &Statement,
        env: &mut Env,
        ret_ty: &str,
        drop_state: &mut DropState,
        scope_owned: &mut Vec<String>,
        heap_closures: &mut Vec<String>,
        defers: &[Expression],
    ) -> bool {
        match stmt {
            Statement::Let(l) => {
                if !l.destructure.is_empty() {
                    let val = self.compile_expr(&l.value, env);
                    let tuple_name = val
                        .ty
                        .trim_start_matches('%')
                        .trim_end_matches('*')
                        .to_string();
                    let llvm_struct = format!("%{tuple_name}");
                    let base_ptr = if val.reg.starts_with('%') {
                        val.reg.clone()
                    } else {
                        format!("%{}", val.reg)
                    };
                    for (idx, name) in l.destructure.iter().enumerate() {
                        let field_idx = idx;
                        let field_ty = self
                            .tuple_fields
                            .get(&tuple_name)
                            .and_then(|fs| fs.get(field_idx))
                            .map(|a| self.llvm_type_of(a))
                            .unwrap_or_else(|| "i32".into());
                        let gep = self.fresh("gep");
                        self.emit(&format!(
                            "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {field_idx}"
                        ));
                        let loaded = self.fresh("load");
                        self.emit(&format!(
                            "  %{loaded} = load {field_ty}, {} %{gep}",
                            llvm_ptr(&field_ty)
                        ));
                        env.insert(
                            name.clone(),
                            Binding::Reg {
                                reg: loaded,
                                ty: field_ty,
                            },
                        );
                    }
                    return false;
                }
                if l.destructure.is_empty() {
                    if let Expression::StructLiteral(sl) = &l.value {
                        if sl.spreads.is_empty()
                            && self.binding_no_escape(&l.name)
                            && self.struct_literal_sroa_eligible(sl)
                        {
                            let binding = self.compile_struct_sroa(sl, env);
                            env.insert(l.name.clone(), binding);
                            return false;
                        }
                    }
                    if self.escape_plan.is_local_channel(&drop_state.func, &l.name)
                        && Self::is_channel_origin_expr(&l.value)
                    {
                        let slot = self.emit_local_channel_alloc();
                        env.insert(
                            l.name.clone(),
                            Binding::LocalChannel { slot },
                        );
                        return false;
                    }
                }
                if let Expression::EnumVariant(ev) = &l.value {
                    if let Some(en) = &ev.enum_name {
                        self.enum_locals.insert(l.name.clone(), en.clone());
                    }
                }
                self.register_call_moves(&l.value, env, drop_state);
                let mut val = if let Expression::StructLiteral(sl) = &l.value {
                    self.compile_struct_literal(sl, env, self.binding_no_escape(&l.name))
                } else {
                    self.compile_expr(&l.value, env)
                };
                if matches!(l.ty.as_ref(), Some(TypeAnnotation::F32)) {
                    self.promote_to_float(&mut val);
                } else if matches!(l.ty.as_ref(), Some(TypeAnnotation::F64)) {
                    self.promote_to_double(&mut val);
                }
                if let Expression::ArrowFn(arrow) = &l.value {
                    self.mark_arrow_capture_moves(arrow, env, drop_state);
                    if arrow_has_captures(arrow) {
                        if let Some(meta) = self.pending_closure_meta.take() {
                            env.insert(l.name.clone(), Binding::Closure(meta.clone()));
                            self.register_closure_local(&l.name, &meta);
                            if meta.heap_owned {
                                heap_closures.push(l.name.clone());
                            }
                            if self.drop_plan.is_owned_in(&drop_state.func, &l.name)
                                || self.drop_plan.needs_struct_drop_in(&drop_state.func, &l.name)
                    || self.drop_plan.is_enum_payload_in(&drop_state.func, &l.name)
                            {
                                scope_owned.push(l.name.clone());
                            }
                            return false;
                        }
                    }
                }
                let needs_drop_stack = self.drop_plan.is_owned_in(&drop_state.func, &l.name)
                    || self.drop_plan.needs_struct_drop_in(&drop_state.func, &l.name)
                    || self.drop_plan.is_enum_payload_in(&drop_state.func, &l.name)
                    || self.drop_plan.is_join_handle_in(&drop_state.func, &l.name);
                let storage_ty = if val.ty.starts_with('%') {
                    val.ty.clone()
                } else {
                    llvm_storage_ty(&val.ty).to_string()
                };
                let scalar_mut_ssa = l.mutable
                    && !needs_drop_stack
                    && !val.ty.starts_with('%')
                    && Self::is_scalar_ssa_ty(&storage_ty);
                let needs_stack = if scalar_mut_ssa {
                    false
                } else {
                    l.mutable || needs_drop_stack
                };
                let is_scalar_ssa = !needs_stack
                    && (val.ty == "i32"
                        || val.ty == "char"
                        || val.ty == "i1"
                        || val.ty == "float"
                        || val.ty == "double"
                        || val.reg.chars().all(|c| {
                            c.is_ascii_digit() || c == '-' || c == '.'
                        }));
                if scalar_mut_ssa {
                    let reg = if val.reg.starts_with('%') {
                        let tmp = self.fresh("ssa");
                        self.emit(&format!(
                            "  %{tmp} = add {storage_ty} 0, {}",
                            val.reg
                        ));
                        tmp
                    } else if val.reg.chars().all(|c| {
                        c.is_ascii_digit() || c == '-' || c == '.'
                    }) {
                        let tmp = self.fresh("ssa");
                        self.emit(&format!(
                            "  %{tmp} = add {storage_ty} 0, {}",
                            val.reg
                        ));
                        tmp
                    } else {
                        val.reg.trim_start_matches('%').to_string()
                    };
                    env.insert(
                        l.name.clone(),
                        Binding::Reg {
                            reg,
                            ty: storage_ty,
                        },
                    );
                    self.mut_ssa_locals.insert(l.name.clone());
                    if matches!(&l.value, Expression::Literal(Literal::Int(n)) if *n >= 0) {
                        self.mark_zero_init_ssa_i32(&l.name);
                    }
                } else if needs_stack && !is_scalar_ssa {
                    if val.ty.starts_with('%') {
                        if val.reg.starts_with('%') {
                            let slot_ty = struct_value_type(&val.ty);
                            let alloca = self.fresh("alloca");
                            self.emit(&format!("  %{alloca} = alloca {slot_ty}"));
                            if is_struct_pointer_type(&val.ty) {
                                let tmp = self.fresh("load");
                                self.emit(&format!(
                                    "  %{tmp} = load {slot_ty}, {} {}",
                                    val.ty, val.reg
                                ));
                                self.emit(&format!(
                                    "  store {slot_ty} %{tmp}, {} %{alloca}",
                                    llvm_ptr(&slot_ty)
                                ));
                            } else {
                                self.emit(&format!(
                                    "  store {} {}, {} %{alloca}",
                                    val.ty, val.reg, llvm_ptr(&slot_ty)
                                ));
                            }
                            env.insert(
                                l.name.clone(),
                                Binding::Stack {
                                    slot: alloca,
                                    ty: slot_ty,
                                },
                            );
                        } else if is_struct_pointer_type(&val.ty) && !l.mutable {
                            // Struct/tuple literal already lives in an alloca slot.
                            env.insert(
                                l.name.clone(),
                                Binding::Stack {
                                    slot: val.reg.trim_start_matches('%').to_string(),
                                    ty: struct_value_type(&val.ty),
                                },
                            );
                        } else if l.mutable {
                            let slot_ty = if is_struct_pointer_type(&val.ty) {
                                struct_value_type(&val.ty)
                            } else {
                                val.ty.clone()
                            };
                            let alloca = self.fresh("alloca");
                            self.emit(&format!("  %{alloca} = alloca {slot_ty}"));
                            self.emit_value_store(&val, &alloca, &slot_ty);
                            env.insert(
                                l.name.clone(),
                                Binding::Stack {
                                    slot: alloca,
                                    ty: slot_ty,
                                },
                            );
                        } else {
                            env.insert(
                                l.name.clone(),
                                Binding::Reg {
                                    reg: val.reg.clone(),
                                    ty: val.ty.clone(),
                                },
                            );
                        }
                    } else if is_array_ty(&val.ty) && !val.reg.starts_with('%') {
                        env.insert(
                            l.name.clone(),
                            Binding::Stack {
                                slot: val.reg.clone(),
                                ty: val.ty.clone(),
                            },
                        );
                    } else {
                        let mut val = val;
                        if l.mutable
                            && val.ty == "ptr"
                            && matches!(&l.value, Expression::Literal(Literal::String(_)))
                        {
                            val = self.heap_clone_string(val);
                            self.heap_string_bindings.insert(l.name.clone());
                        }
                        let storage_ty = llvm_storage_ty(&val.ty).to_string();
                        let alloca = self.fresh("alloca");
                        self.emit(&format!("  %{alloca} = alloca {storage_ty}"));
                        self.emit_value_store(&val, &alloca, &storage_ty);
                        env.insert(
                            l.name.clone(),
                            Binding::Stack {
                                slot: alloca,
                                ty: val.ty,
                            },
                        );
                    }
                } else if is_array_ty(&val.ty) && !val.reg.starts_with('%') {
                    env.insert(
                        l.name.clone(),
                        Binding::Stack {
                            slot: val.reg.clone(),
                            ty: val.ty.clone(),
                        },
                    );
                } else if val.ty.starts_with('%') {
                    env.insert(
                        l.name.clone(),
                        Binding::Reg {
                            reg: val.reg.trim_start_matches('%').to_string(),
                            ty: val.ty.clone(),
                        },
                    );
                } else {
                    env.insert(
                        l.name.clone(),
                        Binding::Reg {
                            reg: val.reg.trim_start_matches('%').to_string(),
                            ty: val.ty.clone(),
                        },
                    );
                }
                if let Expression::StructLiteral(sl) = &l.value {
                    if self.binding_no_escape(&l.name)
                        && self.struct_literal_fields_stack_safe(sl)
                    {
                        self.no_escape_stack_safe.insert(l.name.clone());
                    }
                }
                if self.drop_plan.is_owned_in(&drop_state.func, &l.name)
                    || self.drop_plan.needs_struct_drop_in(&drop_state.func, &l.name)
                    || self.drop_plan.is_enum_payload_in(&drop_state.func, &l.name)
                {
                    scope_owned.push(l.name.clone());
                }
                if self.rvalue_produces_heap_string(&l.value) {
                    self.heap_string_bindings.insert(l.name.clone());
                }
                if let Expression::Variable { name, .. } = &l.value {
                    if self.drop_plan.is_owned_in(&drop_state.func, name) {
                        drop_state.mark_moved(name);
                    }
                }
                if self.expr_is_non_negative_i32(&l.value, env) {
                    self.mark_non_negative_i32(&l.name);
                }
                if let Some(ty_ann) = &l.ty {
                    self.track_local_int_kind_ann(&l.name, ty_ann);
                    if let Some(en) = self.resolved_enum_name(ty_ann) {
                        self.enum_locals.insert(l.name.clone(), en);
                    }
                    if matches!(ty_ann, TypeAnnotation::FnPtr { .. }) {
                        self.register_fn_ptr_local(&l.name, ty_ann, env);
                    }
                } else if let Expression::Literal(Literal::IntKind(_, k)) = &l.value {
                    self.track_local_int_kind(&l.name, *k);
                } else if let Expression::Call(c) = &l.value {
                    if c.callee == "random" {
                        if let Some(TypeAnnotation::Integer(k)) = c.type_args.first() {
                            self.track_local_int_kind(&l.name, *k);
                        } else if c.args.len() == 2 {
                            let k0 = self.infer_random_int_kind(&c.args[0], env);
                            let k1 = self.infer_random_int_kind(&c.args[1], env);
                            self.track_local_int_kind(&l.name, IntKind::unify(k0, k1));
                        } else {
                            self.track_local_int_kind(&l.name, IntKind::I32);
                        }
                    }
                } else if let Expression::ArrowFn(arrow) = &l.value {
                    if !arrow_has_captures(arrow) {
                        let ret_ann = self.infer_arrow_return_ann(arrow);
                        let ann = TypeAnnotation::FnPtr {
                            lifetime_params: vec![],
                            params: arrow.params.iter().map(|p| p.ty.clone()).collect(),
                            return_type: Some(Box::new(ret_ann)),
                        };
                        self.register_fn_ptr_local(&l.name, &ann, env);
                    }
                } else if let Expression::Variable { name: fname, .. } = &l.value {
                    if let Some(func) = self.functions.get(fname) {
                        let ret_ann = func
                            .return_type
                            .clone()
                            .unwrap_or_else(|| self.infer_block_return_ann(&func.body));
                        let ann = TypeAnnotation::FnPtr {
                            lifetime_params: vec![],
                            params: func.params.iter().map(|p| p.ty.clone()).collect(),
                            return_type: Some(Box::new(ret_ann)),
                        };
                        self.register_fn_ptr_local(&l.name, &ann, env);
                    }
                } else if let Expression::Call(c) = &l.value {
                    if let Some(func) = self.functions.get(&c.callee) {
                        if let Some(ret_ann) = &func.return_type {
                            if let Some(en) = self.resolved_enum_name(ret_ann) {
                                self.enum_locals.insert(l.name.clone(), en);
                            }
                        }
                        if let Some(ret_ann) = func.return_type.clone() {
                            if matches!(ret_ann, TypeAnnotation::FnPtr { .. }) {
                                self.register_fn_ptr_local(&l.name, &ret_ann, env);
                            }
                        }
                    }
                }
                false
            }
            Statement::Const(l) => {
                if let Expression::EnumVariant(ev) = &l.value {
                    if let Some(en) = &ev.enum_name {
                        self.enum_locals.insert(l.name.clone(), en.clone());
                    }
                }
                let val = self.compile_expr(&l.value, env);
                env.insert(
                    l.name.clone(),
                    Binding::Reg {
                        reg: val.reg.trim_start_matches('%').to_string(),
                        ty: val.ty.clone(),
                    },
                );
                false
            }
            Statement::Assign(a) => {
                self.compile_assign(&a.target, &a.value, env);
                false
            }
            Statement::Return(r) => {
                self.emit_deferred_exprs(defers, env);
                if let Some(v) = &r.value {
                    if matches!(v, Expression::ArrowFn(_)) {
                        self.closure_force_heap = true;
                    }
                    if let Expression::Variable { name, .. } = v {
                        if self.drop_plan.is_owned_in(&drop_state.func, name) {
                            drop_state.mark_moved(name);
                        }
                    }
                    let val = self.compile_expr(v, env);
                    self.closure_force_heap = false;
                    self.pending_closure_meta = None;
                    self.emit_auto_drops(drop_state, env);
                    if self.current_async_fn && ret_ty == "i32" {
                        let reg = self.fresh("async");
                        self.emit_runtime_call(
                            "async_run",
                            &format!(
                                "  %{reg} = call i32 @async_run(i32 {})",
                                val.reg
                            ),
                        );
                        self.emit(&format!("  ret i32 %{reg}"));
                        return true;
                    }
                    if val.ty.ends_with('*') && ret_ty.starts_with('%') && !ret_ty.ends_with('*') {
                        let struct_ty = val.ty.trim_end_matches('*');
                        let loaded = self.fresh("load");
                        let src = if val.reg.starts_with('%') {
                            val.reg.clone()
                        } else {
                            format!("%{}", val.reg.trim_start_matches('%'))
                        };
                        self.emit(&format!(
                            "  %{loaded} = load {struct_ty}, {struct_ty}* {src}"
                        ));
                        self.emit(&format!("  ret {struct_ty} %{loaded}"));
                    } else if is_array_ty(&val.ty) {
                        let (reg, ty) = self.materialize_array_call_arg(&val);
                        self.emit(&format!("  ret {ty} {reg}"));
                    } else if val.ty.starts_with('%') {
                        if val.reg.starts_with('%') {
                            self.emit(&format!("  ret {} {}", val.ty, val.reg));
                        } else {
                            let loaded = self.fresh("load");
                            self.emit(&format!(
                                "  %{loaded} = load {}, {} %{}",
                                val.ty,
                                llvm_ptr(&val.ty),
                                val.reg.trim_start_matches('%')
                            ));
                            self.emit(&format!("  ret {} %{}", val.ty, loaded));
                        }
                    } else {
                        let ret_reg = if val.reg.starts_with('%')
                            || val.reg.chars().all(|c| c.is_ascii_digit() || c == '-')
                        {
                            self.coerce_value_reg_to_type(&val.reg, &val.ty, ret_ty)
                        } else {
                            val.reg.clone()
                        };
                        self.emit(&format!("  ret {ret_ty} {ret_reg}"));
                    }
                } else {
                    self.emit_auto_drops(drop_state, env);
                    if ret_ty == "void" {
                        self.emit("  ret void");
                    } else {
                        self.emit(&format!("  ret {ret_ty} zeroinitializer"));
                    }
                }
                true
            }
            Statement::If(i) => {
                let cond = self.compile_expr(&i.condition, env);
                let then_label = self.fresh_label("then");
                let else_label = self.fresh_label("else");
                let merge_label = self.fresh_label("endif");
                let mut assigned_else = HashSet::new();
                if let Some(else_b) = &i.else_block {
                    assigned_else = collect_assigned_in_block(else_b);
                }
                let assigned_then = collect_assigned_in_block(&i.then_block);
                let mut merge_vars: Vec<String> = assigned_then
                    .union(&assigned_else)
                    .filter(|n| self.mut_ssa_locals.contains(*n))
                    .cloned()
                    .collect();
                merge_vars.sort();
                let pre_if = self.snapshot_ssa_regs(env, &merge_vars);
                let env_at_if = env.clone();
                self.emit(&format!(
                    "  br i1 {}, label %{then_label}, label %{else_label}",
                    cond.reg
                ));
                self.emit_block_label(&then_label);
                let then_ret = self.compile_block(&i.then_block, env, ret_ty, drop_state);
                let then_end = self.snapshot_ssa_regs(env, &merge_vars);
                let then_pred = if then_ret {
                    None
                } else {
                    let pred = self.current_block.clone();
                    self.ensure_br_to(&merge_label);
                    Some(pred)
                };
                *env = env_at_if.clone();
                self.emit_block_label(&else_label);
                let else_ret = if let Some(else_b) = &i.else_block {
                    self.compile_block(else_b, env, ret_ty, drop_state)
                } else {
                    false
                };
                let else_end = self.snapshot_ssa_regs(env, &merge_vars);
                let else_pred = if else_ret {
                    None
                } else {
                    Some(self.current_block.clone())
                };
                match (then_pred.as_ref(), else_pred.as_ref()) {
                    (Some(then_p), Some(else_p)) => {
                        self.ensure_br_to(&merge_label);
                        self.emit_block_label(&merge_label);
                        self.emit_if_merge_phis(
                            env,
                            &merge_vars,
                            &pre_if,
                            &then_end,
                            &else_end,
                            then_p,
                            else_p,
                        );
                    }
                    (Some(_then_p), None) => {
                        self.emit_block_label(&merge_label);
                        self.apply_ssa_snapshot(env, &then_end);
                    }
                    (None, Some(_)) => {
                        self.ensure_br_to(&merge_label);
                        self.emit_block_label(&merge_label);
                        self.apply_ssa_snapshot(env, &else_end);
                    }
                    (None, None) => {}
                }
                false
            }
            Statement::For(f) => {
                if f.parallel.is_some() {
                    self.compile_parallel_for(f, env, ret_ty, drop_state);
                    return false;
                }
                let progress_label = f
                    .progress
                    .as_ref()
                    .map(|_| self.setup_progress_label(f, env));
                match &f.kind {
                    ForKind::Range { start, end } => {
                        let pred = self.current_block.clone();
                        let env_before_loop = env.clone();
                        let start = self.compile_expr(start, env);
                        let end = self.compile_expr(end, env);
                        let progress_total = progress_label.as_ref().map(|_| {
                            let t = self.fresh("prog.total");
                            let end_op = Self::llvm_int_operand(&end.reg);
                            let start_op = Self::llvm_int_operand(&start.reg);
                            self.emit(&format!("  %{t} = sub i32 {end_op}, {start_op}"));
                            format!("%{t}")
                        });
                        let idx_alloca = self.fresh("alloca");
                        let ty = "i32";
                        self.emit(&format!("  %{idx_alloca} = alloca {ty}"));
                        self.emit(&format!(
                            "  store {ty} {}, {} %{idx_alloca}",
                            start.reg,
                            llvm_ptr(ty)
                        ));
                        env.insert(
                            f.var.clone(),
                            Binding::Stack {
                                slot: idx_alloca.clone(),
                                ty: ty.into(),
                            },
                        );
                        let cond_label = self.fresh_label("for.cond");
                        let body_label = self.fresh_label("for.body");
                        let latch_label = self.fresh_label("loop.latch");
                        let end_label = self.fresh_label("for.end");
                        let carried = self.loop_carried_ssa_in_body(&f.body, env);
                        let mut init_ops: HashMap<String, (String, String)> = HashMap::new();
                        for name in &carried {
                            if let Some(b) = env.get(name) {
                                let ty = Self::binding_ty(b).to_string();
                                let op = Self::reg_operand_from_binding(b);
                                init_ops.insert(name.clone(), (op, ty));
                            }
                        }
                        self.emit(&format!("  br label %{cond_label}"));
                        self.emit_block_label(&cond_label);
                        let mut latch_regs = HashMap::new();
                        let mut header_phi_regs = HashMap::new();
                        for name in &carried {
                            let Some((init_op, phi_ty)) = init_ops.get(name) else {
                                continue;
                            };
                            let phi_reg = self.fresh("loop.phi");
                            let latch = self.fresh("loop.val");
                            latch_regs.insert(name.clone(), latch.clone());
                            header_phi_regs.insert(name.clone(), (phi_reg.clone(), phi_ty.clone()));
                            self.emit(&format!(
                                "  %{phi_reg} = phi {phi_ty} [{init_op}, %{pred}]"
                            ));
                            if crate::const_mod::parse_nonneg_i32_literal(init_op).is_some() {
                                self.mark_zero_init_ssa_i32(name);
                            } else if self.non_negative_vars.contains(name) {
                                self.mark_non_negative_i32(name);
                            }
                            env.insert(
                                name.clone(),
                                Binding::Reg {
                                    reg: phi_reg,
                                    ty: phi_ty.clone(),
                                },
                            );
                        }
                        let cur = self.fresh("load");
                        self.emit(&format!(
                            "  %{cur} = load {ty}, {} %{idx_alloca}",
                            llvm_ptr(ty)
                        ));
                        let cmp = self.fresh("cmp");
                        self.emit(&format!("  %{cmp} = icmp slt {ty} %{cur}, {}", end.reg));
                        self.emit(&format!(
                            "  br i1 %{cmp}, label %{body_label}, label %{end_label}"
                        ));
                        self.loop_stack.push(LoopPhiContext {
                            latch_regs,
                            latch_label: latch_label.clone(),
                            latch_sync_label: latch_label.clone(),
                            body_label: body_label.clone(),
                            end_label: end_label.clone(),
                            cond_label: cond_label.clone(),
                            exit_pred: cond_label.clone(),
                            carried: carried.clone(),
                            header_phi_regs: header_phi_regs.clone(),
                            break_edges: Vec::new(),
                        });
                        self.emit_block_label(&body_label);
                        if let (Some(lbl), Some(tot)) =
                            (progress_label.as_ref(), progress_total.as_ref())
                        {
                            let cur_body = self.fresh("load");
                            self.emit(&format!(
                                "  %{cur_body} = load {ty}, {} %{idx_alloca}",
                                llvm_ptr(ty)
                            ));
                            let start_op = Self::llvm_int_operand(&start.reg);
                            self.emit_progress_from_index(&cur_body, tot, lbl, Some(&start_op));
                        }
                        self.compile_block(&f.body, env, ret_ty, drop_state);
                        let cur_end = self.fresh("load");
                        self.emit(&format!(
                            "  %{cur_end} = load {ty}, {} %{idx_alloca}",
                            llvm_ptr(ty)
                        ));
                        let next = self.fresh("inc");
                        self.emit(&format!("  %{next} = add {ty} %{cur_end}, 1"));
                        self.emit(&format!(
                            "  store {ty} %{next}, {} %{idx_alloca}",
                            llvm_ptr(ty)
                        ));
                        self.append_loop_phi_backedge(env);
                        self.emit(&format!("  br label %{cond_label}"));
                        let loop_ctx = self.loop_stack.pop().expect("loop stack");
                        self.emit_block_label(&end_label);
                        self.emit_loop_exit_phis(env, &loop_ctx);
                        self.restore_env_after_loop(env, &env_before_loop, &loop_ctx.carried);
                        if progress_label.is_some() {
                            self.emit_progress_finish();
                        }
                    }
                    ForKind::Iterable { iterable } => {
                        let collection = self.compile_expr(iterable, env);
                        let idx_alloca = self.fresh("alloca");
                        let idx_ty = "i32";
                        self.emit(&format!("  %{idx_alloca} = alloca {idx_ty}"));
                        self.emit(&format!(
                            "  store {idx_ty} 0, {} %{idx_alloca}",
                            llvm_ptr(idx_ty)
                        ));

                        let (elem_ty, end_bound, iter_kind) =
                            if let Some(n) = array_len_from_ty(&collection.ty) {
                                let elem = collection
                                    .ty
                                    .strip_prefix('[')
                                    .and_then(|inner| inner.split(" x ").nth(1))
                                    .and_then(|s| s.strip_suffix(']'))
                                    .unwrap_or("i32")
                                    .to_string();
                                (elem, n.to_string(), "array")
                            } else if collection.ty == "vec_str" {
                                let len_reg = self.fresh("vec_strlen");
                                let vec_reg = if collection.reg.starts_with('%') {
                                    collection.reg.clone()
                                } else {
                                    format!("%{}", collection.reg)
                                };
                                self.emit_runtime_call(
                                    "vec_str_len",
                                    &format!(
                                        "  %{len_reg} = call i32 @vec_str_len(ptr {vec_reg})"
                                    ),
                                );
                                ("ptr".into(), format!("%{len_reg}"), "vec_str")
                            } else if collection.ty == "ptr" {
                                let len_reg = self.fresh("strlen");
                                let str_reg = if collection.reg.starts_with('%') {
                                    collection.reg.clone()
                                } else {
                                    format!("%{}", collection.reg)
                                };
                                self.emit_runtime_call(
                                    "strlen",
                                    &format!(
                                        "  %{len_reg} = call i32 @strlen(ptr {str_reg})"
                                    ),
                                );
                                ("i32".into(), format!("%{len_reg}"), "string")
                            } else {
                                ("i32".into(), "0".into(), "none")
                            };

                        let var_alloca = self.fresh("alloca");
                        self.emit(&format!("  %{var_alloca} = alloca {elem_ty}"));
                        env.insert(
                            f.var.clone(),
                            Binding::Stack {
                                slot: var_alloca.clone(),
                                ty: elem_ty.clone(),
                            },
                        );

                        let cond_label = self.fresh_label("for.cond");
                        let body_label = self.fresh_label("for.body");
                        let end_label = self.fresh_label("for.end");
                        self.emit(&format!("  br label %{cond_label}"));
                        self.emit_block_label(&cond_label);
                        let cur = self.fresh("load");
                        self.emit(&format!(
                            "  %{cur} = load {idx_ty}, {} %{idx_alloca}",
                            llvm_ptr(idx_ty)
                        ));
                        let cmp = self.fresh("cmp");
                        if end_bound.chars().next().is_some_and(|c| c == '%') {
                            self.emit(&format!(
                                "  %{cmp} = icmp slt {idx_ty} %{cur}, {end_bound}"
                            ));
                        } else {
                            self.emit(&format!(
                                "  %{cmp} = icmp slt {idx_ty} %{cur}, {end_bound}"
                            ));
                        }
                        self.emit(&format!(
                            "  br i1 %{cmp}, label %{body_label}, label %{end_label}"
                        ));
                        let carried_iter = self.loop_carried_ssa_in_body(&f.body, env);
                        self.loop_stack.push(LoopPhiContext {
                            latch_regs: HashMap::new(),
                            latch_label: body_label.clone(),
                            latch_sync_label: body_label.clone(),
                            body_label: body_label.clone(),
                            end_label: end_label.clone(),
                            cond_label: cond_label.clone(),
                            exit_pred: cond_label.clone(),
                            carried: carried_iter,
                            header_phi_regs: HashMap::new(),
                            break_edges: Vec::new(),
                        });
                        self.emit_block_label(&body_label);

                        if iter_kind == "array" {
                            let gep = self.fresh("gep");
                            let arr_ty = &collection.ty;
                            let arr_ptr = self.materialize_array_ptr(&collection);
                            self.emit(&format!(
                                "  %{gep} = getelementptr inbounds {arr_ty}, {arr_ty}* {arr_ptr}, i32 0, i32 %{cur}"
                            ));
                            let loaded = self.fresh("load");
                            self.emit(&format!(
                                "  %{loaded} = load {elem_ty}, {} %{gep}",
                                llvm_ptr(&elem_ty)
                            ));
                            self.emit(&format!(
                                "  store {elem_ty} %{loaded}, {} %{var_alloca}",
                                llvm_ptr(&elem_ty)
                            ));
                        } else if iter_kind == "vec_str" {
                            let vec_reg = if collection.reg.starts_with('%') {
                                collection.reg.clone()
                            } else {
                                format!("%{}", collection.reg)
                            };
                            let part = self.fresh("vec_get");
                            self.emit_runtime_call(
                                "vec_str_get",
                                &format!(
                                    "  %{part} = call ptr @vec_str_get(ptr {vec_reg}, i32 %{cur})"
                                ),
                            );
                            self.emit(&format!(
                                "  store ptr %{part}, ptr %{var_alloca}"
                            ));
                        } else if iter_kind == "string" {
                            let str_reg = if collection.reg.starts_with('%') {
                                collection.reg.clone()
                            } else {
                                format!("%{}", collection.reg)
                            };
                            let ch = self.fresh("char_at");
                            self.emit_runtime_call(
                                "char_at",
                                &format!(
                                    "  %{ch} = call i32 @char_at(ptr {str_reg}, i32 %{cur})"
                                ),
                            );
                            self.emit(&format!(
                                "  store {elem_ty} %{ch}, {} %{var_alloca}",
                                llvm_ptr(&elem_ty)
                            ));
                        }

                        if let Some(lbl) = progress_label.as_ref() {
                            self.emit_progress_from_index(&cur, &end_bound, lbl, None);
                        }
                        self.compile_block(&f.body, env, ret_ty, drop_state);
                        let loop_ctx = self.loop_stack.pop().expect("loop stack");
                        let next = self.fresh("inc");
                        self.emit(&format!("  %{next} = add {idx_ty} %{cur}, 1"));
                        self.emit(&format!(
                            "  store {idx_ty} %{next}, {} %{idx_alloca}",
                            llvm_ptr(idx_ty)
                        ));
                        self.emit(&format!("  br label %{cond_label}"));
                        self.emit_block_label(&end_label);
                        self.emit_loop_exit_phis(env, &loop_ctx);
                        if progress_label.is_some() {
                            self.emit_progress_finish();
                        }
                    }
                }
                false
            }
            Statement::While(w) => {
                let pred = self.current_block.clone();
                let env_before_loop = env.clone();
                let cond_label = self.fresh_label("while.cond");
                let body_label = self.fresh_label("while.body");
                let end_label = self.fresh_label("while.end");
                let carried = self.loop_carried_ssa_in_body(&w.body, env);
                let mut init_ops: HashMap<String, (String, String)> = HashMap::new();
                for name in &carried {
                    if let Some(b) = env.get(name) {
                        let ty = Self::binding_ty(b).to_string();
                        let op = Self::reg_operand_from_binding(b);
                        init_ops.insert(name.clone(), (op, ty));
                    }
                }
                self.emit(&format!("  br label %{cond_label}"));
                self.emit_block_label(&cond_label);
                let mut latch_regs = HashMap::new();
                let mut header_phi_regs = HashMap::new();
                for name in &carried {
                    let Some((init_op, ty)) = init_ops.get(name) else {
                        continue;
                    };
                    let phi_reg = self.fresh("loop.phi");
                    let latch = self.fresh("loop.val");
                    latch_regs.insert(name.clone(), latch.clone());
                    header_phi_regs.insert(name.clone(), (phi_reg.clone(), ty.clone()));
                    self.emit(&format!(
                        "  %{phi_reg} = phi {ty} [{init_op}, %{pred}]"
                    ));
                    if crate::const_mod::parse_nonneg_i32_literal(init_op).is_some() {
                        self.mark_zero_init_ssa_i32(name);
                    } else if self.non_negative_vars.contains(name) {
                        self.mark_non_negative_i32(name);
                    }
                    env.insert(
                        name.clone(),
                        Binding::Reg {
                            reg: phi_reg,
                            ty: ty.clone(),
                        },
                    );
                }
                self.loop_stack.push(LoopPhiContext {
                    latch_regs,
                    latch_label: cond_label.clone(),
                    latch_sync_label: cond_label.clone(),
                    body_label: body_label.clone(),
                    end_label: end_label.clone(),
                    cond_label: cond_label.clone(),
                    exit_pred: cond_label.clone(),
                    carried: carried.clone(),
                    header_phi_regs: header_phi_regs.clone(),
                    break_edges: Vec::new(),
                });
                let cond = self.compile_expr(&w.condition, env);
                if let Some(ctx) = self.loop_stack.last_mut() {
                    ctx.exit_pred = self.current_block.clone();
                }
                self.emit(&format!(
                    "  br i1 {}, label %{body_label}, label %{end_label}",
                    cond.reg
                ));
                self.emit_block_label(&body_label);
                let body_closed = self.compile_block(&w.body, env, ret_ty, drop_state);
                if !body_closed {
                    self.append_loop_phi_backedge(env);
                    self.emit(&format!("  br label %{cond_label}"));
                }
                let loop_ctx = self.loop_stack.pop().expect("loop stack");
                self.emit_block_label(&end_label);
                self.emit_loop_exit_phis(env, &loop_ctx);
                self.restore_env_after_loop(env, &env_before_loop, &loop_ctx.carried);
                false
            }
            Statement::Break { .. } => {
                self.emit_loop_break(env);
                true
            }
            Statement::Continue { .. } => {
                self.emit_loop_continue(env);
                true
            }
            Statement::Print(p) => {
                for arg in &p.args {
                    self.register_call_moves(arg, env, drop_state);
                }
                if let Some(color) = &p.color {
                    self.register_call_moves(color, env, drop_state);
                }
                self.compile_print_stmt(p, env);
                false
            }
            Statement::Expression(expr) => {
                self.register_call_moves(expr, env, drop_state);
                let _ = self.compile_expr(expr, env);
                false
            }
            Statement::Defer(_) => false,
            Statement::Benchmark(body) => {
                self.emit_runtime_call(
                    "benchmark_begin",
                    "  call void @benchmark_begin()",
                );
                self.compile_block(body, env, ret_ty, drop_state);
                self.emit_runtime_call(
                    "benchmark_end",
                    "  call void @benchmark_end()",
                );
                false
            }
            Statement::Spawn(sp) => {
                let handle = self.compile_spawn(sp.kind, &sp.body, env, drop_state);
                self.emit_spawn_handle_drop(&handle.reg, sp.kind);
                false
            }
            Statement::Unsafe(body) => {
                self.compile_block(body, env, ret_ty, drop_state);
                false
            }
            Statement::Asm { template, .. } => {
                self.compile_asm(template);
                false
            }
            Statement::Import(_) => false,
        }
    }
}

