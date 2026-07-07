import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_to_capitalize() {
    let s = "hELLO world"
    let result = s.to_capitalize()
    assert_str_eq(result, "Hello world")
}
