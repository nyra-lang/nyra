extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn i32_to_string(n: i32) -> string
extern fn i64_to_string(n: i64) -> string
extern fn strcmp(a: &string, b: &string) -> i32
extern fn char_at(s: &string, i: i32) -> i32
extern fn substring(s: &string, start: i32, len: i32) -> string
extern fn strstr_pos(hay: &string, needle: &string) -> i32
extern fn str_to_i32(s: &string) -> i32
extern fn str_push_char(s: &string, ch: i32) -> string
extern fn str_pop(s: &string) -> string
extern fn strip_ansi(input: &string) -> string

fn char_from_code(ch: i32) -> string {
    return str_push_char("", ch)
}
