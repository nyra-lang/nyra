extern fn hw_cpu_physical_cores() -> i32
extern fn hw_cpu_logical_cores() -> i32
extern fn hw_cpu_cache_line_size() -> i32
extern fn hw_cpu_has_sse42() -> i32
extern fn hw_cpu_has_avx() -> i32
extern fn hw_cpu_has_avx2() -> i32
extern fn hw_cpu_brand() -> string

// Logical CPUs — prefer the compiler builtin `cpu_count()` (no import).
// Import this module for extended CPU introspection (brand, AVX, cache line, …).

fn cpu_physical_cores() -> i32 {
    return hw_cpu_physical_cores()
}

fn cpu_logical_cores() -> i32 {
    return hw_cpu_logical_cores()
}

fn cpu_cache_line_size() -> i32 {
    return hw_cpu_cache_line_size()
}

fn cpu_has_sse42() -> bool {
    return hw_cpu_has_sse42() == 1
}

fn cpu_has_avx() -> bool {
    return hw_cpu_has_avx() == 1
}

fn cpu_has_avx2() -> bool {
    return hw_cpu_has_avx2() == 1
}

fn cpu_brand() -> string {
    return hw_cpu_brand()
}
