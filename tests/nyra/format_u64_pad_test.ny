// [contrib-dev:format_u64_pad:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_format_u64_pad() {
    assert_str_eq(format_u64_pad(7, 3), "007")
}
// [/contrib-dev:format_u64_pad:strconv_mod]
