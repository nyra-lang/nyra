// [contrib-dev:deg_to_rad_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_deg_to_rad_f64() {
    let x = deg_to_rad_f64(1.0)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:deg_to_rad_f64:math]
