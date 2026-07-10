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

fn parse_bool(s: string) -> i32 {
    return str_to_bool(s)
}

fn format_f64(n: f64) -> string {
    return f64_to_string(n)
}
// [contrib-dev:str_to_bool:strconv_mod]
extern fn str_to_bool(s: &string) -> i32
// [/contrib-dev:str_to_bool:strconv_mod]
// [contrib-dev:f64_to_string_prec:strconv_mod]
extern fn f64_to_string_prec(n: f64, prec: i32) -> string
// [/contrib-dev:f64_to_string_prec:strconv_mod]
// [contrib-dev:f64_to_string_prec:strconv_mod:alias]
fn format_f64(n: f64, prec: i32) -> string {
    return f64_to_string_prec(n, prec)
}
// [/contrib-dev:f64_to_string_prec:strconv_mod:alias]

// [contrib-dev:parse_int_base:strconv_mod]
extern fn parse_int_base(s: &string, base: i32) -> i32
// [/contrib-dev:parse_int_base:strconv_mod]
// [contrib-dev:parse_int_base:strconv_mod:alias]
fn parse_int(s: &string, base: i32) -> i32 {
    return parse_int_base(s, base)
}
// [/contrib-dev:parse_int_base:strconv_mod:alias]

// [contrib-dev:str_to_i64:strconv_mod]
extern fn str_to_i64(s: &string) -> i64
// [/contrib-dev:str_to_i64:strconv_mod]
// [contrib-dev:format_i32_pad:strconv_mod]
extern fn format_i32_pad(n: i32, width: i32) -> string
// [/contrib-dev:format_i32_pad:strconv_mod]
// [contrib-dev:format_i32_pad:strconv_mod:alias]
fn format_pad(n: i32, width: i32) -> string {
    return format_i32_pad(n, width)
}
// [/contrib-dev:format_i32_pad:strconv_mod:alias]

// [contrib-dev:format_i32_hex:strconv_mod]
extern fn format_i32_hex(n: i32) -> string
// [/contrib-dev:format_i32_hex:strconv_mod]
// [contrib-dev:format_i32_hex:strconv_mod:alias]
fn format_hex(n: i32) -> string {
    return format_i32_hex(n)
}
// [/contrib-dev:format_i32_hex:strconv_mod:alias]

// [contrib-dev:i64_to_string:strconv_mod]
extern fn i64_to_string(n: i64) -> string
// [/contrib-dev:i64_to_string:strconv_mod]
// [contrib-dev:i64_to_string:strconv_mod:alias]
fn format_i64(n: i64) -> string {
    return i64_to_string(n)
}
// [/contrib-dev:i64_to_string:strconv_mod:alias]

// [contrib-dev:parse_uint_base:strconv_mod]
extern fn parse_uint_base(s: &string, base: i32) -> i32
// [/contrib-dev:parse_uint_base:strconv_mod]
// [contrib-dev:parse_uint_base:strconv_mod:alias]
fn parse_uint(s: &string, base: i32) -> i32 {
    return parse_uint_base(s, base)
}
// [/contrib-dev:parse_uint_base:strconv_mod:alias]

// [contrib-dev:str_to_f64:strconv_mod:alias]
fn parse_f64(s: &string) -> f64 {
    return str_to_f64(s)
}
// [/contrib-dev:str_to_f64:strconv_mod:alias]

// [contrib-dev:str_to_u64:strconv_mod]
extern fn str_to_u64(s: &string) -> i64
// [/contrib-dev:str_to_u64:strconv_mod]
// [contrib-dev:str_to_u64:strconv_mod:alias]
fn parse_u64(s: &string) -> i64 {
    return str_to_u64(s)
}
// [/contrib-dev:str_to_u64:strconv_mod:alias]

// [contrib-dev:format_bool:strconv_mod]
extern fn format_bool(b: i32) -> string
// [/contrib-dev:format_bool:strconv_mod]
// [contrib-dev:format_bool:strconv_mod:alias]
fn bool_to_string(b: i32) -> string {
    return format_bool(b)
}
// [/contrib-dev:format_bool:strconv_mod:alias]

