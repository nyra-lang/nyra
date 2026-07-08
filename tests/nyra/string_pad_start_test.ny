import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_pad_start() {
    assert_str_eq("hi".pad_start(5, "0"), "000hi")
}
