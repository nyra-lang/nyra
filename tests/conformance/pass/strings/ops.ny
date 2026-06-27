import "stdlib/testing.ny"
import "stdlib/builtins_string.ny"

test fn conf_string_trim_len() {
    assert_str_eq(trim("  ab  "), "ab")
    assert_eq(strlen("hello"), 5)
}

test fn conf_string_concat() {
    assert_str_eq(strcat("a", "b"), "ab")
}
