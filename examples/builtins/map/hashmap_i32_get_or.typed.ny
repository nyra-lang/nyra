// [contrib-dev:hashmap_i32_get_or:map]
import "stdlib/map.ny"

fn main() -> void {
    let m = HashMap_i32_i32_new()
    print(m.get_or(1, 99))
}
// [/contrib-dev:hashmap_i32_get_or:map]
