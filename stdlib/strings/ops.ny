import "../strings.ny"

extern fn str_to_upper(s: &string) -> string
extern fn str_to_lower(s: &string) -> string
extern fn str_trim(s: &string) -> string
extern fn str_len(s: &string) -> i32
extern fn str_contains(hay: &string, needle: &string) -> i32
extern fn str_starts_with(s: &string, prefix: &string) -> i32
extern fn str_ends_with(s: &string, suffix: &string) -> i32
extern fn str_replace(s: &string, from: &string, to: &string) -> string
extern fn str_replacen(s: &string, from: &string, to: &string, count: i32) -> string

// split_once — returns prefix before first sep (MVP; full split array is post-1.0).
fn str_split_once(s: &string, sep: &string) -> string {
    let pos = strstr_pos(s, sep)
    if pos < 0 {
        return substring(s, 0, strlen(s))
    }
    return substring(s, 0, pos)
}
