#![allow(unused_imports)]
//! Struct/tuple literals, field access, and SROA.
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
    llvm_cmp_operand, llvm_ptr, llvm_ptr_reg, llvm_storage_ty, llvm_string_len, llvm_value_operand,
    llvm_struct_size_bytes, llvm_type_ann_resolved, llvm_ty_to_ann, resolve_struct_field_name,
    struct_name_from_llvm_ty, struct_ptr_type, struct_value_type, is_struct_pointer_type,
};

impl Codegen {
    pub(super) fn field_index(&self, struct_name: &str, field: &str) -> Option<usize> {
        let field = resolve_struct_field_name(struct_name, field);
        if let Some(fields) = self.tuple_fields.get(struct_name) {
            return field.parse::<usize>().ok().filter(|i| *i < fields.len());
        }
        if let Some(fields) = self.union_fields.get(struct_name) {
            return fields.iter().position(|(n, _)| n == field);
        }
        self.struct_fields
            .get(struct_name)?
            .iter()
            .position(|(n, _)| n == field)
    }

    pub(super) fn inject_builtin_date_struct(&mut self) {
        if self.struct_fields.contains_key("Date") {
            return;
        }
        let fields: Vec<(String, TypeAnnotation)> = [
            "year",
            "month",
            "day",
            "hour",
            "minute",
            "second",
            "week",
            "millisecond",
        ]
        .into_iter()
        .map(|n| (n.to_string(), TypeAnnotation::Integer(ast::IntKind::I32)))
        .collect();
        self.struct_fields.insert("Date".into(), fields);
        self.emit_module("%Date = type { i32, i32, i32, i32, i32, i32, i32, i32 }");
    }

    pub(super) fn ensure_tuple_type(&mut self, field_anns: &[TypeAnnotation]) -> String {
        let key: String = field_anns
            .iter()
            .map(|a| self.llvm_type_of(a))
            .collect::<Vec<_>>()
            .join("_");
        let name = format!("Tuple{}_{}", field_anns.len(), key.replace('%', "").replace('*', "p"));
        if !self.tuple_fields.contains_key(&name) {
            let llvm_fields: Vec<String> = field_anns
                .iter()
                .map(|a| self.llvm_type_of(a))
                .collect();
            self.emit_module(&format!(
                "%{} = type {{ {} }}",
                name,
                llvm_fields.join(", ")
            ));
            self.tuple_fields
                .insert(name.clone(), field_anns.to_vec());
        }
        name
    }

