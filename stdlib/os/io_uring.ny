// Linux io_uring optional fast path (falls back to epoll via io_register).

extern fn io_uring_available() -> i32
extern fn io_uring_register_read(fd: i32, promise: i32) -> i32
extern fn io_uring_unregister_read(fd: i32) -> i32

fn IoUring_available() -> bool {
    return io_uring_available() == 1
}

fn IoUring_register_read(fd: i32, promise: i32) -> i32 {
    return io_uring_register_read(fd, promise)
}

fn IoUring_unregister_read(fd: i32) -> i32 {
    return io_uring_unregister_read(fd)
}
