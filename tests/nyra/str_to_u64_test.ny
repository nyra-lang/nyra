// [contrib-dev:str_to_u64:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_str_to_u64() {
    assert_eq(str_to_u64("99"), 99)
}
// [/contrib-dev:str_to_u64:strconv_mod]
