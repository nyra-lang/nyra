use std::collections::HashMap;

use ast::{dyn_combo_key, dyn_struct_name, IntKind, TypeAnnotation};

pub mod float;
pub mod integer;
pub mod intrinsics;
pub mod layout;
pub mod simd_intrinsics;
pub use float::{
    float_assignable, float_display, float_kind_of, float_llvm, is_float, is_print_arg,
    is_print_scalar, type_from_float_kind, unify_float_types,
};
pub use intrinsics::{is_math_intrinsic_fn, resolve_math_intrinsic, MathIntrinsic};
pub use layout::{align_of_ann, layout_of_ann, parse_simd_type_name, size_of_ann, union_layout, LayoutDesc};
pub use simd_intrinsics::{is_layout_intrinsic_fn, is_simd_intrinsic_fn, resolve_simd_intrinsic, SimdIntrinsic};
pub use integer::{
    ann_from_int_kind, int_display, int_kind_of, int_kind_of_ann, int_literal_value, int_llvm,
    integer_assignable, integer_literal_fits, is_integer, is_integer_ann, is_numeric,
    type_from_int_kind, unify_integer_types, unify_numeric,
};

fn mangle_type_ann(t: &TypeAnnotation) -> String {
    match t {
        TypeAnnotation::Integer(k) => k.name().into(),
        TypeAnnotation::F32 => "f32".into(),
        TypeAnnotation::F64 => "f64".into(),
        TypeAnnotation::Char => "char".into(),
        TypeAnnotation::Bool => "bool".into(),
        TypeAnnotation::String => "string".into(),
        TypeAnnotation::Bytes => "bytes".into(),
        TypeAnnotation::VecStr => "vec_str".into(),
        TypeAnnotation::Ptr => "ptr".into(),
        TypeAnnotation::RawPtr { inner } => format!("raw_{}", mangle_type_ann(inner)),
        TypeAnnotation::Void => "void".into(),
        TypeAnnotation::Struct(n) => format!("S_{n}"),
        TypeAnnotation::Applied { base, args } => {
            let suffix: String = args.iter().map(mangle_type_ann).collect::<Vec<_>>().join("_");
            format!("{base}__{suffix}")
        }
        TypeAnnotation::Enum(n) => format!("E_{n}"),
        TypeAnnotation::Array { elem, len } => format!(
            "A{}_{}",
            len.map(|n| n.to_string()).unwrap_or_else(|| "x".into()),
            mangle_type_ann(elem)
        ),
        TypeAnnotation::Tuple(elems) => format!(
            "T{}_{}",
            elems.len(),
            elems.iter().map(mangle_type_ann).collect::<Vec<_>>().join("_")
        ),
        TypeAnnotation::Ref { inner, mutable, .. } => format!(
            "R{}{}",
            if *mutable { "mut" } else { "imm" },
            mangle_type_ann(inner)
        ),
        TypeAnnotation::Generic(n) => n.clone(),
        TypeAnnotation::Lifetime(_) => "lt".into(),
        TypeAnnotation::ForAll { inner, .. } => mangle_type_ann(inner),
        TypeAnnotation::FnPtr { .. } => "fnptr".into(),
        TypeAnnotation::DynTrait { traits, .. } => format!("dyn_{}", dyn_combo_key(&traits)),
        TypeAnnotation::Simd { elem, lanes } => {
            format!("simd_{}_{}", mangle_type_ann(elem), lanes)
        }
    }
}

fn collection_struct_alias(base: &str, type_args: &[TypeAnnotation]) -> Option<String> {
    match (base, type_args) {
        ("Vec", [TypeAnnotation::String]) => Some("StrVec".into()),
        ("HashMap", [TypeAnnotation::String, TypeAnnotation::Integer(_)]) => {
            Some("HashMap_str_i32".into())
        }
        ("HashMap", [TypeAnnotation::String, TypeAnnotation::String]) => {
            Some("HashMap_str_str".into())
        }
        ("Future", [TypeAnnotation::Integer(_)]) => Some("Future_i32".into()),
        ("Future", [TypeAnnotation::Bool]) => Some("Future_bool".into()),
        ("Future", [TypeAnnotation::String]) => Some("Future_string".into()),
        _ => None,
    }
}

