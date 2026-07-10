import "stdlib/testing.ny"

test fn test_string_truncate() {
    assert_str_eq("hello".truncate(3), "hel")
    assert_str_eq("hi".truncate(10), "hi")
}
