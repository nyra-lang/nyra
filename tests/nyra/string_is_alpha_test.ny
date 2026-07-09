import "stdlib/testing.ny"

test fn test_string_is_alpha() {
    assert_eq("abc".is_alpha(), 1)
    assert_eq("ab1".is_alpha(), 0)
}
