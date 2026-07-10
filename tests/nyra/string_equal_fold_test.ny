import "stdlib/testing.ny"

test fn test_string_equal_fold() {
    assert_eq("Hello".equal_fold("hello"), 1)
    assert_eq("Hello".equal_fold("world"), 0)
}
