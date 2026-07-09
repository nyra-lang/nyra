import "stdlib/testing.ny"

test fn test_string_common_prefix_len() {
    assert_eq("abcdef".common_prefix_len("abcxyz"), 3)
    assert_eq("x".common_prefix_len("y"), 0)
}
