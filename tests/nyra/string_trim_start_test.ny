import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_trim_start() {
    assert_str_eq("  hi".trim_start(), "hi")
    assert_str_eq(trim_start("  hi"), "hi")
}
