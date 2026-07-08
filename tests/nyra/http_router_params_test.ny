// HttpRouter parametric paths — :id style matching.

fn test_exact_still_works() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/health", 1)
    let ctx = RequestContext_from_raw("GET /health HTTP/1.1\r\n\r\n")
    let m = HttpRouter_match(router, ctx)
    if m.slot != 1 {
        print("exact slot failed")
        return 1
    }
    return 0
}

fn test_param_slot() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_PATCH, "/api/users/edit-user/:id", 10)
    let ctx = RequestContext_from_raw("PATCH /api/users/edit-user/abc123 HTTP/1.1\r\n\r\n")
    let m = HttpRouter_match(router, ctx)
    if m.slot != 10 {
        print("param slot failed")
        return 1
    }
    if strcmp(m.params.get("id"), "abc123") != 0 {
        print("param id failed")
        return 1
    }
    return 0
}

fn test_param_via_context_helper() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_DELETE, "/api/reviews/:id", 20)
    let ctx = RequestContext_from_raw("DELETE /api/reviews/rev9 HTTP/1.1\r\n\r\n")
    let m = HttpRouter_match(router, ctx)
    let ctx2 = RequestContext_with_params(ctx, m.params)
    if strcmp(RequestContext_param(ctx2, "id"), "rev9") != 0 {
        print("RequestContext_param failed")
        return 1
    }
    return 0
}

fn test_multi_params() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/t/:teacher/s/:stage", 3)
    let ctx = RequestContext_from_raw("GET /t/Ali/s/Beginner HTTP/1.1\r\n\r\n")
    let m = HttpRouter_match(router, ctx)
    if m.slot != 3 {
        print("multi slot failed")
        return 1
    }
    if strcmp(m.params.get("teacher"), "Ali") != 0 {
        print("teacher param failed")
        return 1
    }
    if strcmp(m.params.get("stage"), "Beginner") != 0 {
        print("stage param failed")
        return 1
    }
    return 0
}

fn test_prefer_exact_over_param() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/api/users/:id", 1)
    router = HttpRouter_register_slot(router, METHOD_GET, "/api/users/blackList", 2)
    let ctx = RequestContext_from_raw("GET /api/users/blackList HTTP/1.1\r\n\r\n")
    let m = HttpRouter_match(router, ctx)
    if m.slot != 2 {
        print("exact should win over param")
        return 1
    }
    return 0
}

fn test_match_slot_compat() -> i32 {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/items/:id", 9)
    let ctx = RequestContext_from_raw("GET /items/42 HTTP/1.1\r\n\r\n")
    let slot = HttpRouter_match_slot(router, ctx)
    if slot != 9 {
        print("match_slot compat failed")
        return 1
    }
    return 0
}

fn main() {
    if test_exact_still_works() != 0 { return 1 }
    if test_param_slot() != 0 { return 1 }
    if test_param_via_context_helper() != 0 { return 1 }
    if test_multi_params() != 0 { return 1 }
    if test_prefer_exact_over_param() != 0 { return 1 }
    if test_match_slot_compat() != 0 { return 1 }
    print("http router params ok")
    return 0
}
