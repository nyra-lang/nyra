import "../../map.ny"
import "../../strings.ny"
import "../../http/request.ny"
import "../../tls.ny"
import "../tcp.ny"
import "../syscall.ny"
import "types.ny"
import "response.ny"
import "request.ny"
import "headers.ny"

const POOL_TLS_BASE = 1048576

struct HttpPool {
    conns: HashMap_str_i32
}

struct PoolResponse {
    pool: HttpPool
    resp: HttpResponse
}

fn HttpPool_new() -> HttpPool {
    return HttpPool { conns: HashMap_str_i32_new() }
}

fn HttpPool_key(host: string, port: i32) -> string {
    return strcat(host, strcat(":", i32_to_string(port)))
}

fn HttpPool_is_tls(handle: i32) -> bool {
    return handle >= POOL_TLS_BASE
}

fn HttpPool_close_conn(handle: i32) -> void {
    if handle < 0 {
        return
    }
    if HttpPool_is_tls(handle) {
        tls_close(handle)
    } else {
        sys_close(handle)
    }
}

fn HttpPool_read_conn(handle: i32, max_bytes: i32) -> string {
    if HttpPool_is_tls(handle) {
        return tls_read(handle, max_bytes)
    }
    return sys_recv(handle, max_bytes)
}

fn HttpPool_write_conn(handle: i32, data: string) -> i32 {
    if HttpPool_is_tls(handle) {
        return tls_write(handle, data)
    }
    return sys_send(handle, data)
}

fn HttpPool_connect(parsed: HttpUrl) -> i32 {
    if parsed.secure {
        if !tls_require("HttpPool HTTPS") {
            return -1
        }
        return tls_connect_verify(parsed.host, parsed.port)
    }
    let stream = tcp_connect(parsed.host, parsed.port)
    return stream.fd
}

fn HttpPool_request(pool: HttpPool, method: i32, url: string, body: string, content_type: string) -> PoolResponse {
    let parsed = parse_http_url(url)
    let key = HttpPool_key(parsed.host, parsed.port)
    let mut out = pool
    let mut cached = -1
    if out.conns.contains(key) == 1 {
        cached = out.conns.get(key)
    }
    let mut handle = HttpPool_connect(parsed)
    if cached >= 0 {
        handle = cached
    }
    if handle < 0 {
        return PoolResponse { pool: out, resp: response_internal_error() }
    }
    let m = method_name(method)
    let req_line = strcat(strcat(m, " "), parsed.path)
    let mut hdr = strcat(
        strcat(
            strcat(req_line, " HTTP/1.1\r\nHost: "),
            parsed.host
        ),
        "\r\nUser-Agent: Nyra/1.0\r\nAccept: */*\r\nConnection: keep-alive\r\n"
    )
    if strlen(body) > 0 {
        let cl = strcat("Content-Length: ", i32_to_string(strlen(body)))
        hdr = strcat(
            strcat(
                strcat(hdr, strcat("Content-Type: ", content_type)),
                "\r\n"
            ),
            strcat(cl, "\r\n")
        )
    }
    let req = strcat(strcat(hdr, "\r\n"), body)
    if HttpPool_write_conn(handle, req) != 0 {
        HttpPool_close_conn(handle)
        out = HttpPool { conns: out.conns.insert(key, -1) }
        return PoolResponse { pool: out, resp: response_internal_error() }
    }
    let raw = HttpPool_read_conn(handle, 65536)
    if strlen(raw) == 0 {
        HttpPool_close_conn(handle)
        out = HttpPool { conns: out.conns.insert(key, -1) }
        return PoolResponse { pool: out, resp: response_internal_error() }
    }
    let status = http_status_from_header(raw)
    let resp_body = body_from_raw(raw)
    let resp_headers = HeaderMap_parse_raw(raw)
    if wants_keep_alive(raw) == 1 {
        out = HttpPool { conns: out.conns.insert(key, handle) }
    } else {
        HttpPool_close_conn(handle)
        out = HttpPool { conns: out.conns.insert(key, -1) }
    }
    let mut resp = HttpResponse_with_headers(status, resp_body, content_type, resp_headers)
    if method == METHOD_HEAD {
        resp = HttpResponse_with_headers(status, "", "text/plain", resp_headers)
    }
    return PoolResponse { pool: out, resp: resp }
}

fn HttpPool_get(pool: HttpPool, url: string) -> PoolResponse {
    return HttpPool_request(pool, METHOD_GET, url, "", "text/plain")
}
