// Official Nyra async runtime facade with explicit annotations.
// Runnable: nyra run examples/async_runtime_official.typed.ny

import "stdlib/async/mod.ny"

fn main() {
    let rt: NyraRuntime = NyraRuntime_default()
    let f: Future_i32 = sleep_ms_async(20)
    let value: i32 = match NyraRuntime_run_until(rt, f.handle, 1000) {
        Result.Ok(v) => v
        Result.Err(err) => {
            Error_print(err)
            0
        }
    }
    print(value)
}
