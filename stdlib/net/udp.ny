import "syscall.ny"
import "stdlib/os/fd.ny"

extern fn rt_udp_bind(host: string, port: i32) -> i32
extern fn rt_udp_recv(fd: i32, max_bytes: i32) -> string
extern fn rt_udp_send(fd: i32, host: string, port: i32, data: string) -> i32
extern fn rt_udp_close(fd: i32) -> void

struct UdpSocket {
    fd: i32
}

fn UdpSocket_bind(host: string, port: i32) -> UdpSocket {
    let fd = rt_udp_bind(host, port)
    return UdpSocket { fd: fd }
}

fn UdpSocket_new() -> UdpSocket {
    return UdpSocket_bind("0.0.0.0", 0)
}

impl UdpSocket {
    fn recv(self, max_bytes: i32) -> string {
        return rt_udp_recv(self.fd, max_bytes)
    }

    fn send(self, host: string, port: i32, data: string) -> i32 {
        return rt_udp_send(self.fd, host, port, data)
    }

    fn close(self) -> void {
        rt_udp_close(self.fd)
    }
}

fn UdpSocket_borrow_fd(sock: UdpSocket) -> Fd {
    return Fd_borrow(sock.fd)
}

fn UdpSocket_into_fd(sock: UdpSocket) -> Fd {
    return Fd_new(sock.fd)
}
