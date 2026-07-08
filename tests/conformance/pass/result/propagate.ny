import "stdlib/testing.ny"

test fn conf_propagate_001_result_question() {
    let v = match run_ok_pipeline() {
        Result_i32_i32.Ok(x) => x
        Result_i32_i32.Err(_) => 0
    }
    assert_eq(v, 4)
}

enum Result_i32_i32 {
    Ok(i32),
    Err(i32),
}

fn ok_step(n: i32) -> Result_i32_i32 {
    return Result_i32_i32.Ok(n)
}

fn run_ok_pipeline() -> Result_i32_i32 {
    let a = ok_step(1)?
    let b = ok_step(a + 1)?
    return Result_i32_i32.Ok(b * 2)
}
