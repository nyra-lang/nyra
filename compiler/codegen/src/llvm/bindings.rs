#![allow(unused_imports)]
//! Variable bindings: load, store, and SSA promotion.
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
    llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len, llvm_value_operand,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn binding_load(&mut self, binding: &Binding) -> (String, String) {
        match binding {
            Binding::Param { index, ty } => (index.to_string(), ty.clone()),
            Binding::Reg { reg, ty } => (reg.clone(), ty.clone()),
            Binding::Stack { slot, ty } => {
                let loaded = self.fresh("ld");
                let storage = llvm_storage_ty(ty);
                self.emit(&format!(
                    "  %{loaded} = load {storage}, {} %{slot}",
                    llvm_ptr(ty)
                ));
                (loaded, ty.clone())
            }
            Binding::Closure(meta) => (meta.wrap_symbol.clone(), "ptr".into()),
            Binding::PromotedStruct { .. } => ("0".into(), "i32".into()),
            Binding::LocalChannel { slot } => (slot.clone(), "ptr".into()),
        }
    }

    pub(super) fn materialize_promoted_struct(
        &mut self,
        struct_name: &str,
        fields: &HashMap<String, (String, String)>,
    ) -> ExprValue {
        let ty = format!("%{struct_name}");
        let alloca = self.fresh("promoted");
        self.emit(&format!("  %{alloca} = alloca {ty}"));
        let field_defs = self
            .struct_fields
            .get(struct_name)
            .cloned()
            .unwrap_or_default();
        for (idx, (fname, field_ann)) in field_defs.iter().enumerate() {
            let Some((reg, fty)) = fields.get(fname) else {
                continue;
            };
            let llvm_ft = self.llvm_type_of(field_ann);
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {ty}, {ty}* %{alloca}, i32 0, i32 {idx}"
            ));
            let reg_op = llvm_value_operand(reg);
            let store_val = self.coerce_value_reg_to_type(&reg_op, fty, &llvm_ft);
            self.emit(&format!(
                "  store {llvm_ft} {store_val}, {} %{gep}",
                llvm_ptr(&llvm_ft)
            ));
        }
        ExprValue {
            reg: alloca,
            ty: struct_ptr_type(&ty),
        }
    }

    pub(super) fn binding_to_expr(&mut self, binding: &Binding) -> ExprValue {
        match binding {
            Binding::Param { index, ty } => ExprValue {
                reg: format!("%{index}"),
                ty: ty.clone(),
            },
            Binding::Reg { reg, ty } => {
                if is_array_ty(ty) {
                    return ExprValue {
                        reg: reg.trim_start_matches('%').to_string(),
                        ty: ty.clone(),
                    };
                }
                ExprValue {
                    reg: if ty == "ptr" {
                        llvm_ptr_reg(reg)
                    } else {
                        llvm_value_operand(reg)
                    },
                    ty: ty.clone(),
                }
            }
            Binding::Stack { slot, ty } => {
                if is_array_ty(ty) {
                    return ExprValue {
                        reg: slot.clone(),
                        ty: ty.clone(),
                    };
                }
                if ty.ends_with('*') {
                    return ExprValue {
                        reg: format!("%{slot}"),
                        ty: ty.clone(),
                    };
                }
                if ty.starts_with('%') && !ty.ends_with('*') {
                    return ExprValue {
                        reg: format!("%{slot}"),
                        ty: struct_ptr_type(ty),
                    };
                }
                let loaded = self.fresh("ld");
                let storage = llvm_storage_ty(ty);
                self.emit(&format!(
                    "  %{loaded} = load {storage}, {} %{slot}",
                    llvm_ptr(ty)
                ));
                ExprValue {
                    reg: format!("%{loaded}"),
                    ty: ty.clone(),
                }
            }
            Binding::Closure(meta) => ExprValue {
                reg: format!("@{}", meta.wrap_symbol),
                ty: "ptr".into(),
            },
            Binding::PromotedStruct {
                struct_name,
                fields,
                ..
            } => self.materialize_promoted_struct(struct_name, fields),
            Binding::LocalChannel { slot } => ExprValue {
                reg: format!("%{slot}"),
                ty: "ptr".into(),
            },
        }
    }

    pub(super) fn fn_ptr_sig_from_ann(&self, ann: &TypeAnnotation) -> Option<(Vec<String>, String)> {
        if let TypeAnnotation::FnPtr {
            params,
            return_type,
            ..
        } = ann
        {
            let param_tys: Vec<String> = params.iter().map(|p| self.llvm_type_of(p)).collect();
            let ret_ty = return_type
                .as_ref()
                .map(|t| self.llvm_type_of(t))
                .unwrap_or_else(|| "void".to_string());
            Some((param_tys, ret_ty))
        } else {
            None
        }
    }

    pub(super) fn register_fn_ptr_local(&mut self, name: &str, ann: &TypeAnnotation, env: &Env) {
        let Some((_param_tys, ret_ty)) = self.fn_ptr_sig_from_ann(ann) else {
            return;
        };
        let reg = self.fn_ptr_call_target(name, env);
        self.current_fn_ptrs.insert(
            name.to_string(),
            FnPtrSig {
                reg,
                _param_tys: Vec::new(),
                ret_ty,
                invoke_slot: None,
                env_alloca: None,
            },
        );
    }

    pub(super) fn fn_ptr_call_target(&mut self, name: &str, env: &Env) -> String {
        if let Some(binding) = env.get(name) {
            match binding {
                Binding::Closure(meta) => {
                    format!("@{}", meta.wrap_symbol)
                }
                Binding::Param { index, .. } => format!("%{index}"),
                Binding::Reg { reg, ty } => {
                    if reg.starts_with('@') {
                        reg.clone()
                    } else if *ty == "ptr" {
                        format!("%{reg}")
                    } else {
                        format!("%{reg}")
                    }
                }
                Binding::Stack { slot, ty } if ty == "ptr" => {
                    let loaded = self.fresh("fn");
                    self.emit(&format!("  %{loaded} = load ptr, ptr %{slot}"));
                    format!("%{loaded}")
                }
                Binding::Stack { slot, ty } => {
                    let loaded = self.fresh("fn");
                    self.emit(&format!("  %{loaded} = load {ty}, {} %{slot}", llvm_ptr(ty)));
                    format!("%{loaded}")
                }
                Binding::PromotedStruct { .. } => "0".to_string(),
                Binding::LocalChannel { slot } => format!("%{slot}"),
            }
        } else if self.functions.contains_key(name) {
            format!("@{name}")
        } else {
            format!("@{name}")
        }
    }

    pub(super) fn binding_store_expr(&mut self, binding: &Binding, val: &ExprValue) {
        match binding {
            Binding::Reg { reg, ty }
                if ty.starts_with('%') && !ty.ends_with('*') =>
            {
                self.emit_value_store(val, reg.trim_start_matches('%'), ty);
            }
            Binding::Param { .. } | Binding::Reg { .. } | Binding::Closure(_) | Binding::PromotedStruct { .. } | Binding::LocalChannel { .. } => {}
            Binding::Stack { slot, ty } => {
                if ty.starts_with('%') {
                    self.emit_value_store(val, slot, ty);
                } else {
                    self.emit(&format!(
                        "  store {} {}, {} %{slot}",
                        val.ty, val.reg, llvm_ptr(ty)
                    ));
                }
            }
        }
    }

    pub(super) fn binding_ty(binding: &Binding) -> &str {
        match binding {
            Binding::Param { ty, .. } | Binding::Reg { ty, .. } | Binding::Stack { ty, .. } => ty,
            Binding::Closure(_) => "ptr",
            Binding::PromotedStruct { value_ty, .. } => value_ty,
            Binding::LocalChannel { .. } => "ptr",
        }
    }

    pub(super) fn binding_no_escape(&self, name: &str) -> bool {
        if self.escape_plan.is_no_escape_param(&self.current_func, name) {
            return true;
        }
        !self.current_func.is_empty()
            && self.escape_plan.state_in(&self.current_func, name) == EscapeState::NoEscape
    }

    pub(super) fn struct_all_copy_scalars(&self, struct_name: &str) -> bool {
        self.struct_fields.get(struct_name).is_some_and(|fields| {
            fields
                .iter()
                .all(|(_, ann)| self.type_ann_is_copy_scalar(ann))
        })
    }

    pub(super) fn type_ann_is_copy_scalar(&self, ann: &TypeAnnotation) -> bool {
        match ann {
            TypeAnnotation::Integer(_)
            | TypeAnnotation::Bool
            | TypeAnnotation::Char
            | TypeAnnotation::F32
            | TypeAnnotation::F64 => true,
            TypeAnnotation::Enum(n) => {
                self.enum_names.contains(n)
                    && !self
                        .enum_has_payload
                        .get(n)
                        .copied()
                        .unwrap_or(false)
            }
            _ => false,
        }
    }

    pub(super) fn struct_literal_fields_stack_safe(&self, sl: &StructLiteralExpr) -> bool {
        if !sl.spreads.is_empty() {
            return false;
        }
        sl.fields
            .iter()
            .all(|(_, expr)| !self.expr_string_is_heap_owned(expr))
    }

    pub(super) fn struct_literal_sroa_eligible(&self, sl: &StructLiteralExpr) -> bool {
        if self.struct_all_copy_scalars(&sl.name) {
            return true;
        }
        if !self.struct_literal_fields_stack_safe(sl) {
            return false;
        }
        self.struct_fields.get(&sl.name).is_some_and(|fields| {
            fields.iter().all(|(_, ann)| {
                self.type_ann_is_copy_scalar(ann) || matches!(ann, TypeAnnotation::String)
            })
        })
    }

    pub(super) fn is_channel_origin_expr(expr: &Expression) -> bool {
        match expr {
            Expression::Call(c) => matches!(
                c.callee.as_str(),
                "channel_new" | "Channel_i32_new"
            ),
            Expression::StructLiteral(s) => s.fields.iter().any(|(_, v)| {
                matches!(
                    v,
                    Expression::Call(c) if matches!(
                        c.callee.as_str(),
                        "channel_new"
                    )
                )
            }),
            _ => false,
        }
    }

    pub(super) fn reg_operand_from_binding(binding: &Binding) -> String {
        match binding {
            Binding::Reg { reg, ty } => {
                if ty == "ptr" {
                    llvm_ptr_reg(reg)
                } else if reg.chars().all(|c| c.is_ascii_digit() || c == '-') {
                    reg.clone()
                } else if reg.starts_with('%') {
                    reg.clone()
                } else {
                    format!("%{reg}")
                }
            }
            Binding::Param { index, ty } if ty == "ptr" => format!("%{index}"),
            _ => "0".into(),
        }
    }
}

