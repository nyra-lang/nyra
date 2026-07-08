import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_trim_end() {
    assert_str_eq("hi  ".trim_end(), "hi")
    assert_str_eq(trim_end("hi  "), "hi")
}
