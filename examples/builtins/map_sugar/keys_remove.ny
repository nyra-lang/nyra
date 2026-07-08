fn main() {
    let mut m = HashMap_str_str_new().insert("a", "1").insert("b", "2")
    print(m.contains("a"))
    m = m.remove("a")
    print(m.contains("a"))
    print(m.get("b"))
}
