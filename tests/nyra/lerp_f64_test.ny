// [contrib-dev:lerp_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_lerp_f64() {
    let x = lerp_f64(0.0, 10.0, 0.5)
    if x < 4.9 { assert_eq(1, 0) }
    if x > 5.1 { assert_eq(1, 0) }
}
// [/contrib-dev:lerp_f64:math]
