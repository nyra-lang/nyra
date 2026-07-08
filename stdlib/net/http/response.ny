import "../../strings.ny"
import "../../map.ny"
import "../../json/mod.ny"
import "types.ny"
import "headers.ny"
import "body.ny"

fn status_text(code: i32) -> string {
    if code == 200 { return "OK" }
    if code == 201 { return "Created" }
    if code == 204 { return "No Content" }
    if code == 400 { return "Bad Request" }
    if code == 401 { return "Unauthorized" }
    if code == 404 { return "Not Found" }
    if code == 405 { return "Method Not Allowed" }
    if code == 422 { return "Unprocessable Entity" }
    if code == 429 { return "Too Many Requests" }
    if code == 500 { return "Internal Server Error" }
    return "OK"
}

fn response_text(status: i32, body: string) -> HttpResponse {
    return HttpResponse_new(status, body, "text/plain; charset=utf-8")
}

fn response_json(status: i32, body: string) -> HttpResponse {
    return HttpResponse_new(status, body, "application/json; charset=utf-8")
}

fn response_html(status: i32, body: string) -> HttpResponse {
    return HttpResponse_new(status, body, "text/html; charset=utf-8")
}

fn response_ok_json(body: string) -> HttpResponse {
    return response_json(STATUS_OK, body)
}

fn response_created_json(body: string) -> HttpResponse {
    return response_json(STATUS_CREATED, body)
}

fn response_no_content() -> HttpResponse {
    return HttpResponse_new(STATUS_NO_CONTENT, "", "text/plain")
}

fn response_not_found() -> HttpResponse {
    return response_json(STATUS_NOT_FOUND, "{\"error\":\"not found\"}")
}

fn response_bad_request() -> HttpResponse {
    return response_json(STATUS_BAD_REQUEST, "{\"error\":\"bad request\"}")
}

fn response_unauthorized() -> HttpResponse {
    return response_json(STATUS_UNAUTHORIZED, "{\"error\":\"unauthorized\"}")
}

fn response_internal_error() -> HttpResponse {
    return response_json(STATUS_INTERNAL_ERROR, "{\"error\":\"internal server error\"}")
}

fn response_method_not_allowed() -> HttpResponse {
    return response_json(STATUS_METHOD_NOT_ALLOWED, "{\"error\":\"method not allowed\"}")
}

fn build_response(resp: HttpResponse, keep_alive: i32) -> string {
    let code = i32_to_string(resp.status)
    let reason = status_text(resp.status)
    let status_line = strcat(
        strcat(strcat("HTTP/1.1 ", code), " "),
        strcat(reason, "\r\n")
    )
    let ct = strcat("Content-Type: ", resp.content_type)
    let clen = strlen(resp.body)
    let cl = strcat("Content-Length: ", i32_to_string(clen))
    let conn = if keep_alive == 1 {
        "Connection: keep-alive\r\n"
    } else {
        "Connection: close\r\n"
    }
    let extra = HeaderMap_format(resp.headers)
    let hdr = strcat(
        strcat(
            strcat(strcat(strcat(ct, "\r\n"), strcat(cl, "\r\n")), extra),
            conn
        ),
        "\r\n"
    )
    return strcat(strcat(status_line, hdr), resp.body)
}

fn build_options_preflight() -> string {
    let hdr = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nAccess-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n"
    return hdr
}

fn http_status_from_header(header: string) -> i32 {
    if strlen(header) < 5 {
        return 0
    }
    if strcmp(substring(header, 0, 5), "HTTP/") != 0 {
        return 0
    }
    let sp = strstr_pos(header, " ")
    if sp < 0 {
        return 0
    }
    let mut i = sp + 1
    let mut code = 0
    let n = strlen(header)
    while i < n {
        let c = char_at(header, i)
        if c >= 48 && c <= 57 {
            code = code * 10 + (c - 48)
            i = i + 1
        } else {
            return code
        }
    }
    return code
}

fn http_body_from_response(raw: string) -> string {
    // Prefer chunked-aware decode (body_from_raw in request.ny). Kept for
    // callers that only import response.ny — strip headers only; no chunk decode.
    let sep = strstr_pos(raw, "\r\n\r\n")
    if sep < 0 {
        return raw
    }
    return substring(raw, sep + 4, strlen(raw) - (sep + 4))
}

fn response_json_object(resp: HttpResponse) -> HashMap_str_str {
    return JSON_parse_object(resp.body)
}

fn HttpResponse_json(resp: HttpResponse) -> HashMap_str_str {
    return JSON_parse_object(resp.body)
}

fn HttpResponse_array_buffer(resp: HttpResponse) -> ArrayBuffer {
    return ArrayBuffer_from_string(resp.body)
}

fn HttpResponse_blob(resp: HttpResponse) -> Blob {
    return Blob_from_string(resp.body, resp.content_type)
}
