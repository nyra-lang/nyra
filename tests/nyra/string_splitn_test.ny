import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_splitn() {
    let parts = "a,b,c".splitn(",", 2)
    assert_eq(parts.len(), 2)
}
