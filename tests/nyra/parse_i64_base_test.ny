// [contrib-dev:parse_i64_base:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_parse_i64_base() {
    assert_eq(parse_i64_base("ff", 16), 255)
}
// [/contrib-dev:parse_i64_base:strconv_mod]
