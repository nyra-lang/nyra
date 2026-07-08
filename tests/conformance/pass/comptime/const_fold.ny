import "stdlib/testing.ny"

#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

test fn conf_comptime_001_fn_attr_fold() {
    assert_eq(SEED, 42)
}

test fn conf_comptime_002_const_arithmetic() {
    const N = 2 + 3
    assert_eq(N, 5)
}
