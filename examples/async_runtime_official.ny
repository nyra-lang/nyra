// Official Nyra async runtime facade.
// Runnable: nyra run examples/async_runtime_official.ny

import "stdlib/async/mod.ny"

fn main() {
    let rt = NyraRuntime_default()
    let f = sleep_ms_async(20)
    let value = match NyraRuntime_run_until(rt, f.handle, 1000) {
        Result.Ok(v) => v
        Result.Err(err) => {
            Error_print(err)
            0
        }
    }
    print(value)
}
