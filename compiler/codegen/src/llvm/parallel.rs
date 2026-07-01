#![allow(unused_imports)]
//! `parallel for` — lowers to `parallel_for_range` with a per-iteration worker function.
use std::collections::{HashMap, HashSet};

use ast::*;

use super::{
    Binding, Codegen, DropState, Env, NestedFnCodegenScope,
};
use super::util::{
    array_len_from_ty, is_struct_pointer_type, llvm_ptr,
};

enum ParIterPlan {
    Range {
        var: String,
        start_reg: String,
        end_reg: String,
    },
    Array {
        var: String,
        arr_ty: String,
        elem_ty: String,
        arr_ptr: String,
        len: String,
    },
    VecStr {
        var: String,
        vec_reg: String,
        len_reg: String,
    },
    StringChars {
        var: String,
        str_reg: String,
        len_reg: String,
    },
}

impl Codegen {
    pub(super) fn compile_parallel_for(
        &mut self,
        f: &ForStmt,
        env: &Env,
        ret_ty: &str,
        drop_state: &mut DropState,
    ) {
        let par_idx = drop_state.next_par_idx();
        let safe_func = drop_state.func.replace('.', "_");
        let body_symbol = format!("__par_{safe_func}_{par_idx}");
        let cap_ty_name = format!("ParCtx.{safe_func}.{par_idx}");

        let outer: HashSet<String> = env.keys().cloned().collect();
        let mut declared = outer.clone();
        declared.insert(f.var.clone());
        let capture_names: Vec<String> = ownership::collect_captures(&f.body, &declared)
            .into_iter()
            .filter(|n| *n != f.var)
            .collect();

        let mut fields: Vec<(String, String)> = Vec::new();
        for name in &capture_names {
            if let Some(binding) = env.get(name) {
                fields.push((name.clone(), Self::binding_ty(binding).to_string()));
            }
        }

        let plan = match &f.kind {
            ForKind::Range { start, end } => {
                let start_v = self.compile_expr(start, env);
                let end_v = self.compile_expr(end, env);
                ParIterPlan::Range {
                    var: f.var.clone(),
                    start_reg: start_v.reg,
                    end_reg: end_v.reg,
                }
            }
            ForKind::Iterable { iterable } => {
                let collection = self.compile_expr(iterable, env);
                if let Some(n) = array_len_from_ty(&collection.ty) {
                    let elem_ty = collection
                        .ty
                        .strip_prefix('[')
                        .and_then(|inner| inner.split(" x ").nth(1))
                        .and_then(|s| s.strip_suffix(']'))
                        .unwrap_or("i32")
                        .to_string();
                    let arr_ptr = self.materialize_array_ptr(&collection);
                    fields.push(("__iter_arr".into(), format!("{}*", collection.ty)));
                    ParIterPlan::Array {
                        var: f.var.clone(),
                        arr_ty: collection.ty.clone(),
                        elem_ty,
                        arr_ptr,
                        len: n.to_string(),
                    }
                } else if collection.ty == "vec_str" {
                    let len_reg = self.fresh("vec_strlen");
                    let vec_reg = if collection.reg.starts_with('%') {
                        collection.reg.clone()
                    } else {
                        format!("%{}", collection.reg)
                    };
                    self.emit_runtime_call(
                        "vec_str_len",
                        &format!("  %{len_reg} = call i32 @vec_str_len(ptr {vec_reg})"),
                    );
                    fields.push(("__iter_vec".into(), "ptr".into()));
                    ParIterPlan::VecStr {
                        var: f.var.clone(),
                        vec_reg,
                        len_reg,
                    }
                } else if collection.ty == "ptr" {
                    let len_reg = self.fresh("strlen");
                    let str_reg = if collection.reg.starts_with('%') {
                        collection.reg.clone()
                    } else {
                        format!("%{}", collection.reg)
                    };
                    self.emit_runtime_call(
                        "strlen",
                        &format!("  %{len_reg} = call i32 @strlen(ptr {str_reg})"),
                    );
                    fields.push(("__iter_str".into(), "ptr".into()));
                    ParIterPlan::StringChars {
                        var: f.var.clone(),
                        str_reg,
                        len_reg,
                    }
                } else {
                    return;
                }
            }
        };

        let cfg = f.parallel.as_ref().expect("parallel for");
        let (max_w, exact_w, mode, cpu_pct, backend) = self.compile_parallel_opts(cfg, env);

        let llvm_fields: Vec<String> = fields.iter().map(|(_, ty)| ty.clone()).collect();
        if !llvm_fields.is_empty() {
            self.emit_module(&format!(
                "%{cap_ty_name} = type {{ {} }}",
                llvm_fields.join(", ")
            ));
        }

        self.emit_parallel_body_fn(
            &body_symbol,
            &format!("{}__par_{}", drop_state.func, par_idx),
            &f.body,
            &cap_ty_name,
            &fields,
            &plan,
            ret_ty,
        );

        let (start_op, end_op) = match &plan {
            ParIterPlan::Range { start_reg, end_reg, .. } => {
                (start_reg.clone(), end_reg.clone())
            }
            ParIterPlan::Array { len, .. } => ("0".into(), len.clone()),
            ParIterPlan::VecStr { len_reg, .. } | ParIterPlan::StringChars { len_reg, .. } => {
                ("0".into(), format!("%{len_reg}"))
            }
        };

        if llvm_fields.is_empty() {
            self.emit_runtime_call(
                "parallel_for_range",
                &format!(
                    "  call void @parallel_for_range(i32 {start_op}, i32 {end_op}, ptr @{body_symbol}, ptr null, i32 {max_w}, i32 {exact_w}, i32 {mode}, i32 {cpu_pct}, i32 {backend})"
                ),
            );
            return;
        }

        let cap_alloca = self.fresh("par.ctx");
        self.emit(&format!("  %{cap_alloca} = alloca %{cap_ty_name}"));
        for (i, (name, ty)) in fields.iter().enumerate() {
            let val_reg = if name == "__iter_arr" {
                let ParIterPlan::Array { arr_ptr, .. } = &plan else {
                    continue;
                };
                if arr_ptr.starts_with('%') {
                    arr_ptr.clone()
                } else {
                    format!("%{arr_ptr}")
                }
            } else if name == "__iter_vec" {
                let ParIterPlan::VecStr { vec_reg, .. } = &plan else {
                    continue;
                };
                if vec_reg.starts_with('%') {
                    vec_reg.clone()
                } else {
                    format!("%{vec_reg}")
                }
            } else if name == "__iter_str" {
                let ParIterPlan::StringChars { str_reg, .. } = &plan else {
                    continue;
                };
                if str_reg.starts_with('%') {
                    str_reg.clone()
                } else {
                    format!("%{str_reg}")
                }
            } else {
                self.load_binding_for_spawn(name, ty, env)
            };
            let gep = self.fresh("par.gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_alloca}, i64 0, i32 {i}"
            ));
            let op = if val_reg.starts_with('%') {
                val_reg.clone()
            } else {
                format!("%{val_reg}")
            };
            self.emit(&format!(
                "  store {ty} {op}, {} %{gep}",
                llvm_ptr(ty)
            ));
            if !name.starts_with("__iter_")
                && self.drop_plan.is_owned_in(&drop_state.func, name)
            {
                drop_state.mark_moved(name);
            }
        }

