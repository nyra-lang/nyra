import "stdlib/testing.ny"

test fn test_string_starts_with() {
    assert_eq("hello".starts_with("he"), 1)
    assert_eq("hello".starts_with("lo"), 0)
}
