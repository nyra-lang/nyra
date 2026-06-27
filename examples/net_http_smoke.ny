// HTTP router slot smoke — stdlib net/http only (compile-check).
import "stdlib/net/http/mod.ny"

fn main() {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/health", 0)
    let ctx = RequestContext_from_raw("GET /health HTTP/1.1\r\n\r\n")
    let slot = HttpRouter_match_slot(router, ctx)
    if slot != 0 {
        print("slot mismatch")
        return
    }
    print("net http smoke ok")
}
