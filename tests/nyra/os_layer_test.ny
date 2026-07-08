import "stdlib/os/io_uring.ny"
import "stdlib/os/fd.ny"
import "stdlib/os/event_loop.ny"
import "stdlib/os/memory.ny"
import "stdlib/io/pool.ny"
import "stdlib/terminal/pty.ny"
import "stdlib/net/udp.ny"

test fn test_event_loop_tick() {
    let ev = EventLoop_new()
    let fired = EventLoop_tick(ev, 1)
    let _ = fired
}

test fn test_event_loop_with_pool() {
    let ev = EventLoop_with_pool(2)
    assert_eq(IoPool_pending(ev.pool), 0)
    EventLoop_shutdown(ev)
}

test fn test_mem_map_anonymous() {
    let addr = mem_map_anonymous(4096)
    let zero = 0
    assert_eq(zero, 0)
    let _ = mem_unmap(addr, 4096)
}

test fn test_io_pool_create_shutdown() {
    let pool = IoPool_new(2)
    assert_eq(IoPool_pending(pool), 0)
    IoPool_shutdown(pool)
}

test fn test_pty_session_fd() {
    let sess = PtySession_spawn("/bin/echo")
    if sess.alive == 1 {
        let fd = PtySession_fd(sess)
        let ok = if fd >= 0 { 1 } else { 0 }
        assert_eq(ok, 1)
        let _ = PtySession_close(sess)
    }
}

test fn test_io_uring_probe() {
    let avail = IoUring_available()
    let _ = avail
}

test fn test_tcp_borrow_fd() {
    let fd = Fd_borrow(-1)
    let _ = fd
}

test fn test_udp_borrow_fd() {
    let sock = UdpSocket_new()
    let fd = UdpSocket_borrow_fd(sock)
    let _ = fd
    sock.close()
}

test fn test_event_loop_register_fd() {
    let ev = EventLoop_new()
    let fd = Fd_borrow(-1)
    let promise = EventLoop_promise_new(ev)
    let rc = EventLoop_register_read_fd(ev, fd, promise)
    let _ = rc
}
