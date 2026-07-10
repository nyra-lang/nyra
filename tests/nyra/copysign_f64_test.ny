// [contrib-dev:copysign_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_copysign_f64() {
    let x = copysign_f64(1.0, 1.0)
    if x < 0.5 { assert_eq(1, 0) }
}
// [/contrib-dev:copysign_f64:math]
