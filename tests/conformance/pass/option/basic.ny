import "stdlib/testing.ny"

enum Option_i32 {
    None,
    Some(i32),
}

test fn conf_option_some() {
    let v = Option_i32.Some(42)
    let n = match v {
        Option_i32.Some(x) => x
        Option_i32.None => 0
    }
    assert_eq(n, 42)
}

test fn conf_option_none() {
    let v = Option_i32.None
    let hit = match v {
        Option_i32.Some(_) => 1
        Option_i32.None => 0
    }
    assert_eq(hit, 0)
}
