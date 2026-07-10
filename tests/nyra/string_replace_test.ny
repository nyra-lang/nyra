import "stdlib/testing.ny"

test fn test_string_replace() {
    assert_str_eq("a-b-a".replace("a", "x"), "x-b-x")
}
