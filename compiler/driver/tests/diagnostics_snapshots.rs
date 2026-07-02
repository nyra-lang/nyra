//! Diagnostic message golden snapshots (insta).

mod common;

use common::{compile_named, format_all_errors};
use compiler::{CompileOptions, CompileStage, Compiler};

macro_rules! snap_diag {
    ($name:ident, $src:expr) => {
        #[test]
        fn $name() {
            let file = concat!(stringify!($name), ".ny");
            let out = compile_named(file, $src);
            insta::assert_snapshot!(stringify!($name), format_all_errors(&out));
        }
    };
}

snap_diag!(diag_undefined_variable, r#"fn main() {
    let life = 1
    print(lie)
}"#);
snap_diag!(diag_assign_immutable, r#"fn main() {
    let x = 1
    x = 2
}"#);
snap_diag!(diag_use_after_move, r#"fn main() {
    let a = "hello"
    let b = a
    print(a)
}"#);
snap_diag!(diag_mut_borrow_conflict, r#"fn main() {
    let mut v = 1
    let r = &v
    v = 2
    print(r)
}"#);
snap_diag!(diag_function_no_return, r#"fn oops() { print(1) }"#);
snap_diag!(diag_flush_with_args, r#"fn main() { flush(1) }"#);
snap_diag!(diag_time_start_non_string, r#"fn main() { time_start(1) }"#);

#[test]
fn diag_extended_async_warning() {
    let src = r#"async fn work() -> i32 { return 1 }
fn main() { print(0) }"#;
    let out = compile_named("diag_extended_async_warning.ny", src);
    let warnings: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_extended_async_warning", warnings.join("\n"));
}

#[test]
fn diag_extended_spawn_warning() {
    let src = r#"fn main() { spawn { print(1) } }"#;
    let out = compile_named("diag_extended_spawn_warning.ny", src);
    let warnings: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_extended_spawn_warning", warnings.join("\n"));
}

#[test]
fn diag_deny_extended_rejects_async() {
    let src = r#"async fn work() -> i32 { return 1 }
fn main() { print(0) }"#;
    let opts = CompileOptions {
        deny_extended: true,
        ..Default::default()
    };
    let out = Compiler::compile_source(src, "deny.ny", &opts).unwrap();
    insta::assert_snapshot!("diag_deny_extended_rejects_async", format_all_errors(&out));
}

#[test]
fn diag_type_error_line_number() {
    let out = compile_named("diag_type_error_line_number.ny", r#"fn main() {
    let x = y
}"#);
    let err = format!("{}", out.type_errors[0]);
    insta::assert_snapshot!("diag_type_error_line_number", err);
}

#[test]
fn diag_borrow_at_borrow_stage() {
    let src = r#"fn main() {
    let a = "x"
    let b = a
    print(a)
}"#;
    let opts = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..Default::default()
    };
    let out = Compiler::compile_source(src, "b.ny", &opts).unwrap();
    insta::assert_snapshot!("diag_borrow_at_borrow_stage", format_all_errors(&out));
}

#[test]
fn diag_anonymous_object_literal_ok() {
    let src = r#"fn main() {
    let obj = {
        name: "Hamdy",
    }
    print(obj.name)
}"#;
    let out = Compiler::compile_source(src, "obj.ny", &CompileOptions::default()).unwrap();
    assert!(out.parser_errors.is_empty(), "{:?}", out.parser_errors);
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn diag_empty_object_literal_block() {
    let src = r#"fn main() {
    let x = { }
}"#;
    let out = Compiler::compile_source(src, "obj.ny", &CompileOptions::default()).unwrap();
    insta::assert_snapshot!("diag_empty_object_literal_block", format_all_errors(&out));
}

#[test]
fn diag_parser_invalid_expression() {
    let src = r#"fn main() { let x = @ }"#;
    let out = Compiler::compile_source(src, "p.ny", &CompileOptions::default()).unwrap();
    insta::assert_snapshot!("diag_parser_invalid_expression", format_all_errors(&out));
}

#[test]
fn diag_no_std_rejects_print() {
    let src = r#"# no_std
fn main() { print(1) }"#;
    let out = compile_named("diag_no_std_rejects_print.ny", src);
    insta::assert_snapshot!("diag_no_std_rejects_print", format_all_errors(&out));
}

#[test]
fn diag_return_ref_to_local() {
    let src = r#"fn bad() -> &i32 {
    let x = 1
    return &x
}
fn main() { print(0) }"#;
    let out = compile_named("diag_return_ref_to_local.ny", src);
    insta::assert_snapshot!("diag_return_ref_to_local", format_all_errors(&out));
}

#[test]
fn diag_asm_outside_unsafe() {
    let src = r#"fn main() { asm "nop" }"#;
    let out = compile_named("diag_asm_outside_unsafe.ny", src);
    insta::assert_snapshot!("diag_asm_outside_unsafe", format_all_errors(&out));
}

