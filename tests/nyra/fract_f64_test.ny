// [contrib-dev:fract_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_fract_f64() {
    let x = fract_f64(3.0)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:fract_f64:math]
