import "stdlib/testing.ny"

test fn test_string_before_sep() {
    assert_str_eq("a:b".before_sep(":"), "a")
    assert_str_eq("abc".before_sep(":"), "abc")
}
