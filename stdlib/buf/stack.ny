// Fixed stack buffer — zero-cost wrapper over `[T; N]`.

struct StackBuffer_i32_64 {
    data: [i32; 64]
}

fn StackBuffer_i32_64_new() -> StackBuffer_i32_64 {
    let mut buf: [i32; 64] = [0; 64]
    return StackBuffer_i32_64 { data: buf }
}

fn StackBuffer_i32_64_len(_buf: StackBuffer_i32_64) -> i32 {
    return 64
}

fn StackBuffer_i32_64_fill(buf: StackBuffer_i32_64, value: i32) -> StackBuffer_i32_64 {
    let mut i = 0
    let mut out = buf
    while i < 64 {
        out.data[i] = value
        i = i + 1
    }
    return out
}

fn StackBuffer_i32_64_get(buf: StackBuffer_i32_64, index: i32) -> i32 {
    return buf.data[index]
}
