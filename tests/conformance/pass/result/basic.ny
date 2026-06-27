import "stdlib/testing.ny"

enum Result_i32_i32 {
    Ok(i32),
    Err(i32),
}

test fn conf_result_ok() {
    let v = Result_i32_i32.Ok(7)
    let n = match v {
        Result_i32_i32.Ok(x) => x
        Result_i32_i32.Err(_) => 0
    }
    assert_eq(n, 7)
}

test fn conf_result_err() {
    let v = Result_i32_i32.Err(3)
    let e = match v {
        Result_i32_i32.Ok(_) => 0
        Result_i32_i32.Err(x) => x
    }
    assert_eq(e, 3)
}
