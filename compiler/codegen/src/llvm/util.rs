pub(super) fn escape_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b if b.is_ascii() => out.push(char::from(b)),
            b => out.push_str(&format!("\\{:02X}", b)),
        }
    }
    out
}

pub(super) fn llvm_string_len(s: &str) -> usize {
    s.len() + 1
}

pub(super) fn array_len_from_ty(ty: &str) -> Option<usize> {
    // `[N x elem]`
    let inner = ty.strip_prefix('[')?;
    let n_str = inner.split(" x ").next()?;
    n_str.parse().ok()
}

pub(super) fn array_elem_from_ty(ty: &str) -> Option<String> {
    let inner = ty.strip_prefix('[')?;
    let rest = inner.split(" x ").nth(1)?;
    Some(rest.trim_end_matches(']').to_string())
}

pub(super) fn llvm_type_size_bytes(ty: &str) -> i64 {
    if let (Some(len), Some(elem)) = (array_len_from_ty(ty), array_elem_from_ty(ty)) {
        return (len as i64) * llvm_type_size_bytes(&elem);
    }
    if ty.starts_with('<') && ty.contains(" x ") {
        if let Some(inner) = ty
            .trim_start_matches('<')
            .split(" x ")
            .nth(1)
            .map(|s| s.trim_end_matches('>'))
        {
            if let Ok(lanes) = ty
                .trim_start_matches('<')
                .split(" x ")
                .next()
                .unwrap_or("")
                .parse::<i64>()
            {
                return lanes * llvm_type_size_bytes(inner);
            }
        }
    }
    llvm_field_size_align(ty).1
}

pub(super) fn llvm_type_align_bytes(ty: &str) -> i64 {
    if ty.starts_with('<') {
        return 16;
    }
    llvm_field_size_align(ty).0
}

/// Size and alignment of one LLVM field type for spawn/closure capture layout.
fn llvm_field_size_align(ty: &str) -> (i64, i64) {
    if let (Some(len), Some(elem)) = (array_len_from_ty(ty), array_elem_from_ty(ty)) {
        let (elem_align, elem_sz) = llvm_field_size_align(&elem);
        let array_align = elem_align;
        let array_sz = (len as i64) * elem_sz;
        return (array_align, array_sz);
    }
    if ty == "ptr" || ty.ends_with('*') {
        (8, 8)
    } else     if ty == "i1" {
        (1, 1)
    } else if ty == "i8" {
        (1, 1)
    } else if ty == "double" || ty == "f64" {
        (8, 8)
    } else if ty == "float" || ty == "f32" {
        (4, 4)
    } else if ty.starts_with('%') {
        // Opaque struct by value — conservative pointer alignment.
        (8, 8)
    } else {
        (4, 4)
    }
}

pub(super) fn llvm_struct_size_bytes(fields: &[String]) -> i64 {
    let mut size = 0i64;
    for ty in fields {
        let (align, sz) = llvm_field_size_align(ty);
        size = (size + align - 1) / align * align;
        size += sz;
    }
    (size + 7) / 8 * 8
}

pub(super) fn resolve_struct_field_name<'a>(struct_name: &str, field: &'a str) -> &'a str {
    if struct_name == "Date" {
        match field {
            "minutes" => "minute",
            "seconds" => "second",
            "weekday" => "week",
            _ => field,
        }
    } else {
        field
    }
}

pub(super) fn is_string_builtin_method(method: &str) -> bool {
    matches!(
        method,
        "split" | "trim" | "contains" | "starts_with" | "ends_with" | "replace"
            | "replacen"
            | "to_upper" | "to_lower" | "strip_suffix" | "to_snake_case" | "to_lowercase" | "to_titlecase" | "to_capitalize" | "to_camel_case" | "to_kebab_case" | "to_pascal_case" | "to_screaming_snake_case" | "to_train_case" | "to_dot_case")
}

pub(super) fn llvm_ptr_reg(reg: &str) -> String {
    if reg.starts_with('%') || reg.starts_with('@') || reg == "null" {
        reg.to_string()
    } else {
        format!("%{reg}")
    }
}

