// Regression: JS-style string methods dispatch to stdlib `String_*` helpers via
// UFCS. `name.toUpperCase()` -> `String_toUpperCase(name)`; the fully-qualified
// spelling `name.String_toUpperCase()` also works. Auto-prelude must load
// `builtins_string.ny` for these method-only references.
fn main() {
    let name = "Hamdy"
    assert_str_eq(name.toUpperCase(), "HAMDY")
    assert_str_eq(name.toLowerCase(), "hamdy")
    assert_str_eq(name.String_toUpperCase(), "HAMDY")
    assert_str_eq("nyra".toUpperCase(), "NYRA")
    assert_eq(name.includes("amd"), 1)
}
