// [contrib-dev:str_to_f64:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_str_to_f64() {
    let x = str_to_f64("x")
    if x < 0.0 { assert_eq(1, 0) }
}
// [/contrib-dev:str_to_f64:strconv_mod]
