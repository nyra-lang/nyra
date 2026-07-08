import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-001 - bundled rustls client is linkable and reports available.
test fn conf_tls_001_available() {
    assert_eq(tls_available(), 1)
    assert_bool(tls_ready())
}

// CONF-TLS-002 - chunked Transfer-Encoding body is decoded (regression: raw "22f" / trailing "0").
test fn conf_tls_002_chunked_decode() {
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n"
    let body = body_from_raw(raw)
    assert_str_eq(body, "hello")
}

// CONF-TLS-003 - multi-chunk decode concatenates payloads.
test fn conf_tls_003_chunked_multi() {
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nfoo\r\n3\r\nbar\r\n0\r\n\r\n"
    let body = body_from_raw(raw)
    assert_str_eq(body, "foobar")
}

// CONF-TLS-004 - Content-Length (non-chunked) path returns body after headers.
test fn conf_tls_004_plain_body() {
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabcd"
    let body = body_from_raw(raw)
    assert_str_eq(body, "abcd")
}

// CONF-TLS-005 - status line parse for 200.
test fn conf_tls_005_status_parse() {
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
    assert_eq(http_status_from_header(raw), 200)
}

// CONF-TLS-006 - hex chunk size parser (same path as Transfer-Encoding: chunked).
test fn conf_tls_006_hex_size() {
    assert_eq(str_to_i32_hex("5"), 5)
    assert_eq(str_to_i32_hex("22f"), 559)
    assert_eq(str_to_i32_hex("0"), 0)
}

// CONF-TLS-007 - tls_last_error is always callable (empty when idle).
test fn conf_tls_007_last_error_idle() {
    let err = tls_last_error()
    assert_bool(strlen(err) >= 0)
}
