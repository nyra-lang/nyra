fn main() -> void {
    let m = obj().insert("a", "1").insert("b", "two")
    print(jstr(m, "a"))
    print(jraw(m, "b"))
    print(jstringify(m))
}
