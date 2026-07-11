// async channel + select — zero-types
// nyra run examples/async_channel_select.ny

import "stdlib/sync/async_channel.ny"

fn main() {
    // Raw handles from channel_new() are Send into spawn; wrappers own Drop.
    let ah = channel_new()
    let bh = channel_new()
    let a = AsyncChannel_i32 { handle: ah }
    let b = AsyncChannel_i32 { handle: bh }

    spawn {
        channel_send(ah, 11)
    }
    spawn {
        let h = Executor_sleep_ms(20)
        let _ = await h
        channel_send(bh, 22)
    }

    let picked = AsyncChannel_select2_i32(a, b)
    print(picked.index)
    print(picked.value)
}