/// LLVM textual constant for `float` / `double` literals (must include `.` so `1.0` is not parsed as integer).
pub(super) fn llvm_float_const(n: f64, kind: FloatKind) -> String {
    match kind {
        FloatKind::F32 => format!("{:.8}", n as f32),
        FloatKind::F64 => format!("{n:.17}"),
    }
}

/// Register name or numeric literal for typed LLVM operands (`store`, `icmp`, …).
pub(super) fn llvm_value_operand(reg: &str) -> String {
    if reg.starts_with('%') || reg.starts_with('@') {
        reg.to_string()
    } else if reg.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.') {
        reg.to_string()
    } else {
        format!("%{reg}")
    }
}

/// Nyra `fn` names that collide with MSVC UCRT globals when emitted as LLVM symbols.
const WINDOWS_CRT_FN_COLLISIONS: &[&str] = &["atoi", "atof", "atol", "atoll"];

/// LLVM link symbol for a Nyra-defined function (may differ from the source name on Windows).
pub(super) fn llvm_fn_link_name(name: &str, target_triple: &str) -> String {
    if target_triple.to_ascii_lowercase().contains("windows")
        && WINDOWS_CRT_FN_COLLISIONS.contains(&name)
    {
        format!("nyra_{name}")
    } else {
        name.to_string()
    }
}

pub(super) fn host_target_triple() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => other,
    };
    match std::env::consts::OS {
        "macos" => {
            if arch == "aarch64" {
                "arm64-apple-darwin".into()
            } else {
                format!("{arch}-apple-darwin")
            }
        }
        "linux" => format!("{arch}-unknown-linux-gnu"),
        "windows" => format!("{arch}-pc-windows-gnu"),
        _ => format!("{arch}-unknown-linux-gnu"),
    }
}

#[cfg(test)]
mod llvm_fn_link_name_tests {
    use super::llvm_fn_link_name;

    #[test]
    fn windows_crt_collision_gets_nyra_prefix() {
        assert_eq!(
            llvm_fn_link_name("atoi", "x86_64-pc-windows-gnu"),
            "nyra_atoi"
        );
    }

    #[test]
    fn unix_triple_keeps_source_name() {
        assert_eq!(llvm_fn_link_name("atoi", "aarch64-apple-darwin"), "atoi");
    }
}

#[cfg(test)]
mod escape_tests {
    use super::escape_string;

    #[test]
    pub(super) fn escape_string_uses_utf8_bytes_for_non_ascii() {
        assert_eq!(escape_string("█"), "\\E2\\96\\88");
        assert_eq!(escape_string("╗"), "\\E2\\95\\97");
        assert_eq!(escape_string("Nyra"), "Nyra");
    }

    #[test]
    pub(super) fn escape_string_escapes_ascii_controls() {
        assert_eq!(escape_string("a\\b\"c\nd"), "a\\\\b\\22c\\0Ad");
    }

    #[test]
    pub(super) fn llvm_ptr_reg_formats_bare_ssa_numbers() {
        assert_eq!(super::llvm_ptr_reg("0"), "%0");
        assert_eq!(super::llvm_ptr_reg("%1"), "%1");
        assert_eq!(super::llvm_ptr_reg("null"), "null");
    }
}

use std::collections::{HashMap, HashSet};

use ast::*;
use types::monomorph_inst_name;

pub(super) fn is_array_ty(ty: &str) -> bool {
    ty.starts_with('[')
}

pub(super) fn llvm_storage_ty(ty: &str) -> &str {
    match ty {
        "char" => "i32",
        "string" | "vec_str" | "bytes" | "join_handle" => "ptr",
        _ => ty,
    }
}

pub(super) fn is_float_llvm_ty(ty: &str) -> bool {
    matches!(ty, "float" | "double" | "f32" | "f64")
}

pub(super) fn llvm_float_storage_ty(ty: &str) -> &str {
    match ty {
        "float" | "f32" => "float",
        "double" | "f64" => "double",
        _ => ty,
    }
}

/// Typed zero for `add ty ZERO, val` materialization of literal SSA bindings.
pub(super) fn llvm_typed_zero(storage_ty: &str) -> &'static str {
    if is_float_llvm_ty(storage_ty) {
        "0.0"
    } else {
        "0"
    }
}

