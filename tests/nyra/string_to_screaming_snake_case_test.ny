import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_to_screaming_snake_case() {
    let s = "Hello World"
    let result = s.to_screaming_snake_case()
    assert_str_eq(result, "HELLO_WORLD")
}
