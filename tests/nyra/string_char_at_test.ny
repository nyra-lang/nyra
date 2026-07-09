import "stdlib/testing.ny"

test fn test_string_char_at() {
    assert_eq("abc".char_at(1), 98)
}
