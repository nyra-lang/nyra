import "channel.ny"
import "../async/future.ny"

// AsyncChannel_i32 — Future-based recv/select over the channel runtime (Extended).
// Methods that keep the channel alive return `Self` (same pattern as Channel_i32).
struct AsyncChannel_i32 {
    handle: ptr
}

fn AsyncChannel_i32_new() -> AsyncChannel_i32 {
    return AsyncChannel_i32 { handle: channel_new() }
}

impl AsyncChannel_i32 {
    fn send(self, value: i32) -> AsyncChannel_i32 {
        channel_send(self.handle, value)
        return self
    }

    fn recv(self) -> i32 {
        return channel_recv(self.handle)
    }

    // Non-blocking: 1 if ready (then read channel_try_value()), else 0.
    fn try_recv(self) -> i32 {
        return channel_try_recv(self.handle)
    }

    fn try_value(self) -> i32 {
        return channel_try_value()
    }

    fn recv_async(self) -> Future_i32 {
        return Future_from_handle_i32(channel_recv_async(self.handle))
    }
}

impl Drop for AsyncChannel_i32 {
    fn drop(self) -> void {
        channel_free(self.handle)
    }
}

fn async_channel_send(ch: AsyncChannel_i32, value: i32) -> AsyncChannel_i32 {
    return ch.send(value)
}

fn async_channel_recv(ch: AsyncChannel_i32) -> i32 {
    return ch.recv()
}

fn async_channel_recv_async(ch: AsyncChannel_i32) -> Future_i32 {
    return Future_from_handle_i32(channel_recv_async(ch.handle))
}

// Select the first of two pending channel receives (reuses Future_select2).
fn AsyncChannel_select2_i32(a: AsyncChannel_i32, b: AsyncChannel_i32) -> SelectResult_i32 {
    let fa = Future_from_handle_i32(channel_recv_async(a.handle))
    let fb = Future_from_handle_i32(channel_recv_async(b.handle))
    return Future_select2_i32(fa, fb)
}
