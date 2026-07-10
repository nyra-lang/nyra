// [contrib-dev:hashmap_or_insert:map]
import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_or_insert() {
    let m = HashMap_str_i32_new()
    assert_eq(m.or_insert("k", 42), 42)
    assert_eq(m.or_insert("k", 99), 42)
}
// [/contrib-dev:hashmap_or_insert:map]
