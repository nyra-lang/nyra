#![allow(unused_imports)]
//! `spawn` async tasks and nested function codegen scopes.
use std::collections::{BTreeSet, HashMap, HashSet};

use ast::*;
use ownership::{collect_captures, DropPlan, EscapePlan, EscapeState};

use super::{
    Binding, ClosureMeta, Codegen, DropState, Env, EnvKind, ExprValue, NestedFnCodegenScope,
};
use super::util::{
    is_struct_pointer_type, llvm_ptr, llvm_storage_ty, llvm_struct_size_bytes,
    llvm_value_operand, struct_ptr_type,
};

impl Codegen {
    pub(super) fn compile_spawn(
        &mut self,
        kind: ast::SpawnKind,
        body: &Block,
        env: &Env,
        drop_state: &mut DropState,
    ) -> ExprValue {
        self.compile_spawn_inner(kind, body, env, drop_state)
    }

    pub(super) fn emit_spawn_handle_drop(&mut self, reg: &str, kind: ast::SpawnKind) {
        let ptr = if reg.starts_with('%') {
            reg.to_string()
        } else {
            format!("%{reg}")
        };
        let (rt_fn, decl) = match kind {
            ast::SpawnKind::Task => (
                "spawn_task_handle_drop",
                "declare void @spawn_task_handle_drop(ptr)",
            ),
            ast::SpawnKind::Thread => (
                "spawn_handle_drop",
                "declare void @spawn_handle_drop(ptr)",
            ),
        };
        self.ensure_runtime_fn_decl(rt_fn, decl);
        self.emit_runtime_call(
            rt_fn,
            &format!("  call void @{rt_fn}(ptr {ptr})"),
        );
    }

    pub(super) fn emit_spawn_join(&mut self, reg: &str, kind: ast::SpawnKind) {
        let ptr = if reg.starts_with('%') {
            reg.to_string()
        } else {
            format!("%{reg}")
        };
        let (rt_fn, decl) = match kind {
            ast::SpawnKind::Task => (
                "spawn_task_join",
                "declare i32 @spawn_task_join(ptr)",
            ),
            ast::SpawnKind::Thread => ("spawn_join", "declare i32 @spawn_join(ptr)"),
        };
        self.ensure_runtime_fn_decl(rt_fn, decl);
        let tmp = self.fresh("spawn.join");
        self.emit_runtime_call(
            rt_fn,
            &format!("  %{tmp} = call i32 @{rt_fn}(ptr {ptr})"),
        );
    }

    fn compile_spawn_inner(
        &mut self,
        kind: ast::SpawnKind,
        body: &Block,
        env: &Env,
        drop_state: &mut DropState,
    ) -> ExprValue {
        let outer: HashSet<String> = env.keys().cloned().collect();
        let capture_names = collect_captures(body, &outer);
        let spawn_fn = drop_state.next_spawn_key();
        let spawn_idx = drop_state.spawn_id - 1;

        let mut fields: Vec<(String, String)> = Vec::new();
        for name in &capture_names {
            if let Some(binding) = env.get(name) {
                fields.push((name.clone(), Self::binding_ty(binding).to_string()));
            }
        }

        let safe_func = drop_state.func.replace('.', "_");
        let body_symbol = format!("__spawn_{safe_func}_{spawn_idx}");
        let cap_ty_name = format!("SpawnCap.{safe_func}.{spawn_idx}");

        let (capture_fn, capture_decl) = match kind {
            ast::SpawnKind::Task => (
                "spawn_task_capture",
                "declare ptr @spawn_task_capture(ptr, ptr, i64)",
            ),
            ast::SpawnKind::Thread => (
                "spawn_capture",
                "declare ptr @spawn_capture(ptr, ptr, i64)",
            ),
        };
        self.ensure_runtime_fn_decl(capture_fn, capture_decl);

        let reg = self.fresh("spawn.handle");

        if fields.is_empty() {
            self.emit_spawn_body_fn(&body_symbol, &spawn_fn, body, &cap_ty_name, &[]);
            self.emit_runtime_call(
                capture_fn,
                &format!(
                    "  %{reg} = call ptr @{capture_fn}(ptr @{body_symbol}, ptr null, i64 0)"
                ),
            );
        } else {
            let llvm_fields: Vec<String> = fields.iter().map(|(_, ty)| ty.clone()).collect();
            self.emit_module(&format!(
                "%{cap_ty_name} = type {{ {} }}",
                llvm_fields.join(", ")
            ));

            self.emit_spawn_body_fn(
                &body_symbol,
                &spawn_fn,
                body,
                &cap_ty_name,
                &fields,
            );

            let cap_alloca = self.fresh("spawn.cap");
            self.emit(&format!("  %{cap_alloca} = alloca %{cap_ty_name}"));
            for (i, (name, ty)) in fields.iter().enumerate() {
                let val_reg = self.load_binding_for_spawn(name, ty, env);
                let gep = self.fresh("spawn.gep");
                self.emit(&format!(
                    "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_alloca}, i64 0, i32 {i}"
                ));
                self.emit(&format!(
                    "  store {ty} {}, {} %{gep}",
                    llvm_value_operand(&val_reg),
                    llvm_ptr(ty)
                ));
                if self.drop_plan.is_owned_in(&drop_state.func, name) {
                    drop_state.mark_moved(name);
                }
            }

            let size = llvm_struct_size_bytes(&llvm_fields);
            let heap = self.fresh("spawn.heap");
            self.needs_malloc_decl = true;
            self.emit(&format!("  %{heap} = call ptr @malloc(i64 {size})"));
            self.emit(&format!(
                "  call void @llvm.memcpy.p0.p0.i64(ptr %{heap}, ptr %{cap_alloca}, i64 {size}, i1 false)"
            ));
            self.emit_runtime_call(
                capture_fn,
                &format!(
                    "  %{reg} = call ptr @{capture_fn}(ptr @{body_symbol}, ptr %{heap}, i64 {size})"
                ),
            );
        }

        ExprValue {
            reg: format!("%{reg}"),
            ty: "join_handle".into(),
        }
    }

