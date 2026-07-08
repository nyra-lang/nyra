import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_index() {
    assert_eq("hamdy.txt".index("x"), 7)
    assert_eq(index("hello", "ell"), 1)
    assert_eq("abc".index("z"), -1)
}
