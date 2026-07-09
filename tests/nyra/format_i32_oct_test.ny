// [contrib-dev:format_i32_oct:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i32_oct() {
    assert_str_eq(format_i32_oct(8), "10")
}
// [/contrib-dev:format_i32_oct:strconv_mod]
