import "stdlib/async/mod.ny"

test fn test_official_async_sleep_future() {
    let f = sleep_ms_async(5)
    assert_eq(await_i32(f), 5)
}

test fn test_official_runtime_run_until_result() {
    let rt = NyraRuntime_default()
    let h = Executor_sleep_ms(5)
    let value = match NyraRuntime_run_until(rt, h, 1000) {
        Result.Ok(v) => v
        Result.Err(_err) => 0
    }
    assert_eq(value, 5)
}
