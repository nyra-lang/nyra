import "../../strings.ny"
import "../../map.ny"
import "../../vec_str.ny"
import "request.ny"
import "types.ny"

struct HttpRouter {
    routes: HashMap_str_str
    slots: HashMap_str_i32
}

struct RouteMatch {
    slot: i32
    tag: string
    params: HashMap_str_str
}

struct PathMatch {
    ok: i32
    params: HashMap_str_str
}

fn HttpRouter_new() -> HttpRouter {
    return HttpRouter {
        routes: HashMap_str_str_new(),
        slots: HashMap_str_i32_new(),
    }
}

fn RouteMatch_miss() -> RouteMatch {
    return RouteMatch {
        slot: -1,
        tag: "",
        params: HashMap_str_str_new(),
    }
}

fn PathMatch_fail() -> PathMatch {
    return PathMatch { ok: 0, params: HashMap_str_str_new() }
}

fn PathMatch_ok(params: HashMap_str_str) -> PathMatch {
    return PathMatch { ok: 1, params: params }
}

fn HttpRouter_register(router: HttpRouter, method: i32, path: string, tag: string) -> HttpRouter {
    let key = route_key(method, path)
    let routes = router.routes.insert(key, tag)
    return HttpRouter { routes: routes, slots: router.slots }
}

fn HttpRouter_register_slot(router: HttpRouter, method: i32, path: string, slot: i32) -> HttpRouter {
    let key = route_key(method, path)
    let slots = router.slots.insert(key, slot)
    return HttpRouter { routes: router.routes, slots: slots }
}

// Split "/a/b/:id" into non-empty segments.
fn route_segments(path: string) -> StrVec {
    let mut out = strs()
    let n = strlen(path)
    let mut i = 0
    let mut cur = ""
    while i < n {
        let c = char_at(path, i)
        if c == 47 {
            if strlen(cur) > 0 {
                out = out.push(cur)
                cur = ""
            }
        } else {
            cur = strcat(cur, char_from_code(c))
        }
        i = i + 1
    }
    if strlen(cur) > 0 {
        out = out.push(cur)
    }
    return out
}

fn is_param_segment(seg: string) -> i32 {
    if strlen(seg) > 1 && char_at(seg, 0) == 58 {
        return 1
    }
    return 0
}

fn param_name(seg: string) -> string {
    return substring(seg, 1, strlen(seg) - 1)
}

fn pattern_has_params(pattern: string) -> i32 {
    if strstr_pos(pattern, "/:") >= 0 {
        return 1
    }
    if str_starts_with(pattern, ":") == 1 {
        return 1
    }
    return 0
}

// Match pattern like "/users/:id" against "/users/42".
fn match_path_pattern(pattern: string, path: string) -> PathMatch {
    let pats = route_segments(pattern)
    let segs = route_segments(path)
    if pats.len() != segs.len() {
        return PathMatch_fail()
    }
    let mut params = HashMap_str_str_new()
    let n = pats.len()
    let mut i = 0
    while i < n {
        let pat = pats.get(i)
        let seg = segs.get(i)
        if is_param_segment(pat) == 1 {
            params = params.insert(param_name(pat), seg)
        } else {
            if strcmp(pat, seg) != 0 {
                return PathMatch_fail()
            }
        }
        i = i + 1
    }
    return PathMatch_ok(params)
}

fn route_key_path(key: string) -> string {
    let colon = strstr_pos(key, ":")
    if colon < 0 {
        return key
    }
    return substring(key, colon + 1, strlen(key) - colon - 1)
}

fn route_key_method_name(key: string) -> string {
    let colon = strstr_pos(key, ":")
    if colon < 0 {
        return ""
    }
    return substring(key, 0, colon)
}

fn HttpRouter_lookup(router: HttpRouter, ctx: RequestContext) -> string {
    let key = route_key(ctx.method, ctx.path)
    if router.routes.contains(key) == 1 {
        return router.routes.get(key)
    }
    let method_s = method_name(ctx.method)
    let keys = router.routes.keys()
    let n = keys.len()
    let mut i = 0
    while i < n {
        let rk = keys.get(i)
        if strcmp(route_key_method_name(rk), method_s) == 0 {
            let pattern = route_key_path(rk)
            if pattern_has_params(pattern) == 1 {
                let m = match_path_pattern(pattern, ctx.path)
                if m.ok == 1 {
                    return router.routes.get(rk)
                }
            }
        }
        i = i + 1
    }
    return ""
}

fn HttpRouter_match_slot(router: HttpRouter, ctx: RequestContext) -> i32 {
    return HttpRouter_match(router, ctx).slot
}

// Prefer exact method+path; then first parametric pattern for that method.
fn HttpRouter_match(router: HttpRouter, ctx: RequestContext) -> RouteMatch {
    let key = route_key(ctx.method, ctx.path)
    if router.slots.contains(key) == 1 {
        return RouteMatch {
            slot: router.slots.get(key),
            tag: "",
            params: HashMap_str_str_new(),
        }
    }
    if router.routes.contains(key) == 1 {
        return RouteMatch {
            slot: -1,
            tag: router.routes.get(key),
            params: HashMap_str_str_new(),
        }
    }

    let method_s = method_name(ctx.method)

    let slot_keys = router.slots.keys()
    let sn = slot_keys.len()
    let mut i = 0
    while i < sn {
        let rk = slot_keys.get(i)
        if strcmp(route_key_method_name(rk), method_s) == 0 {
            let pattern = route_key_path(rk)
            if pattern_has_params(pattern) == 1 {
                let m = match_path_pattern(pattern, ctx.path)
                if m.ok == 1 {
                    return RouteMatch {
                        slot: router.slots.get(rk),
                        tag: "",
                        params: m.params,
                    }
                }
            }
        }
        i = i + 1
    }

    let route_keys = router.routes.keys()
    let rn = route_keys.len()
    i = 0
    while i < rn {
        let rk = route_keys.get(i)
        if strcmp(route_key_method_name(rk), method_s) == 0 {
            let pattern = route_key_path(rk)
            if pattern_has_params(pattern) == 1 {
                let m = match_path_pattern(pattern, ctx.path)
                if m.ok == 1 {
                    return RouteMatch {
                        slot: -1,
                        tag: router.routes.get(rk),
                        params: m.params,
                    }
                }
            }
        }
        i = i + 1
    }

    return RouteMatch_miss()
}

fn HttpRouter_has(router: HttpRouter, method: i32, path: string) -> i32 {
    let key = route_key(method, path)
    if router.routes.contains(key) == 1 {
        return 1
    }
    if router.slots.contains(key) == 1 {
        return 1
    }
    let ctx = RequestContext {
        method: method,
        path: path,
        body: "",
        query: "",
        raw: "",
        params: HashMap_str_str_new(),
    }
    let m = HttpRouter_match(router, ctx)
    if m.slot >= 0 {
        return 1
    }
    if strlen(m.tag) > 0 {
        return 1
    }
    return 0
}

fn RequestContext_param(ctx: RequestContext, name: string) -> string {
    if ctx.params.contains(name) == 0 {
        return ""
    }
    return ctx.params.get(name)
}

fn RequestContext_with_params(ctx: RequestContext, params: HashMap_str_str) -> RequestContext {
    return RequestContext {
        method: ctx.method,
        path: ctx.path,
        body: ctx.body,
        query: ctx.query,
        raw: ctx.raw,
        params: params,
    }
}
