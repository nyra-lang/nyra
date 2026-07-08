import "../../strings.ny"
import "../../http/request.ny"
import "../../tls.ny"
import "../tcp.ny"
import "types.ny"
import "response.ny"
import "request.ny"

fn http_request(method: i32, url: string, body: string, content_type: string) -> HttpResponse {
    let parsed = parse_http_url(url)
    let m = method_name(method)
    let req_line = strcat(strcat(m, " "), parsed.path)
    let mut hdr = strcat(
        strcat(
            strcat(req_line, " HTTP/1.1\r\nHost: "),
            parsed.host
        ),
        "\r\nUser-Agent: Nyra/1.0\r\nAccept: */*\r\n"
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
    let req = strcat(strcat(hdr, "Connection: close\r\n\r\n"), body)
    let raw = transport_roundtrip(parsed, req)
    if strlen(raw) == 0 {
        let detail = tls_last_error()
        if strlen(detail) > 0 {
            return HttpResponse {
                status: 0,
                body: strcat("{\"error\":\"", strcat(detail, "\"}")),
                content_type: "application/json"
            }
        }
        return HttpResponse {
            status: 0,
            body: "{\"error\":\"request failed\"}",
            content_type: "application/json"
        }
    }
    let status = http_status_from_header(raw)
    let resp_body = body_from_raw(raw)
    if method == METHOD_HEAD {
        return HttpResponse { status: status, body: "", content_type: "text/plain" }
    }
    return HttpResponse { status: status, body: resp_body, content_type: "application/json" }
}

fn transport_roundtrip(parsed: HttpUrl, req: string) -> string {
    if parsed.secure {
        if !tls_ready() {
            let detail = tls_last_error()
            if strlen(detail) > 0 {
                print(strcat("HTTPS failed: ", detail))
            } else {
                print("HTTPS failed: TLS is unavailable")
            }
            return ""
        }
        let handle = tls_connect_verify(parsed.host, parsed.port)
        if handle < 0 {
            let detail = tls_last_error()
            if strlen(detail) > 0 {
                print(strcat("HTTPS connect failed: ", detail))
            } else {
                print("HTTPS connect failed")
            }
            return ""
        }
        if tls_write(handle, req) != 0 {
            let detail = tls_last_error()
            tls_close(handle)
            if strlen(detail) > 0 {
                print(strcat("HTTPS write failed: ", detail))
            }
            return ""
        }
        let raw = tls_read(handle, 65536)
        tls_close(handle)
        return raw
    }
    let stream = tcp_connect(parsed.host, parsed.port)
    if stream.fd < 0 {
        return ""
    }
    if tcp_write(stream, req) != 0 {
        tcp_close_stream(stream)
        return ""
    }
    let raw = tcp_read(stream, 65536)
    tcp_close_stream(stream)
    return raw
}

fn get(url: string) -> string {
    let resp = http_request(METHOD_GET, url, "", "text/plain")
    return resp.body
}

fn head(url: string) -> HttpResponse {
    return http_request(METHOD_HEAD, url, "", "text/plain")
}

fn post(url: string, body: string) -> HttpResponse {
    return http_request(METHOD_POST, url, body, "application/json")
}

fn put(url: string, body: string) -> HttpResponse {
    return http_request(METHOD_PUT, url, body, "application/json")
}

fn patch(url: string, body: string) -> HttpResponse {
    return http_request(METHOD_PATCH, url, body, "application/json")
}

fn delete(url: string) -> HttpResponse {
    return http_request(METHOD_DELETE, url, "", "text/plain")
}

fn fetch(url: string) -> string {
    return get(url)
}

impl Client {
    fn do_get(self, url: string) -> HttpResponse {
        return http_request(METHOD_GET, url, "", "text/plain")
    }

    fn do_post(self, url: string, body: string) -> HttpResponse {
        return http_request(METHOD_POST, url, body, "application/json")
    }

    fn do_put(self, url: string, body: string) -> HttpResponse {
        return http_request(METHOD_PUT, url, body, "application/json")
    }

    fn do_delete(self, url: string) -> HttpResponse {
        return http_request(METHOD_DELETE, url, "", "text/plain")
    }
}
