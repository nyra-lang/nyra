import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_split_once() {
    assert_str_eq("a=b=c".split_once("="), "a")
    assert_str_eq("abc".split_once("="), "abc")
}
