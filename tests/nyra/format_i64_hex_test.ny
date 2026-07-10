// [contrib-dev:format_i64_hex:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i64_hex() {
    assert_str_eq(format_i64_hex(255), "ff")
}
// [/contrib-dev:format_i64_hex:strconv_mod]
