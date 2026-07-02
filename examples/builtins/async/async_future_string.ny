// Future<string> — typed async with Executor_sleep_ms.
allow_extended

import "stdlib/async/future.ny"

async fn greet() -> string {
    let h = Executor_sleep_ms(10)
    await h
    return "Nyra async v2"
}

fn main() {
    let f = greet()
    print(await f)
}
