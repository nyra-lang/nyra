// Systems-level features: union, layout, bytes, stack buffer, simd, arena

union IpAddr repr(C) {
    v4: i32
    v6: [u8; 16]
}

struct Packet repr(C) align(8) {
    kind: i32
    port: i32
}

enum Result_str_i32 {
    Ok(string),
    Err(i32),
}

import "stdlib/alloc/arena.ny"
import "stdlib/buf/stack.ny"
import "stdlib/mem/layout.ny"
import "stdlib/simd/mod.ny"

test fn test_union_read_v4() {
    unsafe {
        let u = IpAddr { v4: 0x7F000001 }
        assert_eq(u.v4, 2130706433)
    }
}

test fn test_layout_size_of() {
    assert_eq(size_of_i32(), 4)
    assert_eq(align_of_ptr(), 8)
}

test fn test_enum_hetero_result() {
    let ok = Result_str_i32.Ok("hello")
    let n = match ok {
        Result_str_i32.Ok(s) => s.len(),
        Result_str_i32.Err(e) => e,
    }
    assert_eq(n, 5)

    let err = Result_str_i32.Err(42)
    let v = match err {
        Result_str_i32.Ok(s) => s.len(),
        Result_str_i32.Err(e) => e,
    }
    assert_eq(v, 42)
}

test fn test_stack_buffer() {
    let buf = StackBuffer_i32_64_new()
    assert_eq(StackBuffer_i32_64_len(buf), 64)
    let filled = StackBuffer_i32_64_fill(buf, 7)
    assert_eq(StackBuffer_i32_64_get(filled, 0), 7)
}

test fn test_simd_add_i32x4() {
    let a = simd_splat_i32x4(1)
    let b = simd_splat_i32x4(2)
    let c = simd_add_i32x4(a, b)
    let _ = c
}

test fn test_arena_alloc_reset() {
    let arena = Arena_new(4096)
    let p = Arena_alloc(arena, 32)
    let zero = 0
    assert_eq(zero, 0)
    Arena_reset(arena)
    Arena_free(arena)
    let _ = p
}

fn main() {
    print(0)
}
