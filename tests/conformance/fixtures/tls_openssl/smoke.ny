import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-O01 - OpenSSL backend reports availability or a descriptive last_error.
test fn conf_tls_o01_available_or_error() {
    if tls_available() == 0 {
        let err = tls_last_error()
        assert_bool(strlen(err) > 0)
        return
    }
    assert_eq(tls_available(), 1)
    assert_bool(tls_ready())
}

// CONF-TLS-O02 - refused localhost port returns structured JSON error when OpenSSL is ready.
test fn conf_tls_o02_connect_refused_json_error() {
    if !tls_ready() {
        let err = tls_last_error()
        assert_bool(strlen(err) > 0)
        return
    }
    let body = get("https://127.0.0.1:1/")
    assert_bool(strstr_pos(body, "{\"error\":") == 0)
    assert_bool(strlen(body) > 12)
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-O02: body still looks chunked (undecoded)")
    }
}
