import "stdlib/testing.ny"

test fn test_string_is_ascii() {
    assert_eq("hello".is_ascii(), 1)
    assert_eq(if "café".is_ascii() == 0 { 1 } else { 0 }, 1)
}
