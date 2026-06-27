//! Multi-module workspace patterns (CONF-WS-*) — language/stdlib only, no Sonic.

use crate::common::{compile, compile_file_rel};

#[test]
fn conf_ws_001_multi_module_domain_import() {
    let out = compile(
        r#"extern fn strlen(s: string) -> i32
struct TenantRecord Send { id: i32 name: string }
fn TenantRecord_new(id: i32, name: string) -> TenantRecord {
    return TenantRecord { id: id, name: name }
}
fn Tenant_name_len(rec: TenantRecord) -> i32 {
    return strlen(rec.name)
}
fn main() {
    let t = TenantRecord_new(1, "acme")
    print(Tenant_name_len(t))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_ws_002_async_ready_probe_pattern() {
    let out = compile(
        r#"enum Option<T> { None, Some(T) }
extern fn async_promise_new() -> i32
extern fn async_promise_complete(handle: i32, value: i32) -> void
extern fn async_await(handle: i32) -> i32

fn bootstrap_ready() -> i32 {
    let h = async_promise_new()
    async_promise_complete(h, 1)
    return async_await(h)
}

fn ready_probe() -> Option<i32> {
    let ready = bootstrap_ready()
    if ready == 1 {
        return Option.Some(1)
    }
    return Option.None
}

fn main() {
    let probe: Option<i32> = ready_probe()
    print(0)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
}

#[test]
fn conf_ws_003_graph_arc_smoke_compiles() {
    let out = compile_file_rel("examples/graph_arc_smoke.ny");
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
}

#[test]
fn conf_ws_004_monolith_struct_smoke_compiles() {
    let out = compile_file_rel("examples/monolith_struct_smoke.ny");
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
}
