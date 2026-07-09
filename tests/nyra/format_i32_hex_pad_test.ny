// [contrib-dev:format_i32_hex_pad:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i32_hex_pad() {
    assert_str_eq(format_i32_hex_pad(255, 4), "00ff")
}
// [/contrib-dev:format_i32_hex_pad:strconv_mod]
