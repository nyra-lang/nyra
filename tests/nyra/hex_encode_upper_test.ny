// [contrib-dev:hex_encode_upper:encoding_mod]
import "stdlib/testing.ny"
import "stdlib/encoding/mod.ny"

test fn test_hex_encode_upper() {
    assert_str_eq(hex_encode_upper("Hi"), "4869")
}
// [/contrib-dev:hex_encode_upper:encoding_mod]
