// [contrib-dev:signum_i32:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_signum_i32() {
    assert_eq(signum_i32(5), 1)
    assert_eq(signum_i32(-3), -1)
    assert_eq(signum_i32(0), 0)
}
// [/contrib-dev:signum_i32:math]
