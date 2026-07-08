import "stdlib/testing.ny"
import "stdlib/async_v1.ny"

async fn give_i32() -> i32 {
    return 21
}

test fn conf_async_001_await_async_fn() {
    let f = give_i32()
    assert_eq(await f, 21)
}

test fn conf_async_002_executor_sleep() {
    let h = Executor_sleep_ms(10)
    assert_eq(await h, 10)
}

test fn conf_async_003_manual_promise() {
    let h = async_promise_new()
    spawn {
        async_promise_complete(h, 88)
    }
    assert_eq(await h, 88)
}
