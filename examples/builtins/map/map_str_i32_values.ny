import "stdlib/map.ny"

fn main() {
    let m = HashMap_str_i32_new().insert("a", 1).insert("b", 2)
    print(m.values().len())
}
