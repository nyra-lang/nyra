#![allow(unused_imports)]
//! Nyra type annotations → LLVM types and FFI coercions.
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
    pub(super) fn resolved_enum_name(&self, ty: &TypeAnnotation) -> Option<String> {
        match ty {
            TypeAnnotation::Struct(n) | TypeAnnotation::Enum(n) => {
                self.enum_names.contains(n).then(|| n.clone())
            }
            TypeAnnotation::Applied { base, args } => {
                let suffix: String = args
                    .iter()
                    .map(|a| self.llvm_type_of(a).trim_start_matches('%').to_string())
                    .collect::<Vec<_>>()
                    .join("_");
                let mangled = format!("{base}__{suffix}");
                if self.enum_names.contains(&mangled) {
                    Some(mangled)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn llvm_type_of(&self, ty: &TypeAnnotation) -> String {
        if let Some(name) = self.resolved_enum_name(ty) {
            if self.enum_has_payload.get(&name).copied().unwrap_or(false) {
                return format!("%{name}");
            }
            return "i32".into();
        }
        llvm_type_ann_resolved(ty, &self.struct_fields, &self.enum_names)
    }

    pub(super) fn llvm_return_type_of(&self, ty: &TypeAnnotation) -> String {
        if let Some(name) = self.resolved_enum_name(ty) {
            return self.enum_llvm_value_type(&name);
        }
        if matches!(ty, TypeAnnotation::Bytes) {
            return "ptr".into();
        }
        self.llvm_type_of(ty)
    }

    pub(super) fn llvm_param_type_of(&self, ty: &TypeAnnotation) -> String {
        if let Some(name) = self.resolved_enum_name(ty) {
            if self.enum_has_payload.get(&name).copied().unwrap_or(false) {
                return format!("%{name}*");
            }
            return "i32".into();
        }
        match ty {
            TypeAnnotation::DynTrait { trait_name, .. } => format!("%Dyn_{trait_name}*"),
            TypeAnnotation::Struct(n) | TypeAnnotation::Enum(n)
                if self.enum_names.contains(n) =>
            {
                if self.enum_has_payload.get(n).copied().unwrap_or(false) {
                    format!("%{n}*")
                } else {
                    "i32".into()
                }
            }
            TypeAnnotation::Struct(n) => format!("%{n}*"),
            TypeAnnotation::Bytes => "ptr".into(),
            _ => self.llvm_type_of(ty),
        }
    }

    pub(super) fn target_is_arm64_apple(&self) -> bool {
        let t = self.target_triple();
        t.contains("arm64-apple") || t.contains("aarch64-apple")
    }

    pub(super) fn repr_c_struct_byte_size(&self, name: &str) -> u64 {
        let Some(fields) = self.struct_fields.get(name) else {
            return 0;
        };
        let mut size = 0u64;
        for (_, ann) in fields {
            let bytes = match ann {
                TypeAnnotation::Integer(k) => (k.bits() as u64 + 7) / 8,
                TypeAnnotation::F32 => 4,
                TypeAnnotation::F64 => 8,
                TypeAnnotation::Bool => 1,
                TypeAnnotation::Char => 4,
                TypeAnnotation::Struct(n) => self.repr_c_struct_byte_size(n),
                _ => 4,
            };
            size += bytes;
        }
        size
    }

    pub(super) fn repr_c_struct_uses_arm64_int_coerce(&self, name: &str) -> bool {
        self.target_is_arm64_apple()
            && self.repr_c_structs.contains(name)
            && self.repr_c_struct_byte_size(name) <= 8
    }

    /// Darwin arm64: structs larger than 16 bytes use sret / indirect pointer (not byval).
    pub(super) fn repr_c_struct_uses_arm64_indirect(&self, name: &str) -> bool {
        self.target_is_arm64_apple()
            && self.repr_c_structs.contains(name)
            && self.repr_c_struct_byte_size(name) > 16
    }

    pub(super) fn extern_call_needs_sret(&self, logical_ret: &str) -> Option<String> {
        if !logical_ret.starts_with('%') {
            return None;
        }
        let name = logical_ret.trim_start_matches('%');
        if self.repr_c_struct_uses_arm64_indirect(name) {
            Some(name.to_string())
        } else {
            None
        }
    }

    /// LLVM parameter type for `extern fn` — `repr(C)` structs follow platform C ABI.
    pub(super) fn llvm_extern_param_type_of(&self, ty: &TypeAnnotation) -> String {
        match ty {
            TypeAnnotation::Struct(n) | TypeAnnotation::Enum(n)
                if self.enum_names.contains(n) =>
            {
                if self.enum_has_payload.get(n).copied().unwrap_or(false) {
                    format!("%{n}*")
                } else {
                    "i32".into()
                }
            }
            TypeAnnotation::Struct(n) if self.repr_c_struct_uses_arm64_int_coerce(n) => {
                "i64".into()
            }
            TypeAnnotation::Struct(n) if self.repr_c_struct_uses_arm64_indirect(n) => {
                "ptr".into()
            }
            TypeAnnotation::Struct(n) if self.repr_c_structs.contains(n) => {
                format!("%{n}* byval(%{n})")
            }
            TypeAnnotation::Struct(n) => format!("%{n}*"),
            TypeAnnotation::Bytes => "ptr".into(),
            _ => self.llvm_type_of(ty),
        }
    }

    pub(super) fn llvm_extern_ret_type_of(&self, ty: &TypeAnnotation) -> String {
        match ty {
            TypeAnnotation::Struct(n) if self.repr_c_struct_uses_arm64_int_coerce(n) => {
                let sz = self.repr_c_struct_byte_size(n);
                if sz <= 4 {
                    "i32".into()
                } else {
                    "i64".into()
                }
            }
            TypeAnnotation::Struct(n) if self.repr_c_struct_uses_arm64_indirect(n) => {
                "void".into()
            }
            TypeAnnotation::Bytes => "ptr".into(),
            _ => self.llvm_type_of(ty),
        }
    }

    fn llvm_int_bits(ty: &str) -> Option<u16> {
        let stored = llvm_storage_ty(ty);
        match stored.as_ref() {
            "i8" => Some(8),
            "i16" => Some(16),
            "i32" => Some(32),
            "i64" => Some(64),
            "i128" => Some(128),
            _ => None,
        }
    }

    pub(super) fn coerce_value_reg_to_type(&mut self, reg: &str, from_ty: &str, to_ty: &str) -> String {
        if from_ty == to_ty {
            return reg.to_string();
        }
        let from = llvm_storage_ty(from_ty);
        let to = llvm_storage_ty(to_ty);
        if from == to {
            return reg.to_string();
        }
        if from == "double" {
            if Self::llvm_int_bits(&to).is_some() {
                let conv = self.fresh("cast");
                self.emit(&format!("  %{conv} = fptosi double {reg} to {to}"));
                return format!("%{conv}");
            }
            return reg.to_string();
        }
        if let (Some(fb), Some(tb)) = (Self::llvm_int_bits(&from), Self::llvm_int_bits(&to)) {
            if fb == tb {
                return reg.to_string();
            }
            let conv = self.fresh("cast");
            if fb > tb {
                self.emit(&format!("  %{conv} = trunc {from} {reg} to {to}"));
            } else if from == "i8" || from == "i16" {
                self.emit(&format!("  %{conv} = zext {from} {reg} to {to}"));
            } else {
                self.emit(&format!("  %{conv} = sext {from} {reg} to {to}"));
            }
            return format!("%{conv}");
        }
        if from == "i32" && to == "double" {
            let conv = self.fresh("sitofp");
            self.emit(&format!("  %{conv} = sitofp i32 {reg} to double"));
            return format!("%{conv}");
        }
        if from == "i32" && to == "float" {
            let conv = self.fresh("sitofp");
            self.emit(&format!("  %{conv} = sitofp i32 {reg} to float"));
            return format!("%{conv}");
        }
        reg.to_string()
    }

    pub(super) fn coerce_expr_to_llvm_type(&mut self, val: ExprValue, to_ty: &str) -> ExprValue {
        if val.ty == to_ty {
            return val;
        }
        if to_ty == "ptr" && val.ty.starts_with('%') && !val.ty.ends_with('*') {
            let slot = self.materialize_struct_ssa_slot(&val);
            return ExprValue {
                reg: slot,
                ty: struct_ptr_type(&val.ty),
            };
        }
        if val.ty.starts_with('%')
            || to_ty.starts_with('%')
            || is_array_ty(&val.ty)
            || is_array_ty(to_ty)
            || val.ty == "ptr"
            || to_ty == "ptr"
        {
            return val;
        }
        if (val.ty == "i32" && to_ty == "double") || (val.ty == "i32" && to_ty == "float") {
            let reg_op = if val.reg.starts_with('%') {
                val.reg.clone()
            } else {
                format!("%{}", val.reg.trim_start_matches('%'))
            };
            let coerced = self.coerce_value_reg_to_type(&reg_op, &val.ty, to_ty);
            return ExprValue {
                reg: coerced,
                ty: to_ty.to_string(),
            };
        }
        if val.ty == "double" || to_ty == "double" {
            return val;
        }
        let reg_op = if val.reg.starts_with('%')
            || val.reg.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.')
        {
            val.reg.clone()
        } else {
            format!("%{}", val.reg.trim_start_matches('%'))
        };
        let coerced = self.coerce_value_reg_to_type(&reg_op, &val.ty, to_ty);
        if coerced == reg_op && val.ty != to_ty {
            return val;
        }
        ExprValue {
            reg: coerced,
            ty: to_ty.to_string(),
        }
    }

    pub(super) fn fn_param_llvm_types(&self, name: &str) -> Option<Vec<String>> {
        if let Some(f) = self.functions.get(name) {
            return Some(
                f.params
                    .iter()
                    .map(|p| self.llvm_param_type_of(&p.ty))
                    .collect(),
            );
        }
        self.extern_functions
            .get(name)
            .map(|e| {
                e.params
                    .iter()
                    .map(|p| self.llvm_extern_param_type_of(&p.ty))
                    .collect()
            })
    }

    pub(super) fn coerce_struct_slot_to_i64(&mut self, slot: &str, struct_name: &str) -> String {
        let sz = self.repr_c_struct_byte_size(struct_name);
        let slot_op = if slot.starts_with('%') {
            slot.to_string()
        } else {
            format!("%{slot}")
        };
        let tmp = self.fresh("coerce");
        self.emit(&format!("  %{tmp} = alloca i64, align 8"));
        self.emit(&format!(
            "  call void @llvm.memcpy.p0.p0.i64(ptr %{tmp}, ptr {slot_op}, i64 {sz}, i1 false)"
        ));
        let loaded = self.fresh("ld");
        self.emit(&format!("  %{loaded} = load i64, ptr %{tmp}"));
        format!("%{loaded}")
    }

    pub(super) fn store_coerced_extern_struct_ret(
        &mut self,
        struct_ty: &str,
        llvm_ret_ty: &str,
        ret_reg: &str,
        dest_alloca: &str,
    ) {
        if llvm_ret_ty == "i32" {
            self.emit(&format!("  store i32 {ret_reg}, ptr %{dest_alloca}"));
        } else if llvm_ret_ty == "i64" {
            let tmp = self.fresh("coerce");
            self.emit(&format!("  %{tmp} = alloca i64, align 8"));
            self.emit(&format!("  store i64 {ret_reg}, ptr %{tmp}"));
            let sz = struct_ty
                .strip_prefix('%')
                .map(|n| self.repr_c_struct_byte_size(n))
                .filter(|s| *s > 0)
                .unwrap_or(8);
            self.emit(&format!(
                "  call void @llvm.memcpy.p0.p0.i64(ptr %{dest_alloca}, ptr %{tmp}, i64 {sz}, i1 false)"
            ));
        }
    }

    pub(super) fn is_extern_c_call(&self, callee: &str) -> bool {
        self.extern_fn_names.contains(callee) && !self.is_runtime_symbol(callee)
    }

    pub(super) fn materialize_struct_ssa_slot(&mut self, val: &ExprValue) -> String {
        let slot = self.fresh("arg.tmp");
        self.emit(&format!("  %{slot} = alloca {}", val.ty));
        self.emit(&format!(
            "  store {} {}, {} %{slot}",
            val.ty,
            val.reg,
            llvm_ptr(&val.ty)
        ));
        slot
    }

    pub(super) fn push_call_arg(
        &mut self,
        v: &ExprValue,
        is_extern_c: bool,
        arg_regs: &mut Vec<String>,
        arg_tys: &mut Vec<String>,
    ) {
        if is_struct_pointer_type(&v.ty) {
            let ptr = format!("%{}", v.reg.trim_start_matches('%'));
            if is_extern_c {
                if let Some(n) = struct_name_from_llvm_ty(&v.ty) {
                    if self.repr_c_struct_uses_arm64_int_coerce(&n) {
                        arg_regs.push(self.coerce_struct_slot_to_i64(&ptr, &n));
                        arg_tys.push("i64".into());
                        return;
                    }
                    if self.repr_c_struct_uses_arm64_indirect(&n) {
                        arg_regs.push(ptr);
                        arg_tys.push("ptr".into());
                        return;
                    }
                    if self.repr_c_structs.contains(&n) {
                        arg_regs.push(ptr);
                        arg_tys.push(format!("%{n}* byval(%{n})"));
                        return;
                    }
                }
            }
            arg_regs.push(ptr);
            arg_tys.push(v.ty.clone());
        } else if v.ty.starts_with('%') && !v.ty.ends_with('*') {
            let slot = self.materialize_struct_ssa_slot(v);
            if is_extern_c {
                if let Some(n) = struct_name_from_llvm_ty(&v.ty) {
                    if self.repr_c_struct_uses_arm64_int_coerce(&n) {
                        arg_regs.push(self.coerce_struct_slot_to_i64(&slot, &n));
                        arg_tys.push("i64".into());
                        return;
                    }
                    if self.repr_c_struct_uses_arm64_indirect(&n) {
                        arg_regs.push(format!("%{slot}"));
                        arg_tys.push("ptr".into());
                        return;
                    }
                    if self.repr_c_structs.contains(&n) {
                        arg_regs.push(format!("%{slot}"));
                        arg_tys.push(format!("%{n}* byval(%{n})"));
                        return;
                    }
                }
            }
            arg_regs.push(format!("%{slot}"));
            arg_tys.push(struct_ptr_type(&v.ty));
        } else if v.ty.starts_with('%') {
            arg_regs.push(self.materialize_ptr_reg(&v.reg));
            arg_tys.push("ptr".into());
        } else {
            arg_regs.push(v.reg.clone());
            arg_tys.push(v.ty.clone());
        }
    }

    #[allow(dead_code)]
    pub(super) fn enum_llvm_value_type(&self, name: &str) -> String {
        if self.enum_has_payload.get(name).copied().unwrap_or(false) {
            format!("%{name}")
        } else {
            "i32".into()
        }
    }

    pub(super) fn enum_scrutinee_ptr(&mut self, scrutinee: &ExprValue) -> String {
        if is_struct_pointer_type(&scrutinee.ty) {
            return self.reg_op(scrutinee);
        }
        if scrutinee.ty.starts_with('%') {
            let struct_ty = scrutinee.ty.as_str();
            let alloca = self.fresh("enum.scrut");
            self.emit(&format!("  %{alloca} = alloca {struct_ty}"));
            self.emit(&format!(
                "  store {struct_ty} {}, {} %{}",
                self.reg_op(scrutinee),
                llvm_ptr(struct_ty),
                alloca
            ));
            return format!("%{alloca}");
        }
        self.reg_op(scrutinee)
    }

    pub(super) fn load_enum_tag(&mut self, scrutinee: &ExprValue, enum_name: &str) -> String {
        if self.enum_has_payload.get(enum_name).copied().unwrap_or(false) {
            let ptr = self.enum_scrutinee_ptr(scrutinee);
            let gep = self.fresh("gep");
            let en = enum_name;
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds %{en}, %{en}* {ptr}, i32 0, i32 0"
            ));
            let reg = self.fresh("tag");
            self.emit(&format!("  %{reg} = load i32, i32* %{gep}"));
            format!("%{reg}")
        } else {
            self.reg_op(scrutinee)
        }
    }

    pub(super) fn reg_op(&self, v: &ExprValue) -> String {
        if v.reg.starts_with('%') || v.reg.starts_with('@') {
            v.reg.clone()
        } else if v.reg.chars().all(|c| c.is_ascii_digit() || c == '-')
            && matches!(
                v.ty.as_str(),
                "i1" | "i8" | "i16" | "i32" | "i64" | "i128" | "char" | "float" | "double"
            )
        {
            v.reg.clone()
        } else {
            format!("%{}", v.reg)
        }
    }

    pub(super) fn logical_call_ret_ty(&self, ty: &TypeAnnotation) -> String {
        if matches!(ty, TypeAnnotation::Bytes) {
            "bytes".into()
        } else {
            self.llvm_return_type_of(ty)
        }
    }

    pub(super) fn llvm_extern_call_ret_ty(&self, callee: &str, logical_ret: &str) -> String {
        if logical_ret == "bytes" {
            return "ptr".into();
        }
        if !self.is_extern_c_call(callee) || !logical_ret.starts_with('%') {
            return logical_ret.to_string();
        }
        let name = logical_ret.trim_start_matches('%');
        if self.repr_c_struct_uses_arm64_int_coerce(name) {
            let sz = self.repr_c_struct_byte_size(name);
            if sz <= 4 {
                return "i32".into();
            }
            return "i64".into();
        }
        if self.repr_c_struct_uses_arm64_indirect(name) {
            return "void".into();
        }
        logical_ret.to_string()
    }
}

