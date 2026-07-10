import "stdlib/testing.ny"

test fn test_string_substring() {
    assert_str_eq("hello".substring(1, 3), "ell")
}
