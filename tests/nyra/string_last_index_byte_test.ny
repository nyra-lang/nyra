import "stdlib/testing.ny"

test fn test_string_last_index_byte() {
    assert_eq("hello".last_index_byte(108), 3)
    assert_eq("hello".last_index_byte(120), -1)
}
