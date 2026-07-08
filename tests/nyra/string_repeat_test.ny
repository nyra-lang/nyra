import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_repeat() {
    assert_str_eq("ab".repeat(3), "ababab")
    assert_str_eq(repeat("x", 0), "")
}
