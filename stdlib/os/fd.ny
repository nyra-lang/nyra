// First-class file descriptor — owns close on Drop when `owns` is set.

import "syscall.ny"

struct Fd {
    raw: i32
    owns: i32
}

fn Fd_new(raw: i32) -> Fd {
    return Fd { raw: raw, owns: 1 }
}

fn Fd_borrow(raw: i32) -> Fd {
    return Fd { raw: raw, owns: 0 }
}

fn Fd_raw(fd: Fd) -> i32 {
    return fd.raw
}

fn Fd_is_valid(fd: Fd) -> bool {
    return fd.raw >= 0
}

fn Fd_drop_impl(fd: Fd) -> void {
    if fd.owns == 1 && fd.raw >= 0 {
        os_close_fd(fd.raw)
    }
}

impl Drop for Fd {
    fn drop(self: Fd) -> void {
        Fd_drop_impl(self)
    }
}