        let ctx_ptr = self.fresh("par.ctx.ptr");
        self.emit(&format!(
            "  %{ctx_ptr} = bitcast %{cap_ty_name}* %{cap_alloca} to ptr"
        ));
        self.emit_runtime_call(
            "parallel_for_range",
            &format!(
                "  call void @parallel_for_range(i32 {start_op}, i32 {end_op}, ptr @{body_symbol}, ptr %{ctx_ptr}, i32 {max_w}, i32 {exact_w}, i32 {mode}, i32 {cpu_pct}, i32 {backend})"
            ),
        );
    }

    fn compile_parallel_opts(
        &mut self,
        cfg: &ParallelConfig,
        env: &Env,
    ) -> (String, String, String, String, String) {
        let mode = match cfg.mode {
            ParallelMode::Auto => "0".to_string(),
            ParallelMode::Balanced => "1".to_string(),
            ParallelMode::MaxPerformance => "2".to_string(),
            ParallelMode::Background => "3".to_string(),
        };
        let backend = match cfg.kind {
            SpawnKind::Task => "0".to_string(),
            SpawnKind::Thread => "1".to_string(),
        };
        let mut max_w = "0".to_string();
        let mut exact_w = "0".to_string();
        let mut cpu_pct = "0".to_string();
        match &cfg.threads {
            ParallelThreads::Auto => {}
            ParallelThreads::Max(e) => max_w = Self::i32_operand(&self.compile_expr(e, env).reg),
            ParallelThreads::Exact(e) => exact_w = Self::i32_operand(&self.compile_expr(e, env).reg),
            ParallelThreads::CpuPercent(e) => {
                cpu_pct = Self::i32_operand(&self.compile_expr(e, env).reg);
            }
        }
        (max_w, exact_w, mode, cpu_pct, backend)
    }

    fn i32_operand(reg: &str) -> String {
        if reg.starts_with('%') || reg.chars().all(|c| c.is_ascii_digit() || c == '-') {
            reg.to_string()
        } else {
            format!("%{reg}")
        }
    }

    fn emit_parallel_body_fn(
        &mut self,
        symbol: &str,
        drop_func: &str,
        body: &Block,
        cap_ty_name: &str,
        captures: &[(String, String)],
        plan: &ParIterPlan,
        ret_ty: &str,
    ) {
        self.emit_buf = Some(Vec::new());
        let nested_scope = self.push_nested_fn_codegen_scope();
        self.emit(&format!("define void @{symbol}(i32 %idx, ptr %raw) {{"));
        self.emit("entry:");
        let mut env: Env = HashMap::new();
        let cap_ptr = if !captures.is_empty() {
            let cap_ptr = self.fresh("par.cap.bc");
            self.emit(&format!(
                "  %{cap_ptr} = bitcast ptr %raw to %{cap_ty_name}*"
            ));
            for (i, (name, ty)) in captures.iter().enumerate() {
                if name.starts_with("__iter_") {
                    continue;
                }
                let gep = self.fresh("par.cap.fld");
                self.emit(&format!(
                    "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {i}"
                ));
                self.bind_capture_field(&mut env, name, ty, &gep);
            }
            Some(cap_ptr)
        } else {
            None
        };

        match plan {
            ParIterPlan::Range { var, .. } => {
                env.insert(
                    var.clone(),
                    Binding::Reg {
                        reg: "idx".into(),
                        ty: "i32".into(),
                    },
                );
            }
            ParIterPlan::Array {
                var,
                arr_ty,
                elem_ty,
                ..
            } => {
                let cap_ptr = cap_ptr.as_ref().expect("iter ctx");
                let iter_gep = self.fresh("par.iter.gep");
                let iter_idx = captures
                    .iter()
                    .position(|(n, _)| n == "__iter_arr")
                    .unwrap_or(captures.len().saturating_sub(1));
                self.emit(&format!(
                    "  %{iter_gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {iter_idx}"
                ));
                let arr_loaded = self.fresh("par.arr.ld");
                self.emit(&format!(
                    "  %{arr_loaded} = load {arr_ty}*, {} %{iter_gep}",
                    llvm_ptr(&format!("{arr_ty}*"))
                ));
                let elem_gep = self.fresh("par.elem.gep");
                self.emit(&format!(
                    "  %{elem_gep} = getelementptr inbounds {arr_ty}, {arr_ty}* %{arr_loaded}, i32 0, i32 %idx"
                ));
                let loaded = self.fresh("par.elem.ld");
                self.emit(&format!(
                    "  %{loaded} = load {elem_ty}, {} %{elem_gep}",
                    llvm_ptr(elem_ty)
                ));
                let var_alloca = self.fresh("par.var");
                self.emit(&format!("  %{var_alloca} = alloca {elem_ty}"));
                self.emit(&format!(
                    "  store {elem_ty} %{loaded}, {} %{var_alloca}",
                    llvm_ptr(elem_ty)
                ));
                env.insert(
                    var.clone(),
                    Binding::Stack {
                        slot: var_alloca,
                        ty: elem_ty.clone(),
                    },
                );
            }
            ParIterPlan::VecStr { var, .. } => {
                let cap_ptr = cap_ptr.as_ref().expect("iter ctx");
                let iter_gep = self.fresh("par.iter.gep");
                let iter_idx = captures
                    .iter()
                    .position(|(n, _)| n == "__iter_vec")
                    .unwrap_or(0);
                self.emit(&format!(
                    "  %{iter_gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {iter_idx}"
                ));
                let vec_loaded = self.fresh("par.vec.ld");
                self.emit(&format!(
                    "  %{vec_loaded} = load ptr, {} %{iter_gep}",
                    llvm_ptr("ptr")
                ));
                let part = self.fresh("par.vec.get");
                self.emit_runtime_call(
                    "vec_str_get",
                    &format!(
                        "  %{part} = call ptr @vec_str_get(ptr %{vec_loaded}, i32 %idx)"
                    ),
                );
                let var_alloca = self.fresh("par.var");
                self.emit(&format!("  %{var_alloca} = alloca ptr"));
                self.emit(&format!("  store ptr %{part}, ptr %{var_alloca}"));
                env.insert(
                    var.clone(),
                    Binding::Stack {
                        slot: var_alloca,
                        ty: "ptr".into(),
                    },
                );
            }
            ParIterPlan::StringChars { var, .. } => {
                let cap_ptr = cap_ptr.as_ref().expect("iter ctx");
                let iter_gep = self.fresh("par.iter.gep");
                let iter_idx = captures
                    .iter()
                    .position(|(n, _)| n == "__iter_str")
                    .unwrap_or(0);
                self.emit(&format!(
                    "  %{iter_gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {iter_idx}"
                ));
                let str_loaded = self.fresh("par.str.ld");
                self.emit(&format!(
                    "  %{str_loaded} = load ptr, {} %{iter_gep}",
                    llvm_ptr("ptr")
                ));
                let ch = self.fresh("par.char");
                self.emit_runtime_call(
                    "char_at",
                    &format!("  %{ch} = call i32 @char_at(ptr %{str_loaded}, i32 %idx)"),
                );
                let var_alloca = self.fresh("par.var");
                self.emit(&format!("  %{var_alloca} = alloca i32"));
                self.emit(&format!("  store i32 %{ch}, {} %{var_alloca}", llvm_ptr("i32")));
                env.insert(
                    var.clone(),
                    Binding::Stack {
                        slot: var_alloca,
                        ty: "i32".into(),
                    },
                );
            }
        }

        let mut par_drop = DropState::new(drop_func);
        let _ = self.compile_block(body, &mut env, ret_ty, &mut par_drop);
        self.emit("  ret void");
        self.emit("}");
        self.pop_nested_fn_codegen_scope(nested_scope);
        if let Some(helper) = self.emit_buf.take() {
            self.module_level.extend(helper);
        }
    }

    fn bind_capture_field(&mut self, env: &mut Env, name: &str, ty: &str, gep: &str) {
        if is_struct_pointer_type(ty) {
            let loaded = self.fresh("par.cap.ld");
            self.emit(&format!(
                "  %{loaded} = load {ty}, {} %{gep}",
                llvm_ptr(ty)
            ));
            env.insert(
                name.to_string(),
                Binding::Reg {
                    reg: loaded,
                    ty: ty.to_string(),
                },
            );
        } else {
            let alloca = self.fresh("par.cap.local");
            self.emit(&format!("  %{alloca} = alloca {ty}"));
            let loaded = self.fresh("par.cap.ld");
            self.emit(&format!(
                "  %{loaded} = load {ty}, {} %{gep}",
                llvm_ptr(ty)
            ));
            self.emit(&format!(
                "  store {ty} %{loaded}, {} %{alloca}",
                llvm_ptr(ty)
            ));
            env.insert(
                name.to_string(),
                Binding::Stack {
                    slot: alloca,
                    ty: ty.to_string(),
                },
            );
        }
    }
}
