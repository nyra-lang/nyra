#![allow(unused_imports)]
//! Module-level IR: struct/enum declarations and function compilation.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use ast::*;
use types;
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
    pub fn compile_program(&mut self, program: &Program) -> String {
        self.enum_names = program.enums.iter().map(|e| e.name.clone()).collect();
        for ti in &program.trait_impls {
            if ti.trait_name == "Drop" || ti.trait_name == "Clone" {
                continue;
            }
            if let Some(trait_def) = program.traits.iter().find(|t| t.name == ti.trait_name) {
                for tm in &trait_def.methods {
                    let mangled = format!("{}_{}_{}", ti.trait_name, ti.type_name, tm.name);
                    self.trait_method_callees.insert(
                        (ti.type_name.clone(), tm.name.clone()),
                        mangled,
                    );
                }
            }
        }
        let mut enums: Vec<_> = program.enums.iter().collect();
        enums.sort_by(|a, b| a.name.cmp(&b.name));
        for e in enums {
            if !e.type_params.is_empty() {
                continue;
            }
            let names: Vec<String> = e.variants.iter().map(|v| v.name.clone()).collect();
            self.enum_variants.insert(e.name.clone(), names);
            let has_payload = e.variants.iter().any(|v| !v.fields.is_empty());
            self.enum_has_payload.insert(e.name.clone(), has_payload);
            if has_payload {
                if let Some(payload_ann) = e.variants.iter().find_map(|v| v.fields.first()) {
                    let payload_llvm = self.llvm_type_of(payload_ann);
                    self.enum_payload_llvm
                        .insert(e.name.clone(), payload_llvm.clone());
                    self.emit(&format!(
                        "%{} = type {{ i32, {} }}",
                        e.name, payload_llvm
                    ));
                }
            }
        }
        let mut structs: Vec<_> = program.structs.iter().collect();
        structs.sort_by(|a, b| a.name.cmp(&b.name));
        for s in structs {
            let fields: Vec<(String, TypeAnnotation)> = s
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.ty.clone()))
                .collect();
            self.struct_fields.insert(s.name.clone(), fields.clone());
            if s.attrs.repr_c {
                self.repr_c_structs.insert(s.name.clone());
            }
            let llvm_fields: Vec<String> = fields
                .iter()
                .map(|(_, ty)| self.llvm_type_of(ty))
                .collect();
            self.emit(&format!(
                "%{} = type {{ {} }}",
                s.name,
                llvm_fields.join(", ")
            ));
        }
        self.emit_trait_object_infrastructure(program);
        for ext in &program.externs {
            let c_sym = crate::runtime_map::c_symbol_for(&ext.name);
            self.extern_c_symbols.insert(ext.name.clone(), c_sym.clone());
            self.extern_fn_names.insert(ext.name.clone());
            self.extern_functions.insert(ext.name.clone(), ext.clone());
            self.skip_runtime_decls.insert(ext.name.clone());
            self.skip_runtime_decls.insert(c_sym);
            let ret = ext
                .return_type
                .clone()
                .map(|t| self.llvm_return_type_of(&t))
                .unwrap_or_else(|| "void".to_string());
            self.call_returns.insert(ext.name.clone(), ret);
        }
        for f in &program.functions {
            self.functions.insert(f.name.clone(), f.clone());
            let ret = f
                .return_type
                .clone()
                .map(|t| self.llvm_return_type_of(&t))
                .unwrap_or_else(|| "i32".to_string());
            self.call_returns.insert(f.name.clone(), ret);
        }
        for ti in &program.trait_impls {
            for method in &ti.methods {
                self.functions
                    .entry(method.name.clone())
                    .or_insert_with(|| method.clone());
                if !self.call_returns.contains_key(&method.name) {
                    let ret = method
                        .return_type
                        .clone()
                        .map(|t| self.llvm_return_type_of(&t))
                        .unwrap_or_else(|| "i32".to_string());
                    self.call_returns.insert(method.name.clone(), ret);
                }
            }
        }

        self.call_returns.insert("time_start".into(), "void".into());
        self.call_returns.insert("time_end".into(), "void".into());
        self.call_returns.insert("mem_start".into(), "void".into());
        self.call_returns.insert("mem_end".into(), "void".into());
        self.call_returns.insert("write".into(), "void".into());
        self.call_returns.insert("println".into(), "void".into());
        self.call_returns.insert("flush".into(), "void".into());
        self.call_returns.insert("input".into(), "ptr".into());
        self.call_returns.insert("date".into(), "%Date".into());
        for name in [
            "abs_i32",
            "min_i32",
            "max_i32",
            "clamp_i32",
            "abs_f64",
            "min_f64",
            "max_f64",
            "sin",
            "cos",
            "atan2",
            "tan",
        ] {
            let ret = if name.ends_with("_f64") || matches!(name, "sin" | "cos" | "atan2" | "tan") {
                "double"
            } else {
                "i32"
            };
            self.call_returns.insert(name.into(), ret.into());
        }
        self.call_returns.insert("abs".into(), "i32".into());
        self.call_returns.insert("channel_new".into(), "ptr".into());
        self.call_returns.insert("channel_recv".into(), "i32".into());
        self.call_returns.insert("channel_send".into(), "void".into());
        self.call_returns.insert("channel_free".into(), "void".into());
        self.call_returns.insert("channel_new".into(), "ptr".into());
        self.call_returns.insert("channel_recv".into(), "i32".into());
        self.call_returns.insert("channel_send".into(), "void".into());
        self.call_returns.insert("channel_free".into(), "void".into());
        self.call_returns.insert("async_await".into(), "i32".into());
        self.call_returns.insert("async_await_bool".into(), "i32".into());
        self.call_returns.insert("async_await_ptr".into(), "ptr".into());
        self.call_returns.insert("async_run".into(), "i32".into());
        self.call_returns.insert("async_promise_new".into(), "i32".into());
        self.call_returns.insert("async_poll".into(), "i32".into());
        self.call_returns.insert("async_poll_bool".into(), "i32".into());
        self.call_returns.insert("async_future_done".into(), "i32".into());
        self.call_returns.insert("async_future_ptr_value".into(), "ptr".into());
        for name in [
            "json_encode_object",
            "json_encode_i32_array",
            "json_encode_ptr_token",
            "bin_buf_new",
            "bin_buf_finish",
            "bin_decode_string_at",
            "bin_decode_blob_at",
            "i32_to_string",
            "i64_to_string",
            "f64_to_string",
            "str_cat",
            "Vec_str_new",
            "vec_str_new",
            "array_i32_debug_string",
            "array_f64_debug_string",
            "array_f32_debug_string",
            "array_bool_debug_string",
            "array_str_debug_string",
        ] {
            self.call_returns.insert(name.into(), "ptr".into());
        }
        for name in ["Vec_str_push", "Vec_str_free", "vec_str_push", "vec_str_free"] {
            self.call_returns.insert(name.into(), "void".into());
        }
        for ext in &program.externs {
            self.emit_extern_decl(ext);
        }

        for c in &program.consts {
            let val = self.compile_module_const_value(&c.value);
            self.module_consts.insert(c.name.clone(), val);
        }

        for ext in &program.externs {
            self.ensure_signature_types_from_extern(ext);
        }
        for func in &program.functions {
            if func.type_params.is_empty() {
                self.ensure_signature_types_from_fn(func);
            }
        }

        let compiled_fn_names: std::collections::HashSet<String> =
            program.functions.iter().map(|f| f.name.clone()).collect();

        let mut compile_order: Vec<&Function> = program
            .functions
            .iter()
            .filter(|f| !types::is_math_intrinsic_fn(&f.name))
            .collect();
        compile_order.sort_by(|a, b| a.name.cmp(&b.name));
        for func in compile_order {
            self.compile_function(func);
        }

        let mut extra_methods: Vec<&Function> = Vec::new();
        for imp in &program.impls {
            for method in &imp.methods {
                if !compiled_fn_names.contains(&method.name) {
                    extra_methods.push(method);
                }
            }
        }
        for ti in &program.trait_impls {
            for method in &ti.methods {
                if !compiled_fn_names.contains(&method.name) {
                    extra_methods.push(method);
                }
            }
        }
        extra_methods.sort_by(|a, b| a.name.cmp(&b.name));
        for method in extra_methods {
            self.compile_function(method);
        }

        self.sync_runtime_symbols_from_ir();

        let mut header = vec![
            "declare i32 @printf(ptr, ...)".to_string(),
            "declare void @abort()".to_string(),
        ];
        if self.uses_puts {
            header.push("declare i32 @puts(ptr)".to_string());
        }
        self.emit_runtime_decls(&mut header);
        if self.needs_malloc_decl {
            header.push("declare ptr @malloc(i64)".into());
            header.push("declare void @free(ptr)".into());
        }
        header.push("declare void @llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1)".into());
        for line in self.intrinsic_decl_lines.iter().rev() {
            header.push(line.clone());
        }
        for line in header.into_iter().rev() {
            self.lines.insert(0, line);
        }

        let mut strings_ir = String::new();
        for (i, s) in self.strings.iter().enumerate() {
            let escaped = escape_string(s);
            let len = llvm_string_len(s);
            writeln!(
                strings_ir,
                "@.str.{i} = private unnamed_addr constant [{len} x i8] c\"{escaped}\\00\", align 1"
            )
            .unwrap();
        }

        let mut body = self.lines.join("\n");
        if !self.fn_attr_sets.is_empty() {
            body = format!("{}\n\n{}", self.fn_attr_sets.join("\n"), body);
        }
        if !self.module_level.is_empty() {
            let (type_defs, rest) = hoist_llvm_type_defs(&body);
            let rest = drop_declares_for_defined_functions(&rest);
            body = format!(
                "{type_defs}\n\n{}\n\n{rest}",
                self.module_level.join("\n")
            );
        }
        format!(
            "; ModuleID = '{}'\nsource_filename = \"{}\"\ntarget triple = \"{}\"\n\n{strings_ir}\n\n{body}\n",
            self.module_name,
            self.module_name,
            self.target_triple(),
        )
    }

    pub(super) fn compile_function(&mut self, func: &Function) {
        if !func.type_params.is_empty() {
            return;
        }
        let prev_async = self.current_async_fn;
        self.current_async_fn = func.is_async;
        let is_void_entry = func.name == "main" || func.is_test;
        let default_ret = if is_void_entry {
            "void".to_string()
        } else {
            "i32".to_string()
        };
        let mut ret_ty = if func.is_async {
            "i32".to_string()
        } else {
            func.return_type
                .clone()
                .map(|t| self.llvm_return_type_of(&t))
                .unwrap_or(default_ret)
        };
        // C entry expects `int main()`; `void @main` leaves an undefined exit code on macOS.
        if func.name == "main" && ret_ty == "void" {
            ret_ty = "i32".into();
        }

        let c_main_entry = func.name == "main" && func.params.is_empty();

        let params: Vec<String> = if c_main_entry {
            vec!["i32 %0".to_string(), "i8** %1".to_string()]
        } else {
            func.params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    format!(
                        "{} %{}",
                        self.llvm_param_type_of(&p.ty),
                        i
                    )
                })
                .collect()
        };

        let linkage = if func.exported { "define " } else { "define " };
        let fn_attrs = self.fn_attr_ref(func);
        let link_name = self.llvm_fn_link_name(&func.name);
        self.emit(&format!(
            "{linkage}{} @{}({}){fn_attrs} {{",
            ret_ty,
            link_name,
            params.join(", ")
        ));
        self.emit("entry:");
        self.current_block = "entry".into();

        if c_main_entry {
            self.emit_runtime_call(
                "rt_args_init",
                "  call void @rt_args_init(i32 %0, i8** %1)",
            );
        }

        self.enum_locals.clear();
        self.current_fn_ptrs.clear();
        self.no_escape_stack_safe.clear();
        self.mut_ssa_locals.clear();
        self.heap_string_bindings.clear();
        self.non_negative_vars.clear();
        self.zero_init_ssa_vars.clear();
        self.loop_stack.clear();
        self.current_func = func.name.clone();
        let mut env: Env = HashMap::new();
        for (i, param) in func.params.iter().enumerate() {
            let ty = self.llvm_param_type_of(&param.ty);
            if is_array_ty(&ty) {
                let slot = self.fresh("param");
                self.emit(&format!("  %{slot} = alloca {ty}"));
                self.emit(&format!("  store {ty} %{i}, {ty}* %{slot}"));
                env.insert(
                    param.name.clone(),
                    Binding::Stack {
                        slot,
                        ty,
                    },
                );
            } else if param.mutable {
                let storage = llvm_storage_ty(&ty);
                if Self::is_scalar_ssa_ty(storage) {
                    let reg = self.fresh("param");
                    self.emit(&format!("  %{reg} = add {storage} 0, %{i}"));
                    env.insert(
                        param.name.clone(),
                        Binding::Reg {
                            reg,
                            ty: storage.to_string(),
                        },
                    );
                    self.mut_ssa_locals.insert(param.name.clone());
                } else if ty.ends_with('*') {
                    env.insert(
                        param.name.clone(),
                        Binding::Stack {
                            slot: i.to_string(),
                            ty: ty.clone(),
                        },
                    );
                } else {
                    let slot = self.fresh("param");
                    self.emit(&format!("  %{slot} = alloca {ty}"));
                    self.emit(&format!("  store {ty} %{i}, {ty}* %{slot}"));
                    env.insert(
                        param.name.clone(),
                        Binding::Stack {
                            slot,
                            ty,
                        },
                    );
                }
            } else {
                env.insert(
                    param.name.clone(),
                    Binding::Param { index: i, ty: ty.clone() },
                );
            }
            if let Some(en) = self.resolved_enum_name(&param.ty) {
                self.enum_locals.insert(param.name.clone(), en);
            }
            if let TypeAnnotation::FnPtr {
                params,
                return_type,
                ..
            } = &param.ty
            {
                let param_tys: Vec<String> = params
                    .iter()
                    .map(|p| self.llvm_param_type_of(p))
                    .collect();
                let ret_ty = return_type
                    .as_ref()
                    .map(|t| self.llvm_type_of(t))
                    .unwrap_or_else(|| "void".to_string());
                self.current_fn_ptrs.insert(
                    param.name.clone(),
                    FnPtrSig {
                        reg: format!("%{i}"),
                        _param_tys: param_tys,
                        ret_ty,
                        invoke_slot: None,
                        env_alloca: None,
                    },
                );
            }
        }

        let mut drop_state = DropState::new(&func.name);
        let has_return = self.compile_block(&func.body, &mut env, &ret_ty, &mut drop_state);

        if !has_return {
            self.emit_auto_drops(&drop_state, &env);
            if self.current_async_fn && ret_ty == "i32" {
                let reg = self.fresh("async");
                self.emit_runtime_call(
                    "async_run",
                    &format!("  %{reg} = call i32 @async_run(i32 0)"),
                );
                self.emit(&format!("  ret i32 %{reg}"));
            } else if ret_ty == "void" {
                self.emit("  ret void");
            } else if ret_ty == "i32" {
                self.emit("  ret i32 0");
            } else {
                self.emit(&format!("  ret {ret_ty} zeroinitializer"));
            }
        }
        self.emit("}");
        self.current_async_fn = prev_async;
    }
}

fn hoist_llvm_type_defs(ir: &str) -> (String, String) {
    let mut type_lines = Vec::new();
    let mut rest_lines = Vec::new();
    for line in ir.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') && trimmed.contains("= type {") {
            type_lines.push(line);
        } else {
            rest_lines.push(line);
        }
    }
    (type_lines.join("\n"), rest_lines.join("\n"))
}

fn drop_declares_for_defined_functions(ir: &str) -> String {
    let mut defined = std::collections::HashSet::new();
    for line in ir.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("define ") {
            if let Some(name) = rest.split('@').nth(1).and_then(|s| s.split('(').next()) {
                defined.insert(name.to_string());
            }
        }
    }
    ir.lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("declare ") {
                if let Some(name) = rest.split('@').nth(1).and_then(|s| s.split('(').next()) {
                    return !defined.contains(name);
                }
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}

