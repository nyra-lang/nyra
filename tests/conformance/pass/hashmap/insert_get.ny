import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn conf_hashmap_insert_get() {
    let mut map = HashMap_str_i32_new()
    map = map.insert("a", 10)
    map = map.insert("b", 20)
    assert_eq(map.get("a"), 10)
    assert_eq(map.get("b"), 20)
}
