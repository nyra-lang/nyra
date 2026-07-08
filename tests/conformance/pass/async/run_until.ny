import "stdlib/testing.ny"
import "stdlib/async_v1.ny"

test fn conf_async_004_executor_run_until() {
    let h = Executor_sleep_ms(5)
    let v = Executor_run_until(h, 5000)
    assert_eq(v, 5)
}
