// Dedicated I/O thread pool — offloads blocking fd wait/read from the main executor.

extern fn io_pool_create(workers: i32) -> i32
extern fn io_pool_shutdown(pool: i32) -> void
extern fn io_pool_submit_wait_readable(pool: i32, fd: i32, promise: i32) -> i32
extern fn io_pool_submit_read(pool: i32, fd: i32, buf: ptr, nbytes: i64, promise: i32) -> i32
extern fn io_pool_queue_depth(pool: i32) -> i32

struct IoPool {
    handle: i32
}

fn IoPool_new(workers: i32) -> IoPool {
    return IoPool { handle: io_pool_create(workers) }
}

fn IoPool_shutdown(pool: IoPool) -> void {
    if pool.handle >= 0 {
        io_pool_shutdown(pool.handle)
    }
}

fn IoPool_wait_readable(pool: IoPool, fd: i32, promise: i32) -> i32 {
    return io_pool_submit_wait_readable(pool.handle, fd, promise)
}

fn IoPool_read_async(pool: IoPool, fd: i32, buf: ptr, nbytes: i64, promise: i32) -> i32 {
    return io_pool_submit_read(pool.handle, fd, buf, nbytes, promise)
}

fn IoPool_pending(pool: IoPool) -> i32 {
    return io_pool_queue_depth(pool.handle)
}