/// Integer `add` vs floating `fadd` when materializing literal SSA bindings.
pub(super) fn llvm_scalar_materialize_op(storage_ty: &str) -> &'static str {
    if is_float_llvm_ty(storage_ty) {
        "fadd"
    } else {
        "add"
    }
}

/// LLVM constant operand for materializing a scalar literal into SSA (`fadd ty 0.0, lit`).
pub(super) fn llvm_materialize_scalar_literal(storage_ty: &str, raw: &str) -> String {
    match llvm_float_storage_ty(storage_ty) {
        "double" => {
            if raw.contains('e') || raw.contains('E') {
                raw.to_string()
            } else {
                llvm_float_const(raw.parse().unwrap_or(0.0), FloatKind::F64)
            }
        }
        "float" => {
            if raw.contains('e') || raw.contains('E') {
                raw.to_string()
            } else {
                llvm_float_const(raw.parse().unwrap_or(0.0), FloatKind::F32)
            }
        }
        _ => raw.to_string(),
    }
}

/// Operand for `icmp`/`fcmp`/`add`/etc.: `add i32 %a, 1` not `add i32 i32 %a, i32 1`.
pub(super) fn llvm_binop_operand(reg: &str) -> String {
    if reg.starts_with('%') || reg.starts_with('@') {
        reg.to_string()
    } else {
        reg.to_string()
    }
}

pub(super) fn llvm_cmp_operand(reg: &str) -> String {
    llvm_binop_operand(reg)
}

pub(super) fn llvm_arith_rhs(ty: &str, left: &str, right: &str) -> String {
    let storage = if ty == "double" { "double" } else { llvm_storage_ty(ty) };
    format!(
        "{storage} {}, {}",
        llvm_binop_operand(left),
        llvm_binop_operand(right)
    )
}

pub(super) fn llvm_ptr(ty: &str) -> String {
    let storage = llvm_storage_ty(ty);
    if storage == "ptr" {
        "ptr".into()
    } else {
        format!("{storage}*")
    }
}

/// Element type when loading through a reference/pointer LLVM type (`i32*` → `i32`, `ptr` → `i32`).
pub(super) fn llvm_pointee_ty(ty: &str) -> String {
    let t = ty.trim_start_matches('%');
    if t == "ptr" || t == "i8*" {
        return "i32".into();
    }
    if let Some(base) = t.strip_suffix('*') {
        if !base.is_empty() {
            return base.to_string();
        }
    }
    t.to_string()
}

pub(super) fn llvm_ty_to_ann(ty: &str) -> TypeAnnotation {
    if let Some(k) = IntKind::parse_name(ty) {
        return TypeAnnotation::Integer(k);
    }
    match ty {
        "float" => TypeAnnotation::F32,
        "double" => TypeAnnotation::F64,
        "char" => TypeAnnotation::Char,
        "i1" => TypeAnnotation::Bool,
        "ptr" => TypeAnnotation::String,
        t if t.starts_with('%') => TypeAnnotation::Struct(t.trim_start_matches('%').to_string()),
        _ => TypeAnnotation::Integer(IntKind::I32),
    }
}

pub(super) fn struct_value_type(ty: &str) -> String {
    if ty.starts_with('%') && ty.ends_with('*') {
        ty.trim_end_matches('*').to_string()
    } else {
        ty.to_string()
    }
}

pub(super) fn is_struct_pointer_type(ty: &str) -> bool {
    ty.starts_with('%') && ty.ends_with('*')
}

pub(super) fn struct_ptr_type(struct_ty: &str) -> String {
    if struct_ty.starts_with('%') {
        format!("{struct_ty}*")
    } else {
        format!("%{struct_ty}*")
    }
}

pub(super) fn struct_name_from_llvm_ty(ty: &str) -> Option<String> {
    if ty.ends_with('*') {
        Some(ty.trim_start_matches('%').trim_end_matches('*').to_string())
    } else if ty.starts_with('%') {
        Some(ty.trim_start_matches('%').to_string())
    } else {
        None
    }
}

