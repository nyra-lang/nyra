// Smoke test for networking limitations fixes (v1.15.0)

fn http_handler(slot, ctx) {
    if slot == 1 {
        return response_ok_json("{\"ok\":true}")
    }
    if ctx.method == METHOD_GET {
        return response_text(STATUS_OK, ctx.path)
    }
    return response_not_found()
}

fn test_spawn_hub() {
    let inbox = channel_new()
    spawn {
        let hub = TcpHub_new(8)
        let fd = channel_recv(inbox)
        let _ = hub.add(fd)
    }
    channel_send(inbox, -1)
    sleep_ms(50)
}

fn test_pool_https() {
    if !tls_ready() {
        print("pool https skip (no tls)")
        return
    }
    let pool = HttpPool_new()
    let r = HttpPool_get(pool, "https://example.com/")
    if r.resp.status > 0 {
        print("pool https ok")
    } else {
        print("pool https unreachable")
    }
}

fn test_ttl_cache() {
    let cache = TtlCache_new(5000, "/tmp/nyra-cache-test", 0)
    let c2 = TtlCache_put(cache, "k", "v")
    if TtlCache_has(c2, "k") == 1 {
        let v = TtlCache_get(c2, "k")
        if strcmp(v, "v") == 0 {
            print("ttl cache ok")
        }
    }
}

fn test_map_insert_drop() {
    let m = HashMap_str_str_new()
    let m2 = m.insert("a", "1")
    let m3 = m2.insert("b", "2")
    if m3.contains("a") == 1 && m3.contains("b") == 1 {
        print("map insert drop ok")
    }
}

fn test_handler_infer_ok() {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/health", 1)
    serve_handlers("127.0.0.1", 19999, 0, router, http_handler)
}

fn main() {
    test_spawn_hub()
    print("spawn+hub ok")
    test_map_insert_drop()
    test_ttl_cache()
    test_pool_https()
    print("gaps smoke done")
}
