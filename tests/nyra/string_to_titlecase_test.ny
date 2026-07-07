import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_to_titlecase() {
    let s = "hello world"
    let result = s.to_titlecase()
    assert_str_eq(result, "Hello World")
}
