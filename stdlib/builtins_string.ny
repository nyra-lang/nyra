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






// [builtin-dev:strip_suffix:string]
fn String_stripSuffix(s: &string, suffix: &string) -> string {
    return str_strip_suffix(s, suffix)
}

fn strip_suffix(s: &string, suffix: &string) -> string {
    return str_strip_suffix(s, suffix)
}
// [/builtin-dev:strip_suffix:string]

// [builtin-dev:to_snake_case:string]
fn String_toSnakeCase(s: &string) -> string {
    return str_to_snake_case(s)
}

fn to_snake_case(s: &string) -> string {
    return str_to_snake_case(s)
}
// [/builtin-dev:to_snake_case:string]


// [builtin-dev:to_lowercase:string]
fn String_toLowercase(s: &string) -> string {
    return str_to_lowercase(s)
}

fn to_lowercase(s: &string) -> string {
    return str_to_lowercase(s)
}
// [/builtin-dev:to_lowercase:string]

// [builtin-dev:to_titlecase:string]
fn String_toTitlecase(s: &string) -> string {
    return str_to_titlecase(s)
}

fn to_titlecase(s: &string) -> string {
    return str_to_titlecase(s)
}
// [/builtin-dev:to_titlecase:string]

// [builtin-dev:to_capitalize:string]
fn String_toCapitalize(s: &string) -> string {
    return str_to_capitalize(s)
}

fn to_capitalize(s: &string) -> string {
    return str_to_capitalize(s)
}
// [/builtin-dev:to_capitalize:string]

// [builtin-dev:to_camel_case:string]
fn String_toCamelCase(s: &string) -> string {
    return str_to_camel_case(s)
}

fn to_camel_case(s: &string) -> string {
    return str_to_camel_case(s)
}
// [/builtin-dev:to_camel_case:string]

// [builtin-dev:to_kebab_case:string]
fn String_toKebabCase(s: &string) -> string {
    return str_to_kebab_case(s)
}

fn to_kebab_case(s: &string) -> string {
    return str_to_kebab_case(s)
}
// [/builtin-dev:to_kebab_case:string]

// [builtin-dev:to_pascal_case:string]
fn String_toPascalCase(s: &string) -> string {
    return str_to_pascal_case(s)
}

fn to_pascal_case(s: &string) -> string {
    return str_to_pascal_case(s)
}
// [/builtin-dev:to_pascal_case:string]

// [builtin-dev:to_screaming_snake_case:string]
fn String_toScreamingSnakeCase(s: &string) -> string {
    return str_to_screaming_snake_case(s)
}

fn to_screaming_snake_case(s: &string) -> string {
    return str_to_screaming_snake_case(s)
}
// [/builtin-dev:to_screaming_snake_case:string]

// [builtin-dev:to_train_case:string]
fn String_toTrainCase(s: &string) -> string {
    return str_to_train_case(s)
}

fn to_train_case(s: &string) -> string {
    return str_to_train_case(s)
}
// [/builtin-dev:to_train_case:string]

// [builtin-dev:to_dot_case:string]
fn String_toDotCase(s: &string) -> string {
    return str_to_dot_case(s)
}

fn to_dot_case(s: &string) -> string {
    return str_to_dot_case(s)
}
// [/builtin-dev:to_dot_case:string]


// [builtin-dev:strip_prefix:string]
fn String_stripPrefix(s: &string, prefix: &string) -> string {
    return str_strip_prefix(s, prefix)
}

fn strip_prefix(s: &string, prefix: &string) -> string {
    return str_strip_prefix(s, prefix)
}
// [/builtin-dev:strip_prefix:string]

// [builtin-dev:index:string]
fn String_index(s: &string, needle: &string) -> i32 {
    return str_index(s, needle)
}

fn index(s: &string, needle: &string) -> i32 {
    return str_index(s, needle)
}
// [/builtin-dev:index:string]

// [builtin-dev:is_empty:string]
fn String_isEmpty(s: &string) -> i32 {
    return str_is_empty(s)
}

fn is_empty(s: &string) -> i32 {
    return str_is_empty(s)
}
// [/builtin-dev:is_empty:string]

// [builtin-dev:last_index:string]
fn String_lastIndex(s: &string, needle: &string) -> i32 {
    return str_last_index(s, needle)
}

fn last_index(s: &string, needle: &string) -> i32 {
    return str_last_index(s, needle)
}
// [/builtin-dev:last_index:string]

// [builtin-dev:repeat:string]
fn String_repeat(s: &string, count: i32) -> string {
    return str_repeat(s, count)
}

fn repeat(s: &string, count: i32) -> string {
    return str_repeat(s, count)
}
// [/builtin-dev:repeat:string]

