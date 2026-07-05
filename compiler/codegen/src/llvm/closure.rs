#![allow(unused_imports)]
//! Capturing closures and arrow-function lowering.
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
    escape_string, host_target_triple, is_string_builtin_method, llvm_arith_rhs, llvm_binop_operand,
    llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len,
    llvm_value_operand,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn infer_block_return_ann(&self, block: &Block) -> TypeAnnotation {
        for stmt in block.statements.iter().rev() {
            if let Statement::Return(r) = stmt {
                if let Some(expr) = &r.value {
                    return self.infer_expr_return_ann(expr);
                }
                return TypeAnnotation::Void;
            }
        }
        TypeAnnotation::Integer(ast::IntKind::I32)
    }

    pub(super) fn infer_expr_return_ann(&self, expr: &Expression) -> TypeAnnotation {
        match expr {
            Expression::Literal(Literal::Int(_)) => TypeAnnotation::Integer(ast::IntKind::I32),
            Expression::Literal(Literal::Float(_, _)) => TypeAnnotation::F64,
            Expression::Literal(Literal::Char(_)) => TypeAnnotation::Char,
            Expression::Literal(Literal::Bool(_)) => TypeAnnotation::Bool,
            Expression::Literal(Literal::String(_)) => TypeAnnotation::String,
            Expression::Call(c) => {
                if let Some(ret) = self.call_returns.get(&c.callee) {
                    llvm_ty_to_ann(ret)
                } else if let Some(func) = self.functions.get(&c.callee) {
                    func.return_type
                        .clone()
                        .unwrap_or_else(|| self.infer_block_return_ann(&func.body))
                } else {
                    TypeAnnotation::Integer(ast::IntKind::I32)
                }
            }
            Expression::If(_) => TypeAnnotation::Integer(ast::IntKind::I32),
            Expression::Binary(_) => TypeAnnotation::Integer(ast::IntKind::I32),
            Expression::Variable { name, .. } => {
                if let Some(func) = self.functions.get(name) {
                    func.return_type
                        .clone()
                        .unwrap_or_else(|| self.infer_block_return_ann(&func.body))
                } else {
                    TypeAnnotation::Integer(ast::IntKind::I32)
                }
            }
            Expression::StructLiteral(sl) => TypeAnnotation::Struct(sl.name.clone()),
            Expression::EnumVariant(ev) => {
                if let Some(ref name) = ev.enum_name {
                    TypeAnnotation::Enum(name.clone())
                } else {
                    TypeAnnotation::Integer(ast::IntKind::I32)
                }
            }
            Expression::Cast(c) => c.target_type.clone(),
            Expression::Grouped(e) => self.infer_expr_return_ann(e),
            _ => TypeAnnotation::Integer(ast::IntKind::I32),
        }
    }

    pub(super) fn infer_arrow_return_ann(&self, arrow: &ArrowFnExpr) -> TypeAnnotation {
        let block = arrow_to_block(arrow);
        let Some(Statement::Return(r)) = block.statements.last() else {
            return TypeAnnotation::Integer(ast::IntKind::I32);
        };
        let Some(expr) = &r.value else {
            return TypeAnnotation::Void;
        };
        self.infer_expr_return_ann(expr)
    }

    pub(super) fn infer_arrow_ret_llvm(&self, arrow: &ArrowFnExpr) -> String {
        self.llvm_type_of(&self.infer_arrow_return_ann(arrow))
    }

    pub(super) fn emit_closure_wrap(
        &mut self,
        wrap_symbol: &str,
        body_symbol: &str,
        env_source: &str,
        use_invoke_slot: bool,
        param_tys: &[String],
        ret_ty: &str,
    ) {
        let user_params: Vec<String> = param_tys
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{t} %{i}"))
            .collect();
        let args: Vec<String> = std::iter::once("ptr %slot".to_string())
            .chain(user_params.iter().cloned())
            .collect();
        if use_invoke_slot {
            self.emit_module(&format!("@{env_source} = internal global ptr null"));
        }
        let slot_load = format!("  %slot = load ptr, ptr @{env_source}");
        self.emit_module(&format!(
            "define {ret_ty} @{wrap_symbol}({}) {{\nentry:\n{slot_load}\n  %ret = call {ret_ty} @{body_symbol}({})\n  ret {ret_ty} %ret\n}}",
            user_params.join(", "),
            args.join(", ")
        ));
    }

    pub(super) fn closure_env_reg(&mut self, meta: &ClosureMeta) -> String {
        match &meta.env_kind {
            EnvKind::Stack { alloca } => format!("%{alloca}"),
            EnvKind::Heap { global } => {
                let loaded = self.fresh("closure.env");
                self.emit(&format!(
                    "  %{loaded} = load ptr, ptr @{global}"
                ));
                format!("%{loaded}")
            }
        }
    }

    pub(super) fn emit_closure_env_free(&mut self, meta: &ClosureMeta) {
        if !meta.heap_owned {
            return;
        }
        if let EnvKind::Heap { global } = &meta.env_kind {
            let loaded = self.fresh("closure.free");
            self.emit(&format!("  %{loaded} = load ptr, ptr @{global}"));
            self.needs_malloc_decl = true;
            self.emit(&format!("  call void @free(ptr %{loaded})"));
        }
    }

    pub(super) fn compile_arrow_fn(
        &mut self,
        arrow: &ArrowFnExpr,
        env: &Env,
        drop_state: &mut DropState,
        force_heap: bool,
    ) -> ExprValue {
        let closure_idx = drop_state.next_closure_idx();
        let safe_func = drop_state.func.replace('.', "_");
        let symbol = format!("__closure_{safe_func}_{closure_idx}");
        let wrap_symbol = format!("__closure_wrap_{safe_func}_{closure_idx}");
        let invoke_slot = format!("__closure_invoke_slot_{safe_func}_{closure_idx}");
        let param_tys: Vec<String> = arrow
            .params
            .iter()
            .map(|p| self.llvm_param_type_of(&p.ty))
            .collect();
        let block = arrow_to_block(arrow);
        let ret_ty = self.infer_arrow_ret_llvm(arrow);

        if !arrow_has_captures(arrow) {
            self.emit_arrow_body_fn(
                &symbol,
                arrow,
                &block,
                &param_tys,
                &ret_ty,
                None,
                &[],
            );
            return ExprValue {
                reg: format!("@{symbol}"),
                ty: "ptr".into(),
            };
        }

        let outer: HashSet<String> = env.keys().cloned().collect();
        let capture_names = collect_arrow_captures(arrow, &outer);
        let mut fields: Vec<(String, String)> = Vec::new();
        for name in &capture_names {
            if let Some(binding) = env.get(name) {
                fields.push((name.clone(), Self::binding_ty(binding).to_string()));
            }
        }

        let cap_ty_name = format!("ClosureCap.{safe_func}.{closure_idx}");

        if !fields.is_empty() {
            let llvm_fields: Vec<String> = fields.iter().map(|(_, ty)| ty.clone()).collect();
            self.emit_module(&format!(
                "%{cap_ty_name} = type {{ {} }}",
                llvm_fields.join(", ")
            ));
        }

        self.emit_arrow_body_fn(
            &symbol,
            arrow,
            &block,
            &param_tys,
            &ret_ty,
            Some(&cap_ty_name),
            &fields,
        );

        if fields.is_empty() {
            self.emit_closure_wrap(
                &wrap_symbol,
                &symbol,
                &invoke_slot,
                true,
                &param_tys,
                &ret_ty,
            );
            return ExprValue {
                reg: format!("@{wrap_symbol}"),
                ty: "ptr".into(),
            };
        }

        let llvm_fields: Vec<String> = fields.iter().map(|(_, ty)| ty.clone()).collect();
        let cap_size = llvm_struct_size_bytes(&llvm_fields);

        let env_alloca = self.fresh("closure.env");
        self.emit(&format!("  %{env_alloca} = alloca %{cap_ty_name}"));
        for (i, (name, ty)) in fields.iter().enumerate() {
            let val_reg = self.load_binding_for_spawn(name, ty, env);
            let store_val = llvm_value_operand(&val_reg);
            let gep = self.fresh("closure.gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{env_alloca}, i64 0, i32 {i}"
            ));
            self.emit(&format!(
                "  store {ty} {store_val}, {} %{gep}",
                llvm_ptr(ty)
            ));
            if self.drop_plan.is_owned_in(&drop_state.func, name) {
                drop_state.mark_moved(name);
            }
        }

        let (env_kind, heap_owned) = if force_heap {
            let heap = self.fresh("closure.heap");
            self.needs_malloc_decl = true;
            self.emit(&format!("  %{heap} = call ptr @malloc(i64 {cap_size})"));
            self.emit(&format!(
                "  call void @llvm.memcpy.p0.p0.i64(ptr %{heap}, ptr %{env_alloca}, i64 {cap_size}, i1 false)"
            ));
            let env_global = format!("__closure_env_{safe_func}_{closure_idx}");
            self.emit_module(&format!("@{env_global} = internal global ptr null"));
            self.emit(&format!("  store ptr %{heap}, ptr @{env_global}"));
            self.emit_closure_wrap(
                &wrap_symbol,
                &symbol,
                &env_global,
                false,
                &param_tys,
                &ret_ty,
            );
            (EnvKind::Heap { global: env_global }, true)
        } else {
            self.emit_closure_wrap(
                &wrap_symbol,
                &symbol,
                &invoke_slot,
                true,
                &param_tys,
                &ret_ty,
            );
            (EnvKind::Stack { alloca: env_alloca.clone() }, false)
        };

        self.pending_closure_meta = Some(ClosureMeta {
            body_symbol: symbol.clone(),
            wrap_symbol: wrap_symbol.clone(),
            invoke_slot: invoke_slot.clone(),
            env_kind,
            heap_owned,
            param_tys: param_tys.clone(),
            ret_ty: ret_ty.clone(),
        });

        ExprValue {
            reg: format!("@{wrap_symbol}"),
            ty: "ptr".into(),
        }
    }

    pub(super) fn register_closure_local(&mut self, name: &str, meta: &ClosureMeta) {
        let (invoke_slot, env_alloca) = match &meta.env_kind {
            EnvKind::Stack { alloca } => (Some(meta.invoke_slot.clone()), Some(alloca.clone())),
            EnvKind::Heap { .. } => (None, None),
        };
        self.current_fn_ptrs.insert(
            name.to_string(),
            FnPtrSig {
                reg: format!("@{}", meta.wrap_symbol),
                _param_tys: meta.param_tys.clone(),
                ret_ty: meta.ret_ty.clone(),
                invoke_slot,
                env_alloca,
            },
        );
    }

    pub(super) fn store_closure_invoke_slot(&mut self, meta: &ClosureMeta) {
        if let EnvKind::Stack { alloca } = &meta.env_kind {
            self.emit(&format!(
                "  store ptr %{alloca}, ptr @{}",
                meta.invoke_slot
            ));
        }
    }

    pub(super) fn compile_closure_call(
        &mut self,
        meta: &ClosureMeta,
        args: &[Expression],
        env: &Env,
    ) -> ExprValue {
        let env_reg = self.closure_env_reg(meta);
        let mut arg_regs = vec![env_reg];
        let mut arg_tys = vec!["ptr".to_string()];
        for a in args {
            let v = self.compile_expr(a, env);
            arg_regs.push(self.reg_op(&v));
            arg_tys.push(v.ty.clone());
        }
        let args = arg_regs
            .iter()
            .zip(arg_tys.iter())
            .map(|(r, t)| format!("{t} {r}"))
            .collect::<Vec<_>>()
            .join(", ");
        let ret_ty = meta.ret_ty.clone();
        if ret_ty == "void" {
            self.emit(&format!(
                "  call void @{}({args})",
                meta.body_symbol
            ));
            return ExprValue {
                reg: "0".into(),
                ty: "void".into(),
            };
        }
        let reg = self.fresh("closure.call");
        self.emit(&format!(
            "  %{reg} = call {ret_ty} @{}({args})",
            meta.body_symbol
        ));
        if ret_ty.starts_with('%') {
            return self.materialize_struct_call_ret(&ret_ty, &ret_ty, &format!("%{reg}"));
        }
        ExprValue {
            reg: format!("%{reg}"),
            ty: ret_ty,
        }
    }

    /// Spill struct/enum call results to stack when match or field access needs a stable pointer.
    pub(super) fn materialize_struct_call_ret(
        &mut self,
        ret_ty: &str,
        llvm_ret_ty: &str,
        reg: &str,
    ) -> ExprValue {
        let enum_payload = struct_name_from_llvm_ty(ret_ty)
            .map(|n| self.enum_has_payload.get(&n).copied().unwrap_or(false))
            .unwrap_or(false);
        if llvm_ret_ty == ret_ty && !enum_payload {
            return ExprValue {
                reg: reg.to_string(),
                ty: ret_ty.to_string(),
            };
        }
        let alloca = self.fresh("alloca");
        self.emit(&format!("  %{alloca} = alloca {ret_ty}"));
        if llvm_ret_ty == ret_ty {
            self.emit(&format!("  store {ret_ty} {reg}, {ret_ty}* %{alloca}"));
        } else {
            self.store_coerced_extern_struct_ret(ret_ty, llvm_ret_ty, reg, &alloca);
        }
        ExprValue {
            reg: alloca,
            ty: struct_ptr_type(ret_ty),
        }
    }

    pub(super) fn emit_arrow_body_fn(
        &mut self,
        symbol: &str,
        arrow: &ArrowFnExpr,
        body: &Block,
        param_tys: &[String],
        ret_ty: &str,
        cap_ty_name: Option<&str>,
        captures: &[(String, String)],
    ) {
        let has_env = cap_ty_name.is_some() && !captures.is_empty();
        let mut params: Vec<String> = Vec::new();
        if has_env {
            params.push("ptr %env".to_string());
        }
        for (i, t) in param_tys.iter().enumerate() {
            let idx = if has_env { i + 1 } else { i };
            params.push(format!("{t} %{idx}"));
        }

        self.emit_buf = Some(Vec::new());
        let nested_scope = self.push_nested_fn_codegen_scope();
        self.emit(&format!(
            "define {ret_ty} @{symbol}({}) {{",
            params.join(", ")
        ));
        self.emit("entry:");

        let mut env: Env = HashMap::new();
        for (i, param) in arrow.params.iter().enumerate() {
            let ty = param_tys[i].clone();
            let idx = if has_env { i + 1 } else { i };
            if param.mutable {
                let storage = llvm_storage_ty(&ty);
                if Self::is_scalar_ssa_ty(storage) {
                    let reg = self.fresh("arrow.param");
                    self.emit(&format!("  %{reg} = add {storage} 0, %{idx}"));
                    env.insert(
                        param.name.clone(),
                        Binding::Reg {
                            reg,
                            ty: storage.to_string(),
                        },
                    );
                    self.mut_ssa_locals.insert(param.name.clone());
                } else {
                    let slot = self.fresh("arrow.param");
                    self.emit(&format!("  %{slot} = alloca {ty}"));
                    self.emit(&format!("  store {ty} %{idx}, {ty}* %{slot}"));
                    env.insert(
                        param.name.clone(),
                        Binding::Stack {
                            slot,
                            ty: ty.clone(),
                        },
                    );
                }
            } else {
                env.insert(
                    param.name.clone(),
                    Binding::Param {
                        index: idx,
                        ty: ty.clone(),
                    },
                );
            }
            if !param.destructure.is_empty() {
                let tuple_name = ty
                    .trim_start_matches('%')
                    .trim_end_matches('*')
                    .to_string();
                let llvm_struct = if ty.ends_with('*') {
                    ty.clone()
                } else {
                    format!("%{tuple_name}*")
                };
                let param_reg = format!("%{idx}");
                for (field_idx, name) in param.destructure.iter().enumerate() {
                    let field_ty = self
                        .tuple_fields
                        .get(&tuple_name)
                        .and_then(|fs| fs.get(field_idx))
                        .map(|a| self.llvm_type_of(a))
                        .unwrap_or_else(|| "i32".into());
                    let gep = self.fresh("arrow.param.gep");
                    self.emit(&format!(
                        "  %{gep} = getelementptr inbounds %{tuple_name}, {llvm_struct} {param_reg}, i32 0, i32 {field_idx}"
                    ));
                    let loaded = self.fresh("arrow.param.ld");
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
            }
        }

        if let Some(cap_ty_name) = cap_ty_name {
            if !captures.is_empty() {
                let cap_ptr = self.fresh("closure.cap.bc");
                self.emit(&format!(
                    "  %{cap_ptr} = bitcast ptr %env to %{cap_ty_name}*"
                ));
                for (i, (name, ty)) in captures.iter().enumerate() {
                    let gep = self.fresh("closure.fld");
                    self.emit(&format!(
                        "  %{gep} = getelementptr inbounds %{cap_ty_name}, %{cap_ty_name}* %{cap_ptr}, i64 0, i32 {i}"
                    ));
                    if is_struct_pointer_type(ty) {
                        let loaded = self.fresh("closure.ld");
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
                        let alloca = self.fresh("closure.local");
                        self.emit(&format!("  %{alloca} = alloca {ty}"));
                        let loaded = self.fresh("closure.ld");
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
        }

        let mut closure_drop = DropState::new(symbol);
        let has_ret = self.compile_block(body, &mut env, ret_ty, &mut closure_drop);
        if !has_ret {
            if ret_ty == "void" {
                self.emit("  ret void");
            } else if ret_ty == "i32" {
                self.emit("  ret i32 0");
            } else {
                self.emit(&format!("  ret {ret_ty} zeroinitializer"));
            }
        }
        self.emit("}");
        self.pop_nested_fn_codegen_scope(nested_scope);
        if let Some(helper) = self.emit_buf.take() {
            self.module_level.extend(helper);
        }
    }
}