// [contrib-dev:format_f64_pad:strconv_mod]
extern fn format_f64_pad(n: f64, width: i32, prec: i32) -> string
// [/contrib-dev:format_f64_pad:strconv_mod]
// [contrib-dev:format_i32_hex_pad:strconv_mod]
extern fn format_i32_hex_pad(n: i32, width: i32) -> string
// [/contrib-dev:format_i32_hex_pad:strconv_mod]
// [contrib-dev:format_i64_pad:strconv_mod]
extern fn format_i64_pad(n: i64, width: i32) -> string
// [/contrib-dev:format_i64_pad:strconv_mod]
// [contrib-dev:i32_to_string_radix:strconv_mod]
extern fn i32_to_string_radix(n: i32, base: i32) -> string
// [/contrib-dev:i32_to_string_radix:strconv_mod]
// [contrib-dev:i32_to_string_radix:strconv_mod:alias]
fn format_radix(n: i32, base: i32) -> string {
    return i32_to_string_radix(n, base)
}
// [/contrib-dev:i32_to_string_radix:strconv_mod:alias]

// [contrib-dev:parse_i64_base:strconv_mod]
extern fn parse_i64_base(s: &string, base: i32) -> i64
// [/contrib-dev:parse_i64_base:strconv_mod]
// [contrib-dev:parse_i64_base:strconv_mod:alias]
fn parse_i64(s: &string, base: i32) -> i64 {
    return parse_i64_base(s, base)
}
// [/contrib-dev:parse_i64_base:strconv_mod:alias]

// [contrib-dev:str_to_f32:strconv_mod]
extern fn str_to_f32(s: &string) -> f64
// [/contrib-dev:str_to_f32:strconv_mod]
// [contrib-dev:str_to_f32:strconv_mod:alias]
fn parse_f32(s: &string) -> f64 {
    return str_to_f32(s)
}
// [/contrib-dev:str_to_f32:strconv_mod:alias]

// [contrib-dev:u64_to_string:strconv_mod]
extern fn u64_to_string(n: i64) -> string
// [/contrib-dev:u64_to_string:strconv_mod]
// [contrib-dev:u64_to_string:strconv_mod:alias]
fn format_u64(n: i64) -> string {
    return u64_to_string(n)
}
// [/contrib-dev:u64_to_string:strconv_mod:alias]

// [contrib-dev:format_i32_bin:strconv_mod]
extern fn format_i32_bin(n: i32) -> string
// [/contrib-dev:format_i32_bin:strconv_mod]
// [contrib-dev:format_i32_bin:strconv_mod:alias]
fn format_bin(n: i32) -> string {
    return format_i32_bin(n)
}
// [/contrib-dev:format_i32_bin:strconv_mod:alias]

// [contrib-dev:format_i32_oct:strconv_mod]
extern fn format_i32_oct(n: i32) -> string
// [/contrib-dev:format_i32_oct:strconv_mod]
// [contrib-dev:format_i32_oct:strconv_mod:alias]
fn format_oct(n: i32) -> string {
    return format_i32_oct(n)
}
// [/contrib-dev:format_i32_oct:strconv_mod:alias]

// [contrib-dev:format_i64_hex:strconv_mod]
extern fn format_i64_hex(n: i64) -> string
// [/contrib-dev:format_i64_hex:strconv_mod]
// [contrib-dev:format_i64_hex:strconv_mod:alias]
fn format_hex_i64(n: i64) -> string {
    return format_i64_hex(n)
}
// [/contrib-dev:format_i64_hex:strconv_mod:alias]

// [contrib-dev:format_u64_pad:strconv_mod]
extern fn format_u64_pad(n: i64, width: i32) -> string
// [/contrib-dev:format_u64_pad:strconv_mod]
// [contrib-dev:format_i64_bin:strconv_mod]
extern fn format_i64_bin(n: i64) -> string
// [/contrib-dev:format_i64_bin:strconv_mod]
// [contrib-dev:format_i64_bin:strconv_mod:alias]
fn format_bin_i64(n: i64) -> string {
    return format_i64_bin(n)
}
// [/contrib-dev:format_i64_bin:strconv_mod:alias]

// [contrib-dev:format_quote:strconv_mod]
extern fn format_quote(s: &string) -> string
// [/contrib-dev:format_quote:strconv_mod]
// [contrib-dev:format_quote:strconv_mod:alias]
fn quote(s: &string) -> string {
    return format_quote(s)
}
// [/contrib-dev:format_quote:strconv_mod:alias]

