// [contrib-dev:hashmap_i32_i32:map]
import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_i32_i32_new()
    let _ = m.insert(1, 42)
    print(m.get(1))
}
// [/contrib-dev:hashmap_i32_i32:map]
