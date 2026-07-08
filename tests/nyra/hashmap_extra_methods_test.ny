import "stdlib/testing.ny"
import "stdlib/map.ny"

test fn test_hashmap_extra_methods() {
    let mut m = HashMap_str_i32_new().insert("a", 1).insert("b", 2)
    assert_eq(m.len(), 2)
    assert_eq(m.contains("a"), 1)
    let vals = m.values()
    assert_eq(vals.len(), 2)
    m = m.clear()
    assert_eq(m.len(), 0)
}
