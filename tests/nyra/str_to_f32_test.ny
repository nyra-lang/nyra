// [contrib-dev:str_to_f32:strconv_mod]
import "stdlib/testing.ny"
import "stdlib/strconv/mod.ny"

test fn test_str_to_f32() {
    let x = str_to_f32("2.5")
    if x < 2.4 { assert_eq(1, 0) }
    if x > 2.6 { assert_eq(1, 0) }
}
// [/contrib-dev:str_to_f32:strconv_mod]
