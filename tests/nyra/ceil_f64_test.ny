import "stdlib/testing.ny"
import "stdlib/math.ny"

test fn test_ceil_f64() {
    let x = ceil_f64(1.2)
    if x < 2.0 {
        assert_eq(1, 0)
    }
}
