import "stdlib/testing.ny"

test fn test_string_split_after() {
    assert_str_eq("a:b:c".split_after(":"), "b:c")
    assert_str_eq("abc".split_after(":"), "abc")
}
