// [contrib-dev:hex_encode:encoding_mod]
import "stdlib/testing.ny"
import "stdlib/encoding/mod.ny"

test fn test_hex_encode() {
    assert_str_eq(hex_encode("Hi"), "4869")
}
// [/contrib-dev:hex_encode:encoding_mod]
