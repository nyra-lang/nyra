extern fn channel_new() -> ptr
extern fn channel_send(ch: ptr, value: i32) -> void
extern fn channel_recv(ch: ptr) -> i32
extern fn channel_try_recv(ch: ptr) -> i32
extern fn channel_try_value() -> i32
extern fn channel_recv_async(ch: ptr) -> i32
extern fn channel_free(ch: ptr) -> void

extern fn channel_str_new() -> ptr
extern fn channel_str_send(ch: ptr, value: string) -> void
extern fn channel_str_recv(ch: ptr) -> string
extern fn channel_str_free(ch: ptr) -> void

struct Channel_str {
    handle: ptr
}

fn Channel_str_new() -> Channel_str {
    return Channel_str { handle: channel_str_new() }
}

impl Channel_str {
    fn send(self, value: string) -> Channel_str {
        channel_str_send(self.handle, value)
        return self
    }

    fn recv(self) -> string {
        return channel_str_recv(self.handle)
    }
}

impl Drop for Channel_str {
    fn drop(self) -> void {
        channel_str_free(self.handle)
    }
}

struct Channel_i32 {
    handle: ptr
}

fn Channel_i32_new() -> Channel_i32 {
    return Channel_i32 { handle: channel_new() }
}

impl Channel_i32 {
    fn send(self, value: i32) -> Channel_i32 {
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

    // Promise handle completed when the next value is available.
    fn recv_async(self) -> i32 {
        return channel_recv_async(self.handle)
    }
}

impl Drop for Channel_i32 {
    fn drop(self) -> void {
        channel_free(self.handle)
    }
}
