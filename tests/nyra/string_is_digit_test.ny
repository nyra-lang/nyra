import "stdlib/testing.ny"

test fn test_string_is_digit() {
    assert_eq("123".is_digit(), 1)
    assert_eq("12a".is_digit(), 0)
}
