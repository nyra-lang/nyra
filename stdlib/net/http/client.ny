import "../../strings.ny"
import "../../map.ny"
import "../../tls.ny"
import "../tcp.ny"
import "../syscall.ny"
import "types.ny"
import "response.ny"
import "request.ny"
import "headers.ny"
import "cookies.ny"
import "abort.ny"
import "fetch.ny"
import "../../http/request.ny"

fn http_error_response(detail: string) -> HttpResponse {
    if strlen(detail) > 0 {
        return HttpResponse_new(0, strcat("{\"error\":\"", strcat(detail, "\"}")), "application/json")
    }
    return HttpResponse_new(0, "{\"error\":\"request failed\"}", "application/json")
}

fn build_request_bytes(method: i32, parsed: HttpUrl, body: string, content_type: string, headers: HashMap_str_str) -> string {
    let m = method_name(method)
    let req_line = strcat(strcat(m, " "), parsed.path)
    let mut hdr = strcat(
        strcat(
            strcat(req_line, " HTTP/1.1\r\nHost: "),
            parsed.host
        ),
        "\r\nUser-Agent: Nyra/1.0\r\nAccept: */*\r\n"
    )
    if HeaderMap_has(headers, "User-Agent") == 0 {
        // default already set
    }
    let custom = HeaderMap_format(headers)
    if strlen(custom) > 0 {
        hdr = strcat(hdr, custom)
    }
    if strlen(body) > 0 {
        let mut ct = content_type
        if strlen(ct) == 0 {
            ct = "application/octet-stream"
        }
        if HeaderMap_has(headers, "Content-Type") == 0 {
            hdr = strcat(hdr, strcat(strcat("Content-Type: ", ct), "\r\n"))
        }
        if HeaderMap_has(headers, "Content-Length") == 0 {
            let cl = strcat("Content-Length: ", i32_to_string(strlen(body)))
            hdr = strcat(hdr, strcat(cl, "\r\n"))
        }
    }
    if HeaderMap_has(headers, "Connection") == 0 {
        hdr = strcat(hdr, "Connection: close\r\n")
    }
    return strcat(strcat(hdr, "\r\n"), body)
}

fn transport_roundtrip_timeout(parsed: HttpUrl, req: string, timeout_ms: i32) -> string {
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
    let stream = if timeout_ms > 0 {
        tcp_connect_timeout(parsed.host, parsed.port, timeout_ms)
    } else {
        tcp_connect(parsed.host, parsed.port)
    }
    if stream.fd < 0 {
        return ""
    }
    if timeout_ms > 0 {
        sys_set_timeout_ms(stream.fd, timeout_ms)
    }
    if tcp_write(stream, req) != 0 {
        tcp_close_stream(stream)
        return ""
    }
    let raw = tcp_read(stream, 65536)
    tcp_close_stream(stream)
    return raw
}

fn transport_roundtrip(parsed: HttpUrl, req: string) -> string {
    return transport_roundtrip_timeout(parsed, req, 0)
}

fn http_request_once(method: i32, url: string, body: string, content_type: string, headers: HashMap_str_str, timeout_ms: i32) -> HttpResponse {
    let parsed = parse_http_url(url)
    let req = build_request_bytes(method, parsed, body, content_type, headers)
    let raw = transport_roundtrip_timeout(parsed, req, timeout_ms)
    if strlen(raw) == 0 {
        let detail = tls_last_error()
        return http_error_response(detail)
    }
    let status = http_status_from_header(raw)
    let resp_body = body_from_raw(raw)
    let resp_headers = HeaderMap_parse_raw(raw)
    if method == METHOD_HEAD {
        return HttpResponse_with_headers(status, "", "text/plain", resp_headers)
    }
    let mut ct = "application/json"
    let hdr_ct = HeaderMap_get(resp_headers, "Content-Type")
    if strlen(hdr_ct) > 0 {
        ct = hdr_ct
    }
    return HttpResponse_with_headers(status, resp_body, ct, resp_headers)
}

fn is_redirect_status(status: i32) -> i32 {
    if status == 301 || status == 302 || status == 303 || status == 307 || status == 308 {
        return 1
    }
    return 0
}

