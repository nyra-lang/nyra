//! Conformance tests: async runtime v1 (CONF-ASYNC-*).

use crate::common::{assert_ir_patterns, compile};

#[test]
fn conf_async_001_promise_await_i32() {
    let out = compile(
        r#"extern fn async_promise_new() -> i32
extern fn async_promise_complete(handle: i32, value: i32) -> void
extern fn async_await(handle: i32) -> i32

fn main() {
    let h = async_promise_new()
    async_promise_complete(h, 42)
    print(async_await(h))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.llvm_ir.is_some());
}

#[test]
fn conf_async_002_spawn_await_in_main() {
    let out = compile(
        r#"extern fn async_promise_new() -> i32
extern fn async_promise_complete(handle: i32, value: i32) -> void
extern fn async_await(handle: i32) -> i32

fn main() {
    let h = async_promise_new()
    spawn {
        async_promise_complete(h, 99)
    }
    print(await h)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    assert!(out.borrow_errors.is_empty(), "{:?}", out.borrow_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["spawn_task_capture", "async_await"], &[]);
}

#[test]
fn conf_async_003_executor_sleep_and_run_until() {
    let out = compile(
        r#"extern fn async_sleep_ms(delay_ms: i32) -> i32
extern fn runtime_executor_run_until(handle: i32, timeout_ms: i32) -> i32
extern fn async_await(handle: i32) -> i32

fn main() {
    let h = async_sleep_ms(10)
    print(runtime_executor_run_until(h, 5000))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(
        &ir,
        &["runtime_executor_run_until", "async_sleep_ms"],
        &[],
    );
}

#[test]
fn conf_async_004_async_fn_desugar_spawn() {
    let out = compile(
        r#"extern fn async_promise_new() -> i32
extern fn async_promise_complete(handle: i32, value: i32) -> void
extern fn async_await(handle: i32) -> i32

async fn compute() -> i32 {
    return 42
}

fn main() {
    let h = compute()
    print(await h)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["spawn_capture", "async_promise_complete"], &[]);
}

#[test]
fn conf_async_005_linear_state_machine_poll() {
    let out = compile(
        r#"extern fn async_promise_new() -> i32
extern fn async_promise_complete(handle: i32, value: i32) -> void
extern fn async_poll(handle: i32) -> i32
extern fn async_sleep_ms(delay_ms: i32) -> i32
extern fn runtime_executor_tick(ms: i32) -> void
extern fn async_await(handle: i32) -> i32

async fn chain() -> i32 {
    let _ = await async_sleep_ms(1)
    let _ = await async_sleep_ms(1)
    return 7
}

fn main() {
    let h = chain()
    print(await h)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(
        &ir,
        &["async_poll", "runtime_executor_tick", "async_promise_complete"],
        &[],
    );
}

#[test]
fn conf_async_006_for_in_array_state_machine() {
    let out = compile(
        r#"import "stdlib/async_v1.ny"

async fn walk(arr: [i32; 2]) -> i32 {
    for n in arr {
        let _ = await Executor_sleep_ms(1)
    }
    return 1
}

fn main() {
    print(await walk([1, 2]))
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert!(ir.contains("async_poll"));
}

#[test]
fn conf_async_007_future_i32_and_select() {
    let out = compile(
        r#"import "stdlib/async/future.ny"

async fn give() -> i32 {
    return 7
}

fn main() {
    let ha = async_promise_new()
    let a = Future_from_handle_i32(ha)
    let b = give()
    spawn { async_promise_complete(ha, 3) }
    let picked = Future_select2_i32(a, b)
    print(picked.value)
}"#,
    );
    assert!(out.type_errors.is_empty(), "{:?}", out.type_errors);
    let ir = out.llvm_ir.expect("ir");
    assert_ir_patterns(&ir, &["Future_i32", "async_poll", "async_promise_complete"], &[]);
}
