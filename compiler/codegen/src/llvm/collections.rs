#![allow(unused_imports)]
//! Array literals, indexing, and sort.
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
    /// Pointer to an array lvalue (`[N x T]*`): struct field GEP or stack slot.
    pub(super) fn array_lvalue_ptr(
        &mut self,
        expr: &Expression,
        env: &Env,
    ) -> (String, String) {
        if let Expression::FieldAccess(fa) = expr {
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
            let gep = self.fresh("arr.fld");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {field_idx}"
            ));
            return (field_ty, format!("%{gep}"));
        }
        let obj = self.compile_expr(expr, env);
        (
            obj.ty.clone(),
            self.materialize_array_ptr(&obj),
        )
    }

    /// Stack slot or alloca name (`%slot`) for indexing / mutation on array values.
    pub(super) fn materialize_array_ptr(&mut self, obj: &ExprValue) -> String {
        if !is_array_ty(&obj.ty) {
            return llvm_ptr_reg(&obj.reg);
        }
        // Stack slots use bare alloca names (no `%`); SSA loads use `%load.*` / `%ld.*`.
        if !obj.reg.starts_with('%') {
            return format!("%{}", obj.reg);
        }
        let slot = self.fresh("arr.tmp");
        self.emit(&format!("  %{slot} = alloca {}", obj.ty));
        self.emit(&format!(
            "  store {} {}, {}* %{slot}",
            obj.ty, obj.reg, obj.ty
        ));
        format!("%{slot}")
    }

    /// By-value aggregate for calling a function that takes `[N x T]` by value.
    pub(super) fn materialize_array_call_arg(&mut self, obj: &ExprValue) -> (String, String) {
        if !is_array_ty(&obj.ty) {
            return (obj.reg.clone(), obj.ty.clone());
        }
        let ptr = if obj.reg.starts_with('%') {
            obj.reg.clone()
        } else {
            format!("%{}", obj.reg)
        };
        let loaded = self.fresh("arr.ld");
        self.emit(&format!(
            "  %{loaded} = load {}, {}* {ptr}",
            obj.ty, obj.ty
        ));
        (format!("%{loaded}"), obj.ty.clone())
    }

    /// Pointer to one array element (`i32* %gep`) for load/store.
    pub(super) fn emit_array_elem_ptr(
        &mut self,
        arr_ty: &str,
        arr_ptr: &str,
        idx_op: &str,
    ) -> (String, String) {
        let elem = array_elem_from_ty(arr_ty).unwrap_or_else(|| "i32".into());
        let base = self.fresh("arr.base");
        self.emit(&format!(
            "  %{base} = getelementptr inbounds {arr_ty}, {arr_ty}* {arr_ptr}, i32 0, i32 0"
        ));
        let gep = self.fresh("gep");
        let idx = if idx_op.starts_with("i32 ") {
            idx_op.to_string()
        } else if idx_op.starts_with('%') {
            format!("i32 {idx_op}")
        } else {
            format!("i32 %{idx_op}")
        };
        self.emit(&format!(
            "  %{gep} = getelementptr inbounds {elem}, {elem}* %{base}, {idx}"
        ));
        (gep, elem)
    }

    /// Abort when `idx >= len`. Emits no extra basic blocks (safe inside loops/if).
    pub(super) fn emit_array_bounds_check(&mut self, idx_op: &str, len: usize) {
        let ok = self.fresh("bounds_cmp");
        self.emit(&format!("  %{ok} = icmp ult {idx_op}, {len}"));
        let ext = self.fresh("bounds_ok");
        self.emit(&format!("  %{ext} = zext i1 %{ok} to i32"));
        self.emit_runtime_call(
            "bounds_assert_i32",
            &format!("  call void @bounds_assert_i32(i32 %{ext})"),
        );
    }

    pub(super) fn llvm_i32_operand(reg: &str) -> String {
        if reg.chars().all(|c| c.is_ascii_digit() || c == '-') {
            format!("i32 {reg}")
        } else if reg.starts_with('%') {
            format!("i32 {reg}")
        } else {
            format!("i32 %{reg}")
        }
    }

    pub(super) fn compile_index(
        &mut self,
        ix: &IndexExpr,
        env: &Env,
    ) -> ExprValue {
        let obj = self.compile_expr(&ix.object, env);
        let idx = self.compile_expr(&ix.index, env);
        if obj.ty == "bytes" {
            self.record_runtime("byte_at");
            let ptr = self.materialize_ptr_reg(&obj.reg);
            let idx64 = self.fresh("idx64");
            self.emit(&format!(
                "  %{idx64} = sext {} to i64",
                Self::llvm_i32_operand(&idx.reg)
            ));
            let reg = self.fresh("byte");
            self.emit(&format!(
                "  %{reg} = call i32 @byte_at(ptr {ptr}, i64 %{idx64})"
            ));
            return ExprValue {
                reg: format!("%{reg}"),
                ty: "i32".into(),
            };
        }
        if let Some(len) = array_len_from_ty(&obj.ty) {
            self.emit_array_bounds_check(&Self::llvm_i32_operand(&idx.reg), len);
        }
        let idx_op = Self::llvm_i32_operand(&idx.reg);
        let arr_ptr = self.materialize_array_ptr(&obj);
        let (gep, elem) = self.emit_array_elem_ptr(&obj.ty, &arr_ptr, &idx_op);
        let reg = self.fresh("load");
        self.emit(&format!("  %{reg} = load {elem}, {elem}* %{gep}"));
        ExprValue {
            reg: format!("%{reg}"),
            ty: elem,
        }
    }

    pub(super) fn compile_array_repeat(
        &mut self,
        element: &Expression,
        count: usize,
        env: &Env,
    ) -> ExprValue {
        if count == 0 {
            return ExprValue {
                reg: "0".into(),
                ty: "[0 x i32]".into(),
            };
        }
        let first = self.compile_expr(element, env);
        let ty = format!("[{count} x {}]", first.ty);
        let alloca = self.fresh("alloca");
        self.emit(&format!("  %{alloca} = alloca {ty}"));
        let mut i = 0;
        while i < count {
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {ty}, {ty}* %{alloca}, i32 0, i32 {i}",
            ));
            self.emit(&format!(
                "  store {} {}, {} %{gep}",
                first.ty, first.reg, llvm_ptr(&first.ty)
            ));
            i = i + 1;
        }
        ExprValue {
            reg: alloca,
            ty,
        }
    }

    pub(super) fn compile_array_literal(
        &mut self,
        al: &ArrayLiteralExpr,
        env: &Env,
    ) -> ExprValue {
        if al.is_empty() {
            return ExprValue {
                reg: "0".into(),
                ty: "[0 x i32]".into(),
            };
        }

        let mut values: Vec<ExprValue> = Vec::new();
        for spread in &al.spreads {
            let spread_val = self.compile_expr(spread, env);
            if let Some(n) = array_len_from_ty(&spread_val.ty) {
                let arr_ty = spread_val.ty.clone();
                let elem_ty_str =
                    array_elem_from_ty(&arr_ty).unwrap_or_else(|| "i32".into());
                let base = self.materialize_array_ptr(&spread_val);
                for i in 0..n {
                    let gep = self.fresh("spread.gep");
                    self.emit(&format!(
                        "  %{gep} = getelementptr inbounds {arr_ty}, {arr_ty}* {base}, i32 0, i32 {i}"
                    ));
                    let load = self.fresh("spread.ld");
                    self.emit(&format!(
                        "  %{load} = load {elem_ty_str}, {elem_ty_str}* %{gep}"
                    ));
                    values.push(ExprValue {
                        reg: format!("%{load}"),
                        ty: elem_ty_str.clone(),
                    });
                }
            } else if let Some(struct_name) = self.expr_receiver_struct_name(spread, env) {
                let field_defs = self.struct_fields[&struct_name].clone();
                for (idx, (_, field_ann)) in field_defs.iter().enumerate() {
                    let val =
                        self.compile_spread_field(&spread_val, &struct_name, idx, field_ann);
                    values.push(val);
                }
            } else {
                panic!("typechecked array spread must be array or struct");
            }
        }
        for el in &al.elems {
            values.push(self.compile_expr(el, env));
        }

        let elem_ty = struct_value_type(&values[0].ty);
        let n = values.len();
        let ty = format!("[{n} x {elem_ty}]");
        let alloca = self.fresh("alloca");
        self.emit(&format!("  %{alloca} = alloca {ty}"));
        for (i, v) in values.iter().enumerate() {
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {ty}, {ty}* %{alloca}, i32 0, i32 {i}",
            ));
            self.emit_store_to_gep(v, &gep, &elem_ty);
        }
        ExprValue {
            reg: alloca,
            ty,
        }
    }

    pub(super) fn compile_array_sort(
        &mut self,
        mc: &MethodCallExpr,
        env: &Env,
    ) -> ExprValue {
        let obj = self.compile_expr(&mc.object, env);
        let n = array_len_from_ty(&obj.ty).expect("typechecked");
        let elem = array_elem_from_ty(&obj.ty).unwrap_or_else(|| "i32".into());
        let arr_ptr = self.materialize_array_ptr(&obj);
        let dst = self.fresh("alloca");
        self.emit(&format!("  %{dst} = alloca {}", obj.ty));
        let src_elem = self.fresh("gep");
        let dst_elem = self.fresh("gep");
        self.emit(&format!(
            "  %{src_elem} = getelementptr inbounds {}, {}* {arr_ptr}, i32 0, i32 0",
            obj.ty, obj.ty
        ));
        self.emit(&format!(
            "  %{dst_elem} = getelementptr inbounds {}, {}* %{dst}, i32 0, i32 0",
            obj.ty, obj.ty
        ));
        if elem == "double" {
            self.emit_runtime_call(
                "array_f64_sort_copy",
                &format!(
                    "  call void @array_f64_sort_copy(double* %{dst_elem}, double* %{src_elem}, i32 {n})"
                ),
            );
        } else {
            self.emit_runtime_call(
                "array_i32_sort_copy",
                &format!(
                    "  call void @array_i32_sort_copy(i32* %{dst_elem}, i32* %{src_elem}, i32 {n})"
                ),
            );
        }
        ExprValue {
            reg: dst,
            ty: obj.ty.clone(),
        }
    }

    fn llvm_elem_byte_size(&self, elem: &str) -> i64 {
        match elem {
            "i32" | "i1" => 4,
            "double" => 8,
            ty if ty.starts_with('%') => {
                let name = ty.trim_start_matches('%');
                if let Some(fields) = self.struct_fields.get(name) {
                    let llvm_fields: Vec<String> = fields
                        .iter()
                        .map(|(_, ann)| self.llvm_type_of(ann))
                        .collect();
                    llvm_struct_size_bytes(&llvm_fields)
                } else {
                    8
                }
            }
            _ => 8,
        }
    }

    fn load_array_elem_value(
        &mut self,
        arr_ty: &str,
        arr_ptr: &str,
        idx_reg: &str,
    ) -> ExprValue {
        let idx_op = Self::llvm_i32_operand(idx_reg);
        let (gep, elem) = self.emit_array_elem_ptr(arr_ty, arr_ptr, &idx_op);
        let reg = self.fresh("sort.ld");
        self.emit(&format!("  %{reg} = load {elem}, {elem}* %{gep}"));
        ExprValue {
            reg: format!("%{reg}"),
            ty: elem,
        }
    }

    fn emit_array_elem_swap(
        &mut self,
        arr_ty: &str,
        arr_ptr: &str,
        elem: &str,
        i_reg: &str,
        j_reg: &str,
    ) {
        let i_op = Self::llvm_i32_operand(i_reg);
        let j_op = Self::llvm_i32_operand(j_reg);
        let (gep_i, _) = self.emit_array_elem_ptr(arr_ty, arr_ptr, &i_op);
        let (gep_j, _) = self.emit_array_elem_ptr(arr_ty, arr_ptr, &j_op);
        match elem {
            "i32" | "i1" => {
                let a = self.fresh("swap.a");
                let b = self.fresh("swap.b");
                self.emit(&format!("  %{a} = load i32, i32* %{gep_i}"));
                self.emit(&format!("  %{b} = load i32, i32* %{gep_j}"));
                self.emit(&format!("  store i32 %{b}, i32* %{gep_i}"));
                self.emit(&format!("  store i32 %{a}, i32* %{gep_j}"));
            }
            "double" => {
                let a = self.fresh("swap.a");
                let b = self.fresh("swap.b");
                self.emit(&format!("  %{a} = load double, double* %{gep_i}"));
                self.emit(&format!("  %{b} = load double, double* %{gep_j}"));
                self.emit(&format!("  store double %{b}, double* %{gep_i}"));
                self.emit(&format!("  store double %{a}, double* %{gep_j}"));
            }
            _ => {
                let sz = self.llvm_elem_byte_size(elem);
                let tmp = self.fresh("swap.tmp");
                self.emit(&format!("  %{tmp} = alloca {elem}"));
                self.emit(&format!(
                    "  call void @llvm.memcpy.p0.p0.i64(ptr %{tmp}, ptr %{gep_i}, i64 {sz}, i1 false)"
                ));
                self.emit(&format!(
                    "  call void @llvm.memcpy.p0.p0.i64(ptr %{gep_i}, ptr %{gep_j}, i64 {sz}, i1 false)"
                ));
                self.emit(&format!(
                    "  call void @llvm.memcpy.p0.p0.i64(ptr %{gep_j}, ptr %{tmp}, i64 {sz}, i1 false)"
                ));
            }
        }
    }

    fn emit_cmp_call_operand(
        &mut self,
        val: &ExprValue,
        param_ty: &str,
    ) -> (String, String) {
        if param_ty.ends_with('*') {
            let struct_ty = param_ty.trim_end_matches('*');
            if val.ty == struct_ty {
                let slot = self.fresh("cmp.arg");
                self.emit(&format!("  %{slot} = alloca {struct_ty}"));
                self.emit(&format!(
                    "  store {struct_ty} {}, {struct_ty}* %{slot}",
                    val.reg
                ));
                return (format!("%{slot}"), param_ty.to_string());
            }
        }
        (val.reg.clone(), param_ty.to_string())
    }

    fn emit_sort_by_cmp_call(
        &mut self,
        elem: &str,
        left: &ExprValue,
        right: &ExprValue,
        callee_sym: &str,
        closure: Option<&ClosureMeta>,
    ) -> String {
        let reg = self.fresh("sort.cmp");
        if let Some(meta) = closure {
            self.store_closure_invoke_slot(meta);
            let env_reg = self.closure_env_reg(meta);
            let mut parts = vec![format!("ptr {env_reg}")];
            for (i, v) in [left, right].iter().enumerate() {
                let pty = meta
                    .param_tys
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or(elem);
                let (r, t) = self.emit_cmp_call_operand(v, pty);
                parts.push(format!("{t} {r}"));
            }
            self.emit(&format!(
                "  %{reg} = call i32 @{}({})",
                meta.body_symbol,
                parts.join(", ")
            ));
            if matches!(meta.env_kind, EnvKind::Stack { .. }) {
                self.emit(&format!("  store ptr null, ptr @{}", meta.invoke_slot));
            }
        } else {
            let callee_name = callee_sym.trim_start_matches('@');
            let param_tys = self.fn_param_llvm_types(callee_name);
            let mut parts = Vec::new();
            for (i, v) in [left, right].iter().enumerate() {
                let pty = param_tys
                    .as_ref()
                    .and_then(|ts| ts.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or(elem);
                let (r, t) = self.emit_cmp_call_operand(v, pty);
                parts.push(format!("{t} {r}"));
            }
            self.emit(&format!(
                "  %{reg} = call i32 {callee_sym}({})",
                parts.join(", ")
            ));
        }
        reg
    }

    pub(super) fn compile_array_sort_by(
        &mut self,
        mc: &MethodCallExpr,
        env: &Env,
    ) -> ExprValue {
        let obj = self.compile_expr(&mc.object, env);
        let n = array_len_from_ty(&obj.ty).expect("typechecked") as i32;
        let elem = array_elem_from_ty(&obj.ty).unwrap_or_else(|| "i32".into());
        let arr_ty = obj.ty.clone();

        self.closure_force_heap = false;
        let cmp_expr = &mc.args[0];
        let (callee, closure_meta) = if let Expression::Variable { name, .. } = cmp_expr {
            if let Some(Binding::Closure(meta)) = env.get(name) {
                (String::new(), Some(meta.clone()))
            } else {
                let v = self.compile_expr(cmp_expr, env);
                let meta = self.pending_closure_meta.take();
                if meta.is_some() {
                    (String::new(), meta)
                } else {
                    (v.reg, None)
                }
            }
        } else {
            let v = self.compile_expr(cmp_expr, env);
            let meta = self.pending_closure_meta.take();
            if meta.is_some() {
                (String::new(), meta)
            } else {
                (v.reg, None)
            }
        };
        self.closure_force_heap = false;

        let closure_ref = closure_meta.as_ref();
        let callee_sym = if closure_ref.is_some() {
            String::new()
        } else if callee.starts_with('@') {
            callee.clone()
        } else {
            format!("@{}", callee.trim_start_matches('%'))
        };

        let src_ptr = self.materialize_array_ptr(&obj);
        let dst = self.fresh("sort.dst");
        self.emit(&format!("  %{dst} = alloca {arr_ty}"));
        let bytes = self.llvm_elem_byte_size(&elem) * n as i64;
        self.emit(&format!(
            "  call void @llvm.memcpy.p0.p0.i64(ptr %{dst}, ptr {src_ptr}, i64 {bytes}, i1 false)"
        ));

        let i_slot = self.fresh("sort.i");
        let j_slot = self.fresh("sort.j");
        self.emit(&format!("  %{i_slot} = alloca i32"));
        self.emit(&format!("  %{j_slot} = alloca i32"));
        self.emit(&format!("  store i32 1, i32* %{i_slot}"));

        let i_hdr = self.fresh_label("sort.i.hdr");
        let i_body = self.fresh_label("sort.i.body");
        let i_end = self.fresh_label("sort.i.end");
        self.emit(&format!("  br label %{i_hdr}"));
        self.emit_block_label(&i_hdr);
        let i_val = self.fresh("sort.i.v");
        self.emit(&format!("  %{i_val} = load i32, i32* %{i_slot}"));
        let i_cond = self.fresh("sort.i.c");
        self.emit(&format!("  %{i_cond} = icmp slt i32 %{i_val}, {n}"));
        self.emit(&format!(
            "  br i1 %{i_cond}, label %{i_body}, label %{i_end}"
        ));

        self.emit_block_label(&i_body);
        self.emit(&format!("  store i32 %{i_val}, i32* %{j_slot}"));
        let j_hdr = self.fresh_label("sort.j.hdr");
        let j_body = self.fresh_label("sort.j.body");
        let j_next = self.fresh_label("sort.j.next");
        let j_end = self.fresh_label("sort.j.end");
        self.emit(&format!("  br label %{j_hdr}"));

        self.emit_block_label(&j_hdr);
        let j_val = self.fresh("sort.j.v");
        self.emit(&format!("  %{j_val} = load i32, i32* %{j_slot}"));
        let j_zero = self.fresh("sort.j.z");
        self.emit(&format!("  %{j_zero} = icmp sgt i32 %{j_val}, 0"));
        self.emit(&format!(
            "  br i1 %{j_zero}, label %{j_body}, label %{j_end}"
        ));

        self.emit_block_label(&j_body);
        let j_prev = self.fresh("sort.jp");
        self.emit(&format!("  %{j_prev} = sub i32 %{j_val}, 1"));
        let left = self.load_array_elem_value(&arr_ty, &format!("%{dst}"), &j_prev);
        let right = self.load_array_elem_value(&arr_ty, &format!("%{dst}"), &j_val);
        let cmp_reg = self.emit_sort_by_cmp_call(&elem, &left, &right, &callee_sym, closure_ref);
        let need_swap = self.fresh("sort.swap");
        self.emit(&format!("  %{need_swap} = icmp sgt i32 %{cmp_reg}, 0"));
        self.emit(&format!(
            "  br i1 %{need_swap}, label %{j_next}, label %{j_end}"
        ));

        self.emit_block_label(&j_next);
        self.emit_array_elem_swap(&arr_ty, &format!("%{dst}"), &elem, &j_prev, &j_val);
        let j_dec = self.fresh("sort.jd");
        self.emit(&format!("  %{j_dec} = sub i32 %{j_val}, 1"));
        self.emit(&format!("  store i32 %{j_dec}, i32* %{j_slot}"));
        self.emit(&format!("  br label %{j_hdr}"));

        self.emit_block_label(&j_end);
        let i_inc = self.fresh("sort.ii");
        self.emit(&format!("  %{i_inc} = add i32 %{i_val}, 1"));
        self.emit(&format!("  store i32 %{i_inc}, i32* %{i_slot}"));
        self.emit(&format!("  br label %{i_hdr}"));

        self.emit_block_label(&i_end);

        ExprValue {
            reg: dst,
            ty: arr_ty,
        }
    }
}