    pub(super) fn push_nested_fn_codegen_scope(&mut self) -> NestedFnCodegenScope {
        NestedFnCodegenScope {
            current_block: std::mem::replace(&mut self.current_block, "entry".into()),
            loop_stack: std::mem::take(&mut self.loop_stack),
            mut_ssa_locals: std::mem::take(&mut self.mut_ssa_locals),
        }
    }

    pub(super) fn pop_nested_fn_codegen_scope(&mut self, saved: NestedFnCodegenScope) {
        self.current_block = saved.current_block;
        self.loop_stack = saved.loop_stack;
        self.mut_ssa_locals = saved.mut_ssa_locals;
    }

    pub(super) fn emit_spawn_body_fn(
        &mut self,
        symbol: &str,
        drop_func: &str,
        body: &Block,
        cap_ty_name: &str,
        captures: &[(String, String)],
    ) {
        let saved_emit_buf = self.emit_buf.take();
        self.emit_buf = Some(Vec::new());
        let nested_scope = self.push_nested_fn_codegen_scope();
        self.emit(&format!("define void @{symbol}(ptr %raw) {{"));
        self.emit("entry:");
        let mut env: Env = HashMap::new();
        if !captures.is_empty() {
            let cap_ptr = self.fresh("cap.bc");
            self.emit(&format!(
                "  %{cap_ptr} = bitcast ptr %raw to %{cap_ty_name}*"
            ));
            for (i, (name, ty)) in captures.iter().enumerate() {
                let gep = self.fresh("cap.fld");
                self.emit(&format!(
                    "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {i}"
                ));
                if is_struct_pointer_type(ty) {
                    let loaded = self.fresh("cap.ld");
                    self.emit(&format!(
                        "  %{loaded} = load {ty}, {} %{gep}",
                        llvm_ptr(ty)
                    ));
                    env.insert(
                        name.clone(),
                        Binding::Reg {
                            reg: loaded,
                            ty: ty.clone(),
                        },
                    );
                } else {
                    let alloca = self.fresh("cap.local");
                    self.emit(&format!("  %{alloca} = alloca {ty}"));
                    let loaded = self.fresh("cap.ld");
                    self.emit(&format!(
                        "  %{loaded} = load {ty}, {} %{gep}",
                        llvm_ptr(ty)
                    ));
                    self.emit(&format!(
                        "  store {ty} %{loaded}, {} %{alloca}",
                        llvm_ptr(ty)
                    ));
                    env.insert(
                        name.clone(),
                        Binding::Stack {
                            slot: alloca,
                            ty: ty.clone(),
                        },
                    );
                }
            }
        }
        let mut spawn_drop = DropState::new(drop_func);
        let _ = self.compile_block(body, &mut env, "void", &mut spawn_drop);
        self.emit("  ret void");
        self.emit("}");
        self.pop_nested_fn_codegen_scope(nested_scope);
        if let Some(helper) = self.emit_buf.take() {
            self.module_level.extend(helper);
        }
        self.emit_buf = saved_emit_buf;
    }

    pub(super) fn load_binding_for_spawn(
        &mut self,
        name: &str,
        ty: &str,
        env: &Env,
    ) -> String {
        let binding = env.get(name).expect("capture in env");
        if is_struct_pointer_type(ty) {
            match binding {
                Binding::Param { index, .. } => return index.to_string(),
                Binding::Reg { reg, .. } => {
                    if reg.chars().all(|c| c.is_ascii_digit()) {
                        return reg.clone();
                    }
                    return reg.trim_start_matches('%').to_string();
                }
                Binding::Stack { slot, ty: stack_ty } => {
                    let loaded = self.fresh("spawn.ld");
                    self.emit(&format!(
                        "  %{loaded} = load {stack_ty}, {} %{slot}",
                        llvm_ptr(stack_ty)
                    ));
                    return loaded;
                }
                Binding::Closure(meta) => return match &meta.env_kind {
                    EnvKind::Stack { alloca } => alloca.clone(),
                    EnvKind::Heap { global } => {
                        let loaded = self.fresh("spawn.env");
                        self.emit(&format!(
                            "  %{loaded} = load ptr, ptr @{global}"
                        ));
                        loaded
                    }
                },
                Binding::PromotedStruct {
                    struct_name,
                    fields,
                    ..
                } => {
                    let mat = self.materialize_promoted_struct(struct_name, fields);
                    return mat.reg;
                }
                Binding::LocalChannel { slot } => return slot.clone(),
            }
        }
        match binding {
            Binding::Param { index, .. } => format!("%{index}"),
            _ => {
                let (loaded, _) = self.binding_load(binding);
                loaded
            }
        }
    }
}
