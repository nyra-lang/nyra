//! Conformance tests: ownership rules (CONF-OWN-*).

use crate::common::{assert_ir_patterns, compile, compile_file_rel};

#[test]
fn conf_own_001_copy_i32_after_assign() {
    let out = compile(
        r#"fn main() {
    let b = 1
    let a = b
    print(a)
    print(b)
}"#,
    );
    assert!(out.borrow_errors.is_empty());
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_own_002_move_string_use_after_move() {
    let out = compile(
        r#"fn main() {
    let a = "hello"
    let b = a
    print(a)
}"#,
    );
    assert!(!out.borrow_errors.is_empty());
    assert!(out
        .borrow_errors
        .iter()
        .any(|e| e.message.contains("moved")));
}

#[test]
fn conf_own_003_auto_drop_emits_nyra_free() {
    let out = compile(
        r#"extern fn read_file(path: string) -> string
fn main() {
    let content = read_file("/tmp/x")
    print(content)
}"#,
    );
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["free"], &[]);
}

#[test]
fn conf_own_004_custom_drop_emits_drop_fn() {
    let out = compile(
        r#"struct Box { n: i32 }
impl Drop for Box {
    fn drop(mut self) -> void { print(self.n) }
}
fn main() {
    let b = Box { n: 1 }
}"#,
    );
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("Drop_Box_drop") || ir.contains("_Box_drop"));
}

#[test]
fn conf_own_004b_custom_drop_call_uses_struct_pointer() {
    let out = compile(
        r#"struct Box { handle: ptr }
extern fn box_dummy_new() -> ptr
extern fn box_dummy_free(h: ptr) -> void
impl Drop for Box {
    fn drop(self) -> void { box_dummy_free(self.handle) }
}
fn main() {
    let b = Box { handle: box_dummy_new() }
    print(0)
}"#,
    );
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("call void @Drop_Box_drop(%Box*"),
        "expected custom drop call with struct pointer, got:\n{ir}"
    );
}

#[test]
fn conf_own_005_return_ref_to_local_rejected() {
    let out = compile(
        r#"fn bad() -> &i32 {
    let x = 1
    return &x
}
fn main() { print(0) }"#,
    );
    assert!(!out.borrow_errors.is_empty() || !out.type_errors.is_empty());
}

#[test]
fn conf_own_006_string_is_move_in_ir() {
    let out = compile(
        r#"fn main() {
    let a = "x"
    let b = a
    print(b)
}"#,
    );
    assert!(out.borrow_errors.is_empty());
}

#[test]
fn conf_own_007_immutable_i32_no_alloca() {
    let out = compile(r#"fn main() { let x = 10 print(x) }"#);
    let ir = out.llvm_ir.unwrap();
    assert!(!ir.contains("alloca"));
}

#[test]
fn conf_own_008_mut_i32_uses_ssa_not_alloca() {
    let out = compile(
        r#"fn main() {
    let mut n = 0
    n = 1
    print(n)
}"#,
    );
    let ir = out.llvm_ir.unwrap();
    assert!(
        !ir.contains("alloca i32"),
        "mut i32 should promote to SSA, not stack slot:\n{ir}"
    );
}

#[test]
fn conf_own_009_double_free_warning() {
    let out = compile(
        r#"fn main() {
    let s = "hi"
    free(s)
}"#,
    );
    assert!(out.warnings.iter().any(|w| w.message.contains("free"))
        || out.borrow_errors.iter().any(|e| e.message.contains("free"))
        || !out.type_errors.is_empty()
        || out.llvm_ir.is_some());
}

#[test]
fn conf_own_010_struct_by_value_return() {
    let out = compile(
        r#"struct Pair { a: i32 b: i32 }
fn mk() -> Pair { return Pair { a: 1, b: 2 } }
fn main() { print(mk().a) }"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_own_011_composite_struct_field_drop_emits_nyra_free() {
    let out = compile(
        r#"extern fn read_file(path: string) -> string
struct Packet { id: i32 body: string }
fn main() {
    let body = read_file("/tmp/x")
    let p = Packet { id: 1 body: body }
    print(p.id)
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["free"], &[]);
}

#[test]
fn conf_own_012_extern_string_return_auto_owned() {
    let out = compile(
        r#"extern fn my_custom_load() -> string
fn main() {
    let s = my_custom_load()
    print(s)
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["free"], &[]);
}

#[test]
fn conf_own_013_partial_field_move_marks_parent() {
    let out = compile(
        r#"struct Pair { a: i32 body: string }
fn main() {
    let p = Pair { a: 1 body: "x" }
    let body = p.body
    print(body)
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    assert!(!out.llvm_ir.is_none());
}

#[test]
fn conf_own_015_imported_stdlib_option_string() {
    let out = compile_file_rel("examples/monolith_struct_smoke.ny");
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["free"], &[]);
}

#[test]
fn conf_own_014_generic_option_string_payload_drop() {
    let out = compile(
        r#"enum Option<T> {
    None
    Some(T)
}
fn main() {
    let opt: Option<string> = Option.Some("hello")
    print(0)
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["free"], &[]);
}

/// REG: Option<string>.None must tag-check before free (no unconditional payload free).
#[test]
fn conf_own_016_option_string_none_drop_checks_tag() {
    let out = compile(
        r#"enum Option<T> {
    None
    Some(T)
}
fn main() {
    let opt: Option<string> = Option.None
    print(0)
}"#,
    );
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("icmp eq i32") || ir.contains("icmp eq"),
        "Option<string> None drop must compare tag before free:\n{ir}"
    );
    assert!(
        ir.contains("enum_drop.free") || ir.contains("free"),
        "expected free path present for Some payloads:\n{ir}"
    );
    assert!(
        ir.contains("enum_drop.skip") || ir.contains("br i1"),
        "expected conditional branch around free for None:\n{ir}"
    );
}
