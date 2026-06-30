import "../strings.ny"
import "../vec_str.ny"

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
extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn i32_to_string(n: i32) -> string

fn decode_string(json: string, key: string) -> string {
    return json_get_string(json, key)
}

fn decode_i32(json: string, key: string) -> i32 {
    return json_get_i32(json, key)
}

fn decode_bool(json: string, key: string) -> i32 {
    return json_get_bool(json, key)
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
