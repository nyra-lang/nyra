// [contrib-dev:is_nan_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_is_nan_f64() {
    let x = is_nan_f64(3.0)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:is_nan_f64:math]
