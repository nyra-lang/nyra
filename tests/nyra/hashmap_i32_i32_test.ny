// [contrib-dev:hashmap_i32_i32:map]
import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_i32_i32() {
    let m = HashMap_i32_i32_new()
    let _ = m.insert(1, 42)
    assert_eq(m.get(1), 42)
}
// [/contrib-dev:hashmap_i32_i32:map]
