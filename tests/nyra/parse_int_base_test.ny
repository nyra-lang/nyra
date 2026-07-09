// [contrib-dev:parse_int_base:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_parse_int_base() {
    assert_eq(parse_int_base("10", 16), 16)
}
// [/contrib-dev:parse_int_base:strconv_mod]
