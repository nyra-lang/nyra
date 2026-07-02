// POSIX shared memory — named regions for cross-process IPC.

import "fd.ny"

extern fn shm_create(name: string, nbytes: i64) -> i32
extern fn shm_open_existing(name: string, nbytes: i64) -> i32
extern fn shm_map(fd: i32, nbytes: i64) -> ptr
extern fn shm_unmap(addr: ptr, nbytes: i64) -> i32
extern fn shm_close_fd(fd: i32) -> i32
extern fn shm_unlink_region(name: string) -> i32

struct ShmRegion {
    fd: i32
    addr: ptr
    size: i64
}

fn ShmRegion_create(name: string, nbytes: i64) -> ShmRegion {
    let fd = shm_create(name, nbytes)
    if fd < 0 {
        return ShmRegion { fd: -1, addr: shm_map(-1, 0), size: nbytes }
    }
    let addr = shm_map(fd, nbytes)
    return ShmRegion { fd: fd, addr: addr, size: nbytes }
}

fn ShmRegion_open(name: string, nbytes: i64) -> ShmRegion {
    let fd = shm_open_existing(name, nbytes)
    if fd < 0 {
        return ShmRegion { fd: -1, addr: shm_map(-1, 0), size: nbytes }
    }
    let addr = shm_map(fd, nbytes)
    return ShmRegion { fd: fd, addr: addr, size: nbytes }
}

fn ShmRegion_unmap(region: ShmRegion) -> void {
    if region.fd >= 0 {
        if region.size > 0 {
            shm_unmap(region.addr, region.size)
        }
        shm_close_fd(region.fd)
    }
}

fn ShmRegion_unlink_name(name: string) -> i32 {
    return shm_unlink_region(name)
}

fn ShmRegion_borrow_fd(region: ShmRegion) -> Fd {
    return Fd_borrow(region.fd)
}

fn ShmRegion_into_fd(region: ShmRegion) -> Fd {
    return Fd_new(region.fd)
}
