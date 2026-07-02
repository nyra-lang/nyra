import "stdlib/map.ny"

fn main() -> void {
    let mut m = HashMap_str_i32_new()
    m = m.insert("score", 100)
    print(m.get("score"))
    print(m.contains("score"))
}
