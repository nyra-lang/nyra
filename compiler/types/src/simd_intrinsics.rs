//! Portable and platform SIMD intrinsics resolved at compile time.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdIntrinsic {
    // Portable i32x4
    AddI32x4,
    MulI32x4,
    SplatI32x4,
    LoadI32x4,
    StoreI32x4,
    // Portable f32x4
    AddF32x4,
    MulF32x4,
    SplatF32x4,
    LoadF32x4,
    StoreF32x4,
    // Portable f64x2
    AddF64x2,
    MulF64x2,
    SplatF64x2,
    LoadF64x2,
    StoreF64x2,
}

pub fn is_simd_intrinsic_fn(name: &str) -> bool {
    resolve_simd_intrinsic(name).is_some()
}

pub fn resolve_simd_intrinsic(name: &str) -> Option<SimdIntrinsic> {
    match name {
        "simd_add_i32x4" => Some(SimdIntrinsic::AddI32x4),
        "simd_mul_i32x4" => Some(SimdIntrinsic::MulI32x4),
        "simd_splat_i32x4" => Some(SimdIntrinsic::SplatI32x4),
        "simd_load_i32x4" => Some(SimdIntrinsic::LoadI32x4),
        "simd_store_i32x4" => Some(SimdIntrinsic::StoreI32x4),
        "simd_add_f32x4" => Some(SimdIntrinsic::AddF32x4),
        "simd_mul_f32x4" => Some(SimdIntrinsic::MulF32x4),
        "simd_splat_f32x4" => Some(SimdIntrinsic::SplatF32x4),
        "simd_load_f32x4" => Some(SimdIntrinsic::LoadF32x4),
        "simd_store_f32x4" => Some(SimdIntrinsic::StoreF32x4),
        "simd_add_f64x2" => Some(SimdIntrinsic::AddF64x2),
        "simd_mul_f64x2" => Some(SimdIntrinsic::MulF64x2),
        "simd_splat_f64x2" => Some(SimdIntrinsic::SplatF64x2),
        "simd_load_f64x2" => Some(SimdIntrinsic::LoadF64x2),
        "simd_store_f64x2" => Some(SimdIntrinsic::StoreF64x2),
        "simd_avx2_add_pd" => Some(SimdIntrinsic::AddF64x2),
        _ => None,
    }
}

pub fn is_layout_intrinsic_fn(name: &str) -> bool {
    matches!(name, "size_of" | "align_of")
}
