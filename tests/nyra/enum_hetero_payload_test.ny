enum Result_str_i32 {
    Ok(string)
    Err(i32)
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

fn main() {
    print(0)
}
