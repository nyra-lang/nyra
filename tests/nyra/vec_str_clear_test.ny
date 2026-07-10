// [contrib-dev:vec_str_clear:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_clear() {
    let v = strs().push("a").clear()
    assert_eq(v.len(), 0)
}
// [/contrib-dev:vec_str_clear:vec_str]
