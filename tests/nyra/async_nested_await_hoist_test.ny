import "stdlib/async_v1.ny"

// Nested await in call args is hoisted into statement-level awaits.
async fn nested_call_await() -> i32 {
    let a = await Executor_sleep_ms(1)
    print(await Executor_sleep_ms(1))
    return a
}

test fn async_nested_await_in_print() {
    let v = await nested_call_await()
    assert_eq(v, 0)
}
