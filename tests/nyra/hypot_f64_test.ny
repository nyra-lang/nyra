import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_hypot_f64() {
    let x = hypot_f64(3.0, 4.0)
    if x < 2.0 {
        assert_eq(1, 0)
    }
}
