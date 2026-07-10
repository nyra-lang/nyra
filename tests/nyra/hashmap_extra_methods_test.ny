// [contrib-dev:hashmap_extra_methods:map]
import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_extra_methods() {
    let m = HashMap_str_i32_new()
    assert_eq(m.is_empty(), 1)
    let _ = m.insert("k", 1)
    assert_eq(m.is_empty(), 0)
}
// [/contrib-dev:hashmap_extra_methods:map]
