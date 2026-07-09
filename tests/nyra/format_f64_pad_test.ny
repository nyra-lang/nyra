// [contrib-dev:format_f64_pad:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_f64_pad() {
    assert_str_eq(format_f64_pad(3.14, 5, 2), " 3.14")
}
// [/contrib-dev:format_f64_pad:strconv_mod]
