// Raw HTTP/1.1 server — accept one connection, parse request, reply (stdlib Core).
import "../../strings.ny"
import "../tcp.ny"
import "request.ny"
import "response.ny"
import "router.ny"
import "handler.ny"

fn Http_builtin_dispatch(ctx: RequestContext) -> HttpResponse {
    if ctx.method == METHOD_OPTIONS {
        return response_no_content()
    }
    if strcmp(ctx.path, "/health") == 0 {
        return response_ok_json("{\"ok\":true}")
    }
    if strcmp(ctx.path, "/") == 0 {
        return response_html(STATUS_OK, "<html><body><h1>Nyra HTTP</h1><p>GET /health · POST /echo</p></body></html>")
    }
    if strcmp(ctx.path, "/echo") == 0 && ctx.method == METHOD_POST {
        return response_text(STATUS_OK, ctx.body)
    }
    return response_not_found()
}

fn Server_handle_stream(stream: TcpStream) -> void {
    let raw = tcp_read(stream, 65536)
    if strlen(raw) == 0 {
        return
    }
    let ctx = RequestContext_from_raw(raw)
    if ctx.method == METHOD_OPTIONS {
        tcp_write(stream, build_options_preflight())
        return
    }
    let resp = Http_builtin_dispatch(ctx)
    let ka = wants_keep_alive(raw)
    tcp_write(stream, build_response(resp, ka))
}

fn Server_handle_stream_keepalive(stream: TcpStream, max_hops: i32) -> void {
    let mut hops = 0
    while hops < max_hops {
        let raw = tcp_read(stream, 65536)
        if strlen(raw) == 0 {
            break
        }
        let ctx = RequestContext_from_raw(raw)
        if ctx.method == METHOD_OPTIONS {
            tcp_write(stream, build_options_preflight())
        } else {
            let resp = Http_builtin_dispatch(ctx)
            let ka = wants_keep_alive(raw)
            tcp_write(stream, build_response(resp, ka))
            if ka == 0 {
                break
            }
        }
        hops = hops + 1
    }
}

fn Server_handle_router(stream: TcpStream, router: HttpRouter) -> void {
    let raw = tcp_read(stream, 65536)
    if strlen(raw) == 0 {
        return
    }
    let ctx = RequestContext_from_raw(raw)
    let tag = HttpRouter_lookup(router, ctx)
    let mut resp = response_not_found()
    if strcmp(tag, "health") == 0 {
        resp = response_ok_json("{\"ok\":true}")
    } else {
        if strcmp(tag, "echo") == 0 && ctx.method == METHOD_POST {
            resp = response_text(STATUS_OK, ctx.body)
        }
    }
    let ka = wants_keep_alive(raw)
    tcp_write(stream, build_response(resp, ka))
}

fn Server_serve_loop(host: string, port: i32, max_requests: i32) -> i32 {
    let listener = tcp_listen(host, port)
    if listener.fd < 0 {
        print("net/http: failed to bind")
        return 0
    }
    print(strcat(
        strcat("net/http listening on ", host),
        strcat(":", i32_to_string(port))
    ))
    let mut count = 0
    while count < max_requests {
        let stream = tcp_accept(listener)
        if stream.fd < 0 {
            break
        }
        Server_handle_stream(stream)
        tcp_close_stream(stream)
        count = count + 1
    }
    tcp_close_listener(listener)
    return 1
}

fn Server_handle_router_handler(stream: TcpStream, router: HttpRouter, handler: fn(i32, RequestContext) -> HttpResponse) -> void {
    let raw = tcp_read(stream, 65536)
    if strlen(raw) == 0 {
        return
    }
    let ctx = RequestContext_from_raw(raw)
    let m = HttpRouter_match(router, ctx)
    let ctx2 = RequestContext_with_params(ctx, m.params)
    let resp = Http_dispatch_slot(m.slot, ctx2, handler)
    let ka = wants_keep_alive(raw)
    tcp_write(stream, build_response(resp, ka))
}

fn Server_serve_handlers(host: string, port: i32, max_requests: i32, router: HttpRouter, handler: fn(i32, RequestContext) -> HttpResponse) -> i32 {
    let listener = tcp_listen(host, port)
    if listener.fd < 0 {
        print("net/http: failed to bind")
        return 0
    }
    print(strcat(
        strcat("net/http listening on ", host),
        strcat(":", i32_to_string(port))
    ))
    let mut count = 0
    while count < max_requests {
        let stream = tcp_accept_wait(listener, 60000)
        if stream.fd < 0 {
            break
        }
        Server_handle_router_handler(stream, router, handler)
        tcp_close_stream(stream)
        count = count + 1
    }
    tcp_close_listener(listener)
    return 1
}

fn serve_handlers(host: string, port: i32, max_requests: i32, router: HttpRouter, handler: fn(i32, RequestContext) -> HttpResponse) -> i32 {
    return Server_serve_handlers(host, port, max_requests, router, handler)
}

fn serve_loop(host: string, port: i32, max_requests: i32) -> i32 {
    return Server_serve_loop(host, port, max_requests)
}

fn Server_listen_once(host: string, port: i32, body: string) -> i32 {
    let listener = tcp_listen(host, port)
    if listener.fd < 0 {
        print("net/http: failed to bind")
        return 0
    }
    print(strcat(
        strcat("net/http listening on ", host),
        strcat(":", i32_to_string(port))
    ))
    let stream = tcp_accept(listener)
    if stream.fd < 0 {
        tcp_close_listener(listener)
        return 0
    }
    let raw = tcp_read(stream, 65536)
    if strlen(raw) > 0 {
        let resp = response_ok_json(body)
        tcp_write(stream, build_response(resp, 0))
    }
    tcp_close_stream(stream)
    tcp_close_listener(listener)
    return 1
}

fn serve_once(host: string, port: i32, body: string) -> i32 {
    return Server_listen_once(host, port, body)
}
