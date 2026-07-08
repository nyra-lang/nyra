struct Packet repr(C) align(8) {
    kind: i32
    port: i32
}

union Value repr(C) {
    as_i32: i32
    as_f64: f64
}

import "stdlib/mem/layout.ny"

test fn test_scalar_size_of() {
    assert_eq(size_of_i32(), 4)
    assert_eq(align_of_ptr(), 8)
}

test fn test_struct_size_of() {
    assert_eq(size_of<Packet>(), 8)
    assert_eq(align_of<Packet>(), 8)
}

test fn test_union_size_of() {
    assert_eq(size_of<Value>(), 8)
}

fn main() {
    print(0)
}
