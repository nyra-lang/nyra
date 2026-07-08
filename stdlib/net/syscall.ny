// Thin OS syscall layer (C runtime). TCP/HTTP logic lives in Nyra under stdlib/net and stdlib/http.

extern fn sys_listen(host: string, port: i32) -> i32
extern fn sys_accept(listener_fd: i32) -> i32
extern fn sys_connect(host: string, port: i32) -> i32
extern fn sys_recv(fd: i32, max_bytes: i32) -> string
extern fn sys_send(fd: i32, data: string) -> i32
extern fn sys_close(fd: i32) -> void
extern fn sys_set_nonblock(fd: i32) -> i32
extern fn sys_set_timeout_ms(fd: i32, timeout_ms: i32) -> i32

extern fn tcp_accept_async(listener_fd: i32) -> i32
