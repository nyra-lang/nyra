import "stdlib/testing.ny"

test fn test_string_compare() {
    assert_eq("abc".compare("abc"), 0)
    assert_eq(if "abc".compare("abd") < 0 { 1 } else { 0 }, 1)
}
