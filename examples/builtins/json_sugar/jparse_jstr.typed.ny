fn main() -> void {
    let o = jparse("{\"name\":\"Nyra\",\"n\":7,\"ok\":true}")
    print(jstr(o, "name"))
    print(jnum(o, "n"))
    print(jbool(o, "ok"))
}
