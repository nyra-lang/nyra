import "syscall.ny"
import "../os/fd.ny"

struct TcpListener Send {
    fd: i32
}

struct TcpStream Send {
    fd: i32
}

fn tcp_listen(host: string, port: i32) -> TcpListener {
    let fd = sys_listen(host, port)
    return TcpListener { fd: fd }
}

fn tcp_accept(listener: TcpListener) -> TcpStream {
    let fd = sys_accept(listener.fd)
    return TcpStream { fd: fd }
}

fn tcp_connect(host: string, port: i32) -> TcpStream {
    let fd = sys_connect(host, port)
    return TcpStream { fd: fd }
}

extern fn rt_tcp_connect_timeout(host: string, port: i32, timeout_ms: i32) -> i32

fn tcp_connect_timeout(host: string, port: i32, timeout_ms: i32) -> TcpStream {
    let fd = rt_tcp_connect_timeout(host, port, timeout_ms)
    return TcpStream { fd: fd }
}

fn tcp_accept_task(listener: TcpListener) -> i32 {
    return tcp_accept_async(listener.fd)
}

extern fn async_poll(handle: i32) -> i32

fn tcp_accept_wait(listener: TcpListener, timeout_ms: i32) -> TcpStream {
    let task = tcp_accept_task(listener)
    let mut waited = 0
    while waited < timeout_ms {
        let fd = async_poll(task)
        if fd >= 0 {
            return TcpStream { fd: fd }
        }
        sleep_ms(10)
        waited = waited + 10
    }
    return TcpStream { fd: -1 }
}

fn tcp_read(stream: TcpStream, max_bytes: i32) -> string {
    return sys_recv(stream.fd, max_bytes)
}

fn tcp_write(stream: TcpStream, data: string) -> i32 {
    return sys_send(stream.fd, data)
}

fn tcp_close_stream(stream: TcpStream) -> void {
    sys_close(stream.fd)
}

fn tcp_close_listener(listener: TcpListener) -> void {
    sys_close(listener.fd)
}

fn tcp_set_nonblock(stream: TcpStream) -> i32 {
    return sys_set_nonblock(stream.fd)
}

fn TcpStream_borrow_fd(stream: TcpStream) -> Fd {
    return Fd_borrow(stream.fd)
}

fn TcpStream_into_fd(stream: TcpStream) -> Fd {
    return Fd_new(stream.fd)
}

fn TcpListener_borrow_fd(listener: TcpListener) -> Fd {
    return Fd_borrow(listener.fd)
}
