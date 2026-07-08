// Platform x86 SIMD intrinsics — use only after cpu feature checks, inside `unsafe`.

import "stdlib/os/cpu.ny"

extern fn simd_avx2_add_pd(a: f64x2, b: f64x2) -> f64x2

fn simd_avx2_add_pd_guarded(a: f64x2, b: f64x2) -> f64x2 {
    if cpu_has_avx2() {
        return simd_avx2_add_pd(a, b)
    }
    return a
}
