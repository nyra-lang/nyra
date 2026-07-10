// [contrib-dev:result_combinators:result_combinators]
import "../result.ny"

fn Result_i32_i32_map(r: Result_i32_i32, f: fn(i32) -> i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => Result_i32_i32.Ok(f(v))
        Result_i32_i32.Err(e) => Result_i32_i32.Err(e)
    }
}

fn Result_i32_i32_map_err(r: Result_i32_i32, f: fn(i32) -> i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => Result_i32_i32.Ok(v)
        Result_i32_i32.Err(e) => Result_i32_i32.Err(f(e))
    }
}

fn Result_i32_i32_and_then(r: Result_i32_i32, f: fn(i32) -> Result_i32_i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => f(v)
        Result_i32_i32.Err(e) => Result_i32_i32.Err(e)
    }
}

fn Result_i32_i32_unwrap_or(r: Result_i32_i32, default_val: i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(v) => v
        Result_i32_i32.Err(_e) => default_val
    }
}

fn Result_i32_i32_is_err(r: Result_i32_i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(_v) => 0
        Result_i32_i32.Err(_e) => 1
    }
}
// [/contrib-dev:result_combinators:result_combinators]

