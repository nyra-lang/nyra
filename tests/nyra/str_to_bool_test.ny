// [contrib-dev:str_to_bool:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_str_to_bool() {
    assert_eq(parse_bool("true"), 1)
    assert_eq(parse_bool("false"), 0)
    assert_eq(parse_bool("yes"), 1)
}
// [/contrib-dev:str_to_bool:strconv_mod]
