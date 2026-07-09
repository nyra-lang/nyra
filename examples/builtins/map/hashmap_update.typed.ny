// [contrib-dev:hashmap_update:map]
import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_str_i32_new().insert("k", 10)
    let _ = m.insert("k", 20)
    print(m.get("k"))
}
// [/contrib-dev:hashmap_update:map]
