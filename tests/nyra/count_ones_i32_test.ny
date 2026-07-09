// [contrib-dev:count_ones_i32:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_count_ones_i32() {
    let x = count_ones_i32(1)
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:count_ones_i32:math]
