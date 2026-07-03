import "stdlib/async/mod.ny"

test fn test_official_async_sleep_future_typed() {
    let f: Future_i32 = sleep_ms_async(5)
    assert_eq(await_i32(f), 5)
}

test fn test_official_runtime_run_until_result_typed() {
    let rt: NyraRuntime = NyraRuntime_default()
    let h: i32 = Executor_sleep_ms(5)
    let value: i32 = match NyraRuntime_run_until(rt, h, 1000) {
        Result.Ok(v) => v
        Result.Err(_err) => 0
    }
    assert_eq(value, 5)
}
