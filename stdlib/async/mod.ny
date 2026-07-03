// Official Nyra async runtime facade.
//
// Low-level promise symbols remain available in stdlib/async.ny and
// stdlib/async_v1.ny. Prefer this module for application code so async code
// has one batteries-included runtime path instead of depending on community
// executors.
import "../error.ny"
import "future.ny"

struct NyraRuntime {
    tick_ms: i32
}

fn NyraRuntime_default() -> NyraRuntime {
    return NyraRuntime { tick_ms: 10 }
}

fn NyraRuntime_tick(rt: NyraRuntime) -> i32 {
    return Executor_tick(rt.tick_ms)
}

fn NyraRuntime_run_once(rt: NyraRuntime) -> i32 {
    return NyraRuntime_tick(rt)
}

fn NyraRuntime_run_until(rt: NyraRuntime, handle: i32, timeout_ms: i32) -> Result<i32, Error> {
    if rt.tick_ms <= 0 {
        return Result.Err(Error_async("async runtime tick must be positive"))
    }
    let value = Executor_run_until(handle, timeout_ms)
    if value < 0 {
        return Result.Err(Error_async("async task timed out or failed"))
    }
    return Result.Ok(value)
}

fn async_promise() -> i32 {
    return async_promise_new()
}

fn async_complete_i32(handle: i32, value: i32) -> void {
    async_promise_complete(handle, value)
}

fn async_sleep(ms: i32) -> Future_i32 {
    return Future_from_handle_i32(Executor_sleep_ms(ms))
}

fn sleep_ms_async(ms: i32) -> Future_i32 {
    return async_sleep(ms)
}

fn await_i32(f: Future_i32) -> i32 {
    return Future_await_i32(f)
}
