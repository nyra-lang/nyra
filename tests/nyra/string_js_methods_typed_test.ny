// Explicit-types variant of the JS-style string method regression test.
// (`nyra test` only discovers `*_test.ny`, so the typed coverage lives in its
// own runnable file rather than a non-executed `.typed.ny` sibling.)
fn main() {
    let name: string = "Hamdy"
    assert_str_eq(name.toUpperCase(), "HAMDY")
    assert_str_eq(name.toLowerCase(), "hamdy")
    assert_str_eq(name.String_toUpperCase(), "HAMDY")
    assert_str_eq("nyra".toUpperCase(), "NYRA")
    assert_eq(name.includes("amd"), 1)
}
