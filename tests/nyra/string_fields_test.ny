import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_fields() {
    let words = "a b c".fields()
    assert_eq(words.len(), 3)
}
