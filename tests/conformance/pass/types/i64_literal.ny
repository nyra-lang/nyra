import "stdlib/testing.ny"

test fn conf_i64_literal() {
    let big = 1_000_000_000
    assert_eq(big, 1000000000)
}
