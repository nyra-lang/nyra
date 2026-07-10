import "stdlib/testing.ny"

test fn test_string_contains() {
    assert_eq("hello".contains("ell"), 1)
    assert_eq("hello".contains("xyz"), 0)
}
