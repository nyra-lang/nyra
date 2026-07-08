import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_pad_end() {
    assert_str_eq("hi".pad_end(5, "0"), "hi000")
}
