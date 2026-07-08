import "stdlib/testing.ny"
import "stdlib/strings.ny"
import "stdlib/builtins_string.ny"

test fn test_string_strip_suffix() {
    let s = "hamdy.txt"
    let result = s.strip_suffix(".txt")
    assert_str_eq(result, "hamdy")  // TODO: fix expected value after C impl
    let result2 = strip_suffix(s, ".txt")
    assert_str_eq(result2, "hamdy")  // TODO: fix expected value
}
