import "../strings.ny"
import "../vec_str.ny"
import "../error.ny"
import "../map.ny"

extern fn json_has_key(json: string, key: string) -> i32
extern fn json_has_string(json: string, key: string) -> i32
extern fn json_has_i32(json: string, key: string) -> i32
extern fn json_has_bool(json: string, key: string) -> i32
extern fn json_get_string(json: string, key: string) -> string
extern fn json_get_i32(json: string, key: string) -> i32
extern fn json_get_bool(json: string, key: string) -> i32
extern fn json_get_object(json: string, key: string) -> string
extern fn json_get_array(json: string, key: string) -> string
extern fn json_encode_object(keys: ptr, values: ptr) -> string
extern fn json_encode_i32_array(values: ptr) -> string
extern fn json_decode_i32_array(array_json: string) -> ptr
extern fn json_encode_str_array(values: ptr) -> string
extern fn json_join_raw_array(values: ptr) -> string
extern fn json_decode_str_array(array_json: string) -> ptr
extern fn json_split_array_elements(array_json: string) -> ptr
extern fn json_encode_ptr_token(value: ptr) -> string
extern fn json_decode_ptr_token(json: string, key: string) -> ptr
extern fn json_top_keys(json: string) -> ptr
extern fn json_raw_get(json: string, key: string) -> string
extern fn json_value_kind(json: string) -> i32
extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn i32_to_string(n: i32) -> string

const JSON_KIND_NULL = 0
const JSON_KIND_OBJECT = 1
const JSON_KIND_ARRAY = 2
const JSON_KIND_STRING = 3
const JSON_KIND_NUMBER = 4
const JSON_KIND_BOOL = 5

fn decode_string(json: string, key: string) -> string {
    return json_get_string(json, key)
}

fn decode_i32(json: string, key: string) -> i32 {
    return json_get_i32(json, key)
}

fn decode_bool(json: string, key: string) -> i32 {
    return json_get_bool(json, key)
}

fn json_string(json: string, key: string) -> Result<string, Error> {
    if json_has_string(json, key) == 0 {
        return Result.Err(Error_json(strcat("missing string field: ", key)))
    }
    return Result.Ok(json_get_string(json, key))
}

fn json_i32(json: string, key: string) -> Result<i32, Error> {
    if json_has_i32(json, key) == 0 {
        return Result.Err(Error_json(strcat("missing integer field: ", key)))
    }
    return Result.Ok(json_get_i32(json, key))
}

fn json_bool(json: string, key: string) -> Result<i32, Error> {
    if json_has_bool(json, key) == 0 {
        return Result.Err(Error_json(strcat("missing boolean field: ", key)))
    }
    return Result.Ok(json_get_bool(json, key))
}

fn decode_object(json: string, key: string) -> string {
    return json_get_object(json, key)
}

fn decode_array(json: string, key: string) -> string {
    return json_get_array(json, key)
}

fn encode_i32_array(values: ptr) -> string {
    return json_encode_i32_array(values)
}

fn decode_i32_array(array_json: string) -> ptr {
    return json_decode_i32_array(array_json)
}

fn encode_str_array(values: ptr) -> string {
    return json_encode_str_array(values)
}

fn decode_str_array(array_json: string) -> ptr {
    return json_decode_str_array(array_json)
}

fn encode_field(key: string, value: string) -> string {
    let keys = Vec_str_new()
    let values = Vec_str_new()
    Vec_str_push(keys, key)
    Vec_str_push(values, value)
    let out = json_encode_object(keys, values)
    Vec_str_free(keys)
    Vec_str_free(values)
    return out
}

fn encode_object(keys: ptr, values: ptr) -> string {
    return json_encode_object(keys, values)
}

fn encode_i32(key: string, value: i32) -> string {
    let val = i32_to_string(value)
    let keys = Vec_str_new()
    let values = Vec_str_new()
    Vec_str_push(keys, key)
    Vec_str_push(values, val)
    let out = json_encode_object(keys, values)
    Vec_str_free(keys)
    Vec_str_free(values)
    return out
}

// Full object parse: top-level JSON object → map of key → raw JSON value text.
fn JSON_parse_object(text: string) -> HashMap_str_str {
    let mut out = HashMap_str_str_new()
    let keys = StrVec { handle: json_top_keys(text) }
    let n = keys.len()
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        let v = json_raw_get(text, k)
        out = out.insert(k, v)
        i = i + 1
    }
    return out
}

fn JSON_stringify_object(obj: HashMap_str_str) -> string {
    let keys = obj.keys()
    let n = keys.len()
    let key_vec = Vec_str_new()
    let val_vec = Vec_str_new()
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        Vec_str_push(key_vec, k)
        Vec_str_push(val_vec, obj.get(k))
        i = i + 1
    }
    let out = json_encode_object(key_vec, val_vec)
    Vec_str_free(key_vec)
    Vec_str_free(val_vec)
    return out
}

fn json_parse(text: string) -> HashMap_str_str {
    return JSON_parse_object(text)
}

fn json_stringify(obj: HashMap_str_str) -> string {
    return JSON_stringify_object(obj)
}

fn json_kind(text: string) -> i32 {
    return json_value_kind(text)
}

fn json_raw(text: string, key: string) -> string {
    return json_raw_get(text, key)
}

// --- short ergonomics (JS-like) ---

fn jparse(text: string) -> HashMap_str_str {
    return JSON_parse_object(text)
}

fn jstringify(obj: HashMap_str_str) -> string {
    return JSON_stringify_object(obj)
}

fn json_unquote(raw: string) -> string {
    let n = strlen(raw)
    if n >= 2 && char_at(raw, 0) == 34 && char_at(raw, n - 1) == 34 {
        return substring(raw, 1, n - 2)
    }
    return raw
}

fn jstr(obj: HashMap_str_str, key: string) -> string {
    return json_unquote(obj.get(key))
}

fn jraw(obj: HashMap_str_str, key: string) -> string {
    return obj.get(key)
}

fn jobj(obj: HashMap_str_str, key: string) -> HashMap_str_str {
    return JSON_parse_object(obj.get(key))
}

fn jnum(obj: HashMap_str_str, key: string) -> i32 {
    return str_to_i32(obj.get(key))
}

fn jbool(obj: HashMap_str_str, key: string) -> i32 {
    let v = obj.get(key)
    if strcmp(v, "true") == 0 {
        return 1
    }
    return 0
}

fn jfield(key: string, value: string) -> string {
    return encode_field(key, value)
}

fn obj() -> HashMap_str_str {
    return HashMap_str_str_new()
}

fn dict() -> HashMap_str_str {
    return HashMap_str_str_new()
}

fn dict_i32() -> HashMap_str_i32 {
    return HashMap_str_i32_new()
}
