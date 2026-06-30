use ast::FloatKind;

use crate::Type;

pub fn is_float(ty: &Type) -> bool {
    matches!(ty, Type::F32 | Type::F64)
}

pub fn float_kind_of(ty: &Type) -> Option<FloatKind> {
    match ty {
        Type::F32 => Some(FloatKind::F32),
        Type::F64 => Some(FloatKind::F64),
        _ => None,
    }
}

pub fn type_from_float_kind(k: FloatKind) -> Type {
    match k {
        FloatKind::F32 => Type::F32,
        FloatKind::F64 => Type::F64,
    }
}

pub fn unify_float_types(left: Type, right: Type) -> Type {
    match (float_kind_of(&left), float_kind_of(&right)) {
        (Some(a), Some(b)) => type_from_float_kind(FloatKind::unify(a, b)),
        (Some(a), None) | (None, Some(a)) => type_from_float_kind(a),
        _ => Type::F64,
    }
}

/// `let x: f32 = 3.14` — default `f64` literals may bind to `f32` or `f64`.
pub fn float_assignable(declared: &Type, value: &Type) -> bool {
    float_kind_of(declared).is_some() && float_kind_of(value).is_some()
}

pub fn float_display(k: FloatKind) -> &'static str {
    k.name()
}

pub fn float_llvm(k: FloatKind) -> &'static str {
    k.llvm_name()
}

pub fn is_print_scalar(ty: &Type) -> bool {
    if let Type::Ref {
        inner,
        mutable: false,
        ..
    } = ty
    {
        return is_print_scalar(inner);
    }
    ty == &Type::String
        || crate::integer::is_integer(ty)
        || is_float(ty)
        || ty == &Type::Char
        || ty == &Type::Bool
        || ty == &Type::Unknown
}

/// Accepted by `print` / `write` / `println`: scalars or fixed arrays of printable scalars.
pub fn is_print_arg(ty: &Type) -> bool {
    if is_print_scalar(ty) {
        return true;
    }
    if let Type::Array { elem, len: Some(_) } = ty {
        return is_print_scalar(elem);
    }
    false
}