/// LLVM type for function parameters (structs passed by pointer).
pub(super) fn llvm_type_ann_resolved(
    ty: &TypeAnnotation,
    structs: &HashMap<String, Vec<(String, TypeAnnotation)>>,
    enum_names: &std::collections::HashSet<String>,
) -> String {
    match ty {
        TypeAnnotation::Integer(k) => k.llvm_name().into(),
        TypeAnnotation::F32 => "float".into(),
        TypeAnnotation::F64 => "double".into(),
        TypeAnnotation::Char => "i32".into(),
        TypeAnnotation::Bool => "i1".into(),
        TypeAnnotation::String => "ptr".into(),
        TypeAnnotation::Bytes => "bytes".into(),
        TypeAnnotation::VecStr => "ptr".into(),
        TypeAnnotation::Ptr | TypeAnnotation::RawPtr { .. } => "ptr".into(),
        TypeAnnotation::Void => "void".into(),
        TypeAnnotation::Struct(n) => {
            if enum_names.contains(n) {
                "i32".into()
            } else {
                format!("%{n}")
            }
        }
        TypeAnnotation::Enum(_) => "i32".into(),
        TypeAnnotation::Array { elem, len } => {
            let inner = llvm_type_ann_resolved(elem, structs, enum_names);
            let n = len.unwrap_or(0);
            format!("[{n} x {inner}]")
        }
        TypeAnnotation::Tuple(elems) => {
            let inner: Vec<String> = elems
                .iter()
                .map(|e| llvm_type_ann_resolved(e, structs, enum_names))
                .collect();
            format!("%Tuple{}_{}", elems.len(), inner.join("_").replace('*', "p"))
        }
        TypeAnnotation::Ref { inner, .. } => llvm_type_ann_resolved(inner, structs, enum_names),
        TypeAnnotation::Generic(_) => "i32".into(),
        TypeAnnotation::Lifetime(_) => "i8".into(),
        TypeAnnotation::ForAll { inner, .. } => llvm_type_ann_resolved(inner, structs, enum_names),
        TypeAnnotation::FnPtr { .. } => "ptr".into(),
        TypeAnnotation::DynTrait { trait_name, .. } => format!("%Dyn_{trait_name}"),
        TypeAnnotation::Applied { base, args } => {
            format!("%{}", monomorph_inst_name(base, args))
        }
        TypeAnnotation::Simd { elem, lanes } => {
            let inner = llvm_type_ann_resolved(elem, structs, enum_names);
            format!("<{lanes} x {inner}>")
        }
    }
}

pub(super) fn assign_target_name(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Variable { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// Mut bindings assigned in a loop body at this nesting level only (not inside nested loops).
pub(super) fn collect_loop_carried_in_block(block: &Block) -> HashSet<String> {
    let mut out = HashSet::new();
    for s in &block.statements {
        match s {
            Statement::Assign(a) => {
                if let Some(n) = assign_target_name(&a.target) {
                    out.insert(n.to_string());
                }
            }
            Statement::If(i) => {
                out.extend(collect_loop_carried_in_block(&i.then_block));
                if let Some(el) = &i.else_block {
                    out.extend(collect_loop_carried_in_block(el));
                }
            }
            Statement::While(_) | Statement::For(_) => {}
            _ => {}
        }
    }
    out
}

/// Mut bindings assigned anywhere in a block (including nested if/while/for).
pub(super) fn collect_assigned_in_block(block: &Block) -> HashSet<String> {
    let mut out = HashSet::new();
    for s in &block.statements {
        match s {
            Statement::Assign(a) => {
                if let Some(n) = assign_target_name(&a.target) {
                    out.insert(n.to_string());
                }
            }
            Statement::If(i) => {
                out.extend(collect_assigned_in_block(&i.then_block));
                if let Some(el) = &i.else_block {
                    out.extend(collect_assigned_in_block(el));
                }
            }
            Statement::While(w) => out.extend(collect_assigned_in_block(&w.body)),
            Statement::For(f) => out.extend(collect_assigned_in_block(&f.body)),
            _ => {}
        }
    }
    out
}

