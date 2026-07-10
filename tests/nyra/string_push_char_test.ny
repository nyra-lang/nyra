import "stdlib/testing.ny"

test fn test_string_push_char() {
    assert_str_eq("ab".push_char(99), "abc")
}