#[test]
fn diag_raw_ptr_outside_unsafe() {
    let src = r#"fn main() {
    let p: *i32 = 0
    let v = *p
    print(v)
}"#;
    let out = compile_named("diag_raw_ptr_outside_unsafe.ny", src);
    insta::assert_snapshot!("diag_raw_ptr_outside_unsafe", format_all_errors(&out));
}

#[test]
fn diag_trait_extended_warning() {
    let src = r#"trait Show { fn show(self) -> void }
fn main() { print(0) }"#;
    let out = compile_named("diag_trait_extended_warning.ny", src);
    let warnings: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_trait_extended_warning", warnings.join("\n"));
}

#[test]
fn diag_macro_extended_warning() {
    let src = r#"macro m(x) { x }
fn main() { print(0) }"#;
    let out = compile_named("diag_macro_extended_warning.ny", src);
    let warnings: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_macro_extended_warning", warnings.join("\n"));
}

#[test]
fn diag_defer_extended_warning() {
    let src = r#"fn main() {
    defer print(1)
    print(0)
}"#;
    let out = compile_named("diag_defer_extended_warning.ny", src);
    let warnings: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_defer_extended_warning", warnings.join("\n"));
}

#[test]
fn diag_comparison_type_mismatch() {
    let src = r#"fn main() {
    let s = "hi"
    if s == 1 { print(1) }
}"#;
    let out = compile_named("diag_comparison_type_mismatch.ny", src);
    insta::assert_snapshot!("diag_comparison_type_mismatch", format_all_errors(&out));
}

#[test]
fn diag_call_wrong_arg_count() {
    let src = r#"fn add(a: i32, b: i32) -> i32 { return a + b }
fn main() { print(add(1)) }"#;
    let out = compile_named("diag_call_wrong_arg_count.ny", src);
    insta::assert_snapshot!("diag_call_wrong_arg_count", format_all_errors(&out));
}

#[test]
fn diag_if_expr_type_mismatch() {
    let src = r#"fn main() {
    let x = if true { 1 } else { "no" }
    print(x)
}"#;
    let out = compile_named("diag_if_expr_type_mismatch.ny", src);
    insta::assert_snapshot!("diag_if_expr_type_mismatch", format_all_errors(&out));
}

#[test]
fn diag_duplicate_fn_definition() {
    let src = r#"fn main() { print(0) }
fn main() { print(1) }"#;
    let out = compile_named("diag_duplicate_fn_definition.ny", src);
    insta::assert_snapshot!("diag_duplicate_fn_definition", format_all_errors(&out));
}

#[test]
fn diag_empty_program() {
    let out = compile_named("diag_empty_program.ny", "");
    insta::assert_snapshot!("diag_empty_program", format_all_errors(&out));
}

#[test]
fn diag_lifetime_elision_ok_no_errors() {
    let src = r#"fn len(s: &string) -> i32 { return 0 }
fn main() { print(0) }"#;
    let out = compile_named("diag_lifetime_elision_ok_no_errors.ny", src);
    insta::assert_snapshot!("diag_lifetime_elision_ok_no_errors", format_all_errors(&out));
}

#[test]
fn diag_repr_c_struct_ok() {
    let src = r#"struct Pair repr(C) {
    a: i32
    b: i32
}
fn main() {
    let p = Pair { a: 1, b: 2 }
    print(p.a)
}"#;
    let out = compile_named("diag_repr_c_struct_ok.ny", src);
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    insta::assert_snapshot!("diag_repr_c_struct_ok", format_all_errors(&out));
}

#[test]
fn diag_spawn_capture_ok_with_warnings() {
    let src = r#"fn main() { spawn { print(1) } }"#;
    let out = compile_named("diag_spawn_capture_ok_with_warnings.ny", src);
    assert!(out.borrow_errors.is_empty());
    let w: Vec<String> = out.warnings.iter().map(|e| format!("{e}")).collect();
    insta::assert_snapshot!("diag_spawn_capture_ok_with_warnings", w.join("\n"));
}

#[test]
fn diag_await_non_async() {
    let src = r#"fn main() {
    let x = 1
    print(await x)
}"#;
    let out = compile_named("diag_await_non_async.ny", src);
    insta::assert_snapshot!("diag_await_non_async", format_all_errors(&out));
}

#[test]
fn diag_self_outside_impl() {
    let src = r#"fn main() { print(self) }"#;
    let out = compile_named("diag_self_outside_impl.ny", src);
    insta::assert_snapshot!("diag_self_outside_impl", format_all_errors(&out));
}

