import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_strip_prefix() {
    let result = "prefix_hello".strip_prefix("prefix_")
    assert_str_eq(result, "hello")
    let result2 = strip_prefix("prefix_hello", "prefix_")
    assert_str_eq(result2, "hello")
    let unchanged = "hello".strip_prefix("x")
    assert_str_eq(unchanged, "hello")
}
