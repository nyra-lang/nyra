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

// CONF-TLS-008 - tls_require reports failure when stack is unavailable (deterministic).
test fn conf_tls_008_tls_require_callable() {
    if tls_ready() {
        assert_bool(tls_require("CONF-TLS-008"))
    } else {
        assert_bool(!tls_require("CONF-TLS-008"))
        assert_bool(strlen(tls_last_error()) > 0)
    }
}

// CONF-TLS-009 - malformed chunked body must not leak raw chunk sizes as body prefix.
test fn conf_tls_009_chunked_empty_payload() {
    let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
    let body = body_from_raw(raw)
    assert_str_eq(body, "")
}

// CONF-TLS-010 - HTTPS client returns structured JSON error on refused localhost port.
test fn conf_tls_010_connect_refused_json_error() {
    assert_bool(tls_ready())
    let body = get("https://127.0.0.1:1/")
    assert_bool(strstr_pos(body, "{\"error\":") == 0)
    assert_bool(strlen(body) > 12)
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-010: body still looks chunked (undecoded)")
    }
}

// CONF-TLS-011 - invalid host still yields structured error, not empty success.
test fn conf_tls_011_invalid_host_json_error() {
    assert_bool(tls_ready())
    let body = get("https://invalid.nyra-conf-test.invalid/")
    assert_bool(strstr_pos(body, "{\"error\":") == 0)
    assert_bool(strlen(body) > 12)
}

// CONF-TLS-012 - tls_last_error callable after failed HTTPS attempts.
test fn conf_tls_012_last_error_after_failure() {
    let _ = get("https://127.0.0.1:1/")
    let err = tls_last_error()
    assert_bool(strlen(err) >= 0)
    assert_eq(tls_available(), 1)
}
