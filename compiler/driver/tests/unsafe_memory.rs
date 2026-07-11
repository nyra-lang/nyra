use compiler::{CompileOptions, Compiler};

fn compile(src: &str) -> compiler::CompileOutput {
    Compiler::compile_source(src, "unsafe.ny", &CompileOptions::default()).unwrap()
}

#[test]
fn raw_ptr_const_mut_type_annotations_ok() {
    let out = compile(
        r#"fn main() {
    mut x = 1
    unsafe {
        let p: *const i32 = &x as *const i32
        let q: *mut i32 = &x as *mut i32
        *q = 2
        print(*p)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
}

#[test]
fn raw_ptr_deref_requires_unsafe() {
    let out = compile(
        r#"fn main() {
    let x = 42
    let p = &x as *i32
    let v = *p
    print(v)
}"#,
    );
    assert!(!out.type_errors.is_empty());
    assert!(out
        .type_errors
        .iter()
        .any(|e| e.message.contains("unsafe")));
}

/// `skip_typecheck` must not be used for `nyra run`: it hides exactly these errors.
#[test]
fn skip_typecheck_suppresses_unsafe_errors() {
    let mut options = CompileOptions::default();
    options.skip_typecheck = true;
    let out = Compiler::compile_source(
        r#"fn main() {
    let x = 42
    let p = &x as *i32
    let v = *p
    print(v)
}"#,
        "skip_tc.ny",
        &options,
    )
    .unwrap();
    assert!(
        out.type_errors.is_empty(),
        "documents that skip_typecheck hides unsafe errors — do not enable for run/build"
    );
    assert!(
        out.llvm_ir.is_some(),
        "skip_typecheck still codegen's; that is why run must never set it"
    );
}

#[test]
fn raw_ptr_deref_in_unsafe_ok() {
    let out = compile(
        r#"fn main() {
    mut x = 99
    unsafe {
        let p = &x as *i32
        let v = *p
        print(v)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.unwrap();
    assert!(ir.contains("load i32"));
}

#[test]
fn pointer_store_in_unsafe() {
    let out = compile(
        r#"fn main() {
    mut x = 1
    unsafe {
        let p = &x as *i32
        *p = 7
    }
    print(x)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.unwrap();
    assert!(ir.contains("store i32"));
}

#[test]
fn pointer_arithmetic_in_unsafe() {
    let out = compile(
        r#"fn main() {
    let buf: [i32; 4] = [10, 20, 30, 40]
    unsafe {
        let base = &buf as *i32
        let second = base + 1
        print(*second)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.unwrap();
    assert!(ir.contains("getelementptr"));
}

#[test]
fn no_std_rejects_print() {
    let mut options = CompileOptions::default();
    options.no_std = true;
    let out = Compiler::compile_source(
        r#"fn main() {
    print(1)
}"#,
        "no_std.ny",
        &options,
    )
    .unwrap();
    assert!(!out.type_errors.is_empty());
    assert!(out
        .type_errors
        .iter()
        .any(|e| e.message.contains("no_std")));
}

#[test]
fn typed_raw_ptr_type_annotation() {
    let out = compile(
        r#"extern fn get_addr() -> *i32

fn main() {
    unsafe {
        let p: *i32 = get_addr()
        let v = *p
        print(v)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
}
