// [contrib-dev:format_i64_bin:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i64_bin() {
    assert_str_eq(format_i64_bin(5), "101")
}
// [/contrib-dev:format_i64_bin:strconv_mod]
