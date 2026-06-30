// nyra test tests/nyra/string_borrow_no_clone_test.ny
import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_strcat_reuses_bindings() {
    let a = "hello"
    let b = " world"
    let c = strcat(a, b)
    assert_str_eq(c, "hello world")
    assert_str_eq(a, "hello")
    assert_str_eq(b, " world")
}

test fn test_substring_twice_same_source() {
    let s = "abcdef"
    let left = substring(s, 0, 3)
    let right = substring(s, 3, 3)
    assert_str_eq(left, "abc")
    assert_str_eq(right, "def")
    assert_str_eq(s, "abcdef")
}
