// [contrib-dev:i32_to_string_radix:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_i32_to_string_radix() {
    assert_str_eq(i32_to_string_radix(255, 16), "ff")
}
// [/contrib-dev:i32_to_string_radix:strconv_mod]
