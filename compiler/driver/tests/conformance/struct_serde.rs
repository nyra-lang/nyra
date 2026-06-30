//! Conformance tests: struct JSON synthesis (CONF-SERDE-STRUCT-*).

use crate::common::{assert_ir_patterns, compile, compile_file_rel};

#[test]
fn conf_serde_struct_001_encode_decode() {
    let out = compile(
        r#"extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_free(v: ptr) -> void
extern fn json_encode_object(keys: ptr, values: ptr) -> string
extern fn json_get_string(json: string, key: string) -> string
extern fn json_get_i32(json: string, key: string) -> i32
extern fn i32_to_string(n: i32) -> string

struct User {
    name: string
    age: i32
}

fn main() {
    let u = User { name: "nyra", age: 21 }
    let json = User_json_encode(u)
    let u2 = User_json_decode(json)
    print(u2.age)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["json_encode_object", "json_get_i32"], &[]);
}

#[test]
fn conf_serde_struct_002_nested_fields() {
    let out = compile(
        r#"extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_free(v: ptr) -> void
extern fn json_encode_object(keys: ptr, values: ptr) -> string
extern fn json_get_string(json: string, key: string) -> string
extern fn json_get_i32(json: string, key: string) -> i32
extern fn json_get_object(json: string, key: string) -> string
extern fn decode_object(json: string, key: string) -> string
extern fn decode_i32(json: string, key: string) -> i32
extern fn i32_to_string(n: i32) -> string

struct Inner {
    x: i32
}

struct Outer {
    label: string
    inner: Inner
}

fn main() {
    let o = Outer { label: "a", inner: Inner { x: 3 } }
    let json = Outer_json_encode(o)
    let o2 = Outer_json_decode(json)
    print(o2.inner.x)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["Inner_json_encode", "decode_object"], &[]);
}

#[test]
fn conf_serde_struct_003_rawptr_field() {
    let out = compile(
        r#"extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_free(v: ptr) -> void
extern fn json_encode_object(keys: ptr, values: ptr) -> string
extern fn json_encode_ptr_token(value: ptr) -> string
extern fn json_decode_ptr_token(json: string, key: string) -> ptr
extern fn json_get_string(json: string, key: string) -> string

struct Handle {
    label: string
    data: *i32
}

fn main() {
    let null_ptr: *i32 = unsafe { 0 as *i32 }
    let h = Handle { label: "x", data: null_ptr }
    let json = Handle_json_encode(h)
    let h2 = Handle_json_decode(json)
    print(h2.label)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["json_encode_ptr_token", "json_decode_ptr_token"], &[]);
}

#[test]
fn conf_serde_struct_004_vec_struct_field() {
    let out = compile_file_rel("tests/nyra/struct_serde_vec_struct_test.ny");
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(
        &ir,
        &["Bag_json_encode", "json_split_array_elements", "Item_json_decode"],
        &["%Vec__%"],
    );
}
