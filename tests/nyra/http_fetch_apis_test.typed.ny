import "stdlib/testing.ny"
import "stdlib/net/http/mod.ny"
import "stdlib/json/mod.ny"
import "stdlib/encoding/mod.ny"

fn test_headers_map_typed() -> void {
    let mut h: HashMap_str_str = HeaderMap_new()
    h = HeaderMap_set(h, "Authorization", "Bearer tok")
    assert_eq(strcmp(HeaderMap_get(h, "Authorization"), "Bearer tok"), 0)
}

fn test_json_parse_full_typed() -> void {
    let obj: HashMap_str_str = JSON_parse_object("{\"a\":1}")
    assert_eq(strcmp(obj.get("a"), "1"), 0)
    let resp: HttpResponse = HttpResponse_new(200, "{\"a\":2}", "application/json")
    let parsed: HashMap_str_str = HttpResponse_json(resp)
    assert_eq(strcmp(parsed.get("a"), "2"), 0)
}

fn test_form_typed() -> void {
    let mut form: FormData = FormData_new()
    form = FormData_append(form, "k", "v")
    let body: string = FormData_to_urlencoded(form)
    assert_eq(strcmp(body, "k=v"), 0)
}

fn main() -> void {
    test_headers_map_typed()
    test_json_parse_full_typed()
    test_form_typed()
    print("http fetch apis typed ok")
}
