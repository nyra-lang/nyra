import "stdlib/testing.ny"

test fn test_string_pad_center() {
    assert_str_eq("hi".pad_center(6, " "), "  hi  ")
}
