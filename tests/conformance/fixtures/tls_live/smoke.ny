import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/tls.ny"

// Optional live HTTPS gate — run via NYRA_CONF_TLS_LIVE=1 in conformance-tests.sh.
// Hard-fails when enabled: no soft-skip on transport errors.

test fn conf_tls_live_001_https_example_com() {
    assert_bool(tls_ready())
    let body = get("https://example.com/")
    if strstr_pos(body, "{\"error\":") == 0 {
        test_fail(strcat("CONF-TLS-LIVE-001 transport error: ", body))
    }
    if strlen(body) == 0 {
        test_fail("CONF-TLS-LIVE-001 empty body")
    }
    if strcmp(substring(body, 0, 3), "22f") == 0 {
        test_fail("CONF-TLS-LIVE-001: body still looks chunked (undecoded)")
    }
    assert_bool(strlen(body) > 10)
}

test fn conf_tls_live_002_html_marker() {
    let body = get("https://example.com/")
    if strstr_pos(body, "Example Domain") < 0 && strstr_pos(body, "example") < 0 {
        test_fail("CONF-TLS-LIVE-002: missing expected example.com content")
    }
}
