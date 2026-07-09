enum Option_i32 {
    None,
    Some(i32),
}

enum Result_i32_i32 {
    Ok(i32),
    Err(i32),
}

fn Option_i32_some(v: i32) -> Option_i32 {
    return Option_i32.Some(v)
}

fn Option_i32_none() -> Option_i32 {
    return Option_i32.None
}

fn Option_i32_is_some(opt: Option_i32) -> i32 {
    return match opt {
        Option_i32.Some(_v) => 1
        Option_i32.None => 0
    }
}

fn Result_i32_i32_ok(v: i32) -> Result_i32_i32 {
    return Result_i32_i32.Ok(v)
}

fn Result_i32_i32_err(e: i32) -> Result_i32_i32 {
    return Result_i32_i32.Err(e)
}

fn Result_i32_i32_is_ok(r: Result_i32_i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(_v) => 1
        Result_i32_i32.Err(_e) => 0
    }
}

fn unwrap_i32_result(r: Result_i32_i32, default_val: i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(v) => v
        Result_i32_i32.Err(_e) => default_val
    }
}


