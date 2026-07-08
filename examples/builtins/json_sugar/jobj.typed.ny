fn main() -> void {
    let o = jparse("{\"user\":{\"id\":1}}")
    let u = jobj(o, "user")
    print(jnum(u, "id"))
}
