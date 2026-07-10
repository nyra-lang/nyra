import "stdlib/testing.ny"

test fn test_string_ends_with() {
    assert_eq("hello".ends_with("lo"), 1)
    assert_eq("hello".ends_with("he"), 0)
}
