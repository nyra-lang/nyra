// Compile-time layout helpers — `size_of` / `align_of` compiler intrinsics.

fn size_of_i32() -> i32 {
    return size_of<i32>()
}

fn size_of_bool() -> i32 {
    return size_of<bool>()
}

fn size_of_ptr() -> i32 {
    return size_of<ptr>()
}

fn align_of_i32() -> i32 {
    return align_of<i32>()
}

fn align_of_ptr() -> i32 {
    return align_of<ptr>()
}

fn align_of_i64() -> i32 {
    return align_of<i64>()
}