// [builtin-dev:trim_end:string]
fn String_trimEnd(s: &string) -> string {
    return str_trim_end(s)
}

fn trim_end(s: &string) -> string {
    return str_trim_end(s)
}
// [/builtin-dev:trim_end:string]

// [builtin-dev:trim_start:string]
fn String_trimStart(s: &string) -> string {
    return str_trim_start(s)
}

fn trim_start(s: &string) -> string {
    return str_trim_start(s)
}
// [/builtin-dev:trim_start:string]

// [builtin-dev:splitn:string]
fn String_splitn(s: &string, sep: &string, n: i32) -> ptr {
    return str_splitn(s, sep, n)
}
// [/builtin-dev:splitn:string]

// [builtin-dev:count:string]
fn String_count(s: &string, needle: &string) -> i32 {
    return str_count(s, needle)
}
// [/builtin-dev:count:string]

// [builtin-dev:fields:string]
fn String_fields(s: &string) -> ptr {
    return str_fields(s)
}
// [/builtin-dev:fields:string]

// [builtin-dev:pad_end:string]
fn String_padEnd(s: &string, width: i32, pad: &string) -> string {
    return str_pad_end(s, width, pad)
}
// [/builtin-dev:pad_end:string]

// [builtin-dev:pad_start:string]
fn String_padStart(s: &string, width: i32, pad: &string) -> string {
    return str_pad_start(s, width, pad)
}
// [/builtin-dev:pad_start:string]

// [builtin-dev:split_once:string]
fn String_splitOnce(s: &string, sep: &string) -> string {
    return str_before_sep(s, sep)
}
// [/builtin-dev:split_once:string]

// [builtin-dev:compare:string]
fn String_compare(s: &string, other: &string) -> i32 {
    return str_compare(s, other)
}

fn compare(s: &string, other: &string) -> i32 {
    return str_compare(s, other)
}
// [/builtin-dev:compare:string]

// [builtin-dev:equal_fold:string]
fn String_equalFold(s: &string, other: &string) -> i32 {
    return str_equal_fold(s, other)
}

fn equal_fold(s: &string, other: &string) -> i32 {
    return str_equal_fold(s, other)
}
// [/builtin-dev:equal_fold:string]

// [builtin-dev:index_byte:string]
fn String_indexByte(s: &string, byte: i32) -> i32 {
    return str_index_byte(s, byte)
}

fn index_byte(s: &string, byte: i32) -> i32 {
    return str_index_byte(s, byte)
}
// [/builtin-dev:index_byte:string]

// [builtin-dev:last_index_byte:string]
fn String_lastIndexByte(s: &string, byte: i32) -> i32 {
    return str_last_index_byte(s, byte)
}

fn last_index_byte(s: &string, byte: i32) -> i32 {
    return str_last_index_byte(s, byte)
}
// [/builtin-dev:last_index_byte:string]

// [builtin-dev:after_sep:string]
fn String_afterSep(s: &string, sep: &string) -> string {
    return str_after_sep(s, sep)
}

fn after_sep(s: &string, sep: &string) -> string {
    return str_after_sep(s, sep)
}
// [/builtin-dev:after_sep:string]

// [builtin-dev:char_at:string]
fn String_charAt(s: &string, index: i32) -> i32 {
    return char_at(s, index)
}

// [/builtin-dev:char_at:string]

// [builtin-dev:contains:string]
fn String_contains(s: &string, needle: &string) -> i32 {
    return str_contains(s, needle)
}

fn contains(s: &string, needle: &string) -> i32 {
    return str_contains(s, needle)
}
// [/builtin-dev:contains:string]

// [builtin-dev:ends_with:string]
fn String_endsWith(s: &string, suffix: &string) -> i32 {
    return str_ends_with(s, suffix)
}

fn ends_with(s: &string, suffix: &string) -> i32 {
    return str_ends_with(s, suffix)
}
// [/builtin-dev:ends_with:string]

// [builtin-dev:pop:string]
fn String_pop(s: &string) -> string {
    return str_pop(s)
}

fn pop(s: &string) -> string {
    return str_pop(s)
}
// [/builtin-dev:pop:string]

// [builtin-dev:push_char:string]
fn String_pushChar(s: &string, ch: i32) -> string {
    return str_push_char(s, ch)
}

fn push_char(s: &string, ch: i32) -> string {
    return str_push_char(s, ch)
}
// [/builtin-dev:push_char:string]

// [builtin-dev:replace:string]
fn String_replace(s: &string, from: &string, to: &string) -> string {
    return str_replace(s, from, to)
}

fn replace(s: &string, from: &string, to: &string) -> string {
    return str_replace(s, from, to)
}
// [/builtin-dev:replace:string]

