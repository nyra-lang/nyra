// Advanced networking stdlib — zero-types smoke.

fn test_router_slots() {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/x", 7)
    let ctx = RequestContext_from_raw("GET /x HTTP/1.1\r\n\r\n")
    let slot = HttpRouter_match_slot(router, ctx)
    if slot != 7 {
        print("HttpRouter_match_slot failed")
        return 1
    }
    return 0
}

fn test_ping_auto() {
    let ms = ping_auto("127.0.0.1", 1, 500)
    if ms < 0 {
        print("ping_auto unreachable ok")
        return 0
    }
    print(strcat("ping_auto ms=", i32_to_string(ms)))
    return 0
}

fn test_ping_icmp_code() {
    let icmp = ping_icmp("127.0.0.1", 500)
    if icmp == -2 {
        print("ping_icmp needs root — ok")
        return 0
    }
    if icmp >= 0 {
        print(strcat("icmp ms=", i32_to_string(icmp)))
        return 0
    }
    return 0
}

fn test_http_pool_types() {
    let pool = HttpPool_new()
    let got = HttpPool_get(pool, "http://127.0.0.1:9/")
    if got.resp.status == 0 {
        print("pool request failed ok")
    }
    return 0
}

fn test_channel_str() {
    let ch = Channel_str_new()
    let ch = ch.send("hello")
    let v = ch.recv()
    if strcmp(v, "hello") != 0 {
        print("channel_str failed")
        return 1
    }
    return 0
}

fn test_tcp_hub() {
    let hub = TcpHub_new(4)
    if hub.add(-1) == 0 {
        print("hub should reject -1")
        return 1
    }
    return 0
}

fn main() {
    if test_router_slots() != 0 { return 1 }
    if test_ping_auto() != 0 { return 1 }
    if test_ping_icmp_code() != 0 { return 1 }
    if test_http_pool_types() != 0 { return 1 }
    if test_channel_str() != 0 { return 1 }
    if test_tcp_hub() != 0 { return 1 }
    print("net advanced ok")
    return 0
}
