//! Conformance: Windows cross-compile accepts spawn/TCP/async (CONF-WIN-*).

use compiler::{CompileOptions, Compiler};

fn compile_windows(src: &str) -> compiler::CompileOutput {
    let opts = CompileOptions {
        target: "x86_64-pc-windows-gnu".into(),
        ..Default::default()
    };
    Compiler::compile_source(src, "test.ny", &opts).unwrap()
}

#[test]
fn conf_win_001_spawn_compiles() {
    let out = compile_windows(
        r#"fn main() {
    spawn:thread {
        print(1)
    }
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.as_ref().expect("llvm");
    assert!(ir.contains("spawn_capture"));
}

#[test]
fn conf_win_002_async_compiles() {
    let out = compile_windows(
        r#"async fn work() -> i32 {
    return 42
}

fn main() {
    let h = work()
    let _v = await h
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_win_003_tcp_extern_compiles() {
    let out = compile_windows(
        r#"extern fn tcp_connect(host: ptr, port: i32) -> i32
extern fn tcp_listen(host: ptr, port: i32) -> i32

fn main() {
    print(0)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.as_ref().expect("llvm");
    assert!(ir.contains("target triple = \"x86_64-pc-windows-gnu\""));
}
