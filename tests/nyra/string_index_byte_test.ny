import "stdlib/testing.ny"

test fn test_string_index_byte() {
    assert_eq("hello".index_byte(101), 1)
    assert_eq("hello".index_byte(120), -1)
}
