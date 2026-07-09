// [contrib-dev:hashmap_i32_get_or:map]
import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_i32_get_or() {
    let m = HashMap_i32_i32_new()
    assert_eq(m.get_or(1, 99), 99)
    let _ = m.insert(1, 42)
    assert_eq(m.get_or_insert(1, 0), 42)
}
// [/contrib-dev:hashmap_i32_get_or:map]
