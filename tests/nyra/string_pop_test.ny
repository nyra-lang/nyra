import "stdlib/testing.ny"

test fn test_string_pop() {
    assert_str_eq("abc".pop(), "ab")
}
