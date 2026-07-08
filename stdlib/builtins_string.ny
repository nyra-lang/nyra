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

