import "stdlib/testing.ny"

fn answer() -> i32 {
    return 42
}

test fn conf_fn_return_value() {
    assert_eq(answer(), 42)
}
