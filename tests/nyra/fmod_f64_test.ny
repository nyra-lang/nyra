// [contrib-dev:fmod_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_fmod_f64() {
    let x = fmod_f64(5.0, 2.0)
    if x < 0.9 { assert_eq(1, 0) }
    if x > 1.1 { assert_eq(1, 0) }
}
// [/contrib-dev:fmod_f64:math]
