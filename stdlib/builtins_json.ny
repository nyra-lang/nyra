import "json/mod.ny"
import "map.ny"

// MVP JSON helpers: single-field encode/decode + full object parse/stringify.
fn JSON_stringify(key: string, value: string) -> string {
    return encode_field(key, value)
}

fn JSON_parse(json: string, key: string) -> string {
    return decode_string(json, key)
}

fn JSON_parse_full(text: string) -> HashMap_str_str {
    return JSON_parse_object(text)
}

fn JSON_stringify_full(obj: HashMap_str_str) -> string {
    return JSON_stringify_object(obj)
}

// Prefer jparse / jstr / jnum / obj() from json/mod.ny for short app code.
