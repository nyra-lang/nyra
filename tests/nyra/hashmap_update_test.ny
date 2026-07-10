// [contrib-dev:hashmap_update:map]
import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_update() {
    let m = HashMap_str_i32_new().insert("k", 10)
    let _ = m.insert("k", 20)
    assert_eq(m.get("k"), 20)
}
// [/contrib-dev:hashmap_update:map]