// [builtin-dev:replacen:string]
fn String_replacen(s: &string, from: &string, to: &string, count: i32) -> string {
    return str_replacen(s, from, to, count)
}

fn replacen(s: &string, from: &string, to: &string, count: i32) -> string {
    return str_replacen(s, from, to, count)
}
// [/builtin-dev:replacen:string]

// [builtin-dev:starts_with:string]
fn String_startsWith(s: &string, prefix: &string) -> i32 {
    return str_starts_with(s, prefix)
}

fn starts_with(s: &string, prefix: &string) -> i32 {
    return str_starts_with(s, prefix)
}
// [/builtin-dev:starts_with:string]

// [builtin-dev:strip_ansi:string]
fn String_stripAnsi(s: &string) -> string {
    return str_strip_ansi(s)
}

fn strip_ansi(s: &string) -> string {
    return str_strip_ansi(s)
}
// [/builtin-dev:strip_ansi:string]

// [builtin-dev:substring:string]
fn String_substring(s: &string, start: i32, len: i32) -> string {
    return substring(s, start, len)
}

// [/builtin-dev:substring:string]

// [builtin-dev:trim:string]
fn String_trim(s: &string) -> string {
    return str_trim(s)
}

fn trim(s: &string) -> string {
    return str_trim(s)
}
// [/builtin-dev:trim:string]

// [builtin-dev:before_sep:string]
fn String_beforeSep(s: &string, sep: &string) -> string {
    return str_before_sep(s, sep)
}

fn before_sep(s: &string, sep: &string) -> string {
    return str_before_sep(s, sep)
}
// [/builtin-dev:before_sep:string]

// [builtin-dev:collapse_ws:string]
fn String_collapseWs(s: &string) -> string {
    return str_collapse_ws(s)
}

fn collapse_ws(s: &string) -> string {
    return str_collapse_ws(s)
}
// [/builtin-dev:collapse_ws:string]

// [builtin-dev:is_ascii:string]
fn String_isAscii(s: &string) -> i32 {
    return str_is_ascii(s)
}

fn is_ascii(s: &string) -> i32 {
    return str_is_ascii(s)
}
// [/builtin-dev:is_ascii:string]

// [builtin-dev:common_prefix_len:string]
fn String_commonPrefixLen(s: &string, other: &string) -> i32 {
    return str_common_prefix_len(s, other)
}

fn common_prefix_len(s: &string, other: &string) -> i32 {
    return str_common_prefix_len(s, other)
}
// [/builtin-dev:common_prefix_len:string]

// [builtin-dev:is_alnum:string]
fn String_isAlnum(s: &string) -> i32 {
    return str_is_alnum(s)
}

fn is_alnum(s: &string) -> i32 {
    return str_is_alnum(s)
}
// [/builtin-dev:is_alnum:string]

// [builtin-dev:is_alpha:string]
fn String_isAlpha(s: &string) -> i32 {
    return str_is_alpha(s)
}

fn is_alpha(s: &string) -> i32 {
    return str_is_alpha(s)
}
// [/builtin-dev:is_alpha:string]

// [builtin-dev:is_digit:string]
fn String_isDigit(s: &string) -> i32 {
    return str_is_digit(s)
}

fn is_digit(s: &string) -> i32 {
    return str_is_digit(s)
}
// [/builtin-dev:is_digit:string]

// [builtin-dev:pad_center:string]
fn String_padCenter(s: &string, width: i32, pad: &string) -> string {
    return str_pad_center(s, width, pad)
}

fn pad_center(s: &string, width: i32, pad: &string) -> string {
    return str_pad_center(s, width, pad)
}
// [/builtin-dev:pad_center:string]

// [builtin-dev:reverse:string]
fn String_reverse(s: &string) -> string {
    return str_reverse(s)
}

fn reverse(s: &string) -> string {
    return str_reverse(s)
}
// [/builtin-dev:reverse:string]

// [builtin-dev:escape_json:string]
fn String_escapeJson(s: &string) -> string {
    return str_escape_json(s)
}

fn escape_json(s: &string) -> string {
    return str_escape_json(s)
}
// [/builtin-dev:escape_json:string]

// [builtin-dev:split_after:string]
fn String_splitAfter(s: &string, sep: &string) -> string {
    return str_split_after(s, sep)
}

fn split_after(s: &string, sep: &string) -> string {
    return str_split_after(s, sep)
}
// [/builtin-dev:split_after:string]

// [builtin-dev:truncate:string]
fn String_truncate(s: &string, max_len: i32) -> string {
    return str_truncate(s, max_len)
}

fn truncate(s: &string, max_len: i32) -> string {
    return str_truncate(s, max_len)
}
// [/builtin-dev:truncate:string]

