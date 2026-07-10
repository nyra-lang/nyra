import "stdlib/testing.ny"

test fn test_string_trim() {
    assert_str_eq("  hi  ".trim(), "hi")
}
