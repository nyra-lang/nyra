import "stdlib/testing.ny"

test fn test_string_is_alnum() {
    assert_eq("abc123".is_alnum(), 1)
    assert_eq("abc-1".is_alnum(), 0)
}
