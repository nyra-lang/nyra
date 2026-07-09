// [contrib-dev:option_combinators:option_combinators]
import "../result.ny"

fn Option_i32_map(opt: Option_i32, f: fn(i32) -> i32) -> Option_i32 {
    return match opt {
        Option_i32.Some(v) => Option_i32.Some(f(v))
        Option_i32.None => Option_i32.None
    }
}

fn Option_i32_and_then(opt: Option_i32, f: fn(i32) -> Option_i32) -> Option_i32 {
    return match opt {
        Option_i32.Some(v) => f(v)
        Option_i32.None => Option_i32.None
    }
}

fn Option_i32_unwrap_or(opt: Option_i32, default_val: i32) -> i32 {
    return match opt {
        Option_i32.Some(v) => v
        Option_i32.None => default_val
    }
}

fn Option_i32_is_none(opt: Option_i32) -> i32 {
    return match opt {
        Option_i32.Some(_v) => 0
        Option_i32.None => 1
    }
}

fn Option_i32_ok_or(opt: Option_i32, err: i32) -> Result_i32_i32 {
    return match opt {
        Option_i32.Some(v) => Result_i32_i32.Ok(v)
        Option_i32.None => Result_i32_i32.Err(err)
    }
}
// [/contrib-dev:option_combinators:option_combinators]

