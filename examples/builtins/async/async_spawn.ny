// async fn + await — call site gets handle immediately; body runs on spawn:thread.
allow_extended

import "stdlib/async_v1.ny"

async fn compute() -> i32 {
    return 42
}

fn main() {
    let h = compute()
    print(await h)
}
