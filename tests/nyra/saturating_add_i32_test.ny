// [contrib-dev:saturating_add_i32:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_saturating_add_i32() {
    let x = saturating_add_i32(1, 1)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:saturating_add_i32:math]
