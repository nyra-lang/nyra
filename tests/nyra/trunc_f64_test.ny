// [contrib-dev:trunc_f64:math]
import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_trunc_f64() {
    let x = trunc_f64(1.9)
    if x < 1.0 {
        assert_eq(1, 0)
    }
    if x > 1.0 {
        assert_eq(1, 0)
    }
    let neg = 0.0 - 1.9
    let y = trunc_f64(neg)
    if y >= 0.0 {
        assert_eq(1, 0)
    }
}
// [/contrib-dev:trunc_f64:math]
