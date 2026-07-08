import "stdlib/simd/mod.ny"

test fn test_simd_i32x4_add() {
    let a = simd_splat_i32x4(1)
    let b = simd_splat_i32x4(2)
    let c = simd_add_i32x4(a, b)
    let _ = c
}

test fn test_simd_f32x4_splat() {
    let v = simd_splat_f32x4(1.5f32)
    let _ = v
}

fn main() {
    print(0)
}
