import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-010 - live HTTPS GET against example.com (requires network + rustls).
// Soft-skip on transport/handshake failures (CI network filters); fail on undecoded chunked body.
test fn conf_tls_010_https_example_com() {
    assert_bool(tls_ready())
    let body = get("https://example.com/")
    // get() returns {"error":"..."} JSON when transport/TLS fails.
    if strstr_pos(body, "{\"error\":") == 0 {
        print(strcat("CONF-TLS-010 soft-skip: ", body))
        return
    }
    if strlen(body) == 0 {
        print("CONF-TLS-010 soft-skip: empty body")
        return
    }
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-010: body still looks chunked (undecoded)")
    }
    assert_bool(strlen(body) > 10)
}

// CONF-TLS-011 - tls_last_error is callable after HTTPS attempts.
test fn conf_tls_011_last_error_callable() {
    let _ = tls_last_error()
    assert_eq(tls_available(), 1)
}
