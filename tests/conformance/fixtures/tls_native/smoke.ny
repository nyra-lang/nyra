import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-N01 - OS native TLS client links and reports available.
test fn conf_tls_n01_available() {
    assert_eq(tls_available(), 1)
    assert_bool(tls_ready())
}

// CONF-TLS-N02 - live HTTPS via native backend (soft-skip on network filters).
test fn conf_tls_n02_https_example_com() {
    assert_bool(tls_ready())
    let body = get("https://example.com/")
    if strstr_pos(body, "{\"error\":") == 0 {
        print(strcat("CONF-TLS-N02 soft-skip: ", body))
        return
    }
    if strlen(body) == 0 {
        print("CONF-TLS-N02 soft-skip: empty body")
        return
    }
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-N02: body still looks chunked (undecoded)")
    }
    assert_bool(strlen(body) > 10)
}
