import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_get_or() {
    let m = HashMap_str_i32_new().insert("a", 1)
    assert_eq(m.get_or("a", 99), 1)
    assert_eq(m.get_or("missing", 99), 99)
}
