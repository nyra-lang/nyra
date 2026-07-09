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

fn char_from_code(ch: i32) -> string {
    return str_push_char("", ch)
}
extern fn str_strip_suffix(str: &string, suffix: &string) -> string
extern fn str_to_snake_case(str: &string) -> string
extern fn str_to_lowercase(str: &string) -> string
extern fn str_to_titlecase(str: &string) -> string
extern fn str_to_capitalize(str: &string) -> string
extern fn str_to_camel_case(str: &string) -> string
extern fn str_to_kebab_case(str: &string) -> string
extern fn str_to_pascal_case(str: &string) -> string
extern fn str_to_screaming_snake_case(str: &string) -> string
extern fn str_to_train_case(str: &string) -> string
extern fn str_to_dot_case(str: &string) -> string
extern fn str_strip_prefix(str: &string, prefix: &string) -> string
extern fn str_index(str: &string, needle: &string) -> i32
extern fn str_is_empty(str: &string) -> i32
extern fn str_last_index(str: &string, needle: &string) -> i32
extern fn str_repeat(str: &string, count: i32) -> string
extern fn str_trim_end(str: &string) -> string
extern fn str_trim_start(str: &string) -> string
extern fn str_splitn(str: &string, sep: &string, n: i32) -> ptr
extern fn str_count(str: &string, needle: &string) -> i32
extern fn str_fields(str: &string) -> ptr
extern fn str_pad_end(str: &string, width: i32, pad: &string) -> string
extern fn str_pad_start(str: &string, width: i32, pad: &string) -> string
extern fn str_before_sep(str: &string, sep: &string) -> string
extern fn str_compare(str: &string, other: &string) -> i32
extern fn str_equal_fold(str: &string, other: &string) -> i32
extern fn str_index_byte(str: &string, byte: i32) -> i32
extern fn str_last_index_byte(str: &string, byte: i32) -> i32
extern fn str_after_sep(str: &string, sep: &string) -> string
extern fn str_contains(str: &string, needle: &string) -> i32
extern fn str_ends_with(str: &string, suffix: &string) -> i32
extern fn str_replace(str: &string, from: &string, to: &string) -> string
extern fn str_replacen(str: &string, from: &string, to: &string, count: i32) -> string
extern fn str_starts_with(str: &string, prefix: &string) -> i32
extern fn str_strip_ansi(str: &string) -> string
extern fn str_collapse_ws(str: &string) -> string
extern fn str_is_ascii(str: &string) -> i32
extern fn str_common_prefix_len(str: &string, other: &string) -> i32
extern fn str_is_alnum(str: &string) -> i32
extern fn str_is_alpha(str: &string) -> i32
extern fn str_is_digit(str: &string) -> i32
extern fn str_pad_center(str: &string, width: i32, pad: &string) -> string
extern fn str_reverse(str: &string) -> string
extern fn str_escape_json(str: &string) -> string
extern fn str_split_after(str: &string, sep: &string) -> string
extern fn str_truncate(str: &string, max_len: i32) -> string
