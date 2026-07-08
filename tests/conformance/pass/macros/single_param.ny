import "stdlib/testing.ny"

macro dbl(x) { x + x }

test fn conf_macro_003_single_param() {
    assert_eq(dbl(4), 8)
}

test fn conf_macro_004_expr_body() {
    assert_eq(dbl(10), 20)
}
