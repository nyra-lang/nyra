import "../strings.ny"

extern fn str_to_i32(s: &string) -> i32
extern fn i32_to_string(n: i32) -> string
extern fn str_to_f64(s: &string) -> f64
extern fn f64_to_string(n: f64) -> string

fn atoi(s: string) -> i32 {
    return str_to_i32(s)
}

fn itoa(n: i32) -> string {
    return i32_to_string(n)
}

fn format_i32(n: i32) -> string {
    return i32_to_string(n)
}

fn parse_i32(s: string) -> i32 {
    return str_to_i32(s)
}

fn parse_f64(s: string) -> f64 {
    return str_to_f64(s)
}

fn format_f64(n: f64) -> string {
    return f64_to_string(n)
}
