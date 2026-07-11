import "stdlib/testing.ny"

// CONF-OPTION-STRING: explicit Option<string> None must default via ?? and exit cleanly.
// Option comes from auto-prelude / stdlib option enum.
test fn conf_option_string_none_nullish() {
    let name: Option<string> = Option.None
    let got = name ?? "Anonymous"
    assert_str_eq(got, "Anonymous")
}

test fn conf_option_string_some_nullish() {
    let name: Option<string> = Option.Some("Hamdy")
    let got = name ?? "Anonymous"
    assert_str_eq(got, "Hamdy")
}

test fn conf_option_string_zero_types_nullish() {
    let name = Option.None
    let got = name ?? "Anonymous"
    assert_str_eq(got, "Anonymous")
}
