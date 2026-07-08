// async state machine — multiple await in one async fn (cooperative poll loop).
allow_extended

import "stdlib/async_v1.ny"

async fn chain() -> i32 {
    let _ = await Executor_sleep_ms(5)
    let _ = await Executor_sleep_ms(5)
    return 100
}

fn main() -> void {
    let h = chain()
    print(await h)
}
