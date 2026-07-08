// Unified event loop — async executor + kqueue/epoll/io_uring + optional I/O pool.

import "../async_v1.ny"
import "../io/pool.ny"
import "fd.ny"
import "io_uring.ny"

extern fn io_register(fd: i32, task_id: i32) -> i32
extern fn io_unregister(fd: i32) -> i32
extern fn async_promise_new() -> i32

struct EventLoop {
    running: i32
    pool: IoPool
    pool_enabled: i32
}

fn EventLoop_new() -> EventLoop {
    return EventLoop { running: 1, pool: IoPool { handle: -1 }, pool_enabled: 0 }
}

fn EventLoop_with_pool(workers: i32) -> EventLoop {
    return EventLoop { running: 1, pool: IoPool_new(workers), pool_enabled: 1 }
}

fn EventLoop_shutdown(loop: EventLoop) -> void {
    if loop.pool_enabled == 1 {
        IoPool_shutdown(loop.pool)
    }
}

fn EventLoop_tick(loop: EventLoop, timeout_ms: i32) -> i32 {
    let _ = loop
    return Executor_tick(timeout_ms)
}

fn EventLoop_poll_ms(loop: EventLoop, timeout_ms: i32) -> i32 {
    let _ = loop
    return Executor_poll_ms(timeout_ms)
}

fn EventLoop_run_until(loop: EventLoop, promise: i32, timeout_ms: i32) -> i32 {
    let _ = loop
    return Executor_run_until(promise, timeout_ms)
}

fn EventLoop_sleep_ms(loop: EventLoop, ms: i32) -> i32 {
    let _ = loop
    return Executor_sleep_ms(ms)
}

fn EventLoop_register_read(loop: EventLoop, fd: i32, promise: i32) -> i32 {
    let _ = loop
    if IoUring_available() {
        return IoUring_register_read(fd, promise)
    }
    return io_register(fd, promise)
}

fn EventLoop_register_read_fd(loop: EventLoop, fd: Fd, promise: i32) -> i32 {
    return EventLoop_register_read(loop, Fd_raw(fd), promise)
}

fn EventLoop_register_read_pooled(loop: EventLoop, fd: i32, promise: i32) -> i32 {
    if loop.pool_enabled == 1 {
        return IoPool_wait_readable(loop.pool, fd, promise)
    }
    return EventLoop_register_read(loop, fd, promise)
}

fn EventLoop_register_read_fd_pooled(loop: EventLoop, fd: Fd, promise: i32) -> i32 {
    return EventLoop_register_read_pooled(loop, Fd_raw(fd), promise)
}

fn EventLoop_unregister_fd(loop: EventLoop, fd: i32) -> i32 {
    let _ = loop
    if IoUring_available() {
        let _ = IoUring_unregister_read(fd)
    }
    return io_unregister(fd)
}

fn EventLoop_unregister_fd_obj(loop: EventLoop, fd: Fd) -> i32 {
    return EventLoop_unregister_fd(loop, Fd_raw(fd))
}

fn EventLoop_promise_new(loop: EventLoop) -> i32 {
    let _ = loop
    return async_promise_new()
}
