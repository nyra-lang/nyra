import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/json/mod.ny"

fn test_fluent_chain() {
    let init = req()
        .verb("GET")
        .header("Accept", "application/json")
        .authorization("Bearer t")
        .timeout(1500)
        .redirect(REDIRECT_MANUAL)
    assert_eq(init.timeout_ms, 1500)
    assert_eq(init.redirect, REDIRECT_MANUAL)
    assert_eq(init.method, METHOD_GET)
    assert_eq(strcmp(HeaderMap_get(init.headers, "Authorization"), "Bearer t"), 0)
    assert_eq(strcmp(HeaderMap_get(init.headers, "Accept"), "application/json"), 0)
    let postish = req().verb("post").json("{\"a\":1}")
    assert_eq(postish.method, METHOD_POST)
    assert_eq(strcmp(postish.content_type, "application/json"), 0)
}

fn test_method_from_name() {
    assert_eq(method_from_name("get"), METHOD_GET)
    assert_eq(method_from_name("POST"), METHOD_POST)
    assert_eq(method_from_name("Patch"), METHOD_PATCH)
}

fn test_form_chain() {
    let body = form().append("a", "1").append("b", "two words").urlencoded()
    assert_eq(1, if strstr_pos(body, "a=1") >= 0 { 1 } else { 0 })
    assert_eq(1, if strstr_pos(body, "b=two%20words") >= 0 { 1 } else { 0 })
    let qs = params().set("q", "hello world").to_string()
    assert_eq(strcmp(qs, "q=hello%20world"), 0)
}

fn test_response_methods() {
    let resp = HttpResponse_new(200, "{\"name\":\"Nyra\",\"n\":7}", "application/json")
    assert_eq(resp.is_ok(), 1)
    assert_eq(strcmp(resp.text(), "{\"name\":\"Nyra\",\"n\":7}"), 0)
    assert_eq(strcmp(jstr(resp.json(), "name"), "Nyra"), 0)
    assert_eq(jnum(resp.json(), "n"), 7)
    let bad = HttpResponse_new(404, "", "text/plain")
    assert_eq(bad.is_ok(), 0)
}

fn test_json_nested() {
    let obj = JSON_parse_object("{\"user\":{\"id\":1,\"name\":\"Ada\"}}")
    let user = jobj(obj, "user")
    assert_eq(strcmp(jstr(user, "name"), "Ada"), 0)
    assert_eq(jnum(user, "id"), 1)
}

fn test_cookies_abort_fluent() {
    let jar = cookies().set("sid", "abc").set("theme", "dark")
    assert_eq(1, if strstr_pos(jar.header(), "sid=abc") >= 0 { 1 } else { 0 })
    let ctrl = AbortController_new().abort()
    assert_eq(ctrl.signal().aborted, 1)
}

fn main() {
    test_fluent_chain()
    test_method_from_name()
    test_form_chain()
    test_response_methods()
    test_json_nested()
    test_cookies_abort_fluent()
    print("http sugar ok")
}
