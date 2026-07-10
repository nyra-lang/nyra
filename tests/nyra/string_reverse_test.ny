import "stdlib/testing.ny"

test fn test_string_reverse() {
    assert_str_eq("abc".reverse(), "cba")
    assert_str_eq("".reverse(), "")
}
