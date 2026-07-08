import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// CONF-TLS-O01 - OpenSSL client links when headers/libs are present.
// Soft-pass (skip assertions) when OpenSSL is not installed (tls_available == 0).
test fn conf_tls_o01_available_or_skip() {
    if tls_available() == 0 {
        print(strcat("CONF-TLS-O01 soft-skip: ", tls_last_error()))
        return
    }
    assert_eq(tls_available(), 1)
    assert_bool(tls_ready())
}

// CONF-TLS-O02 - live HTTPS via openssl backend when available.
test fn conf_tls_o02_https_example_com() {
    if !tls_ready() {
        print(strcat("CONF-TLS-O02 soft-skip: ", tls_last_error()))
        return
    }
    let body = get("https://example.com/")
    if strstr_pos(body, "{\"error\":") == 0 {
        print(strcat("CONF-TLS-O02 soft-skip: ", body))
        return
    }
    if strlen(body) == 0 {
        print("CONF-TLS-O02 soft-skip: empty body")
        return
    }
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-O02: body still looks chunked (undecoded)")
    }
    assert_bool(strlen(body) > 10)
}