fn resolve_redirect_url(base: string, location: string) -> string {
    if strlen(location) >= 7 {
        if strcmp(substring(location, 0, 7), "http://") == 0 {
            return location
        }
    }
    if strlen(location) >= 8 {
        if strcmp(substring(location, 0, 8), "https://") == 0 {
            return location
        }
    }
    let parsed = parse_http_url(base)
    let mut scheme = "http://"
    if parsed.secure {
        scheme = "https://"
    }
    if strlen(location) > 0 && char_at(location, 0) == 47 {
        return strcat(strcat(scheme, parsed.host), location)
    }
    return strcat(strcat(strcat(scheme, parsed.host), "/"), location)
}

fn http_request_with(url: string, init: RequestInit) -> HttpResponse {
    if AbortSignal_aborted(init.signal) == 1 {
        return HttpResponse_new(0, "{\"error\":\"aborted\"}", "application/json")
    }
    let mut jar = init.jar
    let mut method = init.method
    let mut body = init.body
    let mut content_type = init.content_type
    let mut current = url
    let mut hops = 0
    while hops <= init.max_redirects {
        if AbortSignal_aborted(init.signal) == 1 {
            return HttpResponse_new(0, "{\"error\":\"aborted\"}", "application/json")
        }
        let mut headers = init.headers
        let cookie = CookieJar_header(jar)
        if strlen(cookie) > 0 && HeaderMap_has(headers, "Cookie") == 0 {
            headers = HeaderMap_set(headers, "Cookie", cookie)
        }
        let resp = http_request_once(method, current, body, content_type, headers, init.timeout_ms)
        jar = CookieJar_absorb_response(jar, resp.headers)
        if is_redirect_status(resp.status) == 1 {
            if init.redirect == REDIRECT_ERROR {
                return HttpResponse_new(resp.status, "{\"error\":\"redirect\"}", "application/json")
            }
            if init.redirect == REDIRECT_MANUAL {
                return resp
            }
            let loc = HeaderMap_get(resp.headers, "Location")
            if strlen(loc) == 0 {
                return resp
            }
            current = resolve_redirect_url(current, loc)
            if resp.status == 303 {
                method = METHOD_GET
                body = ""
                content_type = ""
            }
            hops = hops + 1
            continue
        }
        return resp
    }
    return HttpResponse_new(0, "{\"error\":\"too many redirects\"}", "application/json")
}

fn http_request(method: i32, url: string, body: string, content_type: string) -> HttpResponse {
    let mut init = RequestInit_new()
    init = RequestInit_method(init, method)
    init = RequestInit_body(init, body, content_type)
    return http_request_with(url, init)
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

// Primary entry (JS-like): returns full response. Body-only → get(url) / resp.text().
fn fetch(url: string) -> HttpResponse {
    return http_request_with(url, RequestInit_new())
}

fn fetch_with(url: string, init: RequestInit) -> HttpResponse {
    return http_request_with(url, init)
}

impl Client {
    fn do_get(self, url: string) -> HttpResponse {
        let mut init = RequestInit_new()
        init = RequestInit_method(init, METHOD_GET)
        init = RequestInit_timeout(init, self.timeout_ms)
        return http_request_with(url, init)
    }

    fn do_post(self, url: string, body: string) -> HttpResponse {
        let mut init = RequestInit_new()
        init = RequestInit_method(init, METHOD_POST)
        init = RequestInit_json(init, body)
        init = RequestInit_timeout(init, self.timeout_ms)
        return http_request_with(url, init)
    }

    fn do_put(self, url: string, body: string) -> HttpResponse {
        let mut init = RequestInit_new()
        init = RequestInit_method(init, METHOD_PUT)
        init = RequestInit_json(init, body)
        init = RequestInit_timeout(init, self.timeout_ms)
        return http_request_with(url, init)
    }

    fn do_delete(self, url: string) -> HttpResponse {
        let mut init = RequestInit_new()
        init = RequestInit_method(init, METHOD_DELETE)
        init = RequestInit_timeout(init, self.timeout_ms)
        return http_request_with(url, init)
    }

    fn do_request(self, url: string, init: RequestInit) -> HttpResponse {
        let timed = RequestInit_timeout(init, self.timeout_ms)
        return http_request_with(url, timed)
    }
}
