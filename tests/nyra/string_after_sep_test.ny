import "stdlib/testing.ny"

test fn test_string_after_sep() {
    assert_str_eq("a:b:c".after_sep(":"), "b:c")
}
