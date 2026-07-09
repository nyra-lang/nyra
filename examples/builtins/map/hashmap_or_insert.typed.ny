// [contrib-dev:hashmap_or_insert:map]
import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_str_i32_new()
    print(m.or_insert("k", 42))
    print(m.or_insert("k", 99))
}
// [/contrib-dev:hashmap_or_insert:map]
