// [contrib-dev:trailing_zeros_i32:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_trailing_zeros_i32() {
    let x = trailing_zeros_i32(1)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:trailing_zeros_i32:math]