#[test]
fn diag_for_range_non_int() {
    let src = r#"fn main() {
    for i in "a".."b" { print(0) }
}"#;
    let out = compile_named("diag_for_range_non_int.ny", src);
    insta::assert_snapshot!("diag_for_range_non_int", format_all_errors(&out));
}

#[test]
fn diag_void_return_with_value() {
    let src = r#"fn main() -> void { return 1 }"#;
    let out = compile_named("diag_void_return_with_value.ny", src);
    insta::assert_snapshot!("diag_void_return_with_value", format_all_errors(&out));
}

#[test]
fn diag_struct_unknown_field() {
    let src = r#"struct S { x: i32 }
fn main() {
    let s = S { x: 1, y: 2 }
    print(s.x)
}"#;
    let out = compile_named("diag_struct_unknown_field.ny", src);
    insta::assert_snapshot!("diag_struct_unknown_field", format_all_errors(&out));
}

#[test]
fn diag_generic_arity_mismatch() {
    let src = r#"fn id<T>(x: T) -> T { return x }
fn main() { print(id<i32, i32>(1)) }"#;
    let out = compile_named("diag_generic_arity_mismatch.ny", src);
    insta::assert_snapshot!("diag_generic_arity_mismatch", format_all_errors(&out));
}

#[test]
fn diag_custom_drop_extended() {
    let src = r#"struct Box { n: i32 }
impl Drop for Box {
    fn drop(mut self) -> void { print(self.n) }
}
fn main() { print(0) }"#;
    let out = compile_named("diag_custom_drop_extended.ny", src);
    insta::assert_snapshot!("diag_custom_drop_extended", format_all_errors(&out));
}

#[test]
fn diag_hrtb_fn_ptr_ok() {
    let src = r#"fn apply(f: for<'a> fn(&'a i32) -> i32, x: i32) -> i32 {
    return f(&x)
}
fn main() { print(0) }"#;
    let out = compile_named("diag_hrtb_fn_ptr_ok.ny", src);
    insta::assert_snapshot!("diag_hrtb_fn_ptr_ok", format_all_errors(&out));
}

#[test]
fn diag_send_bad_spawn() {
    let src = r#"struct Bad { s: string }
fn main() {
    let b = Bad { s: "x" }
    spawn { print(b.s) }
}"#;
    let out = compile_named("diag_send_bad_spawn.ny", src);
    insta::assert_snapshot!("diag_send_bad_spawn", format_all_errors(&out));
}

#[test]
fn diag_match_non_exhaustive() {
    let src = r#"enum E { A B }
fn main() {
    let e = E.A
    let n = match e { E.A => 1 }
    print(n)
}"#;
    let out = compile_named("diag_match_non_exhaustive.ny", src);
    insta::assert_snapshot!("diag_match_non_exhaustive", format_all_errors(&out));
}

#[test]
fn diag_modulo_type_error() {
    let src = r#"fn main() { print(1 % "x") }"#;
    let out = compile_named("diag_modulo_type_error.ny", src);
    insta::assert_snapshot!("diag_modulo_type_error", format_all_errors(&out));
}

#[test]
fn diag_double_free_hint() {
    let src = r#"fn main() {
    let s = "hi"
    free(s)
    print(s)
}"#;
    let out = compile_named("diag_double_free_hint.ny", src);
    insta::assert_snapshot!("diag_double_free_hint", format_all_errors(&out));
}

#[test]
fn diag_ffi_export_inst_typecheck() {
    let src = r#"export fn id<T>(x: T) -> T { return x }
fn main() { print(0) }"#;
    let opts = CompileOptions {
        stop_after: Some(CompileStage::TypeCheck),
        ..Default::default()
    };
    let out = Compiler::compile_source(src, "ffi.ny", &opts).unwrap();
    insta::assert_snapshot!("diag_ffi_export_inst_typecheck", format_all_errors(&out));
}

#[test]
fn diag_import_not_found_message() {
    let result = Compiler::compile_source(
        r#"import "missing.ny"
fn main() { print(0) }"#,
        "imp.ny",
        &CompileOptions::default(),
    );
    let msg = result.err().unwrap_or_else(|| "unexpected ok".into());
    insta::assert_snapshot!("diag_import_not_found_message", msg);
}

#[test]
fn diag_channel_type_mismatch() {
    let src = r#"fn main() {
    let ch = channel<i32>()
    send(ch, "not i32")
}"#;
    let out = compile_named("diag_channel_type_mismatch.ny", src);
    insta::assert_snapshot!("diag_channel_type_mismatch", format_all_errors(&out));
}

#[test]
fn diag_struct_spread_extended() {
    let src = r#"struct Base { a: i32 }
struct Derived { b: i32 }
fn main() {
    let base = Base { a: 1 }
    let d = Derived { ..base, b: 2 }
    print(d.b)
}"#;
    let out = compile_named("diag_struct_spread_extended.ny", src);
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
}
