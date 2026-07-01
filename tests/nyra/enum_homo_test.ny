enum Result_i32_i32 {
    Ok(i32)
    Err(i32)
}

test fn test_homogeneous_result() {
    let err = Result_i32_i32.Err(42)
    let v = match err {
        Result_i32_i32.Ok(x) => x,
        Result_i32_i32.Err(e) => e,
    }
    assert_eq(v, 42)
}

fn main() {
    print(0)
}
