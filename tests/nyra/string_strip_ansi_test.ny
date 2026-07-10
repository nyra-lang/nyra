import "stdlib/testing.ny"

test fn test_string_strip_ansi() {
    assert_str_eq("\033[31mok\033[0m".strip_ansi(), "ok")
}
