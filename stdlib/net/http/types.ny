// net/http — core types and HTTP constants (Go net/http inspired).
import "../../map.ny"
import "../../strings/ops.ny"
import "headers.ny"

const METHOD_GET = 1
const METHOD_POST = 2
const METHOD_PUT = 3
const METHOD_DELETE = 4
const METHOD_OPTIONS = 5
const METHOD_HEAD = 6
const METHOD_PATCH = 7

// Parse "GET" / "post" / etc. into METHOD_* (unknown → GET).
fn method_from_name(name: string) -> i32 {
    let u = str_to_upper(name)
    if strcmp(u, "GET") == 0 { return METHOD_GET }
    if strcmp(u, "POST") == 0 { return METHOD_POST }
    if strcmp(u, "PUT") == 0 { return METHOD_PUT }
    if strcmp(u, "DELETE") == 0 { return METHOD_DELETE }
    if strcmp(u, "PATCH") == 0 { return METHOD_PATCH }
    if strcmp(u, "HEAD") == 0 { return METHOD_HEAD }
    if strcmp(u, "OPTIONS") == 0 { return METHOD_OPTIONS }
    return METHOD_GET
}

const STATUS_OK = 200
const STATUS_CREATED = 201
const STATUS_NO_CONTENT = 204
const STATUS_BAD_REQUEST = 400
const STATUS_UNAUTHORIZED = 401
const STATUS_NOT_FOUND = 404
const STATUS_METHOD_NOT_ALLOWED = 405
const STATUS_UNPROCESSABLE = 422
const STATUS_TOO_MANY_REQUESTS = 429
const STATUS_INTERNAL_ERROR = 500

struct HttpRequest {
    method: i32
    url: string
    body: string
    content_type: string
}

struct HttpResponse {
    status: i32
    body: string
    content_type: string
    headers: HashMap_str_str
}

struct RequestContext {
    method: i32
    path: string
    body: string
    query: string
    raw: string
    params: HashMap_str_str
}

struct Server {
    host: string
    port: i32
    router: ptr
    cors: i32
    keep_alive: i32
}

struct Client {
    user_agent: string
    timeout_ms: i32
}

fn Client_default() -> Client {
    return Client { user_agent: "Nyra/1.0", timeout_ms: 30000 }
}

fn Client_with_timeout(timeout_ms: i32) -> Client {
    return Client { user_agent: "Nyra/1.0", timeout_ms: timeout_ms }
}

fn HttpRequest_new(method: i32, url: string, body: string) -> HttpRequest {
    return HttpRequest { method: method, url: url, body: body, content_type: "application/json" }
}

fn HttpResponse_new(status: i32, body: string, content_type: string) -> HttpResponse {
    return HttpResponse {
        status: status,
        body: body,
        content_type: content_type,
        headers: HeaderMap_new(),
    }
}

fn HttpResponse_with_headers(status: i32, body: string, content_type: string, headers: HashMap_str_str) -> HttpResponse {
    let mut ct = content_type
    let from_hdr = HeaderMap_get(headers, "Content-Type")
    if strlen(from_hdr) > 0 {
        ct = from_hdr
    }
    return HttpResponse {
        status: status,
        body: body,
        content_type: ct,
        headers: headers,
    }
}

fn HttpResponse_ok(body: string) -> HttpResponse {
    return HttpResponse_new(STATUS_OK, body, "application/json")
}

fn HttpResponse_with_status(resp: HttpResponse, status: i32) -> HttpResponse {
    return HttpResponse {
        status: status,
        body: resp.body,
        content_type: resp.content_type,
        headers: resp.headers,
    }
}

fn HttpResponse_with_content_type(resp: HttpResponse, content_type: string) -> HttpResponse {
    return HttpResponse {
        status: resp.status,
        body: resp.body,
        content_type: content_type,
        headers: resp.headers,
    }
}

fn HttpResponse_header(resp: HttpResponse, name: string) -> string {
    return HeaderMap_get(resp.headers, name)
}

fn HttpResponse_text(resp: HttpResponse) -> string {
    return resp.body
}
