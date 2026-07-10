// [contrib-dev:format_i32_pad:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i32_pad() {
    assert_str_eq(format_pad(7, 3), "007")
    assert_str_eq(format_i32_pad(42, 2), "42")
}
// [/contrib-dev:format_i32_pad:strconv_mod]
