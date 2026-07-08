import "stdlib/testing.ny"

macro add_pair(a, b) { a + b }

test fn conf_macro_001_two_param() {
    assert_eq(add_pair(2, 3), 5)
}

test fn conf_macro_002_different_args() {
    assert_eq(add_pair(10, 7), 17)
}
