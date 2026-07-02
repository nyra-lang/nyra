//! Compile-time layout: size and alignment for Nyra types.

use std::collections::HashMap;

use ast::{IntKind, TypeAnnotation};

use crate::{StructInfo, UnionInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutDesc {
    pub size: u64,
    pub align: u64,
}

fn align_up(size: u64, align: u64) -> u64 {
    if align == 0 {
        return size;
    }
    (size + align - 1) / align * align
}

fn scalar_layout(ann: &TypeAnnotation) -> LayoutDesc {
    match ann {
        TypeAnnotation::Integer(k) => {
            let bits = k.bits();
            let size = ((bits + 7) / 8) as u64;
            LayoutDesc { size, align: size }
        }
        TypeAnnotation::F32 => LayoutDesc { size: 4, align: 4 },
        TypeAnnotation::F64 => LayoutDesc { size: 8, align: 8 },
        TypeAnnotation::Char => LayoutDesc { size: 4, align: 4 },
        TypeAnnotation::Bool => LayoutDesc { size: 1, align: 1 },
        TypeAnnotation::String | TypeAnnotation::Bytes | TypeAnnotation::Ptr | TypeAnnotation::VecStr => {
            LayoutDesc { size: 8, align: 8 }
        }
        TypeAnnotation::RawPtr { .. } => LayoutDesc { size: 8, align: 8 },
        TypeAnnotation::Void => LayoutDesc { size: 0, align: 1 },
        TypeAnnotation::Simd { elem, lanes } => {
            let elem_layout = scalar_layout(elem);
            let size = elem_layout.size * (*lanes as u64);
            LayoutDesc {
                size,
                align: elem_layout.align.max(16),
            }
        }
        _ => LayoutDesc { size: 4, align: 4 },
    }
}

pub fn layout_of_ann(
    ann: &TypeAnnotation,
    structs: &HashMap<String, StructInfo>,
    unions: &HashMap<String, UnionInfo>,
) -> LayoutDesc {
    match ann {
        TypeAnnotation::Integer(_)
        | TypeAnnotation::F32
        | TypeAnnotation::F64
        | TypeAnnotation::Char
        | TypeAnnotation::Bool
        | TypeAnnotation::String
        | TypeAnnotation::Bytes
        | TypeAnnotation::Ptr
        | TypeAnnotation::VecStr
        | TypeAnnotation::Void => scalar_layout(ann),
        TypeAnnotation::RawPtr { inner } => layout_of_ann(inner, structs, unions),
        TypeAnnotation::Simd { elem, lanes } => {
            let elem_layout = layout_of_ann(elem, structs, unions);
            let size = elem_layout.size * (*lanes as u64);
            LayoutDesc {
                size,
                align: elem_layout.align.max(16),
            }
        }
        TypeAnnotation::Array { elem, len } => {
            let elem_layout = layout_of_ann(elem, structs, unions);
            let n = len.unwrap_or(0) as u64;
            LayoutDesc {
                size: elem_layout.size * n,
                align: elem_layout.align,
            }
        }
        TypeAnnotation::Struct(name) => {
            if let Some(info) = structs.get(name) {
                struct_layout(info)
            } else if let Some(info) = unions.get(name) {
                union_layout(info)
            } else {
                LayoutDesc { size: 8, align: 8 }
            }
        }
        TypeAnnotation::Applied { base, args } => {
            let mangled = crate::monomorph_inst_name(base, args);
            if let Some(info) = structs.get(&mangled) {
                struct_layout(info)
            } else {
                LayoutDesc { size: 8, align: 8 }
            }
        }
        TypeAnnotation::Enum(_) => LayoutDesc { size: 4, align: 4 },
        TypeAnnotation::Tuple(elems) => {
            let mut size = 0u64;
            let mut align = 1u64;
            for elem in elems {
                let l = layout_of_ann(elem, structs, unions);
                size = align_up(size, l.align);
                size += l.size;
                align = align.max(l.align);
            }
            LayoutDesc {
                size: align_up(size, align),
                align,
            }
        }
        TypeAnnotation::Ref { inner, .. }
        | TypeAnnotation::ForAll { inner, .. } => layout_of_ann(inner, structs, unions),
        TypeAnnotation::Generic(_) | TypeAnnotation::Lifetime(_) | TypeAnnotation::FnPtr { .. } => {
            LayoutDesc { size: 8, align: 8 }
        }
        TypeAnnotation::DynTrait { .. } => LayoutDesc { size: 16, align: 8 },
    }
}

pub fn struct_layout(info: &StructInfo) -> LayoutDesc {
    if info.packed {
        let mut size = 0u64;
        for field in &info.field_order {
            if let Some(ann) = info.field_anns.get(field) {
                let l = layout_of_ann(ann, &HashMap::new(), &HashMap::new());
                size += l.size;
            }
        }
        return LayoutDesc { size, align: 1 };
    }
    let mut size = 0u64;
    let mut align = info.align.map(|a| a as u64).unwrap_or(1);
    for field in &info.field_order {
        if let Some(ann) = info.field_anns.get(field) {
            let l = layout_of_ann(ann, &HashMap::new(), &HashMap::new());
            align = align.max(l.align);
            if info.repr_c {
                size = align_up(size, l.align);
            }
            size += l.size;
        }
    }
    let struct_align = info.align.map(|a| a as u64).unwrap_or(align);
    LayoutDesc {
        size: align_up(size, struct_align),
        align: struct_align,
    }
}

pub fn union_layout(info: &UnionInfo) -> LayoutDesc {
    let mut max_size = 0u64;
    let mut max_align = info.align.map(|a| a as u64).unwrap_or(1);
    for field in &info.field_order {
        if let Some(ann) = info.field_anns.get(field) {
            let l = layout_of_ann(ann, &HashMap::new(), &HashMap::new());
            max_size = max_size.max(l.size);
            max_align = max_align.max(l.align);
        }
    }
    let struct_align = info.align.map(|a| a as u64).unwrap_or(max_align);
    LayoutDesc {
        size: align_up(max_size, struct_align),
        align: struct_align,
    }
}

pub fn size_of_ann(
    ann: &TypeAnnotation,
    structs: &HashMap<String, StructInfo>,
    unions: &HashMap<String, UnionInfo>,
) -> i64 {
    layout_of_ann(ann, structs, unions).size as i64
}

pub fn align_of_ann(
    ann: &TypeAnnotation,
    structs: &HashMap<String, StructInfo>,
    unions: &HashMap<String, UnionInfo>,
) -> i64 {
    layout_of_ann(ann, structs, unions).align as i64
}

pub fn parse_simd_type_name(name: &str) -> Option<TypeAnnotation> {
    let (base, lanes_s) = name.rsplit_once('x')?;
    let lanes: usize = lanes_s.parse().ok()?;
    if !matches!(lanes, 2 | 4 | 8 | 16) {
        return None;
    }
    let elem = match base {
        "f32" => TypeAnnotation::F32,
        "f64" => TypeAnnotation::F64,
        _ => {
            let kind = IntKind::parse_name(base)?;
            TypeAnnotation::Integer(kind)
        }
    };
    Some(TypeAnnotation::Simd {
        elem: Box::new(elem),
        lanes,
    })
}
