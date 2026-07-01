// Portable SIMD vectors — LLVM vector types.

extern fn simd_add_i32x4(a: i32x4, b: i32x4) -> i32x4
extern fn simd_mul_i32x4(a: i32x4, b: i32x4) -> i32x4
extern fn simd_splat_i32x4(v: i32) -> i32x4
extern fn simd_load_i32x4(p: ptr) -> i32x4
extern fn simd_store_i32x4(p: ptr, v: i32x4) -> void

extern fn simd_add_f32x4(a: f32x4, b: f32x4) -> f32x4
extern fn simd_splat_f32x4(v: f32) -> f32x4

extern fn simd_add_f64x2(a: f64x2, b: f64x2) -> f64x2
extern fn simd_splat_f64x2(v: f64) -> f64x2
