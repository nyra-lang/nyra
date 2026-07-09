import "stdlib/testing.ny"

test fn test_string_collapse_ws() {
    assert_str_eq("  a   b  ".collapse_ws(), "a b")
}
