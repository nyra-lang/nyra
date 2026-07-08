// [contrib-dev:hex_decode:encoding_mod]
import "stdlib/testing.ny"
import "stdlib/encoding/mod.ny"

test fn test_hex_decode() {
    assert_str_eq(hex_decode("4869"), "Hi")
}
// [/contrib-dev:hex_decode:encoding_mod]
