import "stdlib/testing.ny"

test fn conf_i64_literal_typed() {
    let big: i64 = 1_000_000_000
    assert_eq(big, 1000000000)
}
