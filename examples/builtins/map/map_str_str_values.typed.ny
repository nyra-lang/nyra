import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_str_str_new().insert("a", "1").insert("b", "2")
    print(m.values().len())
}
