import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-N01 - OS native TLS client links and reports available.
test fn conf_tls_n01_available() {
    assert_eq(tls_available(), 1)
    assert_bool(tls_ready())
}

// CONF-TLS-N02 - refused localhost port returns structured JSON error (no external network).
test fn conf_tls_n02_connect_refused_json_error() {
    assert_bool(tls_ready())
    let body = get("https://127.0.0.1:1/")
    assert_bool(strstr_pos(body, "{\"error\":") == 0)
    assert_bool(strlen(body) > 12)
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-N02: body still looks chunked (undecoded)")
    }
}
