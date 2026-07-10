// [contrib-dev:signum_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_signum_f64() {
    assert_eq(is_nan(signum_f64(0.0 / 0.0)), 0)
    if signum_f64(2.0) <= 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:signum_f64:math]
