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


