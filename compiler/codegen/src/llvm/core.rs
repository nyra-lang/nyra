#![allow(unused_imports)]
//! Core `Codegen` setup, IR emission, and runtime bookkeeping.
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
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            strings: Vec::new(),
            string_intern: HashMap::new(),
            lines: Vec::new(),
            temp_counter: 0,
            label_counter: 0,
            struct_fields: HashMap::new(),
            tuple_fields: HashMap::new(),
            functions: HashMap::new(),
            extern_functions: HashMap::new(),
            call_returns: HashMap::new(),
            enum_variants: HashMap::new(),
            enum_has_payload: HashMap::new(),
            enum_payload_llvm: HashMap::new(),
            enum_locals: HashMap::new(),
            module_consts: HashMap::new(),
            enum_names: std::collections::HashSet::new(),
            target: String::new(),
            drop_plan: DropPlan::default(),
            escape_plan: EscapePlan::default(),
            no_escape_stack_safe: HashSet::new(),
            used_runtime: BTreeSet::new(),
            needs_malloc_decl: false,
            uses_puts: false,
            skip_runtime_decls: HashSet::new(),
            module_level: Vec::new(),
            emit_buf: None,
            current_async_fn: false,
            compiling_drop: None,
            current_fn_ptrs: HashMap::new(),
            pending_closure_meta: None,
            closure_force_heap: false,
            current_func: String::new(),
            local_channel_type_emitted: false,
            mut_ssa_locals: HashSet::new(),
            heap_string_bindings: HashSet::new(),
            non_negative_vars: HashSet::new(),
            zero_init_ssa_vars: HashSet::new(),
            loop_stack: Vec::new(),
            current_block: "entry".into(),
            fn_attr_sets: Vec::new(),
            repr_c_structs: HashSet::new(),
            union_fields: HashMap::new(),
            struct_layout_infos: HashMap::new(),
            union_layout_infos: HashMap::new(),
            repr_c_unions: HashSet::new(),
            enum_variant_payload_llvm: HashMap::new(),
            extern_fn_names: HashSet::new(),
            extern_c_symbols: HashMap::new(),
            declared_c_syms: HashSet::new(),
            intrinsic_decl_lines: Vec::new(),
            intrinsic_decls: HashSet::new(),
            local_int_kinds: HashMap::new(),
            trait_method_callees: HashMap::new(),
        }
    }

    pub(super) fn is_scalar_ssa_ty(ty: &str) -> bool {
        matches!(llvm_storage_ty(ty), "i32" | "i64" | "u32" | "f64" | "f32" | "i1")
    }

    pub(super) fn fn_attr_ref(&mut self, func: &Function) -> String {
        let mut parts = Vec::new();
        if func.inline {
            parts.push("alwaysinline");
        }
        if func.hot {
            parts.push("inlinehint");
        }
        if func.cold {
            parts.push("cold");
        }
        if parts.is_empty() {
            return String::new();
        }
        let idx = self.fn_attr_sets.len();
        self.fn_attr_sets
            .push(format!("attributes #{} = {{ {} }}", idx, parts.join(", ")));
        format!(" #{idx}")
    }

    pub fn take_runtime_profile(&self) -> RuntimeProfile {
        RuntimeProfile {
            symbols: self.used_runtime.clone(),
        }
    }

    pub(super) fn record_runtime(&mut self, sym: &str) {
        self.used_runtime.insert(sym.to_string());
    }

    pub(super) fn ensure_runtime_fn_decl(&mut self, sym: &str, decl: &str) {
        if self.declared_c_syms.insert(sym.to_string()) {
            self.intrinsic_decl_lines.push(decl.to_string());
        }
        self.record_runtime(sym);
    }

    pub(super) fn track_local_int_kind(&mut self, name: &str, kind: IntKind) {
        self.local_int_kinds.insert(name.to_string(), kind);
    }

    pub(super) fn track_local_int_kind_ann(&mut self, name: &str, ann: &TypeAnnotation) {
        if let TypeAnnotation::Integer(k) = ann {
            self.track_local_int_kind(name, *k);
        }
    }

    /// Ensure link pulls in any runtime module referenced by emitted `call @sym(...)`.
    pub(super) fn sync_runtime_symbols_from_ir(&mut self) {
        let map = crate::runtime_map::symbol_module_map();
        let mut needed = Vec::new();
        let mut scan_lines = |lines: &[String]| {
            for line in lines {
                let trimmed = line.trim_start();
                if trimmed.starts_with("declare ") {
                    continue;
                }
                if !trimmed.contains(" call ") {
                    continue;
                }
                for sym in map.keys() {
                    if line.contains(&format!("@{sym}(")) {
                        needed.push((*sym).to_string());
                    }
                }
            }
        };
        scan_lines(&self.lines);
        scan_lines(&self.module_level);
        for sym in needed {
            self.record_runtime(&sym);
        }
    }

    pub(super) fn is_runtime_symbol(&self, sym: &str) -> bool {
        let resolved = crate::runtime_map::c_symbol_for(sym);
        crate::runtime_map::symbol_module_map().contains_key(resolved.as_str())
    }

    pub(super) fn runtime_callee(&self, nyra_name: &str) -> String {
        if let Some(c) = self.extern_c_symbols.get(nyra_name) {
            return c.clone();
        }
        crate::runtime_map::c_symbol_for(nyra_name)
    }

    pub(super) fn emit_runtime_call(&mut self, sym: &str, line: &str) {
        let c_sym = self.runtime_callee(sym);
        self.record_runtime(&c_sym);
        let line = if c_sym == sym {
            line.to_string()
        } else {
            line.replace(&format!("@{sym}"), &format!("@{c_sym}"))
        };
        self.emit(&line);
    }

    pub fn set_target(&mut self, target: &str) {
        self.target = target.to_string();
    }

    pub fn set_drop_plan(&mut self, plan: DropPlan) {
        self.drop_plan = plan;
    }

    pub fn set_escape_plan(&mut self, plan: EscapePlan) {
        self.escape_plan = plan;
    }

    pub(super) fn intern_string(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.string_intern.get(s) {
            return idx;
        }
        let idx = self.strings.len();
        self.string_intern.insert(s.to_string(), idx);
        self.strings.push(s.to_string());
        idx
    }

    pub(super) fn fresh(&mut self, prefix: &str) -> String {
        let n = self.temp_counter;
        self.temp_counter += 1;
        format!("{prefix}.{n}")
    }

    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}.{n}")
    }

    pub(super) fn emit_block_label(&mut self, label: &str) {
        self.emit(&format!("{label}:"));
        self.current_block = label.to_string();
    }

    fn current_block_has_terminator(&self) -> bool {
        if self.current_block.is_empty() || self.current_block.starts_with("__terminated_") {
            return true;
        }
        let body = if let Some(buf) = &self.emit_buf {
            buf.as_slice()
        } else {
            self.lines.as_slice()
        };
        let label_line = format!("{}:", self.current_block);
        let mut start: Option<usize> = None;
        for (i, line) in body.iter().enumerate() {
            if line.trim() == label_line {
                start = Some(i);
            }
        }
        let Some(start) = start else {
            return false;
        };
        for line in body.iter().skip(start + 1) {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.ends_with(':') && !t.contains('=') {
                return false;
            }
            return t.starts_with("br ")
                || t.starts_with("ret ")
                || t.starts_with("switch ")
                || t == "unreachable";
        }
        false
    }

    pub(super) fn ensure_br_to(&mut self, target: &str) {
        if !self.current_block_has_terminator() {
            self.emit(&format!("  br label %{target}"));
        }
    }

    pub(super) fn emit(&mut self, line: &str) {
        if let Some(buf) = &mut self.emit_buf {
            buf.push(line.to_string());
        } else {
            self.lines.push(line.to_string());
        }
    }

    /// Active function IR buffer (`emit_buf` for nested spawn/closure, else `lines`).
    pub(super) fn ir_body_mut(&mut self) -> &mut Vec<String> {
        if let Some(buf) = &mut self.emit_buf {
            buf
        } else {
            &mut self.lines
        }
    }

    pub(super) fn emit_module(&mut self, line: &str) {
        self.module_level.push(line.to_string());
    }

    pub(super) fn compile_asm(&mut self, template: &str) {
        let escaped = template
            .replace('\\', "\\\\")
            .replace('"', "\\22")
            .replace('\n', "\\0A");
        self.emit(&format!(
            "  call void asm sideeffect \"{escaped}\", \"\"()"
        ));
    }

    pub(super) fn emit_runtime_decls(&self, out: &mut Vec<String>) {
        let decls = [
            ("str_len", "declare i32 @str_len(ptr)"),
            ("char_at", "declare i32 @char_at(ptr, i32)"),
            ("str_cat", "declare ptr @str_cat(ptr, ptr)"),
            ("str_clone", "declare ptr @str_clone(ptr)"),
            ("substring", "declare ptr @substring(ptr, i32, i32)"),
            ("i32_to_string", "declare ptr @i32_to_string(i32)"),
            ("i64_to_string", "declare ptr @i64_to_string(i64)"),
            ("str_cmp", "declare i32 @str_cmp(ptr, ptr)"),
            ("str_to_upper", "declare ptr @str_to_upper(ptr)"),
            ("str_to_lower", "declare ptr @str_to_lower(ptr)"),
            ("str_trim", "declare ptr @str_trim(ptr)"),
            ("str_contains", "declare i32 @str_contains(ptr, ptr)"),
            ("str_starts_with", "declare i32 @str_starts_with(ptr, ptr)"),
            ("str_ends_with", "declare i32 @str_ends_with(ptr, ptr)"),
            ("str_replace", "declare ptr @str_replace(ptr, ptr, ptr)"),
            ("str_replacen", "declare ptr @str_replacen(ptr, ptr, ptr, i32)"),
            ("str_split", "declare ptr @str_split(ptr, ptr)"),
            (
                "array_i32_sort_copy",
                "declare void @array_i32_sort_copy(i32*, i32*, i32)",
            ),
            (
                "array_f64_sort_copy",
                "declare void @array_f64_sort_copy(double*, double*, i32)",
            ),
            (
                "array_i32_debug_string",
                "declare ptr @array_i32_debug_string(i32*, i32)",
            ),
            (
                "array_f64_debug_string",
                "declare ptr @array_f64_debug_string(double*, i32)",
            ),
            (
                "array_f32_debug_string",
                "declare ptr @array_f32_debug_string(float*, i32)",
            ),
            (
                "array_bool_debug_string",
                "declare ptr @array_bool_debug_string(i8*, i32)",
            ),
            (
                "array_str_debug_string",
                "declare ptr @array_str_debug_string(ptr, i32)",
            ),
            ("bounds_assert_i32", "declare void @bounds_assert_i32(i32)"),
            ("vec_str_len", "declare i32 @vec_str_len(ptr)"),
            ("vec_str_get", "declare ptr @vec_str_get(ptr, i32)"),
            ("vec_str_free", "declare void @vec_str_free(ptr)"),
            ("read_file", "declare ptr @read_file(ptr)"),
            ("read_file_limit", "declare ptr @read_file_limit(ptr, i32)"),
            ("write_file", "declare i32 @write_file(ptr, ptr)"),
            ("file_size", "declare i64 @file_size(ptr)"),
            ("copy_file", "declare i64 @copy_file(ptr, ptr)"),
            ("list_dir", "declare ptr @list_dir(ptr)"),
            ("path_is_dir", "declare i32 @path_is_dir(ptr)"),
            ("rt_args_init", "declare void @rt_args_init(i32, ptr)"),
            ("os_arg_count", "declare i32 @os_arg_count()"),
            ("os_arg_at", "declare ptr @os_arg_at(i32)"),
            ("heap_free", "declare void @heap_free(ptr)"),
            ("spawn_capture", "declare ptr @spawn_capture(ptr, ptr, i64)"),
            ("spawn_join", "declare i32 @spawn_join(ptr)"),
            ("spawn_handle_drop", "declare void @spawn_handle_drop(ptr)"),
            ("spawn_task_capture", "declare ptr @spawn_task_capture(ptr, ptr, i64)"),
            ("spawn_task_join", "declare i32 @spawn_task_join(ptr)"),
            ("spawn_task_handle_drop", "declare void @spawn_task_handle_drop(ptr)"),
            (
                "parallel_for_range",
                "declare void @parallel_for_range(i32, i32, ptr, ptr, i32, i32, i32, i32, i32)",
            ),
            ("cpu_count", "declare i32 @cpu_count()"),
            ("progress_update", "declare void @progress_update(i32, i32, ptr)"),
            ("progress_finish", "declare void @progress_finish()"),
            ("benchmark_begin", "declare void @benchmark_begin()"),
            ("benchmark_end", "declare void @benchmark_end()"),
            ("async_await", "declare i32 @async_await(i32)"),
            ("async_await_bool", "declare i32 @async_await_bool(i32)"),
            ("async_await_ptr", "declare ptr @async_await_ptr(i32)"),
            ("async_run", "declare i32 @async_run(i32)"),
            ("async_promise_new", "declare i32 @async_promise_new()"),
            (
                "async_promise_complete",
                "declare void @async_promise_complete(i32, i32)",
            ),
            (
                "async_promise_complete_bool",
                "declare void @async_promise_complete_bool(i32, i32)",
            ),
            (
                "async_promise_complete_ptr",
                "declare void @async_promise_complete_ptr(i32, ptr)",
            ),
            ("async_poll", "declare i32 @async_poll(i32)"),
            ("async_poll_bool", "declare i32 @async_poll_bool(i32)"),
            ("async_future_done", "declare i32 @async_future_done(i32)"),
            ("async_future_ptr_value", "declare ptr @async_future_ptr_value(i32)"),
            ("runtime_run", "declare void @runtime_run()"),
            ("io_register", "declare i32 @io_register(i32, i32)"),
            ("io_wait_once", "declare i32 @io_wait_once(i32)"),
            ("channel_new", "declare ptr @channel_new()"),
            ("channel_send", "declare void @channel_send(ptr, i32)"),
            ("channel_recv", "declare i32 @channel_recv(ptr)"),
            ("channel_free", "declare void @channel_free(ptr)"),
            ("time_start", "declare void @time_start(ptr)"),
            ("time_end", "declare void @time_end(ptr)"),
            ("date_now", "declare void @date_now(ptr)"),
            ("mem_start", "declare void @mem_start(ptr)"),
            ("mem_end", "declare void @mem_end(ptr)"),
            ("stdout_write_str", "declare void @stdout_write_str(ptr)"),
            ("stdout_writeln_str", "declare void @stdout_writeln_str(ptr)"),
            ("stdout_write_i32", "declare void @stdout_write_i32(i32)"),
            ("stdout_writeln_i32", "declare void @stdout_writeln_i32(i32)"),
            ("stdout_flush", "declare void @stdout_flush()"),
            ("stdin_read_line", "declare ptr @stdin_read_line(ptr)"),
            ("process_exit", "declare void @process_exit(i32)"),
            ("vec_str_from_argv", "declare ptr @vec_str_from_argv(i32)"),
            ("bytes_read_file", "declare ptr @bytes_read_file(ptr)"),
            ("bytes_len", "declare i64 @bytes_len(ptr)"),
            ("byte_at", "declare i32 @byte_at(ptr, i64)"),
            ("bytes_to_string", "declare ptr @bytes_to_string(ptr)"),
            ("bytes_from_string", "declare ptr @bytes_from_string(ptr)"),
            ("bytes_free", "declare void @bytes_free(ptr)"),
            ("bytes_write_file", "declare i32 @bytes_write_file(ptr, ptr)"),
            ("stdin_read_bytes", "declare ptr @stdin_read_bytes(i32)"),
            ("stdout_write_bytes", "declare void @stdout_write_bytes(ptr)"),
            ("regex_compile", "declare ptr @regex_compile(ptr)"),
            ("regex_is_match", "declare i32 @regex_is_match(ptr, ptr)"),
            ("tar_create", "declare i32 @tar_create(ptr, ptr)"),
            ("tar_extract", "declare i32 @tar_extract(ptr, ptr)"),
            ("gzip_file", "declare i32 @gzip_file(ptr, ptr)"),
            ("gunzip_file", "declare i32 @gunzip_file(ptr, ptr)"),
            ("println", "declare i32 @println(ptr)"),
            ("color_ansi", "declare ptr @color_ansi(ptr)"),
            ("ansi_reset", "declare ptr @ansi_reset()"),
            ("blackbox_i32", "declare i32 @blackbox_i32(i32)"),
            ("rand_i32", "declare i32 @rand_i32()"),
            ("rand_range", "declare i32 @rand_range(i32, i32)"),
            ("rand_i64", "declare i64 @rand_i64()"),
            ("rand_range_i64", "declare i64 @rand_range_i64(i64, i64)"),
            ("rand_u32", "declare i32 @rand_u32()"),
            ("rand_range_u32", "declare i32 @rand_range_u32(i32, i32)"),
            ("rand_u64", "declare i64 @rand_u64()"),
            ("rand_range_u64", "declare i64 @rand_range_u64(i64, i64)"),
            ("rand_f64", "declare double @rand_f64()"),
            (
                "rand_f64_range",
                "declare double @rand_f64_range(double, double)",
            ),
        ];
        for (name, line) in decls {
            if self.skip_runtime_decls.contains(name) || self.declared_c_syms.contains(name) {
                continue;
            }
            if self.used_runtime.contains(name) {
                out.push(line.to_string());
            }
        }
    }

    pub(super) fn llvm_fn_link_name(&self, name: &str) -> String {
        super::util::llvm_fn_link_name(name, &self.target_triple())
    }

    pub(super) fn target_triple(&self) -> String {
        if !self.target.is_empty() {
            return self.target.clone();
        }
        host_target_triple()
    }
}

