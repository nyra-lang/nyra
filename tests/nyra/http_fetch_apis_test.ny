import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/json/mod.ny"
import "stdlib/encoding/mod.ny"

fn test_headers_map() {
    let mut h = HeaderMap_new()
    h = HeaderMap_set(h, "Authorization", "Bearer tok")
    h = HeaderMap_set(h, "Cookie", "a=1")
    assert_eq(strcmp(HeaderMap_get(h, "authorization"), "Bearer tok"), 0)
    assert_eq(strcmp(HeaderMap_get(h, "Cookie"), "a=1"), 0)
    let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Test: yes\r\n\r\n{\"ok\":1}"
    let parsed = HeaderMap_parse_raw(raw)
    assert_eq(strcmp(HeaderMap_get(parsed, "Content-Type"), "application/json"), 0)
    assert_eq(strcmp(HeaderMap_get(parsed, "X-Test"), "yes"), 0)
}

fn test_url_encode_decode() {
    let enc = url_encode("a b/c")
    assert_eq(strcmp(enc, "a%20b%2fc"), 0)
    let dec = url_decode("a%20b%2Fc")
    assert_eq(strcmp(dec, "a b/c"), 0)
    let plus = url_decode("a+b")
    assert_eq(strcmp(plus, "a b"), 0)
}

fn test_form_and_params() {
    let mut form = FormData_new()
    form = FormData_append(form, "name", "Nyra")
    let body = FormData_to_urlencoded(form)
    assert_eq(strcmp(body, "name=Nyra"), 0)
    let mut params = URLSearchParams_new()
    params = URLSearchParams_set(params, "q", "hello world")
    let qs = URLSearchParams_to_string(params)
    assert_eq(strcmp(qs, "q=hello%20world"), 0)
}

fn test_json_parse_full() {
    let obj = JSON_parse_object("{\"name\":\"Nyra\",\"n\":42,\"ok\":true}")
    assert_eq(strcmp(obj.get("name"), "\"Nyra\""), 0)
    assert_eq(strcmp(obj.get("n"), "42"), 0)
    assert_eq(strcmp(obj.get("ok"), "true"), 0)
    let resp = HttpResponse_new(200, "{\"status\":\"ok\"}", "application/json")
    let parsed = HttpResponse_json(resp)
    assert_eq(strcmp(parsed.get("status"), "\"ok\""), 0)
}

fn test_abort_and_cookies() {
    let mut ctrl = AbortController_new()
    assert_eq(AbortSignal_aborted(AbortController_signal(ctrl)), 0)
    ctrl = AbortController_abort(ctrl)
    assert_eq(AbortSignal_aborted(AbortController_signal(ctrl)), 1)
    let mut jar = CookieJar_new()
    jar = CookieJar_set(jar, "sid", "abc")
    assert_eq(strcmp(CookieJar_header(jar), "sid=abc"), 0)
    jar = CookieJar_apply_set_cookie(jar, "token=xyz; Path=/; HttpOnly")
    assert_eq(strcmp(CookieJar_get(jar, "token"), "xyz"), 0)
}

fn test_blob_array_buffer() {
    let blob = Blob_from_string("hi", "text/plain")
    assert_eq(strcmp(Blob_text(blob), "hi"), 0)
    assert_eq(Blob_size(blob), 2)
    let buf = ArrayBuffer_from_string("ab")
    assert_eq(ArrayBuffer_byte_length(buf), 2)
    let resp = HttpResponse_new(200, "data", "text/plain")
    let b2 = HttpResponse_blob(resp)
    assert_eq(strcmp(Blob_text(b2), "data"), 0)
}

fn test_request_init_headers() {
    let mut init = RequestInit_new()
    init = RequestInit_authorization(init, "Bearer SECRET")
    init = RequestInit_timeout(init, 1500)
    init = RequestInit_redirect(init, REDIRECT_MANUAL)
    assert_eq(strcmp(HeaderMap_get(init.headers, "Authorization"), "Bearer SECRET"), 0)
    assert_eq(init.timeout_ms, 1500)
    assert_eq(init.redirect, REDIRECT_MANUAL)
}

fn main() {
    test_headers_map()
    test_url_encode_decode()
    test_form_and_params()
    test_json_parse_full()
    test_abort_and_cookies()
    test_blob_array_buffer()
    test_request_init_headers()
    print("http fetch apis ok")
}
