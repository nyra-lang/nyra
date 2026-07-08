enum Result_str_i32 {
    Ok(string)
    Err(i32)
}

test fn test_err_branch() {
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
