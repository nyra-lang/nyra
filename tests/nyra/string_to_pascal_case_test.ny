import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_to_pascal_case() {
    let s = "hello world"
    let result = s.to_pascal_case()
    assert_str_eq(result, "HelloWorld")
}
