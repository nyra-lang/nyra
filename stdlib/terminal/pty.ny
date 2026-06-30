extern fn pty_spawn(shell: string, rows: i32, cols: i32) -> i32
extern fn pty_write(master: i32, data: string) -> i32
extern fn pty_read(master: i32, max_bytes: i32) -> string
extern fn pty_drain(master: i32, max_bytes: i32) -> string
extern fn pty_drain_raw(master: i32, max_bytes: i32) -> string
extern fn pty_flush_stdout(master: i32, max_bytes: i32, timeout_ms: i32) -> void
extern fn pty_read_wait(master: i32, max_bytes: i32, timeout_ms: i32) -> string
extern fn pty_read_wait_raw(master: i32, max_bytes: i32, timeout_ms: i32) -> string
extern fn pty_poll(master: i32) -> i32
extern fn pty_resize(master: i32, rows: i32, cols: i32) -> void
extern fn pty_close(master: i32) -> void
extern fn pty_wait(master: i32) -> i32
extern fn strlen(s: &string) -> i32

const PTY_ROWS = 36
const PTY_COLS = 120

struct PtySession {
    master_fd: i32
    rows: i32
    cols: i32
    alive: i32
}

fn PtySession_spawn(shell: string) -> PtySession {
    let fd = pty_spawn(shell, PTY_ROWS, PTY_COLS)
    return PtySession {
        master_fd: fd,
        rows: PTY_ROWS,
        cols: PTY_COLS,
        alive: if fd >= 0 { 1 } else { 0 },
    }
}

fn PtySession_write(sess: PtySession, data: string) -> void {
    if sess.alive == 1 {
        pty_write(sess.master_fd, data)
    }
}

fn PtySession_flush(sess: PtySession, timeout_ms: i32) -> void {
    if sess.alive == 1 {
        pty_flush_stdout(sess.master_fd, 4096, timeout_ms)
    }
}

fn PtySession_mark_dead(sess: PtySession) -> PtySession {
    return PtySession {
        master_fd: -1,
        rows: sess.rows,
        cols: sess.cols,
        alive: 0,
    }
}

fn PtySession_reap(sess: PtySession) -> PtySession {
    if sess.alive == 0 {
        return sess
    }
    if pty_wait(sess.master_fd) == 1 {
        return PtySession_mark_dead(sess)
    }
    return sess
}

fn PtySession_drain(sess: PtySession) -> string {
    if sess.alive == 0 {
        return ""
    }
    return pty_drain(sess.master_fd, 4096)
}

fn PtySession_drain_raw(sess: PtySession) -> string {
    if sess.alive == 0 {
        return ""
    }
    return pty_drain_raw(sess.master_fd, 4096)
}

fn PtySession_read_wait_raw(sess: PtySession, timeout_ms: i32) -> string {
    if sess.alive == 0 {
        return ""
    }
    return pty_read_wait_raw(sess.master_fd, 4096, timeout_ms)
}

fn PtySession_read(sess: PtySession) -> string {
    return PtySession_drain(sess)
}

fn PtySession_read_wait(sess: PtySession, timeout_ms: i32) -> string {
    if sess.alive == 0 {
        return ""
    }
    return pty_read_wait(sess.master_fd, 4096, timeout_ms)
}

fn PtySession_poll(sess: PtySession) -> i32 {
    if sess.alive == 0 {
        return 0
    }
    return pty_poll(sess.master_fd)
}

fn PtySession_resize(sess: PtySession, rows: i32, cols: i32) -> void {
    pty_resize(sess.master_fd, rows, cols)
}

fn PtySession_close(sess: PtySession) -> PtySession {
    pty_close(sess.master_fd)
    return PtySession_mark_dead(sess)
}
