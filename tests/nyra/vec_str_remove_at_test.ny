// [contrib-dev:vec_str_remove_at:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_remove_at() {
    let v = strs().push("a").push("b")
    assert_str_eq(v.remove_at(0), "a")
    assert_eq(v.len(), 1)
}
// [/contrib-dev:vec_str_remove_at:vec_str]
