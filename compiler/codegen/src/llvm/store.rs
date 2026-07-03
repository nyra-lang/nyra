#![allow(unused_imports)]
//! LLVM store helpers for structs and GEP destinations.
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
    llvm_value_operand,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn emit_struct_store(&mut self, val: &ExprValue, dest_alloca: &str, dest_ty: &str) {
        self.emit_value_store(val, dest_alloca, dest_ty);
    }

    pub(super) fn emit_store_to_gep(&mut self, val: &ExprValue, gep: &str, field_ty: &str) {
        let ptr_ty = llvm_ptr(field_ty);
        if val.ty.ends_with('*') {
            let struct_ty = val.ty.trim_end_matches('*');
            let tmp = self.fresh("load");
            let src = if val.reg.starts_with('%') {
                val.reg.clone()
            } else {
                format!("%{}", val.reg.trim_start_matches('%'))
            };
            self.emit(&format!(
                "  %{tmp} = load {struct_ty}, {ptr_ty} {src}"
            ));
            self.emit(&format!(
                "  store {field_ty} %{tmp}, {ptr_ty} %{gep}"
            ));
        } else if field_ty.starts_with('%') && !val.reg.starts_with('%') {
            let tmp = self.fresh("load");
            self.emit(&format!(
                "  %{tmp} = load {field_ty}, {ptr_ty} %{src}",
                src = val.reg.trim_start_matches('%')
            ));
            self.emit(&format!(
                "  store {field_ty} %{tmp}, {ptr_ty} %{gep}"
            ));
        } else if is_array_ty(field_ty) {
            let src_ptr = self.materialize_array_ptr(val);
            let loaded = self.fresh("arr.ld");
            self.emit(&format!(
                "  %{loaded} = load {field_ty}, {field_ty}* {src_ptr}"
            ));
            self.emit(&format!(
                "  store {field_ty} %{loaded}, {field_ty}* %{gep}"
            ));
        } else {
            let store_val = if val.reg.starts_with('%') {
                self.coerce_value_reg_to_type(&val.reg, &val.ty, field_ty)
            } else if val.ty == "double" {
                val.reg.clone()
            } else if val.reg.chars().all(|c| c.is_ascii_digit() || c == '-')
                && matches!(
                    val.ty.as_str(),
                    "i1" | "i8" | "i16" | "i32" | "i64" | "i128" | "char" | "float" | "double"
                )
            {
                val.reg.clone()
            } else {
                let reg_op = format!("%{}", val.reg.trim_start_matches('%'));
                self.coerce_value_reg_to_type(&reg_op, &val.ty, field_ty)
            };
            self.emit(&format!(
                "  store {field_ty} {store_val}, {ptr_ty} %{gep}"
            ));
        }
    }

    pub(super) fn emit_value_store(&mut self, val: &ExprValue, dest_alloca: &str, dest_ty: &str) {
        let ptr_ty = llvm_ptr(dest_ty);
        if is_struct_pointer_type(&val.ty) && dest_ty.starts_with('%') && !dest_ty.ends_with('*') {
            let src = if val.reg.starts_with('%') {
                val.reg.clone()
            } else {
                format!("%{}", val.reg)
            };
            let tmp = self.fresh("load");
            self.emit(&format!(
                "  %{tmp} = load {dest_ty}, {} {src}",
                val.ty
            ));
            self.emit(&format!(
                "  store {dest_ty} %{tmp}, {ptr_ty} %{dest}",
                dest = dest_alloca
            ));
        } else if val.ty.ends_with('*') && dest_ty.starts_with('%') && !dest_ty.ends_with('*') {
            let tmp = self.fresh("load");
            let src = if val.reg.starts_with('%') {
                val.reg.clone()
            } else {
                format!("%{}", val.reg)
            };
            self.emit(&format!(
                "  %{tmp} = load {dest_ty}, {ptr_ty} {src}"
            ));
            self.emit(&format!(
                "  store {dest_ty} %{tmp}, {ptr_ty} %{dest}",
                dest = dest_alloca
            ));
        } else if val.ty.ends_with('*') && dest_ty.ends_with('*') {
            let src = if val.reg.starts_with('%') {
                val.reg.clone()
            } else {
                format!("%{}", val.reg)
            };
            self.emit(&format!(
                "  store {dest_ty} {src}, {ptr_ty} %{dest}",
                dest = dest_alloca
            ));
        } else if dest_ty.starts_with('%') && !val.reg.starts_with('%') && !val.ty.ends_with('*') {
            let tmp = self.fresh("load");
            self.emit(&format!(
                "  %{tmp} = load {dest_ty}, {ptr_ty} %{src}",
                dest_ty = dest_ty,
                ptr_ty = ptr_ty,
                src = val.reg
            ));
            self.emit(&format!(
                "  store {dest_ty} %{tmp}, {ptr_ty} %{dest}",
                dest_ty = dest_ty,
                tmp = tmp,
                ptr_ty = ptr_ty,
                dest = dest_alloca
            ));
        } else if is_array_ty(dest_ty) {
            let src_ptr = self.materialize_array_ptr(val);
            let loaded = self.fresh("arr.cp");
            self.emit(&format!(
                "  %{loaded} = load {dest_ty}, {dest_ty}* {src_ptr}"
            ));
            self.emit(&format!(
                "  store {dest_ty} %{loaded}, {dest_ty}* %{dest}",
                dest = dest_alloca
            ));
        } else {
            let store_ty = llvm_storage_ty(&val.ty);
            let store_val = if store_ty == "ptr" {
                llvm_ptr_reg(&val.reg)
            } else {
                llvm_value_operand(&val.reg)
            };
            self.emit(&format!(
                "  store {store_ty} {store_val}, {} %{}",
                ptr_ty, dest_alloca
            ));
        }
    }
}

