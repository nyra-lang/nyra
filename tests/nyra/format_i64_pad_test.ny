// [contrib-dev:format_i64_pad:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_i64_pad() {
    assert_str_eq(format_i64_pad(7, 3), "007")
}
// [/contrib-dev:format_i64_pad:strconv_mod]
