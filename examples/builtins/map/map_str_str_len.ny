import "stdlib/map.ny"

fn main() {
    let m = HashMap_str_str_new().insert("a", "1").insert("b", "2")
    print(m.len())
}