    pub(super) fn compile_tuple_literal(
        &mut self,
        elems: &[Expression],
        env: &Env,
    ) -> ExprValue {
        let compiled: Vec<ExprValue> = elems
            .iter()
            .map(|e| self.compile_expr(e, env))
            .collect();
        let field_anns: Vec<TypeAnnotation> = compiled
            .iter()
            .map(|v| llvm_ty_to_ann(&v.ty))
            .collect();
        let name = self.ensure_tuple_type(&field_anns);
        let ty = format!("%{name}");
        let alloca = self.fresh("alloca");
        self.emit(&format!("  %{alloca} = alloca {ty}"));
        for (idx, val) in compiled.iter().enumerate() {
            let llvm_ft = self.llvm_type_of(&field_anns[idx]);
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {ty}, {ty}* %{alloca}, i32 0, i32 {idx}"
            ));
            self.emit_store_to_gep(val, &gep, &llvm_ft);
        }
        ExprValue {
            reg: alloca,
            ty: struct_ptr_type(&ty),
        }
    }

    pub(super) fn compile_struct_sroa(&mut self, sl: &StructLiteralExpr, env: &Env) -> Binding {
        use std::collections::HashMap;

        let explicit: HashMap<&str, &Expression> = sl
            .fields
            .iter()
            .map(|(n, e)| (n.as_str(), e))
            .collect();
        let mut fields = HashMap::new();
        let field_defs = self.struct_fields[&sl.name].clone();
        for (fname, field_ann) in &field_defs {
            if let Some(fexpr) = explicit.get(fname.as_str()) {
                let val = self.compile_expr(fexpr, env);
                fields.insert(
                    fname.clone(),
                    (
                        val.reg.trim_start_matches('%').to_string(),
                        val.ty.clone(),
                    ),
                );
                let _ = field_ann;
            }
        }
        Binding::PromotedStruct {
            struct_name: sl.name.clone(),
            value_ty: format!("%{}", sl.name),
            fields,
        }
    }

    pub(super) fn compile_struct_literal(
        &mut self,
        sl: &StructLiteralExpr,
        env: &Env,
        no_escape: bool,
    ) -> ExprValue {
        use std::collections::HashMap;

        let ty = format!("%{}", sl.name);
        let alloca = self.fresh("alloca");
        self.emit(&format!("  %{alloca} = alloca {ty}"));
        let is_union = self.union_fields.contains_key(&sl.name);
        let field_defs = if is_union {
            self.union_fields[&sl.name].clone()
        } else {
            self.struct_fields[&sl.name].clone()
        };
        let spread_vals: Vec<(String, ExprValue)> = if is_union {
            Vec::new()
        } else {
            sl.spreads
                .iter()
                .filter_map(|b| {
                    let val = self.compile_expr(b, env);
                    self.expr_receiver_struct_name(b, env)
                        .map(|name| (name, val))
                })
                .collect()
        };
        let explicit: HashMap<&str, &Expression> = sl
            .fields
            .iter()
            .map(|(n, e)| (n.as_str(), e))
            .collect();
        for (idx, (fname, field_ann)) in field_defs.iter().enumerate() {
            let llvm_ft = self.llvm_type_of(field_ann);
            if is_union {
                if explicit.contains_key(fname.as_str()) {
                    let fexpr = explicit[fname.as_str()];
                    let mut val = self.compile_expr(fexpr, env);
                    if matches!(field_ann, TypeAnnotation::String)
                        && !no_escape
                        && !self.expr_string_is_heap_owned(fexpr)
                    {
                        val = self.heap_clone_string(val);
                    }
                    let bc = self.fresh("bc");
                    self.emit(&format!("  %{bc} = bitcast ptr %{alloca} to ptr"));
                    self.emit_store_to_gep(&val, &bc, &llvm_ft);
                }
                continue;
            }
            let gep = self.fresh("gep");
            self.emit(&format!(
                "  %{gep} = getelementptr inbounds {ty}, {ty}* %{alloca}, i32 0, i32 {idx}"
            ));
            if let Some(fexpr) = explicit.get(fname.as_str()) {
                if let TypeAnnotation::Struct(n) = field_ann {
                    if self.struct_has_heap_fields(n)
                        && matches!(
                            fexpr,
                            Expression::FieldAccess(_) | Expression::Variable { .. }
                        )
                    {
                        let val = self.compile_expr(fexpr, env);
                        self.emit_struct_deep_copy_to_field(&val, n, &gep, field_ann);
                        continue;
                    }
                }
                let mut val = self.compile_expr(fexpr, env);
                if matches!(field_ann, TypeAnnotation::String)
                    && !no_escape
                    && !self.expr_string_is_heap_owned(fexpr)
                {
                    val = self.heap_clone_string(val);
                }
                self.emit_store_to_gep(&val, &gep, &llvm_ft);
            } else if let Some((src_name, base)) = spread_vals.iter().find(|(src, _)| {
                self.struct_fields
                    .get(src)
                    .is_some_and(|fields| fields.iter().any(|(n, _)| n == fname))
            }) {
                let field_idx = self
                    .struct_fields
                    .get(src_name)
                    .and_then(|fields| {
                        fields
                            .iter()
                            .position(|(n, _)| n == fname)
                    })
                    .unwrap_or(idx);
                let val = self.compile_spread_field(base, src_name, field_idx, field_ann);
                if let TypeAnnotation::Struct(n) = field_ann {
                    if self.struct_has_heap_fields(n) {
                        self.emit_struct_deep_copy_to_field(&val, n, &gep, field_ann);
                        continue;
                    }
                }
                self.emit_store_to_gep(&val, &gep, &llvm_ft);
            } else {
                continue;
            };
        }
        ExprValue {
            reg: alloca,
            ty: struct_ptr_type(&ty),
        }
    }

    /// Deep-copy a struct value into a parent struct field, cloning heap strings and nested structs.
    pub(super) fn emit_struct_deep_copy_to_field(
        &mut self,
        src_val: &ExprValue,
        struct_name: &str,
        dest_field_gep: &str,
        field_ann: &TypeAnnotation,
    ) {
        let struct_ty = format!("%{struct_name}");
        let llvm_ft = self.llvm_type_of(field_ann);
        let src_ptr = if src_val.ty.ends_with('*') {
            if src_val.reg.starts_with('%') {
                src_val.reg.clone()
            } else {
                format!("%{}", src_val.reg)
            }
        } else {
            let tmp = self.fresh("src_slot");
            self.emit(&format!("  %{tmp} = alloca {struct_ty}"));
            let src_reg = if src_val.reg.starts_with('%') {
                src_val.reg.clone()
            } else {
                llvm_value_operand(&src_val.reg)
            };
            self.emit(&format!(
                "  store {struct_ty} {src_reg}, {llvm_ft}* %{tmp}"
            ));
            format!("%{tmp}")
        };
        let Some(fields) = self.struct_fields.get(struct_name).cloned() else {
            return;
        };
        for (idx, (_, sub_ann)) in fields.iter().enumerate() {
            let src_fgep = self.fresh("src_fgep");
            self.emit(&format!(
                "  %{src_fgep} = getelementptr inbounds {struct_ty}, {struct_ty}* {src_ptr}, i32 0, i32 {idx}"
            ));
            let dest_fgep = self.fresh("dst_fgep");
            self.emit(&format!(
                "  %{dest_fgep} = getelementptr inbounds {llvm_ft}, {llvm_ft}* %{dest_field_gep}, i32 0, i32 {idx}"
            ));
            let sub_ty = self.llvm_type_of(sub_ann);
            if matches!(sub_ann, TypeAnnotation::String) {
                let loaded = self.fresh("ld");
                self.emit(&format!("  %{loaded} = load ptr, ptr %{src_fgep}"));
                let cloned = self.fresh("clone");
                self.emit_runtime_call(
                    "str_clone",
                    &format!("  %{cloned} = call ptr @str_clone(ptr %{loaded})"),
                );
                self.emit(&format!("  store ptr %{cloned}, ptr %{dest_fgep}"));
            } else if sub_ty.starts_with('%') && !sub_ty.ends_with('*') {
                let nested = sub_ty.trim_start_matches('%');
                let loaded = self.fresh("ld");
                self.emit(&format!(
                    "  %{loaded} = load {sub_ty}, {sub_ty}* %{src_fgep}"
                ));
                let nested_val = ExprValue {
                    reg: format!("%{loaded}"),
                    ty: sub_ty.clone(),
                };
                self.emit_struct_deep_copy_to_field(&nested_val, nested, &dest_fgep, sub_ann);
            } else {
                let loaded = self.fresh("ld");
                self.emit(&format!(
                    "  %{loaded} = load {sub_ty}, {} %{src_fgep}",
                    llvm_ptr(&sub_ty)
                ));
                self.emit(&format!(
                    "  store {sub_ty} %{loaded}, {} %{dest_fgep}",
                    llvm_ptr(&sub_ty)
                ));
            }
        }
    }

    /// Load one field from a spread base, cloning heap strings for Move ergonomics.
    pub(super) fn compile_spread_field(
        &mut self,
        base: &ExprValue,
        struct_name: &str,
        field_idx: usize,
        field_ann: &TypeAnnotation,
    ) -> ExprValue {
        let llvm_struct = format!("%{struct_name}");
        let llvm_ft = self.llvm_type_of(field_ann);
        let gep = self.fresh("gep");
        let base_ptr = if base.ty.ends_with('*') {
            if base.reg.starts_with('%') {
                base.reg.clone()
            } else {
                format!("%{}", base.reg)
            }
        } else if base.reg.starts_with('%') {
            let tmp = self.fresh("spread");
            self.emit(&format!("  %{tmp} = alloca {llvm_struct}"));
            self.emit(&format!(
                "  store {llvm_struct} {}, {llvm_struct}* %{tmp}",
                base.reg
            ));
            format!("%{tmp}")
        } else {
            format!("%{}", base.reg)
        };
        self.emit(&format!(
            "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {field_idx}"
        ));
        let reg = self.fresh("load");
        self.emit(&format!(
            "  %{reg} = load {llvm_ft}, {} %{gep}",
            llvm_ptr(&llvm_ft)
        ));
        if matches!(field_ann, TypeAnnotation::String) {
            let len = self.fresh("len");
            self.emit(&format!("  %{len} = call i32 @strlen(ptr %{reg})"));
            let cloned = self.fresh("clone");
            self.emit(&format!(
                "  %{cloned} = call ptr @substring(ptr %{reg}, i32 0, i32 %{len})"
            ));
            return ExprValue {
                reg: format!("%{cloned}"),
                ty: "ptr".into(),
            };
        }
        ExprValue {
            reg: format!("%{reg}"),
            ty: llvm_ft,
        }
    }

    pub(super) fn compile_field_access(
        &mut self,
        fa: &FieldAccessExpr,
        env: &Env,
    ) -> ExprValue {
        if let Expression::Variable { name, .. } = &fa.object {
            if let Some(Binding::PromotedStruct { fields, .. }) = env.get(name) {
                if let Some((reg, ty)) = fields.get(&fa.field) {
                    return ExprValue {
                        reg: llvm_value_operand(reg),
                        ty: ty.clone(),
                    };
                }
            }
        }
        let obj = self.compile_expr(&fa.object, env);
        let struct_name = obj
            .ty
            .trim_start_matches('%')
            .trim_end_matches('*')
            .to_string();
        let llvm_struct = format!("%{struct_name}");
        let field_idx = self.field_index(&struct_name, &fa.field).unwrap_or(0);
        let field_ty = if let Some(fields) = self.tuple_fields.get(&struct_name) {
            fields
                .get(field_idx)
                .map(|a| self.llvm_type_of(a))
                .unwrap_or_else(|| "i32".into())
        } else if self.union_fields.contains_key(&struct_name) {
            self.llvm_type_of(&self.union_fields[&struct_name][field_idx].1)
        } else {
            self.llvm_type_of(&self.struct_fields[&struct_name][field_idx].1)
        };
        let gep = self.fresh("gep");
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
        if self.union_fields.contains_key(&struct_name) {
            let bc = self.fresh("bc");
            self.emit(&format!("  %{bc} = bitcast ptr {base_ptr} to ptr"));
            let reg = self.fresh("load");
            self.emit(&format!("  %{reg} = load {field_ty}, ptr %{bc}"));
            return ExprValue {
                reg: format!("%{reg}"),
                ty: field_ty,
            };
        }
        self.emit(&format!(
            "  %{gep} = getelementptr inbounds {llvm_struct}, {llvm_struct}* {base_ptr}, i32 0, i32 {field_idx}"
        ));
        let reg = self.fresh("load");
        self.emit(&format!("  %{reg} = load {field_ty}, {} %{gep}", llvm_ptr(&field_ty)));
        ExprValue {
            reg: format!("%{reg}"),
            ty: field_ty,
        }
    }
}

