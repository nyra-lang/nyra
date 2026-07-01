#![allow(unused_imports)]
//! Blocks, assignments, casts, and scope-end auto-drops.
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
    llvm_binop_operand, llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn compile_assign(&mut self, target: &Expression, value: &Expression, env: &mut Env) {
        let val = self.compile_expr(value, env);
        match target {
            Expression::Variable { name, .. } => {
                if self.mut_ssa_locals.contains(name) {
                    let reg = if val.reg.starts_with('%') {
                        val.reg.trim_start_matches('%').to_string()
                    } else {
                        val.reg.clone()
                    };
                    let ty = env
                        .get(name)
                        .map(|b| Self::binding_ty(b).to_string())
                        .unwrap_or_else(|| val.ty.clone());
                    env.insert(
                        name.clone(),
                        Binding::Reg {
                            reg: reg.clone(),
                            ty,
                        },
                    );
                    if self.expr_is_non_negative_i32(value, env) {
                        self.mark_non_negative_i32(name);
                    }
                    self.mark_non_negative_from_mod_assign(name, value, env);
                } else if let Some(binding) = env.get(name).cloned() {
                    if self.heap_string_bindings.contains(name) {
                        let ty = Self::binding_ty(&binding);
                        if ty == "ptr" {
                            let (loaded, _) = self.binding_load(&binding);
                            self.emit_runtime_call(
                                "free",
                                &format!("  call void @free(ptr %{loaded})"),
                            );
                        }
                    }
                    self.binding_store_expr(&binding, &val);
                    if self.rvalue_produces_heap_string(value) {
                        self.heap_string_bindings.insert(name.clone());
                    } else if matches!(value, Expression::Literal(Literal::String(_))) {
                        self.heap_string_bindings.remove(name);
                    }
                }
            }
            Expression::Unary(u) if u.op == UnaryOp::Deref => {
                let ptr = self.compile_expr(&u.operand, env);
                let p = self.materialize_ptr_reg(&ptr.reg);
                let store_ty = if val.ty == "ptr" {
                    "i32".to_string()
                } else {
                    val.ty.clone()
                };
                let store_val = if val.reg.starts_with('%') {
                    val.reg.clone()
                } else if val.reg.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.') {
                    val.reg.clone()
                } else {
                    format!("%{}", val.reg.trim_start_matches('%'))
                };
                self.emit(&format!("  store {store_ty} {store_val}, ptr {p}"));
            }
            Expression::FieldAccess(fa) => {
                let obj = self.compile_expr(&fa.object, env);
                let struct_name = obj
                    .ty
                    .trim_start_matches('%')
                    .trim_end_matches('*')
                    .to_string();
                let field_idx = self.field_index(&struct_name, &fa.field).unwrap_or(0);
                let field_ty = if let Some(fields) = self.tuple_fields.get(&struct_name) {
                    fields
                        .get(field_idx)
                        .map(|a| self.llvm_type_of(a))
                        .unwrap_or_else(|| "i32".into())
                } else {
                    self.llvm_type_of(&self.struct_fields[&struct_name][field_idx].1)
                };
                let llvm_struct = format!("%{struct_name}");
                let base_ptr = if obj.ty.ends_with('*') {
                    if obj.reg.starts_with('%') {
                        obj.reg.clone()
                    } else {
                        format!("%{}", obj.reg)
                    }
                } else if obj.reg.starts_with('%') {
                    let tmp = self.fresh("alloca");
                    self.emit(&format!("  %{tmp} = alloca {llvm_struct}"));
                    self.emit(&format!(
                        "  store {llvm_struct} {}, {llvm_struct}* %{tmp}",
                        obj.reg
                    ));
                    format!("%{tmp}")
                } else {
                    format!("%{}", obj.reg)
                };
                let gep = self.fresh("gep");
                self.emit(&format!(
                    "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {field_idx}"
                ));
                self.emit(&format!(
                    "  store {field_ty} {}, {} %{gep}",
                    val.reg,
                    llvm_ptr(&field_ty)
                ));
            }
            Expression::Index(ix) => {
                let idx = self.compile_expr(&ix.index, env);
                let idx_op = Self::llvm_i32_operand(&idx.reg);
                let (arr_ty, arr_ptr) = self.array_lvalue_ptr(&ix.object, env);
                if let Some(len) = array_len_from_ty(&arr_ty) {
                    self.emit_array_bounds_check(&idx_op, len);
                }
                let (gep, elem) = self.emit_array_elem_ptr(&arr_ty, &arr_ptr, &idx_op);
                self.emit(&format!(
                    "  store {elem} {}, {elem}* %{gep}",
                    val.reg,
                ));
            }
            _ => {}
        }
    }

    pub(super) fn compile_cast(&mut self, c: &CastExpr, env: &Env) -> ExprValue {
        if let TypeAnnotation::DynTrait { trait_name, .. } = &c.target_type {
            return self.compile_trait_object_box(trait_name, &c.expr, env);
        }
        let inner = self.compile_expr(&c.expr, env);
        let target = self.llvm_type_of(&c.target_type);
        let reg = self.fresh("cast");
        match (inner.ty.as_str(), target.as_str()) {
            ("i32", "ptr") | ("i64", "ptr") => {
                self.emit(&format!(
                    "  %{reg} = inttoptr {} {} to ptr",
                    if inner.ty == "i64" { "i64" } else { "i32" },
                    inner.reg
                ));
            }
            ("ptr", "i32") | ("ptr", "i64") => {
                let p = self.materialize_ptr_reg(&inner.reg);
                self.emit(&format!(
                    "  %{reg} = ptrtoint ptr {p} to {target}"
                ));
            }
            ("ptr", "ptr") => {
                return inner;
            }
            (from, to) if from == to => {
                return inner;
            }
            (from, to) => {
                let coerced = self.coerce_value_reg_to_type(&inner.reg, from, to);
                if coerced != inner.reg {
                    return ExprValue {
                        reg: coerced,
                        ty: target,
                    };
                }
                self.emit(&format!("  %{reg} = bitcast {from} {} to {to}", inner.reg));
            }
        }
        ExprValue {
            reg: format!("%{reg}"),
            ty: target,
        }
    }

    pub(super) fn compile_block(
        &mut self,
        block: &Block,
        env: &mut Env,
        ret_ty: &str,
        drop_state: &mut DropState,
    ) -> bool {
        self.compiling_drop = Some(drop_state as *mut DropState);
        let out = self.compile_block_inner(block, env, ret_ty, drop_state);
        self.compiling_drop = None;
        out
    }

    pub(super) fn compile_block_inner(
        &mut self,
        block: &Block,
        env: &mut Env,
        ret_ty: &str,
        drop_state: &mut DropState,
    ) -> bool {
        let mut scope_owned = Vec::new();
        let mut heap_closures = Vec::new();
        let mut defers = Vec::new();
        let mut has_return = false;
        for stmt in &block.statements {
            if let Statement::Defer(e) = stmt {
                defers.push(e.clone());
                continue;
            }
            if self.compile_statement(
                stmt,
                env,
                ret_ty,
                drop_state,
                &mut scope_owned,
                &mut heap_closures,
                &defers,
            ) {
                has_return = true;
                break;
            }
        }
        for name in heap_closures {
            if let Some(Binding::Closure(meta)) = env.get(&name) {
                self.emit_closure_env_free(meta);
            }
        }
        if !has_return {
            for e in defers.iter().rev() {
                self.emit_deferred_expr(e, env);
            }
        }
        has_return
    }

    pub(super) fn compile_block_as_expr(
        &mut self,
        block: &Block,
        env: &Env,
        drop_state: &mut DropState,
    ) -> ExprValue {
        let mut child_env = env.clone();
        let mut scope_owned = Vec::new();
        let mut heap_closures = Vec::new();
        let defers: Vec<Expression> = Vec::new();
        let mut last: Option<ExprValue> = None;
        for stmt in &block.statements {
            match stmt {
                Statement::Return(r) => {
                    if let Some(v) = &r.value {
                        return self.compile_expr(v, &mut child_env);
                    }
                    return ExprValue {
                        reg: "0".into(),
                        ty: "void".into(),
                    };
                }
                Statement::Expression(e) => {
                    self.register_call_moves(e, &mut child_env, drop_state);
                    last = Some(self.compile_expr(e, &mut child_env));
                }
                Statement::If(i) if i.else_block.is_some() => {
                    let if_expr = IfExpr {
                        condition: i.condition.clone(),
                        then_block: i.then_block.clone(),
                        else_block: i.else_block.clone().unwrap(),
                        span: expr_span(&i.condition),
                    };
                    last = Some(self.compile_if_expr(&if_expr, &mut child_env));
                }
                _ => {
                    let _ = self.compile_statement(
                        stmt,
                        &mut child_env,
                        "void",
                        drop_state,
                        &mut scope_owned,
                        &mut heap_closures,
                        &defers,
                    );
                }
            }
        }
        last.unwrap_or_else(|| ExprValue {
            reg: "0".into(),
            ty: "i32".into(),
        })
    }

    pub(super) fn infer_block_expr_llvm_ty(&self, block: &Block, env: &Env) -> String {
        for stmt in block.statements.iter().rev() {
            match stmt {
                Statement::Return(r) => {
                    if let Some(v) = &r.value {
                        return self.infer_expr_llvm_ty(v, env);
                    }
                    return "void".into();
                }
                Statement::Expression(e) => return self.infer_expr_llvm_ty(e, env),
                Statement::If(i) if i.else_block.is_some() => {
                    return self.infer_block_expr_llvm_ty(&i.then_block, env);
                }
                _ => {}
            }
        }
        "i32".into()
    }

    pub(super) fn emit_deferred_expr(&mut self, e: &Expression, env: &mut Env) {
        let v = self.compile_expr(e, env);
        if v.ty == "ptr" {
            let p = self.materialize_ptr_reg(&v.reg);
            self.emit_runtime_call(
                "free",
                &format!("  call void @free(ptr {p})"),
            );
        }
    }

    pub(super) fn emit_deferred_exprs(&mut self, defers: &[Expression], env: &mut Env) {
        for e in defers.iter().rev() {
            self.emit_deferred_expr(e, env);
        }
    }

    pub(super) fn emit_auto_drops(
        &mut self,
        drop_state: &DropState,
        env: &Env,
    ) {
        let mut to_drop: HashSet<String> = HashSet::new();
        for map in [
            &self.drop_plan.owned_bindings,
            &self.drop_plan.custom_struct_bindings,
            &self.drop_plan.composite_struct_bindings,
            &self.drop_plan.enum_payload_bindings,
        ] {
            if let Some(set) = map.get(&drop_state.func) {
                for name in set {
                    if !drop_state.moved.contains(name)
                        && !self
                            .drop_plan
                            .is_manually_freed_in(&drop_state.func, name)
                    {
                        to_drop.insert(name.clone());
                    }
                }
            }
        }
        for name in to_drop {
            self.emit_drop_local(&name, env, drop_state);
        }
    }

    pub(super) fn register_call_moves(
        &mut self,
        expr: &Expression,
        env: &Env,
        drop_state: &mut DropState,
    ) {
        let Expression::Call(c) = expr else {
            return;
        };
        if c.callee == "free" {
            return;
        }
        for arg in &c.args {
            if let Expression::ArrowFn(arrow) = arg {
                self.mark_arrow_capture_moves(arrow, env, drop_state);
            } else if let Expression::Unary(u) = arg {
                if matches!(u.op, UnaryOp::Ref | UnaryOp::RefMut) {
                    continue;
                }
                if let Expression::Variable { name, .. } = &u.operand {
                    if self.drop_plan.is_owned_in(&drop_state.func, name) {
                        drop_state.mark_moved(name);
                    }
                }
            } else if let Expression::Variable { name, .. } = arg {
                if self.drop_plan.is_owned_in(&drop_state.func, name) {
                    drop_state.mark_moved(name);
                }
            }
        }
    }

    pub(super) fn mark_arrow_capture_moves(
        &mut self,
        arrow: &ArrowFnExpr,
        env: &Env,
        drop_state: &mut DropState,
    ) {
        if !arrow_has_captures(arrow) {
            return;
        }
        let outer: HashSet<String> = env.keys().cloned().collect();
        for name in collect_arrow_captures(arrow, &outer) {
            if self.drop_plan.is_owned_in(&drop_state.func, &name) {
                drop_state.mark_moved(&name);
            }
        }
    }
}

