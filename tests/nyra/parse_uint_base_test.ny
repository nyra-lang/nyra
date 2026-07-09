// [contrib-dev:parse_uint_base:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_parse_uint_base() {
    assert_eq(parse_uint_base("ff", 16), 255)
}
// [/contrib-dev:parse_uint_base:strconv_mod]