/// Monomorph instantiation name for `Base<Args…>` (matches `monomorph` struct names).
pub fn monomorph_inst_name(base: &str, type_args: &[TypeAnnotation]) -> String {
    if let Some(alias) = collection_struct_alias(base, type_args) {
        return alias;
    }
    if type_args.is_empty() {
        base.to_string()
    } else {
        let suffix: String = type_args
            .iter()
            .map(mangle_type_ann)
            .collect::<Vec<_>>()
            .join("_");
        format!("{base}__{suffix}")
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnumVariantInfo {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone, Default)]
pub struct EnumInfo {
    pub variants: Vec<EnumVariantInfo>,
}

/// `Option` / `Result` patterns may use the generic name (`Result`) against a monomorph scrutinee (`Result_i32_i32`, `Result__i32__i32`, …).
pub fn enum_pattern_matches(pattern_enum: &str, scrutinee_enum: &str) -> bool {
    if pattern_enum == scrutinee_enum {
        return true;
    }
    scrutinee_enum.starts_with(&format!("{pattern_enum}__"))
        || scrutinee_enum.starts_with(&format!("{pattern_enum}_"))
}

impl EnumInfo {
    pub fn variant_names(&self) -> Vec<String> {
        self.variants.iter().map(|v| v.name.clone()).collect()
    }

    pub fn has_payload(&self) -> bool {
        self.variants.iter().any(|v| !v.fields.is_empty())
    }

    pub fn payload_type(&self) -> Option<Type> {
        self.variants.iter().find_map(|v| v.fields.first().cloned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Integer(IntKind),
    F32,
    F64,
    Char,
    Bool,
    String,
    /// Binary blob — opaque handle, distinct from UTF-8 `string`.
    Bytes,
    Ptr,
    /// Typed raw pointer `*T` — address of `T` (unsafe deref/arith).
    RawPtr {
        inner: Box<Type>,
    },
    /// Opaque runtime handle (LLVM `ptr`); Copy (e.g. channels).
    Handle,
    /// Thread join handle returned by `spawn { }` (LLVM `ptr`).
    JoinHandle,
    /// `ptr`-backed vector of strings (e.g. `str.split(sep)`).
    VecStr,
    /// Portable SIMD vector (`i32x4`, `f32x4`, …).
    Simd {
        elem: Box<Type>,
        lanes: usize,
    },
    Void,
    Struct(String),
    /// C-style union — fields overlap at offset 0.
    Union(String),
    Enum(String),
    Array {
        elem: Box<Type>,
        len: Option<usize>,
    },
    Tuple {
        elems: Vec<Type>,
    },
    Ref {
        inner: Box<Type>,
        mutable: bool,
        lifetime: Option<String>,
    },
    Generic(String),
    ForAll {
        lifetimes: Vec<String>,
        inner: Box<Type>,
    },
    FnPtr {
        lifetime_params: Vec<String>,
        params: Vec<Type>,
        return_type: Option<Box<Type>>,
    },
    Unknown,
}

impl From<TypeAnnotation> for Type {
    fn from(t: TypeAnnotation) -> Self {
        match t {
            TypeAnnotation::Integer(k) => Type::Integer(k),
            TypeAnnotation::F32 => Type::F32,
            TypeAnnotation::F64 => Type::F64,
            TypeAnnotation::Char => Type::Char,
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::String => Type::String,
            TypeAnnotation::Bytes => Type::Bytes,
            TypeAnnotation::VecStr => Type::VecStr,
            TypeAnnotation::Ptr => Type::Ptr,
            TypeAnnotation::RawPtr { inner } => Type::RawPtr {
                inner: Box::new((*inner).into()),
            },
            TypeAnnotation::Void => Type::Void,
            TypeAnnotation::Struct(n) => Type::Struct(n),
            TypeAnnotation::Simd { elem, lanes } => Type::Simd {
                elem: Box::new((*elem).clone().into()),
                lanes,
            },
            TypeAnnotation::Applied { base, args } => {
                Type::Struct(mangle_type_ann(&TypeAnnotation::Applied {
                    base,
                    args,
                }))
            }
            TypeAnnotation::Enum(n) => Type::Enum(n),
            TypeAnnotation::Array { elem, len } => Type::Array {
                elem: Box::new((*elem).into()),
                len,
            },
            TypeAnnotation::Tuple(elems) => Type::Tuple {
                elems: elems.iter().map(|e| Type::from(e.clone())).collect(),
            },
            TypeAnnotation::Ref {
                inner,
                mutable,
                lifetime,
            } => Type::Ref {
                inner: Box::new((*inner).into()),
                mutable,
                lifetime: lifetime.clone(),
            },
            TypeAnnotation::Lifetime(lt) => Type::Ref {
                inner: Box::new(Type::Unknown),
                mutable: false,
                lifetime: Some(lt),
            },
            TypeAnnotation::ForAll { lifetimes, inner } => Type::ForAll {
                lifetimes: lifetimes.clone(),
                inner: Box::new((*inner).into()),
            },
            TypeAnnotation::FnPtr {
                lifetime_params,
                params,
                return_type,
            } => Type::FnPtr {
                lifetime_params: lifetime_params.clone(),
                params: params.iter().map(|p| p.clone().into()).collect(),
                return_type: return_type
                    .as_ref()
                    .map(|t| Box::new((**t).clone().into())),
            },
            TypeAnnotation::DynTrait { traits, .. } => Type::Struct(dyn_struct_name(&traits)),
            TypeAnnotation::Generic(n) => Type::Generic(n),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StructInfo {
    pub fields: HashMap<String, Type>,
    pub field_anns: HashMap<String, TypeAnnotation>,
    pub field_order: Vec<String>,
    pub repr_c: bool,
    pub align: Option<u32>,
    pub packed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UnionInfo {
    pub fields: HashMap<String, Type>,
    pub field_anns: HashMap<String, TypeAnnotation>,
    pub field_order: Vec<String>,
    pub repr_c: bool,
    pub align: Option<u32>,
    pub packed: bool,
}

pub fn literal_type(lit: &ast::Literal) -> Type {
    match lit {
        ast::Literal::Int(_) => Type::Integer(IntKind::default_literal()),
        ast::Literal::IntKind(_, k) => Type::Integer(*k),
        ast::Literal::Float(_, k) => crate::float::type_from_float_kind(*k),
        ast::Literal::Char(_) => Type::Char,
        ast::Literal::Bool(_) => Type::Bool,
        ast::Literal::String(_) => Type::String,
    }
}
