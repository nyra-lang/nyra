import "strings/ops.ny"

extern fn str_split(s: &string, sep: &string) -> ptr

fn String_toUpperCase(s: &string) -> string {
    return str_to_upper(s)
}

fn String_toLowerCase(s: &string) -> string {
    return str_to_lower(s)
}

fn String_includes(s: &string, needle: &string) -> i32 {
    return str_contains(s, needle)
}

// Full split into a string vector (ptr handle).
fn String_split(s: &string, sep: &string) -> ptr {
    return str_split(s, sep)
}

fn String_replace(s: &string, from: &string, to: &string) -> string {
    return str_replace(s, from, to)
}

fn String_replacen(s: &string, from: &string, to: &string, count: i32) -> string {
    return str_replacen(s, from, to, count)
}

fn trim(s: &string) -> string {
    return str_trim(s)
}
