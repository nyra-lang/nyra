#![allow(unused_imports)]
//! Drop glue emission for owned locals and composite values.
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
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, llvm_value_operand, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn binding_struct_ptr(&mut self, binding: &Binding, struct_ty: &str) -> Option<String> {
        match binding {
            Binding::Stack { slot, .. } => Some(format!("%{slot}")),
            Binding::Reg { reg, ty } if ty.ends_with('*') => Some(self.reg_op(&ExprValue {
                reg: reg.clone(),
                ty: ty.clone(),
            })),
            Binding::Reg { reg, ty } if ty == struct_ty => {
                let tmp = self.fresh("drop_slot");
                self.emit(&format!("  %{tmp} = alloca {struct_ty}"));
                self.emit(&format!(
                    "  store {struct_ty} {}, {struct_ty}* %{tmp}",
                    llvm_value_operand(reg)
                ));
                Some(format!("%{tmp}"))
            }
            _ => None,
        }
    }

    pub(super) fn emit_drop_local(&mut self, name: &str, env: &Env, drop_state: &DropState) {
        let Some(binding) = env.get(name) else {
            return;
        };
        if matches!(binding, Binding::PromotedStruct { .. } | Binding::LocalChannel { .. }) {
            return;
        }
        if drop_state.moved.contains(name) {
            return;
        }
        if self.drop_plan.is_join_handle_in(&drop_state.func, name) {
            let (loaded, _) = self.binding_load(binding);
            let kind = self.drop_plan.join_handle_kind(&drop_state.func, name);
            self.emit_spawn_handle_drop(&loaded, kind);
            return;
        }
        let ty = Self::binding_ty(binding).to_string();
        if ty == "vec_str" {
            let (loaded, _) = self.binding_load(binding);
            self.emit_runtime_call(
                "vec_str_free",
                &format!("  call void @vec_str_free(ptr %{loaded})"),
            );
            return;
        }
        if ty == "ptr" {
            let (loaded, _) = self.binding_load(binding);
            self.emit_runtime_call(
                "free",
                &format!("  call void @free(ptr %{loaded})"),
            );
            return;
        }
        if ty.starts_with('%') && !ty.ends_with('*') {
            let type_name = ty.trim_start_matches('%');
            if type_name.starts_with("Dyn_") {
                let trait_name = type_name.trim_start_matches("Dyn_");
                let drop_fn = format!("__dyn_{trait_name}_drop");
                if let Some(struct_ptr) = self.binding_struct_ptr(binding, &ty) {
                    self.emit(&format!(
                        "  call void @{drop_fn}({ty}* {struct_ptr})"
                    ));
                }
                return;
            }
            if self.drop_plan.is_enum_payload_in(&drop_state.func, name) {
                self.emit_enum_payload_drop(type_name, binding);
                return;
            }
            if let Some(drop_fn) = self
                .drop_plan
                .custom_drop_fns
                .get(type_name)
                .cloned()
            {
                if let Some(struct_ptr) = self.binding_struct_ptr(binding, &ty) {
                    self.emit(&format!(
                        "  call void @{drop_fn}({ty}* {struct_ptr})"
                    ));
                }
            } else if self
                .drop_plan
                .is_composite_struct_in(&drop_state.func, name)
            {
                if !self.no_escape_stack_safe.contains(name) {
                    self.emit_composite_struct_drop(type_name, binding);
                }
            }
        } else if ty.ends_with('*') {
            let en = ty.trim_start_matches('%').trim_end_matches('*');
            if self.drop_plan.is_enum_payload_in(&drop_state.func, name) {
                self.emit_enum_payload_drop(en, binding);
            }
        }
    }

    pub(super) fn emit_enum_payload_drop(&mut self, enum_name: &str, binding: &Binding) {
        let enum_ty = format!("%{enum_name}");
        let enum_ptr = match binding {
            Binding::Stack { slot, .. } => format!("%{slot}"),
            Binding::Reg { reg, .. } => format!("%{reg}"),
            _ => return,
        };
        let heap_tags: Vec<i64> = self
            .enum_variant_payload_llvm
            .get(enum_name)
            .map(|m| {
                m.iter()
                    .filter_map(|(variant, llvm_ty)| {
                        if llvm_ty == "ptr" {
                            Some(self.variant_tag(enum_name, variant))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if heap_tags.is_empty() {
            return;
        }
        let tag_gep = self.fresh("enum_drop_tag_gep");
        self.emit(&format!(
            "  %{tag_gep} = getelementptr inbounds {enum_ty}, {enum_ty}* {enum_ptr}, i32 0, i32 0"
        ));
        let tag_reg = self.fresh("enum_drop_tag");
        self.emit(&format!(
            "  %{tag_reg} = load i32, ptr %{tag_gep}"
        ));
        let pay_gep = self.fresh("enum_drop_gep");
        self.emit(&format!(
            "  %{pay_gep} = getelementptr inbounds {enum_ty}, {enum_ty}* {enum_ptr}, i32 0, i32 1"
        ));
        // Always branch on the tag before freeing. Unit variants (e.g. Option.None)
        // share the payload slot layout but must never free — a prior fast-path that
        // skipped the tag check double-freed / freed garbage for Option<string>.None.
        let skip_l = self.fresh_label("enum_drop.skip");
        let free_l = self.fresh_label("enum_drop.free");
        let end_l = self.fresh_label("enum_drop.end");
        let mut checks = heap_tags.clone();
        let first = checks.remove(0);
        let cmp = self.fresh("enum_drop_cmp");
        self.emit(&format!(
            "  %{cmp} = icmp eq i32 %{tag_reg}, {first}"
        ));
        if checks.is_empty() {
            self.emit(&format!(
                "  br i1 %{cmp}, label %{free_l}, label %{skip_l}"
            ));
        } else {
            let next_l = self.fresh_label("enum_drop.next");
            self.emit(&format!(
                "  br i1 %{cmp}, label %{free_l}, label %{next_l}"
            ));
            self.emit(&format!("{next_l}:"));
            let mut prev_cmp = cmp;
            for (i, tag) in checks.iter().enumerate() {
                let c = self.fresh("enum_drop_cmp");
                self.emit(&format!(
                    "  %{c} = icmp eq i32 %{tag_reg}, {tag}"
                ));
                if i + 1 == checks.len() {
                    self.emit(&format!(
                        "  br i1 %{c}, label %{free_l}, label %{skip_l}"
                    ));
                } else {
                    let n = self.fresh_label("enum_drop.next");
                    self.emit(&format!(
                        "  br i1 %{c}, label %{free_l}, label %{n}"
                    ));
                    self.emit(&format!("{n}:"));
                }
                prev_cmp = c;
            }
            let _ = prev_cmp;
        }
        self.emit(&format!("{free_l}:"));
        let loaded = self.fresh("enum_drop_load");
        self.emit(&format!("  %{loaded} = load ptr, ptr %{pay_gep}"));
        self.emit_runtime_call(
            "free",
            &format!("  call void @free(ptr %{loaded})"),
        );
        self.emit(&format!("  br label %{end_l}"));
        self.emit(&format!("{skip_l}:"));
        self.emit(&format!("  br label %{end_l}"));
        self.emit(&format!("{end_l}:"));
    }

    /// Drop heap-owned fields of a struct without a custom `Drop` impl (reverse field order).
    pub(super) fn emit_composite_struct_drop(&mut self, struct_name: &str, binding: &Binding) {
        let Some(fields) = self.struct_fields.get(struct_name).cloned() else {
            return;
        };
        let struct_ty = format!("%{struct_name}");
        let base_ptr = match binding {
            Binding::Stack { slot, .. } => format!("%{slot}"),
            Binding::Reg { reg, ty } if ty.ends_with('*') => self.reg_op(&ExprValue {
                reg: reg.clone(),
                ty: ty.clone(),
            }),
            Binding::Reg { reg, ty } if ty.starts_with('%') && !ty.ends_with('*') => {
                format!("%{}", reg.trim_start_matches('%'))
            }
            Binding::Reg { reg, ty } => {
                let tmp_slot = self.fresh("drop_slot");
                self.emit(&format!("  %{tmp_slot} = alloca {ty}"));
                self.emit(&format!(
                    "  store {ty} %{reg}, {} %{tmp_slot}",
                    llvm_ptr(ty)
                ));
                format!("%{tmp_slot}")
            }
            Binding::Param { .. } | Binding::Closure(_) | Binding::PromotedStruct { .. } | Binding::LocalChannel { .. } => return,
        };
        for (field_idx, (_, field_ann)) in fields.iter().enumerate().rev() {
            self.emit_field_drop(struct_name, &struct_ty, &base_ptr, field_idx, field_ann);
        }
    }

    pub(super) fn emit_field_drop(
        &mut self,
        _struct_name: &str,
        struct_ty: &str,
        base_ptr: &str,
        field_idx: usize,
        field_ann: &TypeAnnotation,
    ) {
        let field_ty = self.llvm_type_of(field_ann);
        let gep = self.fresh("drop_gep");
        self.emit(&format!(
            "  %{gep} = getelementptr inbounds {struct_ty}, {struct_ty}* {base_ptr}, i32 0, i32 {field_idx}"
        ));
        if field_ty == "ptr" {
            let loaded = self.fresh("drop_load");
            self.emit(&format!(
                "  %{loaded} = load ptr, ptr %{gep}"
            ));
            self.emit_runtime_call(
                "free",
                &format!("  call void @free(ptr %{loaded})"),
            );
        } else if field_ty.starts_with('%') && !field_ty.ends_with('*') {
            let nested = field_ty.trim_start_matches('%');
            if self
                .drop_plan
                .custom_drop_fns
                .get(nested)
                .is_some()
            {
                if let Some(drop_fn) = self.drop_plan.custom_drop_fns.get(nested) {
                    self.emit(&format!(
                        "  call void @{drop_fn}({field_ty}* %{gep})"
                    ));
                }
            } else if self.struct_has_heap_fields(nested) {
                self.emit_composite_struct_drop_from_gep(nested, &field_ty, &gep);
            }
        }
    }

    pub(super) fn struct_has_heap_fields(&self, name: &str) -> bool {
        self.struct_fields.get(name).is_some_and(|fields| {
            fields.iter().any(|(_, ann)| {
                let ty = self.llvm_type_of(ann);
                ty == "ptr"
                    || (ty.starts_with('%')
                        && !ty.ends_with('*')
                        && self.struct_has_heap_fields(ty.trim_start_matches('%')))
            })
        })
    }

    pub(super) fn emit_composite_struct_drop_from_gep(
        &mut self,
        struct_name: &str,
        struct_ty: &str,
        field_gep: &str,
    ) {
        let tmp_slot = self.fresh("nested_slot");
        self.emit(&format!("  %{tmp_slot} = alloca {struct_ty}"));
        let loaded = self.fresh("nested_val");
        self.emit(&format!(
            "  %{loaded} = load {struct_ty}, {} %{field_gep}",
            llvm_ptr(struct_ty)
        ));
        self.emit(&format!(
            "  store {struct_ty} %{loaded}, {} %{tmp_slot}",
            llvm_ptr(struct_ty)
        ));
        let binding = Binding::Stack {
            slot: tmp_slot,
            ty: struct_ty.to_string(),
        };
        self.emit_composite_struct_drop(struct_name, &binding);
    }
}

