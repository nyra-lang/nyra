import "../map.ny"

extern fn rt_tcp_hub_new(max_clients: i32) -> ptr
extern fn rt_tcp_hub_add(hub: ptr, fd: i32) -> i32
extern fn rt_tcp_hub_remove(hub: ptr, fd: i32) -> void
extern fn rt_tcp_hub_broadcast(hub: ptr, msg: string) -> void
extern fn rt_tcp_hub_free(hub: ptr) -> void

struct TcpHub Send {
    handle: ptr
}

fn TcpHub_new(max_clients: i32) -> TcpHub {
    return TcpHub { handle: rt_tcp_hub_new(max_clients) }
}

impl TcpHub {
    fn add(self, fd: i32) -> i32 {
        return rt_tcp_hub_add(self.handle, fd)
    }

    fn remove(self, fd: i32) -> TcpHub {
        rt_tcp_hub_remove(self.handle, fd)
        return self
    }

    fn broadcast(self, msg: string) -> TcpHub {
        rt_tcp_hub_broadcast(self.handle, msg)
        return self
    }
}

impl Drop for TcpHub {
    fn drop(self) -> void {
        rt_tcp_hub_free(self.handle)
    }
}
