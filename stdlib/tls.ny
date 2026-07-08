// TLS — HTTPS client via selectable backend (`tls rustls|native|openssl` in nyra.mod).
// Default: bundled rustls (`libnyra_rt_tls.a`). TLS *server* may still use OpenSSL when available.
extern fn tls_available() -> i32
extern fn rt_tls_connect(host: string, port: i32) -> i32
extern fn rt_tls_connect_verify(host: string, port: i32) -> i32
extern fn rt_tls_connect_ca(host: string, port: i32, ca_path: string) -> i32
extern fn rt_tls_read(handle: i32, max_bytes: i32) -> string
extern fn rt_tls_write(handle: i32, data: string) -> i32
extern fn rt_tls_close(handle: i32) -> void
extern fn rt_tls_listen(cert_path: string, key_path: string, host: string, port: i32) -> i32
extern fn rt_tls_accept(listener: i32) -> i32
extern fn rt_tls_listener_close(listener: i32) -> void
extern fn rt_tls_last_error() -> string
extern fn rt_tls_validate_pem_files(cert_path: string, key_path: string) -> i32

fn tls_connect(host: string, port: i32) -> i32 {
    return rt_tls_connect(host, port)
}

fn tls_connect_verify(host: string, port: i32) -> i32 {
    return rt_tls_connect_verify(host, port)
}

fn tls_connect_ca(host: string, port: i32, ca_path: string) -> i32 {
    return rt_tls_connect_ca(host, port, ca_path)
}

fn tls_last_error() -> string {
    return rt_tls_last_error()
}

fn tls_validate_pem(cert_path: string, key_path: string) -> i32 {
    return rt_tls_validate_pem_files(cert_path, key_path)
}

fn tls_read(handle: i32, max_bytes: i32) -> string {
    return rt_tls_read(handle, max_bytes)
}

fn tls_write(handle: i32, data: string) -> i32 {
    return rt_tls_write(handle, data)
}

fn tls_close(handle: i32) -> void {
    rt_tls_close(handle)
}

fn tls_ready() -> bool {
    return tls_available() != 0
}

fn tls_require(feature: string) -> bool {
    if tls_ready() {
        return true
    }
    let detail = tls_last_error()
    if strlen(detail) > 0 {
        print(strcat(strcat(feature, ": "), detail))
    } else {
        print(strcat(feature, ": TLS is unavailable"))
    }
    return false
}

fn tls_listen(cert_path: string, key_path: string, host: string, port: i32) -> i32 {
    return rt_tls_listen(cert_path, key_path, host, port)
}

fn tls_accept(listener: i32) -> i32 {
    return rt_tls_accept(listener)
}

fn tls_listener_close(listener: i32) -> void {
    rt_tls_listener_close(listener)
}

extern fn rt_tls_upgrade_client(plain_fd: i32, hostname: string) -> i32
extern fn rt_tls_upgrade_client_verify(plain_fd: i32, hostname: string) -> i32
extern fn rt_tls_upgrade_client_ex(plain_fd: i32, hostname: string, ca_path: string, verify_peer: i32) -> i32

fn tls_upgrade_fd(plain_fd: i32, hostname: string) -> i32 {
    return rt_tls_upgrade_client(plain_fd, hostname)
}

fn tls_upgrade_verify(plain_fd: i32, hostname: string) -> i32 {
    return rt_tls_upgrade_client_verify(plain_fd, hostname)
}

fn tls_upgrade_ca(plain_fd: i32, hostname: string, ca_path: string) -> i32 {
    return rt_tls_upgrade_client_ex(plain_fd, hostname, ca_path, 1)
}
