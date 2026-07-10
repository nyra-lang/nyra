// [contrib-dev:i64_to_string:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_i64_to_string() {
    assert_str_eq(i64_to_string(42), "42")
}
// [/contrib-dev:i64_to_string:strconv_mod]
