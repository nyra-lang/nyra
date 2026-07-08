import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_last_index() {
    assert_eq("hamdy.txt".last_index("t"), 8)
    assert_eq("abcabc".last_index("bc"), 4)
    assert_eq("abc".last_index("z"), -1)
}
