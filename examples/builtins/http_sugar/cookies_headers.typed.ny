fn main() -> void {
    let jar = cookies()
    let j2 = CookieJar_set(jar, "sid", "abc")
    print(CookieJar_get(j2, "sid"))
    print(CookieJar_header(j2))
    let h = headers().insert("X-Test", "1")
    print(h.get("X-Test"))
}
