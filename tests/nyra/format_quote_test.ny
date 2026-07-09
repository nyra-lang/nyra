// [contrib-dev:format_quote:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_quote() {
    assert_str_eq(format_quote("hi"), "\"hi\"")
}
// [/contrib-dev:format_quote:strconv_mod]
