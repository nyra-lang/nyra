// [contrib-dev:f64_to_string_prec:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_f64_to_string_prec() {
    assert_str_eq(f64_to_string_prec(1.0, 1), "1.0")
    assert_str_eq(f64_to_string_prec(3.14159, 2), "3.14")
}
// [/contrib-dev:f64_to_string_prec:strconv_mod]
