import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_count() {
    assert_eq("aaaba".count("a"), 4)
    assert_eq("aaaba".count("z"), 0)
}
