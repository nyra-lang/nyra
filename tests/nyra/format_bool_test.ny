// [contrib-dev:format_bool:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_bool() {
    assert_str_eq(format_bool(1), "true")
    assert_str_eq(format_bool(0), "false")
}
// [/contrib-dev:format_bool:strconv_mod]
