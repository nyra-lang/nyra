import "stdlib/testing.ny"

test fn test_string_escape_json() {
    assert_str_eq("a\"b".escape_json(), "a\\\"b")
    assert_str_eq("\n".escape_json(), "\\n")
}
