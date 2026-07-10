// [contrib-dev:hashmap_extra_methods:map]
import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_str_i32_new()
    print(m.is_empty())
    let _ = m.insert("k", 1)
    print(m.is_empty())
}
// [/contrib-dev:hashmap_extra_methods:map]
