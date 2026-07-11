import "stdlib/sync/async_channel.ny"

test fn async_channel_recv_async_and_select() {
    let ah = channel_new()
    let bh = channel_new()

    spawn {
        channel_send(ah, 7)
    }

    let fa = Future_from_handle_i32(channel_recv_async(ah))
    let fb = Future_from_handle_i32(channel_recv_async(bh))
    let picked = Future_select2_i32(fa, fb)
    assert_eq(picked.index, 0)
    assert_eq(picked.value, 7)
    channel_free(ah)
    channel_free(bh)
}

test fn async_channel_try_recv_empty_then_value() {
    let ch = channel_new()
    assert_eq(channel_try_recv(ch), 0)
    channel_send(ch, 42)
    assert_eq(channel_try_recv(ch), 1)
    assert_eq(channel_try_value(), 42)
    channel_free(ch)
}

test fn async_channel_recv_async_same_thread() {
    let h = channel_new()
    let f = Future_from_handle_i32(channel_recv_async(h))
    channel_send(h, 99)
    assert_eq(Future_await_i32(f), 99)
    channel_free(h)
}
