import "stdlib/testing.ny"

test fn test_string_replacen() {
    assert_str_eq("a-b-a".replacen("a", "x", 1), "x-b-a")
}
