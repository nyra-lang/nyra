//! Conformance: TLS stack stages (CONF-TLS-*).
//!
//! Complements Nyra-source tests under `tests/conformance/pass/tls/`.
//! Ensures the HTTP -> TLS ABI surface still typechecks and emits the expected
//! runtime symbols after the rustls client split.

use crate::common::compile;

#[test]
fn conf_tls_101_tls_available_symbol() {
    let out = compile(
        r#"extern fn tls_available() -> i32
fn main() {
    print(tls_available())
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("tls_available"),
        "missing tls_available in IR:\n{ir}"
    );
}

#[test]
fn conf_tls_102_connect_verify_abi() {
    let out = compile(
        r#"extern fn rt_tls_connect_verify(host: string, port: i32) -> i32
extern fn rt_tls_close(handle: i32) -> void
extern fn rt_tls_last_error() -> string
fn main() {
    let h = rt_tls_connect_verify("example.com", 443)
    if h >= 0 {
        rt_tls_close(h)
    }
    print(rt_tls_last_error())
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(
        ir.contains("rt_tls_connect_verify"),
        "missing connect_verify:\n{ir}"
    );
    assert!(
        ir.contains("rt_tls_last_error"),
        "missing last_error:\n{ir}"
    );
}

#[test]
fn conf_tls_103_http_https_client_symbols() {
    let out = compile(
        r#"extern fn tls_available() -> i32
extern fn rt_tls_connect_verify(host: string, port: i32) -> i32
extern fn rt_tls_write(handle: i32, data: string) -> i32
extern fn rt_tls_read(handle: i32, max_bytes: i32) -> string
extern fn rt_tls_close(handle: i32) -> void
fn main() {
    if tls_available() != 0 {
        let h = rt_tls_connect_verify("example.com", 443)
        if h >= 0 {
            let _ = rt_tls_write(h, "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
            let _ = rt_tls_read(h, 1024)
            rt_tls_close(h)
        }
    }
    print(1)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("rt_tls_write"), "missing write:\n{ir}");
    assert!(ir.contains("rt_tls_read"), "missing read:\n{ir}");
}
