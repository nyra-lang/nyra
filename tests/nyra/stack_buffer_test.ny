import "stdlib/buf/stack.ny"

test fn test_stack_buffer_fill_get() {
    let buf = StackBuffer_i32_64_new()
    assert_eq(StackBuffer_i32_64_len(buf), 64)
    let filled = StackBuffer_i32_64_fill(buf, 7)
    assert_eq(StackBuffer_i32_64_get(filled, 0), 7)
}

fn main() {
    print(0)
}
