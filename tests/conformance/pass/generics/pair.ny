import "stdlib/testing.ny"

fn add_i32(a: i32, b: i32) -> i32 {
    return a + b
}

fn add_str(a: string, b: string) -> string {
    return strcat(a, b)
}

test fn conf_generic_add_i32() {
    assert_eq(add_i32(2, 3), 5)
}

test fn conf_generic_add_str() {
    assert_str_eq(add_str("x", "y"), "xy")
}
