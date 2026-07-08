import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/json/mod.ny"

fn test_fluent_chain_typed() -> void {
    let init: RequestInit = req().timeout(1200).header("X-A", "1")
    assert_eq(init.timeout_ms, 1200)
    assert_eq(strcmp(HeaderMap_get(init.headers, "X-A"), "1"), 0)
}

fn test_json_helpers_typed() -> void {
    let resp: HttpResponse = HttpResponse_new(201, "{\"ok\":true,\"n\":3}", "application/json")
    assert_eq(resp.is_ok(), 1)
    assert_eq(jnum(resp.json(), "n"), 3)
}

fn main() -> void {
    test_fluent_chain_typed()
    test_json_helpers_typed()
    print("http sugar typed ok")
}
