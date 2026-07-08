import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_is_empty() {
    assert_eq("".is_empty(), 1)
    assert_eq("x".is_empty(), 0)
}
