// [contrib-dev:rad_to_deg_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_rad_to_deg_f64() {
    let x = rad_to_deg_f64(1.0)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:rad_to_deg_f64:math]
